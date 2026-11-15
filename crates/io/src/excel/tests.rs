use super::*;
use crate::batch::{BatchLayout, BatchOptions, import_batch, import_batch_with_excel};
use crate::{ImportKind, ascii, chi, import_file};
use sheaf_core::{Column, ColumnId, Designation, Project};

fn fixture(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/excel").join(name)
}

fn temp(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("sheaf-io-excel-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn read(name: &str) -> ExcelImport {
    read_workbook(&fixture(name), &ExcelOptions::default()).unwrap()
}

fn numbers(col: &Column) -> Vec<f64> {
    col.numeric().unwrap().to_vec()
}

fn same(a: &[f64], b: &[f64]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.to_bits() == y.to_bits())
}

fn column(id: u64, short: &str, designation: Designation, data: ColumnData) -> Column {
    Column::new(ColumnId(id), short, designation, data)
}

fn labelled(mut c: Column, long: &str, units: &str, comments: &str) -> Column {
    c.long_name = long.into();
    c.units = units.into();
    c.comments = comments.into();
    c
}

fn sheet(name: &str, columns: Vec<Column>) -> Worksheet {
    Worksheet { name: name.into(), columns, filter: None }
}

fn warned(r: &ExcelImport, needle: &str) -> bool {
    r.warnings.iter().any(|w| w.contains(needle))
}

// ----- the fixtures --------------------------------------------------------------------

#[test]
fn a_workbook_with_header_rows_imports_numbers_text_ints_and_dates() {
    let r = read("basic.xlsx");
    let book = &r.workbook;
    assert_eq!(book.long_name, "basic");
    assert_eq!(book.source.as_ref().unwrap().importer, "excel");
    let names: Vec<&str> = book.sheets.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, ["Data", "Cached", "Uncached"]);
    let data = &book.sheets[0];
    assert_eq!(data.row_count(), 5);
    let a = &data.columns[0];
    assert_eq!((a.long_name.as_str(), a.units.as_str(), a.comments.as_str()), ("Potential", "V", "scan 1"));
    assert_eq!(a.designation, Designation::X);
    assert_eq!(numbers(a), [-0.2, -0.1, 0.0, 0.1, 0.2]);
    assert_eq!(numbers(&data.columns[1]), [1.5e-6, 3.0e-6, 4.5e-6, 6.0e-6, 7.5e-6]);
    let sample = &data.columns[2];
    assert_eq!(
        (sample.designation, sample.text().unwrap()),
        (Designation::Label, &["aa", "bb", "cc", "dd", "ee"].map(String::from)[..])
    );
    let when = &data.columns[3];
    assert_eq!(when.format, NumberFormat::DateTime(DatePattern::Iso, TimePattern::Hms));
    assert_eq!(numbers(when)[0], datetime::date_value(2026, 11, 5).unwrap() + 0.5);
    assert_eq!(numbers(&data.columns[4]), [1.0, 2.0, 3.0, 4.0, 5.0]);
    let layout = r.sheets[0].layout.as_ref().unwrap();
    assert_eq!(
        (layout.header_lines, layout.long_name_line, layout.units_line, layout.comments_line, layout.rows),
        (3, 1, 2, 3, 5)
    );
}

#[test]
fn a_formula_cell_gives_its_stored_value_and_one_without_gives_a_warning() {
    let r = read("basic.xlsx");
    let cached = &r.workbook.sheets[1];
    assert_eq!(numbers(&cached.columns[1]), [2.0, 4.0, 6.0, 8.0]);
    assert!(!warned(&r, "sheet 'Cached'"));
    let uncached = &r.workbook.sheets[2];
    // The column keeps its place and its name; its cells are missing.
    assert_eq!(uncached.columns.len(), 2);
    assert_eq!(uncached.columns[1].long_name, "triple");
    assert!(numbers(&uncached.columns[1]).iter().all(|x| x.is_nan()));
    assert!(warned(&r, "sheet 'Uncached': 4 formula cells have no stored value"));
    assert!(warned(&r, "first is B2"));
}

#[test]
fn a_legacy_xls_file_reads_like_an_xlsx_file() {
    let r = read("legacy.xls");
    let old = &r.workbook.sheets[0];
    assert_eq!(old.name, "Old");
    assert_eq!(old.columns[0].long_name, "Time");
    assert_eq!(old.columns[0].units, "s");
    assert_eq!(numbers(&old.columns[1]), [10.5, 11.5, 12.5, 13.5]);
    assert_eq!(old.columns[2].text().unwrap()[3], "t3");
    assert_eq!(old.columns[3].format, NumberFormat::Date(DatePattern::Iso));
    assert_eq!(numbers(&old.columns[3])[0], datetime::date_value(2020, 2, 26).unwrap());
    let second = &r.workbook.sheets[1];
    assert_eq!((second.name.as_str(), second.row_count()), ("Second", 3));
    assert_eq!(numbers(&second.columns[1]), [1.0, 4.0, 9.0]);
}

#[test]
fn the_1904_date_system_gives_the_same_dates_as_the_1900_system() {
    let r = read("date1904.xlsx");
    let d = &r.workbook.sheets[0];
    let jan1 = datetime::date_value(2020, 1, 1).unwrap();
    assert_eq!(numbers(&d.columns[0]), [jan1 + 0.5, jan1 + 1.5, jan1 + 2.5]);
    assert!(matches!(d.columns[0].format, NumberFormat::DateTime(DatePattern::Iso, _)));
    // A time of day is a fraction of a day in a Time column.
    assert_eq!(d.columns[1].format, NumberFormat::Time(TimePattern::Hms));
    let clock = numbers(&d.columns[1]);
    assert!((clock[0] - (13.0 * 3600.0 + 45.0 * 60.0 + 10.0) / 86_400.0).abs() < 1e-9, "{clock:?}");
    // A duration is a count of days, more than one day included.
    assert_eq!(d.columns[2].format, NumberFormat::Time(TimePattern::Hms));
    assert_eq!(numbers(&d.columns[2]), [1.5, 2.5, 3.5]);
    assert_eq!(datetime::format_time(1.5, TimePattern::Hms), "36:00:00");
}

#[test]
fn excels_1900_leap_year_bug_is_followed_and_the_missing_day_is_reported() {
    let r = read("leap1900.xlsx");
    let col = &r.workbook.sheets[0].columns[0];
    assert_eq!(col.format, NumberFormat::Date(DatePattern::Dmy));
    let day = |y, m, d| datetime::date_value(y, m, d).unwrap();
    // Excel days 1, 59, 60 (the day that never existed), 61, 25569 and 45000.
    assert_eq!(
        numbers(col),
        [day(1900, 1, 1), day(1900, 2, 28), day(1900, 2, 28), day(1900, 3, 1), day(1970, 1, 1), day(2023, 3, 15)]
    );
    assert!(warned(&r, "1900-02-29"));
}

#[test]
fn merged_cells_keep_their_top_left_value_and_a_merged_title_is_not_a_label_row() {
    let r = read("merged.xlsx");
    let s = &r.workbook.sheets[0];
    let x = &s.columns[0];
    assert_eq!((x.long_name.as_str(), x.units.as_str()), ("X", "s"));
    assert_eq!((s.columns[1].long_name.as_str(), s.columns[2].units.as_str()), ("Y", "A"));
    // Row 8 is blank and stays a row of missing values; the block B9:B10 holds 9 in its top cell
    // and nothing below.
    let y = numbers(&s.columns[1]);
    assert_eq!(y.len(), 7);
    assert!(y[4].is_nan());
    assert_eq!(y[5], 9.0);
    assert!(y[6].is_nan());
    assert!(warned(&r, "1 header line was not imported"));
}

#[test]
fn hidden_sheets_are_skipped_unless_asked_for() {
    let r = read("hidden.xlsx");
    assert_eq!(r.workbook.sheets.len(), 1);
    assert!(warned(&r, "sheet 'Hidden' is hidden"));
    assert!(r.sheets.iter().filter(|s| s.hidden).all(|s| !s.imported));

    let all =
        read_workbook(&fixture("hidden.xlsx"), &ExcelOptions { include_hidden: true, ..Default::default() }).unwrap();
    let names: Vec<&str> = all.workbook.sheets.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, ["Visible", "Hidden", "Secret"]);
    assert!(all.warnings.is_empty());

    // A sheet that is asked for by name is read although it is hidden.
    let one = ExcelOptions { sheets: vec![SheetRef::Name("hidden".into())], ..Default::default() };
    let r = read_workbook(&fixture("hidden.xlsx"), &one).unwrap();
    assert_eq!(numbers(&r.workbook.sheets[0].columns[0]), [10.0, 20.0, 30.0]);
}

#[test]
fn sheets_that_hold_only_a_chart_are_skipped_with_a_warning() {
    let r = read("charts.xlsx");
    assert_eq!(r.workbook.sheets.len(), 1);
    assert_eq!(r.workbook.sheets[0].name, "Data");
    assert!(warned(&r, "sheet 'OnlyChart' was not imported"));
    assert!(warned(&r, "sheet 'ChartSheet' is a chart"));
    let e = read_workbook(
        &fixture("charts.xlsx"),
        &ExcelOptions { sheets: vec![SheetRef::Index(3)], ..Default::default() },
    )
    .unwrap_err();
    assert!(matches!(e, IoError::NoData { .. }), "{e}");
}

#[test]
fn text_among_numbers_stays_text_and_booleans_errors_and_big_integers_are_mapped() {
    let r = read("mixed.xlsx");
    let s = &r.workbook.sheets[0];
    // A numeric column keeps its text cells, as the text import does with `n.d.` and `<LOD`.
    let conc = &s.columns[0];
    assert!(conc.data.is_mixed());
    assert_eq!(conc.data.text_cell(1), Some("n.d."));
    assert_eq!(conc.data.text_cell(3), Some("<LOD"));
    assert!(same(&numbers_of_mixed(conc), &[1.5, f64::NAN, 3.5, f64::NAN]));
    // A column of words is a text column.
    assert_eq!(s.columns[1].designation, Designation::Label);
    // Booleans are 1 and 0.
    assert_eq!(numbers_of_mixed(&s.columns[2]), [1.0, 0.0, 1.0, 0.0]);
    // An error cell is a missing value, and counted.
    let ratio = numbers_of_mixed(&s.columns[3]);
    assert!(ratio[1].is_nan() && ratio[2] == 4.0);
    assert!(warned(&r, "1 cells hold errors"));
    // Dates among other values become text, with a warning.
    assert_eq!(s.columns[4].text().unwrap()[0], "2026-03-01");
    assert_eq!(s.columns[4].text().unwrap()[1], "not a date");
    assert!(warned(&r, "column E mixes dates"));
    // Integers up to 2^53 are exact.
    assert_eq!(numbers_of_mixed(&s.columns[5])[0], 9_007_199_254_740_992.0);
    assert_eq!(numbers_of_mixed(&s.columns[5])[1], 12_345_678_901.0);
}

fn numbers_of_mixed(col: &Column) -> Vec<f64> {
    match &col.data {
        ColumnData::Mixed { numbers, .. } => numbers.to_vec(),
        ColumnData::Numeric(v) => v.to_vec(),
        other => panic!("not numeric: {other:?}"),
    }
}

#[test]
fn data_that_starts_below_and_right_of_the_corner_drops_the_empty_rows_and_columns() {
    let r = read("leading.xlsx");
    let s = &r.workbook.sheets[0];
    assert_eq!(s.columns.len(), 2);
    assert_eq!((s.columns[0].short_name.as_str(), s.columns[0].long_name.as_str()), ("A", "Time"));
    assert_eq!(s.columns[1].units, "mV");
    assert_eq!(numbers_of_mixed(&s.columns[0]), [0.0, 0.5, 1.0, 1.5]);
    let layout = r.sheets[0].layout.as_ref().unwrap();
    // Row numbers are Excel's; the first imported column is C.
    assert_eq!((layout.long_name_line, layout.units_line, layout.header_lines, layout.first_column), (4, 5, 5, 3));
}

// ----- options ------------------------------------------------------------------------

#[test]
fn sheets_can_be_chosen_by_name_and_by_position_and_a_wrong_name_is_an_error() {
    let by_name = ExcelOptions { sheets: SheetRef::parse_list("Second, 1"), ..Default::default() };
    let r = read_workbook(&fixture("legacy.xls"), &by_name).unwrap();
    let names: Vec<&str> = r.workbook.sheets.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, ["Second", "Old"]);
    let e = read_workbook(
        &fixture("legacy.xls"),
        &ExcelOptions { sheets: vec![SheetRef::Name("Nope".into())], ..Default::default() },
    )
    .unwrap_err();
    assert!(e.to_string().contains("no sheet 'Nope'") && e.to_string().contains("Old, Second"), "{e}");
    let e =
        read_workbook(&fixture("legacy.xls"), &ExcelOptions { sheets: vec![SheetRef::Index(9)], ..Default::default() })
            .unwrap_err();
    assert!(e.to_string().contains("no sheet number 9"), "{e}");
    let e = read_workbook(
        &fixture("legacy.xls"),
        &ExcelOptions { sheets: SheetRef::parse_list("9"), ..Default::default() },
    )
    .unwrap_err();
    assert!(e.to_string().contains("named or numbered '9'"), "{e}");
    assert_eq!(SheetRef::parse("3"), SheetRef::NameOrIndex("3".into()));
    assert_eq!(SheetRef::parse(" Run 3 "), SheetRef::Name("Run 3".into()));
    assert_eq!(SheetRef::parse_list("1,,Data"), [SheetRef::NameOrIndex("1".into()), SheetRef::Name("Data".into())]);
}

#[test]
fn header_rows_follow_the_options_as_in_a_text_import() {
    let path = fixture("basic.xlsx");
    let only_data = ExcelOptions { sheets: vec![SheetRef::Index(1)], ..Default::default() };
    // Labels switched off: the header rows are skipped and the columns keep empty labels.
    let r = read_workbook(&path, &ExcelOptions { rename_columns: false, ..only_data.clone() }).unwrap();
    assert_eq!(r.workbook.sheets[0].columns[0].long_name, "");
    assert_eq!(r.workbook.sheets[0].row_count(), 5);
    // The long names taken from row 2 and units from row 1 (an odd choice, but the options rule).
    let swapped =
        ExcelOptions { long_name_line: Some(2), units_line: Some(1), comments_line: Some(0), ..only_data.clone() };
    let c = &read_workbook(&path, &swapped).unwrap().workbook.sheets[0].columns[0];
    assert_eq!((c.long_name.as_str(), c.units.as_str(), c.comments.as_str()), ("V", "Potential", ""));
    // The short names from row 1.
    let short = ExcelOptions { short_name_line: 1, ..only_data.clone() };
    let s = &read_workbook(&path, &short).unwrap().workbook.sheets[0];
    assert_eq!(s.columns[0].short_name, "Potential");
    // A parameter row named by its first cell.
    let params = ExcelOptions { param_lines: vec![3], param_name_from_first_cell: false, ..only_data.clone() };
    let s = &read_workbook(&path, &params).unwrap().workbook.sheets[0];
    assert_eq!(s.columns[1].params.get("Param1").map(String::as_str), Some("scan 1"));
    // The data start given explicitly.
    let start = ExcelOptions {
        header_lines: Some(2),
        long_name_line: Some(1),
        units_line: Some(2),
        comments_line: Some(0),
        ..only_data
    };
    let s = &read_workbook(&path, &start).unwrap().workbook.sheets[0];
    assert_eq!(s.row_count(), 6, "the third row is data now");
    assert_eq!(s.columns[0].units, "V");
}

#[test]
fn rows_and_columns_can_be_imported_in_part() {
    let path = fixture("basic.xlsx");
    let part = ExcelOptions {
        sheets: vec![SheetRef::Index(1)],
        skip_rows: 1,
        max_rows: Some(3),
        columns: ImportSelection { first: 2, last: 4, read: 1, skip: 0 },
        ..Default::default()
    };
    let s = &read_workbook(&path, &part).unwrap().workbook.sheets[0];
    assert_eq!(s.row_count(), 3);
    assert_eq!(s.columns.len(), 3);
    assert_eq!(s.columns[0].long_name, "Current");
    assert_eq!(numbers(&s.columns[0]), [3.0e-6, 4.5e-6, 6.0e-6]);
    let every_other = ExcelOptions {
        sheets: vec![SheetRef::Index(1)],
        rows: ImportSelection { first: 1, last: 0, read: 1, skip: 1 },
        ..Default::default()
    };
    let s = &read_workbook(&path, &every_other).unwrap().workbook.sheets[0];
    assert_eq!(numbers(&s.columns[0]), [-0.2, 0.0, 0.2]);
    let bad = ExcelOptions { rows: ImportSelection { first: 0, last: 0, read: 1, skip: 0 }, ..Default::default() };
    assert!(matches!(read_workbook(&path, &bad), Err(IoError::InvalidOption(_))));
    let none = ExcelOptions {
        sheets: vec![SheetRef::Index(1)],
        columns: ImportSelection { first: 20, ..Default::default() },
        ..Default::default()
    };
    assert!(matches!(read_workbook(&path, &none), Err(IoError::NoData { .. })));
    let first = part.without_partial_import();
    assert_eq!((first.skip_rows, first.max_rows, first.columns.first), (0, None, 1));
}

#[test]
fn dates_can_be_kept_as_text() {
    let opts = ExcelOptions { dates: DateImport::AsText, sheets: vec![SheetRef::Index(1)], ..Default::default() };
    let s = &read_workbook(&fixture("basic.xlsx"), &opts).unwrap().workbook.sheets[0];
    assert_eq!(s.columns[3].text().unwrap()[0], "2026-11-05 12:00:00");
}

#[test]
fn a_text_import_and_an_excel_import_of_the_same_table_agree() {
    // The same table as a CSV file and as a workbook: same labels, designations, numbers and text.
    let csv = "Potential,Current,Sample\nV,A,\nscan 1,scan 1,\n-0.2,1.5e-06,aa\n-0.1,3e-06,bb\n0.0,4.5e-06,cc\n";
    let text = ascii::parse_ascii(csv, "t", &ascii::AsciiOptions::default()).unwrap().workbook;
    let dir = temp("agree");
    let table = sheet(
        "Data",
        vec![
            labelled(
                column(1, "A", Designation::X, ColumnData::numeric(vec![-0.2, -0.1, 0.0])),
                "Potential",
                "V",
                "scan 1",
            ),
            labelled(
                column(2, "B", Designation::Y, ColumnData::numeric(vec![1.5e-6, 3e-6, 4.5e-6])),
                "Current",
                "A",
                "scan 1",
            ),
            labelled(
                column(3, "C", Designation::Label, ColumnData::text(vec!["aa".into(), "bb".into(), "cc".into()])),
                "Sample",
                "",
                "",
            ),
        ],
    );
    let out = write_xlsx(&[&table], &dir.join("t.xlsx"), &ExcelExportOptions::default()).unwrap();
    let book = read_workbook(&out.path, &ExcelOptions::default()).unwrap().workbook;
    for (a, b) in text.sheets[0].columns.iter().zip(&book.sheets[0].columns) {
        assert_eq!(
            (&a.short_name, &a.long_name, &a.units, &a.comments, a.designation),
            (&b.short_name, &b.long_name, &b.units, &b.comments, b.designation)
        );
        assert_eq!(a.data, b.data, "column {}", a.short_name);
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_single_title_cell_is_not_a_label_row_but_two_adjacent_label_rows_are() {
    let dir = temp("title");
    // Written by hand: a title row, then names and units, then data.
    let mut book = rust_xlsxwriter::Workbook::new();
    let ws = book.add_worksheet();
    ws.write_string(0, 0, "Run 7").unwrap();
    ws.write_string(1, 0, "Time").unwrap();
    ws.write_string(1, 1, "Signal").unwrap();
    ws.write_string(2, 0, "s").unwrap();
    ws.write_string(2, 1, "mV").unwrap();
    for i in 0..4u32 {
        ws.write_number(3 + i, 0, f64::from(i)).unwrap();
        ws.write_number(3 + i, 1, f64::from(i) * 2.0).unwrap();
    }
    book.save(dir.join("title.xlsx")).unwrap();
    let r = read_workbook(&dir.join("title.xlsx"), &ExcelOptions::default()).unwrap();
    let cols = &r.workbook.sheets[0].columns;
    assert_eq!((cols[0].long_name.as_str(), cols[0].units.as_str()), ("Time", "s"));
    assert_eq!(cols[1].long_name, "Signal");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_file_is_read_by_its_bytes_not_its_name() {
    let dir = temp("bytes");
    // An xlsx saved with the extension of the old format, and an old file with the new extension.
    std::fs::copy(fixture("basic.xlsx"), dir.join("really_xlsx.xls")).unwrap();
    std::fs::copy(fixture("legacy.xls"), dir.join("really_xls.xlsx")).unwrap();
    assert_eq!(read_workbook(&dir.join("really_xlsx.xls"), &ExcelOptions::default()).unwrap().workbook.sheets.len(), 3);
    assert_eq!(read_workbook(&dir.join("really_xls.xlsx"), &ExcelOptions::default()).unwrap().workbook.sheets.len(), 2);
    // Something else entirely is an error that names the file.
    std::fs::write(dir.join("notes.xlsx"), "just text, not a workbook").unwrap();
    let e = read_workbook(&dir.join("notes.xlsx"), &ExcelOptions::default()).unwrap_err();
    assert!(matches!(e, IoError::ExcelRead { .. }) && e.to_string().contains("notes.xlsx"), "{e}");
    let e = read_workbook(&dir.join("missing.xlsx"), &ExcelOptions::default()).unwrap_err();
    assert!(matches!(e, IoError::Io { .. }), "{e}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn sheets_are_listed_and_previewed_for_a_dialog() {
    let list = list_sheets(&fixture("charts.xlsx")).unwrap();
    let shown: Vec<(&str, SheetKind)> = list.iter().map(|s| (s.name.as_str(), s.kind)).collect();
    assert_eq!(
        shown,
        [("Data", SheetKind::Worksheet), ("OnlyChart", SheetKind::Worksheet), ("ChartSheet", SheetKind::Chart)]
    );
    assert!(list_sheets(&fixture("hidden.xlsx")).unwrap()[1].hidden);

    let p = preview_sheet(&fixture("leading.xlsx"), &SheetRef::Index(1), &ExcelOptions::default(), 4).unwrap();
    assert_eq!((p.first_row, p.first_column), (4, 3));
    assert_eq!(p.rows[0], ["Time", "Value"]);
    assert_eq!(p.rows.len(), 4);
    assert_eq!(p.layout.unwrap().units_line, 5);
    let p =
        preview_sheet(&fixture("basic.xlsx"), &SheetRef::Name("Data".into()), &ExcelOptions::default(), 100).unwrap();
    assert_eq!(p.rows.len(), 8);
    assert_eq!(p.rows[3][3], "2026-11-05 12:00:00");
}

// ----- the writer ---------------------------------------------------------------------

#[test]
fn a_written_workbook_reads_back_with_the_same_values_labels_and_types() {
    let dir = temp("roundtrip");
    let values = vec![
        0.1 + 0.2,
        -1e-300,
        1.7976931348623157e308,
        123_456_789_012_345.0,
        f64::NAN,
        5e-324,
        9_007_199_254_740_992.0,
    ];
    let mut mixed_texts = std::collections::BTreeMap::new();
    mixed_texts.insert(1, "n.d.".to_string());
    mixed_texts.insert(3, "<LOD".to_string());
    let a = sheet(
        "Run 1",
        vec![
            labelled(column(1, "A", Designation::X, ColumnData::numeric(values.clone())), "Time", "s", "first"),
            labelled(
                column(
                    2,
                    "B",
                    Designation::Y,
                    ColumnData::mixed(vec![1.0, f64::NAN, 3.0, f64::NAN, 5.0, 6.0, 7.0], mixed_texts),
                ),
                "Conc",
                "mM",
                "",
            ),
            labelled(
                column(
                    3,
                    "C",
                    Designation::Label,
                    ColumnData::text(vec![
                        "x".into(),
                        String::new(),
                        "z".into(),
                        "w".into(),
                        "v".into(),
                        "u".into(),
                        "t".into(),
                    ]),
                ),
                "Name",
                "",
                "",
            ),
        ],
    );
    let b = sheet("Run 2", vec![column(1, "A", Designation::X, ColumnData::numeric(vec![1.0, 2.0]))]);
    let book = Workbook { sheets: vec![a.clone(), b], ..Workbook::new(sheaf_core::WindowId(1), "Book1") };
    let out = write_workbook_xlsx(&book, &dir.join("rt.xlsx"), &ExcelExportOptions::default()).unwrap();
    assert_eq!(out.sheet_names, ["Run 1", "Run 2"]);
    assert!(out.warnings.is_empty());

    let r = read_workbook(&out.path, &ExcelOptions::default()).unwrap();
    assert_eq!(r.workbook.sheets.len(), 2);
    let back = &r.workbook.sheets[0];
    assert_eq!(back.name, "Run 1");
    assert!(same(&numbers_of_mixed(&back.columns[0]), &values), "{:?}", back.columns[0].data);
    assert_eq!(back.columns[0].designation, Designation::X);
    assert_eq!(
        (back.columns[0].long_name.as_str(), back.columns[0].units.as_str(), back.columns[0].comments.as_str()),
        ("Time", "s", "first")
    );
    assert_eq!(back.columns[1].data, a.columns[1].data);
    assert_eq!(back.columns[2].data, a.columns[2].data);
    assert_eq!(back.columns[2].designation, Designation::Label);
    assert_eq!(r.workbook.sheets[1].columns[0].long_name, "");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn label_rows_are_written_only_when_some_column_has_the_label() {
    let dir = temp("labels");
    let plain = sheet("S", vec![column(1, "A", Designation::X, ColumnData::numeric(vec![1.0, 2.0]))]);
    let out = write_xlsx(&[&plain], &dir.join("p.xlsx"), &ExcelExportOptions::default()).unwrap();
    let p = preview_sheet(&out.path, &SheetRef::Index(1), &ExcelOptions::default(), 10).unwrap();
    assert_eq!(p.rows, [["1"], ["2"]]);

    let units_only =
        sheet("S", vec![labelled(column(1, "A", Designation::X, ColumnData::numeric(vec![1.0, 2.0])), "", "mV", "")]);
    let out = write_xlsx(&[&units_only], &dir.join("u.xlsx"), &ExcelExportOptions::default()).unwrap();
    let p = preview_sheet(&out.path, &SheetRef::Index(1), &ExcelOptions::default(), 10).unwrap();
    // The units sit in the second label row: an empty first row keeps them from being read as
    // long names.
    assert_eq!(p.rows, [[""], ["mV"], ["1"], ["2"]]);

    let all = sheet("S", vec![labelled(column(1, "A", Designation::X, ColumnData::numeric(vec![1.0])), "L", "U", "C")]);
    let opts = ExcelExportOptions { short_names: true, ..Default::default() };
    let out = write_xlsx(&[&all], &dir.join("a.xlsx"), &opts).unwrap();
    let p = preview_sheet(&out.path, &SheetRef::Index(1), &ExcelOptions::default(), 10).unwrap();
    assert_eq!(p.rows, [["A"], ["L"], ["U"], ["C"], ["1"]]);
    let opts = ExcelExportOptions { long_names: false, units: false, comments: false, short_names: false };
    let out = write_xlsx(&[&all], &dir.join("n.xlsx"), &opts).unwrap();
    let p = preview_sheet(&out.path, &SheetRef::Index(1), &ExcelOptions::default(), 10).unwrap();
    assert_eq!(p.rows, [["1"]]);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn date_and_time_columns_are_written_as_excel_dates_and_read_back_with_their_format() {
    let dir = temp("dates");
    let day = |y, m, d| datetime::date_value(y, m, d).unwrap();
    let dates = vec![day(2026, 11, 5), day(1900, 2, 28), day(1900, 3, 1), f64::NAN, day(2100, 12, 31)];
    let stamps =
        vec![day(2026, 11, 5) + 0.5, day(2026, 11, 6) + 0.25, day(1999, 12, 31) + 0.999, f64::NAN, day(2000, 1, 1)];
    let clocks = vec![0.5, 0.25, 0.999, 0.0, f64::NAN];
    let spans = vec![1.5, 2.25, 10.0, 0.0, f64::NAN];
    let mut cols = Vec::new();
    for (i, (name, format, data)) in [
        ("D", NumberFormat::Date(DatePattern::Iso), &dates),
        ("E", NumberFormat::Date(DatePattern::Dmy), &dates),
        ("F", NumberFormat::Date(DatePattern::DMonY), &dates),
        ("G", NumberFormat::DateTime(DatePattern::Mdy, TimePattern::Hms), &stamps),
        ("H", NumberFormat::Time(TimePattern::Hm), &clocks),
        ("I", NumberFormat::Time(TimePattern::Hms), &spans),
    ]
    .into_iter()
    .enumerate()
    {
        let mut c = column(i as u64 + 1, name, Designation::Y, ColumnData::numeric(data.clone()));
        c.format = format;
        cols.push(c);
    }
    let out =
        write_xlsx(&[&sheet("Dates", cols.clone())], &dir.join("d.xlsx"), &ExcelExportOptions::default()).unwrap();
    let back = &read_workbook(&out.path, &ExcelOptions::default()).unwrap().workbook.sheets[0];
    for (a, b) in cols.iter().zip(&back.columns) {
        assert_eq!(a.format, b.format, "column {}", a.short_name);
        let (x, y) = (numbers(a), numbers(b));
        assert_eq!(x.len(), y.len());
        for (p, q) in x.iter().zip(&y) {
            assert!((p.is_nan() && q.is_nan()) || (p - q).abs() < 1e-8, "column {}: {p} vs {q}", a.short_name);
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_date_before_1900_is_written_as_a_number_with_a_warning_and_infinity_is_left_out() {
    let dir = temp("early");
    let mut d = column(
        1,
        "A",
        Designation::Y,
        ColumnData::numeric(vec![datetime::date_value(1850, 1, 1).unwrap(), f64::INFINITY, 2.0]),
    );
    d.format = NumberFormat::Date(DatePattern::Iso);
    let out = write_xlsx(&[&sheet("S", vec![d])], &dir.join("e.xlsx"), &ExcelExportOptions::default()).unwrap();
    assert!(out.warnings.iter().any(|w| w.contains("before 1900-01-01")), "{:?}", out.warnings);
    assert!(out.warnings.iter().any(|w| w.contains("infinite")), "{:?}", out.warnings);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn sheet_names_follow_excels_rules_and_stay_unique() {
    let dir = temp("names");
    let one = |name: &str| sheet(name, vec![column(1, "A", Designation::X, ColumnData::numeric(vec![1.0]))]);
    let long = "A very long sheet name that is over thirty-one characters";
    let sheets = [
        one("a/b:c*d?e[f]g\\h"),
        one(long),
        one("run"),
        one("RUN"),
        one(""),
        one("History"),
        one("'quoted'"),
        one(&format!("{}x", &long[..30])),
    ];
    let refs: Vec<&Worksheet> = sheets.iter().collect();
    let out = write_xlsx(&refs, &dir.join("n.xlsx"), &ExcelExportOptions::default()).unwrap();
    assert_eq!(out.sheet_names[0], "a_b_c_d_e_f_g_h");
    assert_eq!(out.sheet_names[1].chars().count(), 31);
    assert_eq!(&out.sheet_names[2..6], ["run", "RUN_2", "Sheet5", "History_"]);
    assert_eq!(out.sheet_names[6], "quoted");
    for n in &out.sheet_names {
        assert!(n.chars().count() <= 31 && !n.contains(['[', ']', ':', '*', '?', '/', '\\']), "{n}");
    }
    let mut lower: Vec<String> = out.sheet_names.iter().map(|n| n.to_lowercase()).collect();
    lower.sort();
    lower.dedup();
    assert_eq!(lower.len(), out.sheet_names.len());
    assert!(out.warnings.iter().any(|w| w.contains("'RUN' is called 'RUN_2'")));
    // Excel itself accepts the names: the reader finds them.
    let names: Vec<String> = list_sheets(&out.path).unwrap().into_iter().map(|s| s.name).collect();
    assert_eq!(names, out.sheet_names);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn an_export_never_replaces_a_file_that_is_not_a_workbook() {
    let dir = temp("exists");
    let s = sheet("S", vec![column(1, "A", Designation::X, ColumnData::numeric(vec![1.0]))]);
    let o = ExcelExportOptions::default();
    // Another extension is refused; no extension gets .xlsx.
    let e = write_xlsx(&[&s], &dir.join("x.csv"), &o).unwrap_err();
    assert!(matches!(e, IoError::InvalidOption(_)) && e.to_string().contains(".xlsx"), "{e}");
    assert_eq!(write_xlsx(&[&s], &dir.join("bare"), &o).unwrap().path, dir.join("bare.xlsx"));
    // A file of that name that is not a workbook stays as it is.
    std::fs::write(dir.join("keep.xlsx"), "my notes").unwrap();
    let e = write_xlsx(&[&s], &dir.join("keep.xlsx"), &o).unwrap_err();
    assert!(matches!(e, IoError::ExportExists { .. }), "{e}");
    assert_eq!(std::fs::read_to_string(dir.join("keep.xlsx")).unwrap(), "my notes");
    // A zip that is not a workbook stays too; a workbook is replaced.
    std::fs::copy(fixture("basic.xlsx"), dir.join("old.xlsx")).unwrap();
    write_xlsx(&[&s], &dir.join("old.xlsx"), &o).unwrap();
    assert_eq!(list_sheets(&dir.join("old.xlsx")).unwrap().len(), 1);
    // Nothing else is left in the folder (no temporary file).
    let mut files: Vec<String> =
        std::fs::read_dir(&dir).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    files.sort();
    assert_eq!(files, ["bare.xlsx", "keep.xlsx", "old.xlsx"]);
    let e = write_xlsx(&[], &dir.join("none.xlsx"), &o).unwrap_err();
    assert!(matches!(e, IoError::ExcelWrite { .. }), "{e}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_sheet_with_more_columns_than_excel_holds_is_refused() {
    let dir = temp("toowide");
    let cols = (0..16_385u64).map(|i| column(i + 1, "A", Designation::Y, ColumnData::numeric(vec![1.0]))).collect();
    let e = write_xlsx(&[&sheet("Wide", cols)], &dir.join("w.xlsx"), &ExcelExportOptions::default()).unwrap_err();
    assert!(e.to_string().contains("16384"), "{e}");
    assert!(!dir.join("w.xlsx").exists());
    let _ = std::fs::remove_dir_all(&dir);
}

// ----- size ---------------------------------------------------------------------------

#[test]
fn a_sheet_with_more_than_a_thousand_columns_round_trips() {
    let dir = temp("wide");
    let n = 1500u64;
    let cols: Vec<Column> = (0..n)
        .map(|i| {
            let short = if i == 0 { "A".to_string() } else { format!("c{i}") };
            labelled(
                column(i + 1, &short, Designation::Y, ColumnData::numeric(vec![i as f64, i as f64 + 0.5, -(i as f64)])),
                &format!("col {i}"),
                "",
                "",
            )
        })
        .collect();
    let out = write_xlsx(&[&sheet("Wide", cols)], &dir.join("w.xlsx"), &ExcelExportOptions::default()).unwrap();
    let r = read_workbook(&out.path, &ExcelOptions::default()).unwrap();
    let back = &r.workbook.sheets[0];
    assert_eq!(back.columns.len(), 1500);
    assert_eq!(back.columns[1499].long_name, "col 1499");
    assert_eq!(numbers(&back.columns[1499]), [1499.0, 1499.5, -1499.0]);
    assert_eq!(r.sheets[0].layout.as_ref().unwrap().columns, 1500);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_sheet_with_a_hundred_thousand_rows_is_read_in_one_pass() {
    let dir = temp("tall");
    let n = 100_000usize;
    let x: Vec<f64> = (0..n).map(|i| i as f64 * 0.001).collect();
    let y: Vec<f64> = (0..n).map(|i| if i % 7 == 0 { f64::NAN } else { (i as f64).sin() }).collect();
    let cols = vec![
        labelled(column(1, "A", Designation::X, ColumnData::numeric(x.clone())), "t", "s", ""),
        labelled(column(2, "B", Designation::Y, ColumnData::numeric(y.clone())), "v", "V", ""),
    ];
    let out = write_xlsx(&[&sheet("Tall", cols)], &dir.join("t.xlsx"), &ExcelExportOptions::default()).unwrap();
    let r = read_workbook(&out.path, &ExcelOptions::default()).unwrap();
    let back = &r.workbook.sheets[0];
    assert_eq!(back.row_count(), n);
    assert!(same(&numbers_of_mixed(&back.columns[0]), &x));
    assert!(same(&numbers_of_mixed(&back.columns[1]), &y));
    let _ = std::fs::remove_dir_all(&dir);
}

// ----- the app's and the command line's way in ----------------------------------------

#[test]
fn import_file_reads_an_excel_file_by_its_extension_unless_a_kind_is_forced() {
    let (chi, ascii) = (chi::ChiOptions::default(), ascii::AsciiOptions::default());
    let a = import_file(&fixture("basic.xlsx"), ImportKind::Auto, &chi, &ascii).unwrap();
    assert_eq!(a.format, "excel");
    assert_eq!(a.workbook.sheets.len(), 3);
    assert_eq!(a.workbook.long_name, "basic");
    assert_eq!(a.workbook.source.as_ref().unwrap().importer, "excel");
    assert_eq!(a.meta, [("Sheets".to_string(), "Data, Cached, Uncached".to_string())]);
    assert_eq!(a.suggested_template, sheaf_core::template::Template::Line);
    // Forced to delimited text it is not an Excel import (and fails as binary data).
    assert!(import_file(&fixture("basic.xlsx"), ImportKind::Ascii, &chi, &ascii).is_err());
    assert!(
        is_excel_file(Path::new("A.XLSX")) && is_excel_file(Path::new("b.xls")) && is_excel_file(Path::new("c.xlsm"))
    );
    assert!(
        !is_excel_file(Path::new("d.csv")) && !is_excel_file(Path::new("e.xlsb")) && !is_excel_file(Path::new("xlsx"))
    );
}

#[test]
fn excel_files_go_through_the_batch_modes_like_text_files() {
    let dir = temp("batch");
    std::fs::copy(fixture("legacy.xls"), dir.join("a.xls")).unwrap();
    std::fs::copy(fixture("hidden.xlsx"), dir.join("b.xlsx")).unwrap();
    std::fs::write(dir.join("c.csv"), "x,y\n1,2\n3,4\n5,6\n").unwrap();

    // One book per file: Excel books keep their sheets.
    let mut p = Project::new();
    let files = vec![dir.join("a.xls"), dir.join("b.xlsx"), dir.join("c.csv")];
    let r = import_batch(&mut p, &files, &BatchOptions::default());
    assert!(r.errors.is_empty(), "{:?}", r.errors);
    assert_eq!(r.items.iter().map(|i| i.format).collect::<Vec<_>>(), ["excel", "excel", "ascii"]);
    assert_eq!(p.workbook(r.items[0].book).unwrap().sheets.len(), 2);
    assert!(r.items[1].warnings.iter().any(|w| w.contains("hidden")));

    // One sheet per file: an Excel file adds all its sheets.
    let mut p = Project::new();
    let r = import_batch(&mut p, &files, &BatchOptions { layout: BatchLayout::Sheets, ..Default::default() });
    let book = p.workbook(r.items[0].book).unwrap();
    let names: Vec<&str> = book.sheets.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, ["Old", "Second", "Visible", "c"]);

    // New columns: only the first Excel sheet is added, with a warning.
    let mut p = Project::new();
    let all = ExcelOptions { include_hidden: true, ..Default::default() };
    let r = import_batch_with_excel(
        &mut p,
        &files[..2],
        &BatchOptions { layout: BatchLayout::Columns, ..Default::default() },
        &all,
    );
    assert_eq!(p.workbook(r.items[0].book).unwrap().sheets[0].columns.len(), 5);
    assert!(r.items[1].warnings.iter().any(|w| w.contains("only the first was added")), "{:?}", r.items[1].warnings);

    // Excel options reach the files: a chosen sheet and a partial import that spares the first file.
    let mut p = Project::new();
    let excel = ExcelOptions { sheets: vec![SheetRef::Name("Second".into())], max_rows: Some(1), ..Default::default() };
    let opts = BatchOptions { partial_except_first: true, ..Default::default() };
    let r = import_batch_with_excel(&mut p, &files[..1], &opts, &excel);
    let first = &p.workbook(r.items[0].book).unwrap().sheets[0];
    assert_eq!((first.name.as_str(), first.row_count()), ("Second", 3), "the first file is read in full");
    let r = import_batch_with_excel(&mut p, &[dir.join("a.xls")], &BatchOptions::default(), &excel);
    assert_eq!(p.workbook(r.items[0].book).unwrap().sheets[0].row_count(), 1);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_folder_import_lists_excel_files_but_not_excels_lock_files() {
    let dir = temp("folder");
    for name in ["a.xlsx", "b.xls", "c.xlsm", "~$a.xlsx", "d.csv", "e.docx"] {
        std::fs::write(dir.join(name), "x").unwrap();
    }
    let found = crate::batch::expand_inputs(&dir).unwrap();
    let names: Vec<String> = found.iter().map(|p| p.file_name().unwrap().to_string_lossy().into_owned()).collect();
    assert_eq!(names, ["a.xlsx", "b.xls", "c.xlsm", "d.csv"]);
    let _ = std::fs::remove_dir_all(&dir);
}

// ----- dates and format codes -------------------------------------------------------------

#[test]
fn excel_number_formats_map_to_sheaf_patterns() {
    use dates::patterns_from_code as p;
    let d = |c| p(c).0;
    let t = |c| p(c).1;
    assert_eq!(d("yyyy-mm-dd"), Some(DatePattern::Iso));
    assert_eq!(d("yyyy\\-mm\\-dd"), Some(DatePattern::Iso));
    assert_eq!(d("m/d/yyyy"), Some(DatePattern::Mdy));
    assert_eq!(d("mm-dd-yy"), Some(DatePattern::MdyDash));
    assert_eq!(d("dd/mm/yyyy"), Some(DatePattern::Dmy));
    assert_eq!(d("dd-mm-yyyy"), Some(DatePattern::DmyDash));
    assert_eq!(d("dd.mm.yyyy"), Some(DatePattern::DmyDot));
    assert_eq!(d("d-mmm-yy"), Some(DatePattern::DMonY));
    assert_eq!(d("dddd, mmmm d, yyyy"), Some(DatePattern::LongDate));
    assert_eq!(d("mmm d"), Some(DatePattern::MonthDay));
    assert_eq!(d("m/d"), Some(DatePattern::MonthDayNum));
    assert_eq!(d("yyyy"), Some(DatePattern::YearOnly));
    assert_eq!(d("[$-409]yyyy-mm-dd;@"), Some(DatePattern::Iso));
    assert_eq!(d("\"Day\" d"), Some(DatePattern::DayOnly));
    assert_eq!(d("h:mm:ss"), None);
    assert_eq!(t("h:mm:ss"), Some(TimePattern::Hms));
    assert_eq!(t("hh:mm"), Some(TimePattern::Hm));
    assert_eq!(t("[h]:mm:ss"), Some(TimePattern::Hms));
    assert_eq!(t("h:mm:ss.00"), Some(TimePattern::HmsCs));
    assert_eq!(t("h:mm:ss.000"), Some(TimePattern::HmsMs));
    assert_eq!(t("h:mm AM/PM"), Some(TimePattern::Hm12));
    assert_eq!(t("h AM/PM"), Some(TimePattern::H12));
    assert_eq!(t("mm:ss"), Some(TimePattern::Ms));
    assert_eq!(t("mm:ss.0"), Some(TimePattern::MsCs));
    assert_eq!(t("yyyy-mm-dd"), None);
    // A date and a time in one code; the `m` after `h` is minutes and the first `m` a month.
    assert_eq!(p("m/d/yyyy h:mm"), (Some(DatePattern::Mdy), Some(TimePattern::Hm)));
    assert_eq!(p("yyyy-mm-dd hh:mm:ss"), (Some(DatePattern::Iso), Some(TimePattern::Hms)));
    assert_eq!(p("General"), (None, None));
}

#[test]
fn every_sheaf_date_and_time_pattern_has_an_excel_code_that_maps_back_where_excel_can_show_it() {
    for pat in DatePattern::ALL {
        let code = dates::date_code(pat);
        let (back, time) = dates::patterns_from_code(code);
        assert_eq!(time, None, "{pat:?} -> {code}");
        // Patterns Excel cannot tell apart (a quarter, a weekday letter) land on a near one.
        if !matches!(
            pat,
            DatePattern::Quarter
                | DatePattern::WeekdayLetter
                | DatePattern::MonthLetter
                | DatePattern::MonthShort
                | DatePattern::Year2
                | DatePattern::DayOnly
                | DatePattern::YearOnly
        ) {
            assert_eq!(back, Some(pat), "{pat:?} -> {code}");
        }
    }
    for pat in TimePattern::ALL {
        let code = dates::time_code(pat, false);
        assert!(!code.is_empty(), "{pat:?}");
    }
    assert_eq!(dates::patterns_from_code(dates::time_code(TimePattern::Hms, true)).1, Some(TimePattern::Hms));
}

#[test]
fn serial_numbers_follow_the_1900_system_and_come_back() {
    let day = |y, m, d| datetime::date_value(y, m, d).unwrap();
    assert_eq!(dates::serial_from_date(day(1900, 1, 1)), Some(1.0));
    assert_eq!(dates::serial_from_date(day(1900, 2, 28)), Some(59.0));
    assert_eq!(dates::serial_from_date(day(1900, 3, 1)), Some(61.0));
    assert_eq!(dates::serial_from_date(day(1970, 1, 1) + 0.25), Some(25569.25));
    assert_eq!(dates::serial_from_date(day(1899, 12, 31)), None);
    assert_eq!(dates::serial_from_date(f64::NAN), None);
}

// ----- rows, labels and dates that must come back as they were written -----------------------

fn round_trip(dir: &Path, name: &str, sheets: &[&Worksheet]) -> ExcelImport {
    let out = write_xlsx(sheets, &dir.join(name), &ExcelExportOptions::default()).unwrap();
    read_workbook(&out.path, &ExcelOptions::default()).unwrap()
}

#[test]
fn rows_without_a_value_between_data_rows_stay_rows_of_missing_values() {
    let dir = temp("blankrows");
    let x = vec![1.0, f64::NAN, f64::NAN, 4.0, 5.0];
    let one = sheet("One", vec![column(1, "A", Designation::X, ColumnData::numeric(x.clone()))]);
    let two = sheet(
        "Two",
        vec![
            column(1, "A", Designation::X, ColumnData::numeric(vec![1.0, f64::NAN, 3.0])),
            column(2, "B", Designation::Y, ColumnData::numeric(vec![f64::NAN, f64::NAN, 30.0])),
        ],
    );
    let r = round_trip(&dir, "gaps.xlsx", &[&one, &two]);
    assert!(same(&numbers_of_mixed(&r.workbook.sheets[0].columns[0]), &x));
    assert!(same(&numbers_of_mixed(&r.workbook.sheets[1].columns[0]), &[1.0, f64::NAN, 3.0]));
    assert!(same(&numbers_of_mixed(&r.workbook.sheets[1].columns[1]), &[f64::NAN, f64::NAN, 30.0]));
    assert_eq!(r.sheets[0].layout.as_ref().unwrap().rows, 5);

    // Row windows count the blank rows too: they are rows.
    let opts = ExcelOptions { skip_rows: 1, max_rows: Some(3), ..Default::default() };
    let part =
        read_workbook(&dir.join("gaps.xlsx"), &ExcelOptions { sheets: vec![SheetRef::Index(1)], ..opts }).unwrap();
    assert!(same(&numbers_of_mixed(&part.workbook.sheets[0].columns[0]), &[f64::NAN, f64::NAN, 4.0]));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn blank_rows_at_the_start_or_end_of_the_data_are_reported_by_the_writer() {
    let dir = temp("edgeblank");
    let s = sheet("S", vec![column(1, "A", Designation::X, ColumnData::numeric(vec![f64::NAN, 1.0, 2.0, f64::NAN]))]);
    let out = write_xlsx(&[&s], &dir.join("e.xlsx"), &ExcelExportOptions::default()).unwrap();
    assert!(out.warnings.iter().any(|w| w.contains("2 rows at the start or end")), "{:?}", out.warnings);
    let inner = sheet("S", vec![column(1, "A", Designation::X, ColumnData::numeric(vec![1.0, f64::NAN, 2.0]))]);
    let out = write_xlsx(&[&inner], &dir.join("i.xlsx"), &ExcelExportOptions::default()).unwrap();
    assert!(out.warnings.is_empty(), "{:?}", out.warnings);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn every_combination_of_labels_comes_back_in_its_own_field() {
    let dir = temp("labelcombos");
    for mask in 0..8u8 {
        let (long, units, comments) = (mask & 1 != 0, mask & 2 != 0, mask & 4 != 0);
        let mut cols = Vec::new();
        for (i, short) in ["A", "B"].into_iter().enumerate() {
            let c = column(i as u64 + 1, short, Designation::Y, ColumnData::numeric(vec![1.5, 2.5, 3.5]));
            let (l, u, k) = (format!("long{i}"), format!("unit{i}"), format!("note{i}"));
            cols.push(labelled(
                c,
                if long { &l } else { "" },
                if units { &u } else { "" },
                if comments { &k } else { "" },
            ));
        }
        let r = round_trip(&dir, &format!("labels{mask}.xlsx"), &[&sheet("S", cols.clone())]);
        let back = &r.workbook.sheets[0];
        for (a, b) in cols.iter().zip(&back.columns) {
            assert_eq!(
                (&b.long_name, &b.units, &b.comments),
                (&a.long_name, &a.units, &a.comments),
                "mask {mask}, column {}",
                a.short_name
            );
            assert_eq!(numbers(b), [1.5, 2.5, 3.5], "mask {mask}");
        }
        assert!(r.warnings.is_empty(), "mask {mask}: {:?}", r.warnings);
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// A sheet with one column of numbers in the given Excel number format, written without labels.
fn numbers_with_format(dir: &Path, name: &str, values: &[f64], code: &str) -> std::path::PathBuf {
    let mut book = rust_xlsxwriter::Workbook::new();
    let ws = book.add_worksheet();
    let format = rust_xlsxwriter::Format::new().set_num_format(code);
    for (i, v) in values.iter().enumerate() {
        ws.write_number_with_format(i as u32, 0, *v, &format).unwrap();
    }
    let path = dir.join(name);
    book.save(&path).unwrap();
    path
}

#[test]
fn a_date_column_holding_day_zero_or_a_time_of_day_stays_a_date_column() {
    let dir = temp("dayzero");
    let path = numbers_with_format(&dir, "d.xlsx", &[0.0, 43000.0, 43001.5, 0.25], "yyyy-mm-dd hh:mm:ss");
    let r = read_workbook(&path, &ExcelOptions::default()).unwrap();
    let col = &r.workbook.sheets[0].columns[0];
    assert_eq!(col.format, NumberFormat::DateTime(DatePattern::Iso, TimePattern::Hms), "{:?}", col.data);
    let v = numbers(col);
    let day = |y, m, d| datetime::date_value(y, m, d).unwrap();
    assert!((v[0] - day(1899, 12, 31)).abs() < 1e-9);
    assert!((v[1] - day(2017, 9, 22)).abs() < 1e-9);
    assert!((v[2] - (day(2017, 9, 23) + 0.5)).abs() < 1e-9);
    assert!((v[3] - (day(1899, 12, 31) + 0.25)).abs() < 1e-9);
    assert!(!warned(&r, "mixes dates"), "{:?}", r.warnings);

    // A time column that goes past one day keeps the count of days.
    let path = numbers_with_format(&dir, "t.xlsx", &[0.25, 1.5, 0.75], "h:mm:ss");
    let r = read_workbook(&path, &ExcelOptions::default()).unwrap();
    let col = &r.workbook.sheets[0].columns[0];
    assert_eq!(col.format, NumberFormat::Time(TimePattern::Hms), "{:?}", col.data);
    assert_eq!(numbers(col), [0.25, 1.5, 0.75]);

    // Without a number format a column of fractions is a time column and a column with dates is a
    // date column.
    let mut book = rust_xlsxwriter::Workbook::new();
    let ws = book.add_worksheet();
    for (i, v) in [0.5, 0.25, 0.75].iter().enumerate() {
        ws.write_number(i as u32, 0, *v).unwrap();
    }
    book.save(dir.join("plain.xlsx")).unwrap();
    let r = read_workbook(&dir.join("plain.xlsx"), &ExcelOptions::default()).unwrap();
    assert_eq!(r.workbook.sheets[0].columns[0].format, NumberFormat::General);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_time_column_with_negative_values_round_trips() {
    let dir = temp("negtime");
    for values in [vec![-0.5, 0.25, 0.75], vec![-2.5, 0.25, 1.5], vec![-0.25, f64::NAN, -0.75]] {
        let mut c = column(1, "A", Designation::Y, ColumnData::numeric(values.clone()));
        c.format = NumberFormat::Time(TimePattern::Hms);
        let r = round_trip(&dir, "neg.xlsx", &[&sheet("S", vec![c])]);
        let back = &r.workbook.sheets[0].columns[0];
        assert_eq!(back.format, NumberFormat::Time(TimePattern::Hms), "{values:?}: {:?}", back.data);
        assert!(same(&numbers(back), &values), "{values:?}: {:?}", back.data);
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn weekday_and_compact_date_patterns_come_back_as_they_were_written() {
    use dates::patterns_from_code as p;
    assert_eq!(p("ddd").0, Some(DatePattern::WeekdayShort));
    assert_eq!(p("ddd, d mmm").0, Some(DatePattern::MonthDay));
    assert_eq!(p("d").0, Some(DatePattern::DayOnly));
    assert_eq!(p("dddd").0, Some(DatePattern::LongDate));
    assert_eq!(p("yymmdd").0, Some(DatePattern::Yymmdd));
    assert_eq!(p("yy-mm-dd").0, Some(DatePattern::Iso));
    let dir = temp("patterns");
    let day = datetime::date_value(2026, 11, 4).unwrap();
    for pattern in [DatePattern::WeekdayShort, DatePattern::Yymmdd, DatePattern::Iso] {
        let mut c = column(1, "A", Designation::Y, ColumnData::numeric(vec![day, day + 1.0]));
        c.format = NumberFormat::Date(pattern);
        let r = round_trip(&dir, "p.xlsx", &[&sheet("S", vec![c])]);
        assert_eq!(r.workbook.sheets[0].columns[0].format, NumberFormat::Date(pattern));
    }
    let _ = std::fs::remove_dir_all(&dir);
}

// ----- damaged and hostile files ---------------------------------------------------------------

#[test]
fn a_panic_inside_the_workbook_reader_becomes_an_error_that_names_the_file() {
    let e = guarded::<()>(Path::new("broken.xls"), || panic!("index out of range")).unwrap_err();
    assert!(matches!(e, IoError::ExcelRead { .. }), "{e}");
    assert!(e.to_string().contains("broken.xls") && e.to_string().contains("damaged"), "{e}");
}

#[test]
fn a_legacy_file_with_damaged_bytes_gives_an_error_or_data_but_never_a_panic() {
    let dir = temp("fuzz");
    let original = std::fs::read(fixture("legacy.xls")).unwrap();
    let mut state = 0x2545_F491_4F6C_DD1Du64;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    let path = dir.join("damaged.xls");
    for _ in 0..400 {
        let mut bytes = original.clone();
        for _ in 0..4 {
            let at = (next() % bytes.len() as u64) as usize;
            bytes[at] = (next() & 0xFF) as u8;
        }
        std::fs::write(&path, &bytes).unwrap();
        // Any answer is fine as long as the call returns.
        let _ = read_workbook(&path, &ExcelOptions::default());
        let _ = list_sheets(&path);
        let _ = preview_sheet(&path, &SheetRef::Index(1), &ExcelOptions::default(), 5);
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_sheet_that_spans_a_huge_area_is_refused_before_anything_is_allocated() {
    let dir = temp("farcells");
    let mut book = rust_xlsxwriter::Workbook::new();
    let ws = book.add_worksheet();
    ws.write_number(0, 0, 1.0).unwrap();
    ws.write_number(1_048_575, 16_383, 2.0).unwrap();
    book.save(dir.join("far.xlsx")).unwrap();
    let e = read_workbook(&dir.join("far.xlsx"), &ExcelOptions::default()).unwrap_err();
    assert!(e.to_string().contains("million cells"), "{e}");
    let e = preview_sheet(&dir.join("far.xlsx"), &SheetRef::Index(1), &ExcelOptions::default(), 5).unwrap_err();
    assert!(e.to_string().contains("million cells"), "{e}");
    // A tall sheet within the limit is fine.
    let mut book = rust_xlsxwriter::Workbook::new();
    let ws = book.add_worksheet();
    ws.write_number(0, 0, 1.0).unwrap();
    ws.write_number(999_999, 3, 2.0).unwrap();
    book.save(dir.join("tall.xlsx")).unwrap();
    assert!(read_workbook(&dir.join("tall.xlsx"), &ExcelOptions::default()).is_ok());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_workbook_whose_parts_unpack_to_far_more_than_they_weigh_is_refused() {
    use std::io::Write;
    let dir = temp("zipbomb");
    let path = dir.join("bomb.xlsx");
    let mut zip = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
    let options = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    zip.start_file("xl/worksheets/sheet1.xml", options).unwrap();
    let chunk = vec![b' '; 1 << 20];
    for _ in 0..80 {
        zip.write_all(&chunk).unwrap();
    }
    zip.finish().unwrap();
    assert!(std::fs::metadata(&path).unwrap().len() < 1 << 20);
    let e = read_workbook(&path, &ExcelOptions::default()).unwrap_err();
    assert!(matches!(e, IoError::ExcelRead { .. }) && e.to_string().contains("far more than a workbook"), "{e}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn text_named_like_a_workbook_is_read_as_text_and_a_damaged_workbook_is_an_error() {
    let dir = temp("textxls");
    std::fs::write(dir.join("text.xls"), "x\ty\n1\t2\n3\t4\n5\t6\n").unwrap();
    assert!(is_excel_file(&dir.join("text.xls")) && !is_excel_workbook(&dir.join("text.xls")));
    let (chi, ascii) = (chi::ChiOptions::default(), ascii::AsciiOptions::default());
    let r = import_file(&dir.join("text.xls"), ImportKind::Auto, &chi, &ascii).unwrap();
    assert_eq!(r.format, "ascii");
    assert_eq!(numbers(&r.workbook.sheets[0].columns[1]), [2.0, 4.0, 6.0]);
    assert!(r.warnings.iter().any(|w| w.contains("holds text")), "{:?}", r.warnings);
    // A real workbook, by its bytes, under any of the three names.
    for name in ["a.xlsx", "b.xlsm", "c.xls"] {
        std::fs::copy(fixture("basic.xlsx"), dir.join(name)).unwrap();
        assert!(is_excel_workbook(&dir.join(name)), "{name}");
    }
    std::fs::copy(fixture("legacy.xls"), dir.join("old.xls")).unwrap();
    assert!(is_excel_workbook(&dir.join("old.xls")));
    // A file that starts like a zip but is cut short is still an Excel error, not text.
    std::fs::write(dir.join("cut.xlsx"), b"PK\x03\x04 not really").unwrap();
    let e = import_file(&dir.join("cut.xlsx"), ImportKind::Auto, &chi, &ascii).unwrap_err();
    assert!(matches!(e, IoError::ExcelRead { .. }), "{e}");
    assert!(!is_excel_workbook(&dir.join("missing.xlsx")));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn sheets_named_with_digits_are_chosen_by_name_before_position() {
    let dir = temp("digitnames");
    let one = |name: &str, v: f64| sheet(name, vec![column(1, "A", Designation::X, ColumnData::numeric(vec![v]))]);
    let sheets = [one("2024", 24.0), one("2023", 23.0), one("1", 100.0)];
    let refs: Vec<&Worksheet> = sheets.iter().collect();
    let out = write_xlsx(&refs, &dir.join("y.xlsx"), &ExcelExportOptions::default()).unwrap();
    let pick = |list: &str| {
        let o = ExcelOptions { sheets: SheetRef::parse_list(list), ..Default::default() };
        let r = read_workbook(&out.path, &o).unwrap();
        r.workbook.sheets.iter().map(|s| (s.name.clone(), numbers(&s.columns[0])[0])).collect::<Vec<_>>()
    };
    assert_eq!(pick("2023"), [("2023".to_string(), 23.0)]);
    // "1" is a sheet's name, so it is that sheet and not the one at position 1.
    assert_eq!(pick("1"), [("1".to_string(), 100.0)]);
    // No sheet has the name "2": it is a position.
    assert_eq!(pick("2"), [("2023".to_string(), 23.0)]);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn sheet_names_are_cut_by_utf16_units_and_compared_by_unicode_case() {
    let dir = temp("unicodenames");
    let one = |name: &str| sheet(name, vec![column(1, "A", Designation::X, ColumnData::numeric(vec![1.0]))]);
    let emoji = "😀".repeat(31);
    let sheets = [one(&emoji), one("Ärger"), one("ärger"), one(&"é".repeat(31)), one(&"É".repeat(31))];
    let refs: Vec<&Worksheet> = sheets.iter().collect();
    let out = write_xlsx(&refs, &dir.join("u.xlsx"), &ExcelExportOptions::default()).unwrap();
    for n in &out.sheet_names {
        assert!(n.encode_utf16().count() <= 31, "{n}");
    }
    assert_eq!(out.sheet_names[0].chars().count(), 15);
    let mut lower: Vec<String> = out.sheet_names.iter().map(|n| n.to_lowercase()).collect();
    lower.sort();
    lower.dedup();
    assert_eq!(lower.len(), out.sheet_names.len(), "{:?}", out.sheet_names);
    let names: Vec<String> = list_sheets(&out.path).unwrap().into_iter().map(|s| s.name).collect();
    assert_eq!(names, out.sheet_names);
    let _ = std::fs::remove_dir_all(&dir);
}
