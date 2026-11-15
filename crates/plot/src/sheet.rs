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
            PrintOp::Text { x, y, max_w, text, align, bold, size } => {
                let font =
                    Font { family: FAMILY.into(), size_pt: *size, bold: *bold, italic: false, color: Color::BLACK };
                let text = cut(text, *max_w, &font, *size, m);
                if text.is_empty() {
                    continue;
                }
                let anchor = match align {
                    Align::Left => Anchor::Start,
                    Align::Center => Anchor::Middle,
                    Align::Right => Anchor::End,
                };
                items.push(Item::Text(TextItem {
                    x: x + dx,
                    y: y + dy,
                    text,
                    family: FAMILY.into(),
                    size: *size,
                    bold: *bold,
                    italic: false,
                    color: Color::BLACK,
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

/// How many pages the sheet prints on and the printed area.
pub fn sheet_pages(sheet: &Worksheet, setup: &PageSetup) -> Result<SheetPages, PlotError> {
    let area = check(setup)?;
    let table = SheetTable::new(sheet, PrintStyle::default());
    Ok(SheetPages { area, count: table.pages(area.0, area.1).len() })
}

/// One metafile per page, each the size of the printed area; the printer path places it inside
/// the margins ([`sheaf_core::page_setup::placement`]), at scale 1 when the printer can reach the
/// margins.
pub fn render_sheet_emf_pages(sheet: &Worksheet, setup: &PageSetup) -> Result<Vec<Vec<u8>>, PlotError> {
    let area = check(setup)?;
    let fonts = SystemFonts::shared();
    let table = SheetTable::new(sheet, PrintStyle::default());
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
    let table = SheetTable::new(sheet, PrintStyle::default());
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
    let table = SheetTable::new(sheet, PrintStyle::default());
    let pages = table.pages(area.0, area.1);
    let slot = pages.get(page).ok_or_else(|| PlotError::Size(format!("the sheet has no page {}", page + 1)))?;
    let scene = scene_of(&table.draw_page(slot, pages.len()), area, (0.0, 0.0), fonts);
    raster::rasterise(&svg::write_svg(&scene), pixels_per_point, fonts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sheaf_core::column::{Column, ColumnData, Designation};
    use sheaf_core::ids::ColumnId;
    use sheaf_core::page_setup::{Margins, Orientation, Paper};

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

    #[test]
    fn long_text_is_cut_to_its_cell() {
        let m = crate::fonts::FixedMetrics;
        let font = Font::new("Arial", 8.0);
        // 0.55 * 8 = 4.4 pt a character: 20 pt hold four.
        assert_eq!(cut("abcdefgh", 20.0, &font, 8.0, &m), "abcd");
        assert_eq!(cut("ab", 20.0, &font, 8.0, &m), "ab");
    }
}
