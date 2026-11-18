//! Inspect the public XLSX output independently of the writer's format helpers.
use sheaf_core::cellformat::{CellFormat, FormatTarget, HAlign, LabelKind};
use sheaf_core::color::Color;
use sheaf_core::datetime::DatePattern;
use sheaf_core::format::NumberFormat;
use sheaf_core::{Column, ColumnData, ColumnId, Designation, Worksheet};
use sheaf_io::excel::{ExcelExportOptions, write_xlsx};
use std::io::Read;
use std::path::PathBuf;

struct Output(PathBuf);
impl Output {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("sheaf-xlsx-formats-{tag}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }

    fn export(&self, sheet: &Worksheet, options: &ExcelExportOptions) -> (String, String, Vec<String>) {
        let out = write_xlsx(&[sheet], &self.0.join("styles.xlsx"), options).unwrap();
        let mut zip = zip::ZipArchive::new(std::fs::File::open(out.path).unwrap()).unwrap();
        let mut part = |name| {
            let mut text = String::new();
            zip.by_name(name).unwrap().read_to_string(&mut text).unwrap();
            text
        };
        (part("xl/worksheets/sheet1.xml"), part("xl/styles.xml"), out.warnings)
    }
}
impl Drop for Output {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

fn nodes<'a>(xml: &'a str, tag: &str) -> Vec<&'a str> {
    let start = format!("<{tag}");
    let end = format!("</{tag}>");
    xml.match_indices(&start)
        .filter_map(|(i, _)| {
            let rest = &xml[i..];
            if !matches!(rest.as_bytes().get(start.len()), Some(b' ' | b'>' | b'/')) {
                return None;
            }
            let opening = rest.find('>').unwrap() + 1;
            let count = if rest[..opening].ends_with("/>") { opening } else { rest.find(&end).unwrap() + end.len() };
            Some(&rest[..count])
        })
        .collect()
}
fn attr<'a>(node: &'a str, name: &str) -> Option<&'a str> {
    let opening = node.split('>').next().unwrap();
    let (_, rest) = opening.split_once(&format!(" {name}=\""))?;
    Some(rest.split('"').next().unwrap())
}
fn cell<'a>(xml: &'a str, address: &str) -> &'a str {
    nodes(xml, "c")
        .into_iter()
        .find(|c| attr(c, "r") == Some(address))
        .unwrap_or_else(|| panic!("missing {address}: {xml}"))
}
fn style<'a>(xml: &str, styles: &'a str, address: &str) -> &'a str {
    let id = attr(cell(xml, address), "s").unwrap_or("0").parse::<usize>().unwrap();
    nodes(nodes(styles, "cellXfs")[0], "xf")[id]
}
fn font<'a>(xf: &str, styles: &'a str) -> &'a str {
    nodes(nodes(styles, "fonts")[0], "font")[attr(xf, "fontId").unwrap().parse::<usize>().unwrap()]
}
fn fill<'a>(xf: &str, styles: &'a str) -> &'a str {
    nodes(nodes(styles, "fills")[0], "fill")[attr(xf, "fillId").unwrap().parse::<usize>().unwrap()]
}
fn column(id: u64, values: Vec<f64>) -> Column {
    Column::new(ColumnId(id), format!("C{id}"), Designation::Y, ColumnData::numeric(values))
}
fn plain() -> ExcelExportOptions {
    ExcelExportOptions { long_names: false, units: false, comments: false, short_names: false }
}

#[test]
fn worksheet_label_default_families_preserve_sizes_manual_faces_and_data_types() {
    let mut a = column(1, vec![1.23456789012345, f64::NAN, 3.0]);
    a.long_name = "Potential".into();
    a.units = "V".into();
    a.comments = "note".into();
    let mut s = Worksheet { columns: vec![a, column(2, vec![101.0])], ..Worksheet::new("Families") };
    s.set_label_font_family(&LabelKind::LongName, "Times New Roman");
    s.set_label_font_family(&LabelKind::Units, "Courier New");
    s.set_label_font_family(&LabelKind::Comments, "Arial");
    s.set_label_font_size(&LabelKind::LongName, 18.0);
    s.apply_cell_format(
        FormatTarget::label_rows(vec![ColumnId(1)], vec![LabelKind::LongName]),
        CellFormat { font: Some("Courier New".into()), size_pt: Some(12.0), bold: Some(true), ..Default::default() },
    );
    let before = s.clone();
    let (xml, styles, warnings) = Output::new("label-families").export(&s, &ExcelExportOptions::default());
    assert_eq!(s, before);
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    for (address, name, size) in [
        ("A1", "Courier New", 12),
        ("B1", "Times New Roman", 18),
        ("A2", "Courier New", 9),
        ("B2", "Courier New", 9),
        ("A3", "Arial", 9),
    ] {
        let f = font(style(&xml, &styles, address), &styles);
        assert!(f.contains(&format!("val=\"{name}\"")) && f.contains(&format!("val=\"{size}\"")), "{address}: {f}");
        assert_eq!(f.contains("<b/>"), address == "A1");
    }
    assert!(xml.contains("<v>1.23456789012345</v>"));
    let data = font(style(&xml, &styles, "B4"), &styles);
    assert!(data.contains("val=\"Arial\"") && data.contains("val=\"9\""));
    assert!(!xml.contains("r=\"A5\""));
    let (_, _, warnings) = Output::new("label-family-omitted").export(&s, &plain());
    assert!(warnings.iter().any(|w| w.contains("label rows omitted")));
}

#[test]
fn worksheet_label_default_sizes_export_independently_without_styling_data_blanks() {
    let mut a = column(1, vec![1.23456789012345, f64::NAN, 3.0]);
    a.long_name = "Potential".into();
    a.units = "V".into();
    a.comments = "note".into();
    let mut b = column(2, vec![101.0]);
    b.long_name = "Future".into();
    let mut s = Worksheet { columns: vec![a], ..Worksheet::new("Labels") };
    s.set_label_font_size(&LabelKind::LongName, 18.0);
    s.set_label_font_size(&LabelKind::Units, 12.0);
    s.set_label_font_size(&LabelKind::Comments, 24.0);
    s.columns.push(b);
    s.apply_cell_format(
        FormatTarget::label_rows(vec![ColumnId(1)], vec![LabelKind::LongName]),
        CellFormat { size_pt: Some(10.0), bold: Some(true), ..Default::default() },
    );
    let before = s.clone();
    let (xml, styles, warnings) = Output::new("label-defaults").export(&s, &ExcelExportOptions::default());
    assert_eq!(s, before);
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    for (address, size) in [("A1", 10), ("B1", 18), ("A2", 12), ("B2", 12), ("A3", 24), ("B3", 24)] {
        let f = font(style(&xml, &styles, address), &styles);
        assert!(f.contains(&format!("val=\"{size}\"")), "{address}: {f}");
        assert!(f.contains("val=\"Arial\""));
        assert_eq!(f.contains("<b/>"), address == "A1");
    }
    assert!(xml.contains("<v>1.23456789012345</v>"));
    assert!(xml.contains("<v>101</v>"));
    for address in ["A5", "B5", "B6"] {
        assert!(!xml.contains(&format!("r=\"{address}\"")), "label defaults must not manufacture styled data blanks");
    }
    let (_, _, warnings) = Output::new("label-defaults-omitted").export(&s, &plain());
    assert!(warnings.iter().any(|w| w.contains("label rows omitted")));
}

#[test]
fn worksheet_default_fonts_reach_values_and_blanks_below_manual_formats() {
    use sheaf_core::sheet_fonts::WorksheetFont;
    let mut a = column(1, vec![1.234, f64::NAN, 3.0]);
    a.format = NumberFormat::Decimal(3);
    a.long_name = "Potential".into();
    let mut s = Worksheet { columns: vec![a, column(2, vec![2.0])], ..Worksheet::new("Default fonts") };
    s.set_data_font(WorksheetFont { family: "Times New Roman".into(), size_pt: 18.0 });
    s.apply_cell_format(
        FormatTarget::cells(vec![ColumnId(1)], 0, Some(0)),
        CellFormat { size_pt: Some(12.0), bold: Some(true), ..Default::default() },
    );
    let before = s.clone();
    let (xml, styles, warnings) =
        Output::new("worksheet-defaults").export(&s, &ExcelExportOptions { long_names: true, ..plain() });
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(warnings[0].contains("future cells use Excel defaults"));
    assert_eq!(s, before);
    let manual = font(style(&xml, &styles, "A2"), &styles);
    assert!(manual.contains("val=\"Times New Roman\"") && manual.contains("val=\"12\"") && manual.contains("<b/>"));
    for address in ["A3", "A4", "B2", "B3", "B4"] {
        let f = font(style(&xml, &styles, address), &styles);
        assert!(f.contains("val=\"Times New Roman\"") && f.contains("val=\"18\""), "{address}: {f}");
        assert!(!f.contains("<b/>"));
    }
    let label = font(style(&xml, &styles, "A1"), &styles);
    assert!(label.contains("val=\"Arial\"") && label.contains("val=\"9\""));
    assert!(styles.contains("formatCode=\"0.000\""));
    assert!(cell(&xml, "A2").contains("<v>1.234</v>"));
    assert!(nodes(&xml, "row").iter().all(|r| attr(r, "r").unwrap().parse::<usize>().unwrap() <= 4));
}

#[test]
fn xlsx_formats_combine_number_codes_fonts_and_rgb_without_mutating_the_sheet() {
    let mut decimal = column(7, vec![1.23456789012345, -2.5]);
    decimal.format = NumberFormat::Decimal(3);
    let mut date = column(19, vec![sheaf_core::datetime::date_value(2026, 11, 16).unwrap()]);
    date.format = NumberFormat::Date(DatePattern::Iso);
    let mut sheet = Worksheet { columns: vec![decimal, date], ..Worksheet::new("S") };
    sheet.apply_cell_format(
        FormatTarget::whole_columns(vec![ColumnId(7), ColumnId(19)]),
        CellFormat {
            font: Some("Times New Roman".into()),
            size_pt: Some(18.5),
            bold: Some(true),
            italic: Some(true),
            underline: Some(true),
            text_color: Some(Color::rgb(17, 34, 51)),
            fill: Some(Color::rgb(32, 128, 224)),
            h_align: Some(HAlign::Center),
        },
    );
    let before = serde_json::to_vec(&sheet).unwrap();
    let (xml, styles, warnings) = Output::new("combined").export(&sheet, &plain());
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(serde_json::to_vec(&sheet).unwrap(), before);
    assert!(cell(&xml, "A1").contains("<v>1.23456789012345</v>"));
    let a = style(&xml, &styles, "A1");
    let b = style(&xml, &styles, "B1");
    assert_ne!(attr(a, "numFmtId"), attr(b, "numFmtId"));
    assert!(styles.contains("formatCode=\"0.000\""));
    assert!(styles.contains("formatCode=\"yyyy-mm-dd\""));
    for xf in [a, b] {
        let f = font(xf, &styles);
        for expected in ["<b/>", "<i/>", "<u/>", "val=\"18.5\"", "val=\"Times New Roman\"", "rgb=\"FF112233\""] {
            assert!(f.contains(expected), "{expected}: {f}");
        }
        assert!(fill(xf, &styles).contains("FF2080E0"));
        assert!(xf.contains("horizontal=\"center\""));
    }
}

#[test]
fn xlsx_formats_resolve_overlaps_resets_and_label_offsets_by_column_id() {
    let mut data = column(42, vec![1.0, 2.0, 3.0]);
    data.long_name = "Potential".into();
    data.units = "V".into();
    let mut sheet = Worksheet { columns: vec![column(9, vec![9.0]), data], ..Worksheet::new("S") };
    sheet.apply_cell_format(
        FormatTarget::whole_columns(vec![ColumnId(42)]),
        CellFormat {
            bold: Some(true),
            fill: Some(Color::rgb(255, 0, 0)),
            h_align: Some(HAlign::Right),
            ..Default::default()
        },
    );
    sheet.apply_cell_format(
        FormatTarget::cells(vec![ColumnId(42)], 1, Some(1)),
        CellFormat {
            bold: Some(false),
            fill: Some(Color::TRANSPARENT),
            h_align: Some(HAlign::Auto),
            italic: Some(true),
            ..Default::default()
        },
    );
    sheet.apply_cell_format(
        FormatTarget::label_rows(vec![ColumnId(42)], vec![LabelKind::Units]),
        CellFormat { underline: Some(true), text_color: Some(Color::rgb(0, 128, 0)), ..Default::default() },
    );
    let (xml, styles, warnings) = Output::new("overlaps").export(&sheet, &ExcelExportOptions::default());
    assert!(warnings.is_empty(), "{warnings:?}");
    assert!(!font(style(&xml, &styles, "B1"), &styles).contains("<b/>"));
    assert!(font(style(&xml, &styles, "B2"), &styles).contains("<u/>"));
    assert!(font(style(&xml, &styles, "B3"), &styles).contains("<b/>"));
    let reset = style(&xml, &styles, "B4");
    assert_eq!(attr(reset, "fillId"), Some("0"));
    assert!(!font(reset, &styles).contains("<b/>"));
    assert!(font(reset, &styles).contains("<i/>"));
    assert!(!reset.contains("horizontal="));
    assert!(font(style(&xml, &styles, "B5"), &styles).contains("<b/>"));
    assert!(!font(style(&xml, &styles, "A3"), &styles).contains("<b/>"));
}

#[test]
fn xlsx_formats_keep_styled_missing_cells_and_do_not_expand_open_ended_ranges() {
    let mut sheet =
        Worksheet { columns: vec![column(1, vec![1.0, f64::NAN, 3.0]), column(2, vec![])], ..Worksheet::new("S") };
    sheet.apply_cell_format(
        FormatTarget::whole_columns(vec![ColumnId(1), ColumnId(2)]),
        CellFormat { fill: Some(Color::rgb(255, 255, 0)), ..Default::default() },
    );
    sheet.apply_cell_format(
        FormatTarget::cells(vec![ColumnId(1)], 900_000, Some(999_999)),
        CellFormat { bold: Some(true), ..Default::default() },
    );
    let (xml, styles, warnings) = Output::new("sparse").export(&sheet, &plain());
    for address in ["A2", "B1", "B2", "B3"] {
        assert!(!cell(&xml, address).contains("<v>"));
        assert!(fill(style(&xml, &styles, address), &styles).contains("FFFFFF00"));
    }
    assert_eq!(nodes(&xml, "c").len(), 6);
    assert!(xml.contains("ref=\"A1:B3\""));
    assert!(warnings.iter().any(|w| w.contains("outside") && w.contains("format")), "{warnings:?}");
}

#[test]
fn xlsx_formats_keep_mixed_text_numeric_types_and_early_date_fallback_styles() {
    use std::collections::BTreeMap;
    let mut mixed = column(1, vec![]);
    mixed.data = ColumnData::mixed(
        vec![1.25, f64::NAN],
        BTreeMap::from([(1, "=2+2".into()), (3, "\u{03bc}A cm\u{2212}2".into())]),
    );
    mixed.format = NumberFormat::Scientific(2);
    let mut early = column(2, vec![sheaf_core::datetime::date_value(1800, 1, 1).unwrap()]);
    early.format = NumberFormat::Date(DatePattern::Iso);
    let mut text = column(3, vec![]);
    text.data = ColumnData::text(vec!["0012".into(), "".into(), "H2O".into()]);
    let mut sheet = Worksheet { columns: vec![mixed, early, text], ..Worksheet::new("S") };
    sheet.apply_cell_format(
        FormatTarget::whole_columns(vec![ColumnId(1), ColumnId(2), ColumnId(3)]),
        CellFormat { italic: Some(true), size_pt: Some(120.0), h_align: Some(HAlign::Left), ..Default::default() },
    );
    let output = Output::new("mixed");
    let (xml, styles, warnings) = output.export(&sheet, &plain());
    let mut zip = zip::ZipArchive::new(std::fs::File::open(output.0.join("styles.xlsx")).unwrap()).unwrap();
    let mut shared = String::new();
    std::io::Read::read_to_string(&mut zip.by_name("xl/sharedStrings.xml").unwrap(), &mut shared).unwrap();
    for text in ["\u{03bc}A cm\u{2212}2", "0012", "=2+2"] {
        assert!(shared.contains(text), "{text}: {shared}");
    }
    assert!(warnings.iter().any(|w| w.contains("before 1900")), "{warnings:?}");
    assert!(cell(&xml, "A1").contains("<v>1.25</v>"));
    assert_ne!(attr(style(&xml, &styles, "A1"), "numFmtId"), Some("0"));
    for address in ["A2", "A4", "C1", "C3"] {
        assert_eq!(attr(cell(&xml, address), "t"), Some("s"), "{address}");
        assert_eq!(attr(style(&xml, &styles, address), "numFmtId"), Some("0"));
    }
    assert!(!xml.contains("<f>"));
    assert_eq!(attr(style(&xml, &styles, "B1"), "numFmtId"), Some("0"));
    for address in ["A1", "A2", "A4", "B1", "C1", "C2", "C3"] {
        let xf = style(&xml, &styles, address);
        assert!(font(xf, &styles).contains("val=\"96\""));
        assert!(font(xf, &styles).contains("<i/>"));
        assert!(xf.contains("horizontal=\"left\""));
    }
}

#[test]
fn xlsx_formats_keep_conditional_colour_precedence_as_a_reported_snapshot() {
    use sheaf_core::condformat::{Colours, CondFormat, CondRange, Rule};
    let mut sheet = Worksheet { columns: vec![column(1, vec![1.0, 9.0, 3.0])], ..Worksheet::new("S") };
    sheet.apply_cell_format(
        FormatTarget::whole_columns(vec![ColumnId(1)]),
        CellFormat {
            bold: Some(true),
            text_color: Some(Color::rgb(255, 0, 0)),
            fill: Some(Color::rgb(255, 255, 0)),
            ..Default::default()
        },
    );
    sheet.cond_formats.push(CondFormat::new(
        &sheet,
        CondRange::whole_columns(vec![ColumnId(1)]),
        Rule::TopBottom {
            top: true,
            n: 1.0,
            percent: false,
            colours: Colours { fill: Some(Color::rgb(0, 128, 255)), font: Some(Color::rgb(255, 255, 255)) },
        },
    ));
    let before = sheet.clone();
    let (xml, styles, warnings) = Output::new("conditional").export(&sheet, &plain());
    assert_eq!(sheet, before);
    assert!(warnings.iter().any(|w| w.contains("snapshot") && w.contains("rules")), "{warnings:?}");
    assert!(!xml.contains("conditionalFormatting"));
    assert!(fill(style(&xml, &styles, "A2"), &styles).contains("FF0080FF"));
    assert!(font(style(&xml, &styles, "A2"), &styles).contains("FFFFFFFF"));
    for address in ["A1", "A3"] {
        assert!(fill(style(&xml, &styles, address), &styles).contains("FFFFFF00"));
        assert!(font(style(&xml, &styles, address), &styles).contains("FFFF0000"));
    }
    for address in ["A1", "A2", "A3"] {
        assert!(font(style(&xml, &styles, address), &styles).contains("<b/>"));
    }
}

#[test]
fn xlsx_formats_report_omitted_labels_alpha_and_restore_default_font_and_alignment() {
    let mut sheet = Worksheet { columns: vec![column(1, vec![1.0, 2.0])], ..Worksheet::new("S") };
    sheet.apply_cell_format(
        FormatTarget::whole_columns(vec![ColumnId(1)]),
        CellFormat {
            font: Some("Courier New".into()),
            size_pt: Some(1.0),
            underline: Some(true),
            text_color: Some(Color::rgba(17, 34, 51, 128)),
            fill: Some(Color::rgb(255, 0, 255)),
            h_align: Some(HAlign::Right),
            ..Default::default()
        },
    );
    sheet.apply_cell_format(
        FormatTarget::cells(vec![ColumnId(1)], 1, Some(1)),
        CellFormat {
            font: Some(String::new()),
            size_pt: Some(0.0),
            underline: Some(false),
            text_color: Some(Color::TRANSPARENT),
            fill: Some(Color::TRANSPARENT),
            h_align: Some(HAlign::Auto),
            ..Default::default()
        },
    );
    sheet.apply_cell_format(
        FormatTarget::label_rows(vec![ColumnId(1)], vec![LabelKind::Formula, LabelKind::Param("Sample".into())]),
        CellFormat { bold: Some(true), ..Default::default() },
    );
    let (xml, styles, warnings) = Output::new("warnings").export(&sheet, &plain());
    assert_eq!(warnings.len(), 2, "{warnings:?}");
    assert!(warnings.iter().any(|w| w.contains("label rows omitted")));
    assert!(warnings.iter().any(|w| w.contains("transparent") && w.contains("opaque RGB")));
    let first = style(&xml, &styles, "A1");
    let second = style(&xml, &styles, "A2");
    assert!(font(first, &styles).contains("val=\"4\""));
    assert!(font(first, &styles).contains("FF112233"));
    assert!(first.contains("horizontal=\"right\""));
    let reset = font(second, &styles);
    assert!(reset.contains("val=\"9\""));
    assert!(reset.contains("val=\"Arial\""));
    assert!(!reset.contains("<u/>"));
    assert!(!reset.contains("rgb="));
    assert_eq!(attr(second, "fillId"), Some("0"));
    assert!(!second.contains("horizontal="));
}
