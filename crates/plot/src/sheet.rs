//! A printed worksheet (File > Print with a workbook active): the drawing instructions of
//! `sheaf_core::sheet_print` as scenes, so the table goes through the same EMF, SVG and PDF writers
//! as a graph page. See `docs/spec/graphing.md`, section "Printing".

use crate::fonts::{SystemFonts, TextMeasurer};
use crate::scene::{Anchor, Item, LayerScene, Scene, Stroke, TextItem};
use crate::{PlotError, Raster, Rect, emf, multipage, raster, svg};
use sheaf_core::Color;
use sheaf_core::graph::Font;
use sheaf_core::page_setup::PageSetup;
use sheaf_core::sheet_print::{Align, PrintOp, PrintStyle, SheetTable, print_area};
use sheaf_core::worksheet::Worksheet;

mod image_formats;
pub use image_formats::sheet_image_bytes;

const FAMILY: &str = "Arial";

/// What a printed sheet comes to: the pages of the table and the size of the printed area (the
/// paper inside the margins), in points.
pub struct SheetPages {
    pub area: (f64, f64),
    pub count: usize,
}

fn cut(text: &str, max_w: f64, font: &Font, size: f64, m: &dyn TextMeasurer) -> String {
    if m.width(text, font, size) <= max_w {
        return text.to_string();
    }
    // The width grows with the prefix, so the longest fitting prefix is found by bisection
    // (a long cell must not cost a measurement per character).
    let ends: Vec<usize> = text.char_indices().map(|(i, _)| i).skip(1).chain(std::iter::once(text.len())).collect();
    let (mut lo, mut hi) = (0usize, ends.len()); // chars kept: lo fits, hi is the whole text
    while lo < hi {
        let mid = (lo + hi).div_ceil(2);
        if m.width(&text[..ends[mid - 1]], font, size) <= max_w {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    if lo == 0 { String::new() } else { text[..ends[lo - 1]].to_string() }
}

/// The scene of `ops` on a page `w` x `h` points with the content moved by `(dx, dy)`.
fn scene_of(ops: &[PrintOp], size: (f64, f64), offset: (f64, f64), m: &dyn TextMeasurer) -> Scene {
    let (dx, dy) = offset;
    let mut items = Vec::with_capacity(ops.len());
    for op in ops {
        match op {
            PrintOp::Fill { x, y, w, h, color } => items.push(Item::Rect {
                rect: Rect { x: x + dx, y: y + dy, w: *w, h: *h },
                fill: Some(*color),
                stroke: None,
            }),
            PrintOp::Line { x1, y1, x2, y2, width, color } => {
                let mut d = String::new();
                crate::scene::move_to(&mut d, x1 + dx, y1 + dy);
                crate::scene::line_to(&mut d, x2 + dx, y2 + dy);
                items.push(Item::Path { d, fill: None, stroke: Some(Stroke::solid(*color, *width)) });
            }
            PrintOp::Text { x, y, max_w, text, align, bold, size, fmt } => {
                let family = fmt.family.as_deref().unwrap_or(FAMILY);
                let color = fmt.color.unwrap_or(Color::BLACK);
                let font = Font { family: family.into(), size_pt: *size, bold: *bold, italic: fmt.italic, color };
                let text = cut(text, *max_w, &font, *size, m);
                if text.is_empty() {
                    continue;
                }
                let anchor = match align {
                    Align::Left => Anchor::Start,
                    Align::Center => Anchor::Middle,
                    Align::Right => Anchor::End,
                };
                if fmt.underline {
                    // A line under the text, a little below its baseline.
                    let w = m.width(&text, &font, *size);
                    let x0 = match align {
                        Align::Left => *x,
                        Align::Center => x - w / 2.0,
                        Align::Right => x - w,
                    };
                    let (ly, thick) = (y + dy + size * 0.12 + 0.6, (size / 16.0).max(0.4));
                    let mut d = String::new();
                    crate::scene::move_to(&mut d, x0 + dx, ly);
                    crate::scene::line_to(&mut d, x0 + w + dx, ly);
                    items.push(Item::Path { d, fill: None, stroke: Some(Stroke::solid(color, thick)) });
                }
                items.push(Item::Text(TextItem {
                    x: x + dx,
                    y: y + dy,
                    text,
                    family: family.into(),
                    size: *size,
                    bold: *bold,
                    italic: fmt.italic,
                    color,
                    anchor,
                    rotate_ccw: 0.0,
                }));
            }
        }
    }
    let full = Rect { x: 0.0, y: 0.0, w: size.0, h: size.1 };
    Scene {
        width: size.0,
        height: size.1,
        background: Color::WHITE,
        layers: vec![LayerScene {
            clip: full,
            grid: Vec::new(),
            plots: items,
            overlay: Vec::new(),
            underlay_len: 0,
            data_on_top: false,
            objects: Vec::new(),
        }],
    }
}

fn check(setup: &PageSetup) -> Result<(f64, f64), PlotError> {
    let m = setup.margins;
    if ![m.left, m.top, m.right, m.bottom].iter().all(|v| v.is_finite() && *v >= 0.0) {
        return Err(PlotError::Size("invalid print margins".into()));
    }
    let area = print_area(setup);
    if !(area.0 > 0.0 && area.1 > 0.0) {
        return Err(PlotError::Size("print margins leave no space on the paper".into()));
    }
    Ok(area)
}

fn sheet_table(sheet: &Worksheet) -> SheetTable<'_> {
    SheetTable::with_metrics(sheet, PrintStyle::default(), &|style, size| {
        let em = crate::worksheet_font(style.font.unwrap_or(FAMILY), style.bold, style.italic)
            .map_or(1.2, |f| f.line_height_em);
        size * em + 2.0
    })
}

/// Where the sheet's printed pages break on the paper of `setup` (Page Break Preview Lines, T185).
pub fn sheet_page_breaks(
    sheet: &Worksheet,
    setup: &PageSetup,
) -> Result<sheaf_core::sheet_print::PageBreaks, PlotError> {
    let area = check(setup)?;
    Ok(sheet_table(sheet).page_breaks(area.0, area.1))
}

/// How many pages the sheet prints on and the printed area.
pub fn sheet_pages(sheet: &Worksheet, setup: &PageSetup) -> Result<SheetPages, PlotError> {
    let area = check(setup)?;
    let table = sheet_table(sheet);
    Ok(SheetPages { area, count: table.pages(area.0, area.1).len() })
}

/// One metafile per page, each the size of the printed area; the printer path places it inside
/// the margins ([`sheaf_core::page_setup::placement`]), at scale 1 when the printer can reach the
/// margins.
pub fn render_sheet_emf_pages(sheet: &Worksheet, setup: &PageSetup) -> Result<Vec<Vec<u8>>, PlotError> {
    let area = check(setup)?;
    let fonts = SystemFonts::shared();
    let table = sheet_table(sheet);
    let pages = table.pages(area.0, area.1);
    Ok(pages
        .iter()
        .map(|p| emf::write_emf(&scene_of(&table.draw_page(p, pages.len()), area, (0.0, 0.0), fonts), 1.0, fonts))
        .collect())
}

fn paper_svgs<'a>(
    table: &'a SheetTable<'a>,
    setup: &'a PageSetup,
    area: (f64, f64),
) -> impl Iterator<Item = Result<String, PlotError>> + 'a {
    let fonts = SystemFonts::shared();
    let pages = table.pages(area.0, area.1);
    let n = pages.len();
    let (pw, ph) = setup.paper_pt(0.0, 1.0);
    pages.into_iter().map(move |p| {
        let scene = scene_of(&table.draw_page(&p, n), (pw, ph), (setup.margins.left, setup.margins.top), fonts);
        Ok(svg::write_svg(&scene))
    })
}

/// The sheet as a vector PDF, one paper-sized page per table page, the table inside the margins.
pub fn render_sheet_pdf(sheet: &Worksheet, setup: &PageSetup) -> Result<Vec<u8>, PlotError> {
    let area = check(setup)?;
    let table = sheet_table(sheet);
    multipage::svgs_to_pdf(paper_svgs(&table, setup, area), SystemFonts::shared())
}

/// One page of the printed area as pixels (Print Preview), `pixels_per_point` 1 = 72 dpi.
/// `page` counts from 0.
pub fn render_sheet_raster(
    sheet: &Worksheet,
    setup: &PageSetup,
    page: usize,
    pixels_per_point: f64,
) -> Result<Raster, PlotError> {
    let area = check(setup)?;
    let fonts = SystemFonts::shared();
    let table = sheet_table(sheet);
    let pages = table.pages(area.0, area.1);
    let slot = pages.get(page).ok_or_else(|| PlotError::Size(format!("the sheet has no page {}", page + 1)))?;
    let scene = scene_of(&table.draw_page(slot, pages.len()), area, (0.0, 0.0), fonts);
    raster::rasterise(&svg::write_svg(&scene), pixels_per_point, fonts)
}

/// File > Export > Worksheet as Image (T97): the sheet as Origin 2016's `expWks` draws it, measured
/// 2026-11-16 (`docs/spec/workbook-model.md` 13k): the label rows filled light yellow and the cells,
/// without the column heads, the row numbers or grid lines, one picture for the whole sheet. Built from
/// the printed table (one page holding every row and column) with those parts left out.
pub fn sheet_image_svg(sheet: &Worksheet) -> Result<String, PlotError> {
    let fonts = SystemFonts::shared();
    let table = sheet_table(sheet);
    let style = PrintStyle::default();
    let pages = table.pages(1.0e9, 1.0e9);
    let slot = pages.first().ok_or_else(|| PlotError::Size("the sheet has nothing to draw".into()))?;
    let (dx, dy) = (style.rownum_w, style.row_h);
    let head_fill = Color::rgb(222, 222, 222);
    let mut ops = Vec::new();
    let (mut w, mut h) = (0.0_f64, 0.0_f64);
    for op in table.draw_page(slot, 1) {
        match op {
            PrintOp::Line { .. } => {}
            PrintOp::Fill { color, .. } if color == head_fill => {}
            PrintOp::Fill { x, y, w: fw, h: fh, color } => {
                w = w.max(x + fw - dx);
                h = h.max(y + fh - dy);
                ops.push(PrintOp::Fill { x: x - dx, y: y - dy, w: fw, h: fh, color });
            }
            // Row numbers and label row titles stand left of the first column; the head names on its row.
            PrintOp::Text { x, y, .. } if x <= dx || y <= dy => {}
            PrintOp::Text { x, y, max_w, text, align, bold, size, fmt } => {
                h = h.max(y - dy + size * 0.4);
                ops.push(PrintOp::Text { x: x - dx, y: y - dy, max_w, text, align, bold, size, fmt });
            }
        }
    }
    let width = w.max(table.column_widths().iter().sum::<f64>()).max(1.0);
    let height = h.max(table.head_height() - dy).max(1.0);
    Ok(svg::write_svg(&scene_of(&ops, (width, height), (0.0, 0.0), fonts)))
}

/// The worksheet image as PNG at `dpi` (the resolution is recorded in the file).
pub fn sheet_image_png(sheet: &Worksheet, dpi: f64) -> Result<Vec<u8>, PlotError> {
    let svg = sheet_image_svg(sheet)?;
    let r = raster::rasterise(&svg, dpi / 72.0, SystemFonts::shared())?;
    raster::encode_png(&r, dpi)
}

/// The worksheet image as a one-page vector PDF.
pub fn sheet_image_pdf(sheet: &Worksheet) -> Result<Vec<u8>, PlotError> {
    raster::to_pdf(&sheet_image_svg(sheet)?, SystemFonts::shared())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sheaf_core::column::{Column, ColumnData, Designation};
    use sheaf_core::ids::ColumnId;
    use sheaf_core::page_setup::{Margins, Orientation, Paper};

    #[test]
    fn the_worksheet_image_has_the_labels_and_cells_but_no_heads_numbers_or_grid() {
        // Origin's expWks picture: label rows (yellow) and cells; no "A(X)" heads, row numbers or lines.
        let mut ws = Worksheet::new("Sheet1");
        let mut b = Column::new(ColumnId(2), "B", Designation::Y, ColumnData::numeric(vec![10.0, 20.0]));
        b.long_name = "Bee".into();
        b.units = "mA".into();
        ws.columns.push(Column::new(ColumnId(1), "A", Designation::X, ColumnData::numeric(vec![7.0, 8.0])));
        ws.columns.push(b);
        let svg = sheet_image_svg(&ws).unwrap();
        for want in [">Bee<", ">mA<", ">10<", ">20<", ">7<"] {
            assert!(svg.contains(want), "{want} missing");
        }
        for not in [">A(X)<", ">B(Y)<", ">Long Name<", ">Units<", ">1<", ">2<"] {
            assert!(!svg.contains(not), "{not} drawn");
        }
        assert!(!svg.contains("<line") && !svg.contains("<path"), "no grid lines");
        assert!(
            svg.contains("#FFFFD6") || svg.contains("#ffffd6") || svg.contains("255,255,214"),
            "the yellow label fill"
        );
        let png = sheet_image_png(&ws, 96.0).unwrap();
        assert_eq!(&png[1..4], b"PNG");
        assert!(sheet_image_pdf(&ws).unwrap().starts_with(b"%PDF"));
    }

    #[test]
    fn cut_keeps_the_longest_prefix_that_fits() {
        let f = Font { family: FAMILY.into(), size_pt: 10.0, bold: false, italic: false, color: Color::BLACK };
        let m = crate::fonts::FixedMetrics; // 5.5 pt per char
        assert_eq!(cut("abc", 100.0, &f, 10.0, &m), "abc");
        assert_eq!(cut("abcdefghij", 16.6, &f, 10.0, &m), "abc");
        assert_eq!(cut("abcdefghij", 5.0, &f, 10.0, &m), "");
        assert_eq!(cut("aébcd", 16.5, &f, 10.0, &m), "aéb");
        let long = "x".repeat(2_000_000);
        assert_eq!(cut(&long, 55.0, &f, 10.0, &m).len(), 10);
    }

    /// 130 rows, three columns, labels on two of them: 3 pages on Letter portrait.
    fn sheet() -> Worksheet {
        let mut ws = Worksheet::new("Sheet1");
        let n = 130;
        ws.columns = vec![
            Column::new(ColumnId(1), "A", Designation::X, ColumnData::numeric((1..=n).map(f64::from).collect()))
                .with_labels("Index", "n", ""),
            Column::new(
                ColumnId(2),
                "B",
                Designation::Y,
                ColumnData::numeric((1..=n).map(|i| f64::from(i).sqrt()).collect()),
            )
            .with_labels("Root", "", "of the index"),
            Column::new(
                ColumnId(3),
                "C",
                Designation::Y,
                ColumnData::text((1..=n).map(|i| format!("row {i}")).collect()),
            ),
        ];
        ws
    }

    fn setup() -> PageSetup {
        PageSetup { paper: Paper::Letter, orientation: Orientation::Portrait, margins: Margins::all(72.0) }
    }

    #[test]
    fn the_sheet_has_the_pages_its_rows_need() {
        let p = sheet_pages(&sheet(), &setup()).unwrap();
        assert_eq!(p.area, (468.0, 648.0));
        assert_eq!(p.count, 3);
    }

    #[test]
    fn margins_that_leave_no_room_are_refused() {
        let s = PageSetup { margins: Margins::all(400.0), ..setup() };
        assert!(sheet_pages(&sheet(), &s).is_err());
        assert!(render_sheet_pdf(&sheet(), &s).is_err());
    }

    #[test]
    fn one_metafile_per_page_of_the_printed_area() {
        let pages = render_sheet_emf_pages(&sheet(), &setup()).unwrap();
        assert_eq!(pages.len(), 3);
        // An enhanced metafile starts with an EMR_HEADER record (type 1) and has " EMF" at 40.
        for p in &pages {
            assert_eq!(u32::from_le_bytes(p[0..4].try_into().unwrap()), 1);
            assert_eq!(&p[40..44], b" EMF");
        }
        // The last page holds fewer rows, so its metafile is smaller than the first.
        assert!(pages[2].len() < pages[0].len());
    }

    #[test]
    fn the_pdf_has_a_paper_sized_page_per_table_page() {
        let pdf = render_sheet_pdf(&sheet(), &setup()).unwrap();
        assert!(pdf.starts_with(b"%PDF"));
        let text = String::from_utf8_lossy(&pdf);
        assert_eq!(text.matches("/Type /Page\n").count() + text.matches("/Type /Page ").count(), 3, "pages");
        assert!(text.contains("612") && text.contains("792"), "Letter portrait media box");
    }

    #[test]
    fn a_preview_page_is_the_printed_area_and_has_ink() {
        let r = render_sheet_raster(&sheet(), &setup(), 0, 1.0).unwrap();
        assert_eq!((r.width, r.height), (468, 648));
        assert!(r.rgba.chunks(4).any(|p| p[0] < 100), "grid and text are drawn");
        assert!(render_sheet_raster(&sheet(), &setup(), 3, 1.0).is_err(), "no fourth page");
    }

    fn styled_sheet() -> Worksheet {
        use sheaf_core::cellformat::{CellFormat, FormatTarget, HAlign};
        let mut ws = sheet();
        ws.apply_cell_format(
            FormatTarget::cells(vec![ColumnId(3)], 0, Some(0)),
            CellFormat {
                bold: Some(true),
                italic: Some(true),
                underline: Some(true),
                font: Some("Courier New".into()),
                size_pt: Some(18.0),
                text_color: Some(Color::rgb(200, 0, 0)),
                fill: Some(Color::rgb(0, 200, 0)),
                h_align: Some(HAlign::Center),
                rich_text: None,
            },
        );
        ws
    }

    #[test]
    fn a_styled_cell_becomes_styled_text_an_underline_and_a_fill_in_the_scene() {
        let ws = styled_sheet();
        let table = SheetTable::new(&ws, PrintStyle::default());
        let area = (468.0, 648.0);
        let pages = table.pages(area.0, area.1);
        let scene = scene_of(&table.draw_page(&pages[0], pages.len()), area, (0.0, 0.0), &crate::fonts::FixedMetrics);
        let items = &scene.layers[0].plots;
        let text = items
            .iter()
            .find_map(|i| match i {
                Item::Text(t) if t.text == "row 1" => Some(t),
                _ => None,
            })
            .expect("the formatted cell is drawn");
        assert!(text.bold && text.italic);
        assert_eq!(
            (text.family.as_str(), text.color, text.anchor),
            ("Courier New", Color::rgb(200, 0, 0), Anchor::Middle)
        );
        assert_eq!(text.size, 18.0 * 8.0 / 9.0);
        let plain = items
            .iter()
            .find_map(|i| match i {
                Item::Text(t) if t.text == "row 2" => Some(t),
                _ => None,
            })
            .unwrap();
        assert!(!plain.bold && !plain.italic && plain.family == FAMILY && plain.color == Color::BLACK);
        // The underline is a red line just below the baseline, as wide as the text.
        let line = items
            .iter()
            .find(|i| matches!(i, Item::Path { stroke: Some(s), .. } if s.color == Color::rgb(200, 0, 0)))
            .expect("underline");
        let Item::Path { d, .. } = line else { unreachable!() };
        assert!(d.starts_with('M'), "{d}");
        assert!(
            items.iter().any(|i| matches!(i, Item::Rect { fill: Some(f), .. } if *f == Color::rgb(0, 200, 0))),
            "the cell fill"
        );
    }

    #[test]
    fn a_styled_sheet_prints_to_pdf_and_a_preview_with_the_fill_in_it() {
        let ws = styled_sheet();
        let pdf = render_sheet_pdf(&ws, &setup()).unwrap();
        assert!(pdf.starts_with(b"%PDF"));
        let r = render_sheet_raster(&ws, &setup(), 0, 2.0).unwrap();
        let green = r.rgba.chunks(4).filter(|p| p[0] < 10 && p[1] > 190 && p[2] < 10).count();
        assert!(green > 100, "the green cell fill is in the preview ({green} pixels)");
        let plain = render_sheet_raster(&sheet(), &setup(), 0, 2.0).unwrap();
        assert_eq!(plain.rgba.chunks(4).filter(|p| p[0] < 10 && p[1] > 190 && p[2] < 10).count(), 0);
    }

    #[test]
    fn long_text_is_cut_to_its_cell() {
        let m = crate::fonts::FixedMetrics;
        let font = Font::new("Arial", 8.0);
        // 0.55 * 8 = 4.4 pt a character: 20 pt hold four.
        assert_eq!(cut("abcdefgh", 20.0, &font, 8.0, &m), "abcd");
        assert_eq!(cut("ab", 20.0, &font, 8.0, &m), "ab");
    }
}
