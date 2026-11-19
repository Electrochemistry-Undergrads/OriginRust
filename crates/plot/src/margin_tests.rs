//! Tests of margin control and export size units (spec graphing.md 9.2 and 9.3).

use super::*;
use crate::scene::{Anchor, Item, LayerScene, Scene, Stroke, TextItem};
use sheaf_core::template::Template;
use sheaf_core::{Color, ColumnData, Designation};

fn stroke(w: f64) -> Option<Stroke> {
    Some(Stroke::solid(Color::BLACK, w))
}

fn line(d: &str, w: f64) -> Item {
    Item::Path { d: d.into(), fill: None, stroke: stroke(w) }
}

/// A 100 x 80 pt page with a layer whose frame is (10, 10, 60 x 50).
fn scene(plots: Vec<Item>, overlay: Vec<Item>) -> Scene {
    Scene {
        width: 100.0,
        height: 80.0,
        background: Color::WHITE,
        layers: vec![LayerScene {
            clip: Rect { x: 10.0, y: 10.0, w: 60.0, h: 50.0 },
            grid: vec![],
            plots,
            overlay,
            underlay_len: 0,
            data_on_top: false,
            objects: vec![],
        }],
    }
}

fn near(r: Rect, x: f64, y: f64, w: f64, h: f64) {
    let ok = (r.x - x).abs() < 1e-6 && (r.y - y).abs() < 1e-6 && (r.w - w).abs() < 1e-6 && (r.h - h).abs() < 1e-6;
    assert!(ok, "{r:?} is not ({x}, {y}, {w} x {h})");
}

fn view(s: &Scene, margin: Margin, border_px: Option<f64>, dpi: Option<f64>) -> Rect {
    ExportLayout { margin, border_px }.view(s, &FixedMetrics, dpi)
}

#[test]
fn margin_modes_parse_and_auto_is_border() {
    for (text, m) in [
        ("auto", Margin::Auto),
        ("Border", Margin::Border),
        ("tight", Margin::Tight),
        ("PAGE", Margin::Page),
        ("tight-in-page", Margin::TightInPage),
        ("tight_in_page", Margin::TightInPage),
    ] {
        assert_eq!(Margin::parse(text), Some(m), "{text}");
        assert_eq!(Margin::parse(m.name()), Some(m));
    }
    assert_eq!(Margin::parse("wide"), None);
    assert_eq!(Margin::Auto.resolved(), Margin::Border);
    assert_eq!(Margin::default(), Margin::Page, "the library default stays the whole page");
}

#[test]
fn page_is_the_whole_page_and_tight_is_the_box_round_what_is_drawn() {
    // A 2 pt line from (20, 20) to (40, 40), and a text far off to the right of it.
    let s = scene(vec![line("M20 20L40 40", 2.0)], vec![]);
    near(view(&s, Margin::Page, None, None), 0.0, 0.0, 100.0, 80.0);
    near(view(&s, Margin::Tight, None, None), 19.0, 19.0, 22.0, 22.0);
    near(view(&s, Margin::TightInPage, None, None), 19.0, 19.0, 22.0, 22.0);
}

#[test]
fn border_adds_the_width_in_pixels_of_the_export() {
    let s = scene(vec![line("M20 20L40 40", 2.0)], vec![]);
    // The default width, in points.
    near(view(&s, Margin::Border, None, None), 19.0 - 5.0, 19.0 - 5.0, 22.0 + 10.0, 22.0 + 10.0);
    // A negative width is the default too (Origin's -1).
    near(view(&s, Margin::Border, Some(-1.0), None), 14.0, 14.0, 32.0, 32.0);
    // 10 px at 300 dpi is 2.4 pt; for a vector format a pixel is a point.
    near(view(&s, Margin::Border, Some(10.0), Some(300.0)), 19.0 - 2.4, 19.0 - 2.4, 22.0 + 4.8, 22.0 + 4.8);
    near(view(&s, Margin::Border, Some(10.0), None), 9.0, 9.0, 42.0, 42.0);
    // Auto is Border.
    near(view(&s, Margin::Auto, None, None), 14.0, 14.0, 32.0, 32.0);
}

#[test]
fn tight_counts_what_hangs_over_the_page_edge_and_tight_in_page_does_not() {
    // A box mostly off the left edge (x from -30 to -10), a line on the page, and a box over the
    // right edge with 3/4 of it inside the page (x from 90 to 110 is half; 85 to 105 is 3/4).
    let off = Item::Rect { rect: Rect { x: -30.0, y: 20.0, w: 20.0, h: 10.0 }, fill: Some(Color::BLACK), stroke: None };
    let over = Item::Rect { rect: Rect { x: 85.0, y: 20.0, w: 20.0, h: 10.0 }, fill: Some(Color::BLACK), stroke: None };
    let s = scene(vec![], vec![line("M20 20L40 40", 0.0), off, over]);
    // Tight: from the box off the left edge to the one over the right edge.
    near(view(&s, Margin::Tight, None, None), -30.0, 20.0, 135.0, 20.0);
    // Tight in page: the box off the page is ignored; the one 3/4 inside counts, and the result is
    // cut at the page edge.
    near(view(&s, Margin::TightInPage, None, None), 20.0, 20.0, 80.0, 20.0);
    // Page does not change.
    near(view(&s, Margin::Page, None, None), 0.0, 0.0, 100.0, 80.0);
}

#[test]
fn an_item_that_is_exactly_half_inside_the_page_counts_for_tight_in_page() {
    let half = Item::Rect { rect: Rect { x: 90.0, y: 20.0, w: 20.0, h: 10.0 }, fill: Some(Color::BLACK), stroke: None };
    let s = scene(vec![], vec![half]);
    near(view(&s, Margin::TightInPage, None, None), 90.0, 20.0, 10.0, 10.0);
}

#[test]
fn data_is_cut_at_the_frame_and_circles_use_their_full_extent() {
    // A long line that leaves the frame (10..70 x 10..60): only the part inside counts.
    let s = scene(vec![line("M-500 30L500 30", 2.0)], vec![]);
    near(view(&s, Margin::Tight, None, None), 10.0, 29.0, 60.0, 2.0);
    // A circle symbol made of two half arcs, 10 pt wide, centred on (50, 40).
    let mut d = String::new();
    crate::plots::symbol_path(&mut d, sheaf_core::graph::SymbolShape::Circle, 50.0, 40.0, 10.0);
    let c = scene(vec![], vec![Item::Path { d, fill: Some(Color::BLACK), stroke: None }]);
    let b = view(&c, Margin::Tight, None, None);
    assert!((b.x - 45.0).abs() < 0.01 && (b.w - 10.0).abs() < 0.01, "{b:?}");
    assert!((b.y - 35.0).abs() < 0.01 && (b.h - 10.0).abs() < 0.01, "top and bottom of the circle: {b:?}");
}

#[test]
fn text_counts_by_its_measured_box() {
    let t = Item::Text(TextItem {
        x: 50.0,
        y: 40.0,
        text: "abcd".into(),
        family: "Arial".into(),
        size: 10.0,
        bold: false,
        italic: false,
        color: Color::BLACK,
        anchor: Anchor::Middle,
        rotate_ccw: 0.0,
    });
    let s = scene(vec![], vec![t]);
    let b = view(&s, Margin::Tight, None, None);
    // FixedMetrics: 4 characters of 0.55 * 10 pt.
    assert!((b.w - 22.0).abs() < 1e-9 && (b.x - 39.0).abs() < 1e-9, "{b:?}");
    assert!(b.y < 40.0 && b.y + b.h > 40.0, "the baseline is inside the box: {b:?}");
}

#[test]
fn an_empty_figure_falls_back_to_the_page() {
    let s = scene(vec![], vec![]);
    for m in [Margin::Border, Margin::Tight, Margin::TightInPage, Margin::Auto] {
        near(view(&s, m, None, None), 0.0, 0.0, 100.0, 80.0);
    }
}

#[test]
fn size_units_convert_to_inches_and_to_a_scale() {
    // 5 cm, 600 px at 300 dpi, 2 in, 50 % of a 10 in wide area.
    let close = |a: f64, b: f64| assert!((a - b).abs() < 1e-12, "{a} vs {b}");
    close(SizeUnit::Cm.to_inches(5.0, 300.0, 10.0), 5.0 / 2.54);
    close(SizeUnit::Pixel.to_inches(600.0, 300.0, 10.0), 2.0);
    close(SizeUnit::PageRatio.to_inches(50.0, 300.0, 10.0), 5.0);
    for u in [SizeUnit::Inch, SizeUnit::Cm, SizeUnit::Pixel, SizeUnit::PageRatio] {
        close(u.from_inches(u.to_inches(7.5, 150.0, 9.0), 150.0, 9.0), 7.5);
    }
    assert_eq!(SizeUnit::parse("PX"), Some(SizeUnit::Pixel));
    assert_eq!(SizeUnit::parse("%"), Some(SizeUnit::PageRatio));
    assert_eq!(SizeUnit::parse("furlong"), None);
    // A pixel size on a 720 x 540 pt (10 x 7.5 in) area at 300 dpi: 1500 px wide is half the page.
    let px = ExportSize::from_unit(Some(1500.0), None, SizeUnit::Pixel, 300.0);
    close(px.scale(720.0, 540.0).unwrap(), 0.5);
    // A page ratio scales whatever area is exported, by the factor; height works like width.
    let pct = ExportSize::from_unit(Some(40.0), None, SizeUnit::PageRatio, 300.0);
    close(pct.scale(300.0, 200.0).unwrap(), 0.4);
    close(ExportSize::from_unit(None, Some(40.0), SizeUnit::PageRatio, 300.0).scale(300.0, 200.0).unwrap(), 0.4);
    for bad in [0.0, -5.0, f64::NAN] {
        assert!(ExportSize::from_unit(Some(bad), None, SizeUnit::PageRatio, 300.0).scale(300.0, 200.0).is_err());
    }
}

fn project() -> (Project, WindowId) {
    let mut p = Project::new();
    let book = p.add_workbook("data");
    let x: Vec<f64> = (0..=10).map(f64::from).collect();
    p.add_column(book, 0, Designation::X, ColumnData::numeric(x.clone())).unwrap();
    let g = p.add_graph(Template::Line);
    let yc = p.add_column(book, 0, Designation::Y, ColumnData::numeric(x.iter().map(|v| v * v).collect())).unwrap();
    p.add_plot(g, 0, yc).unwrap();
    (p, g)
}

fn png_size(png: &[u8]) -> (u32, u32) {
    let be = |i: usize| u32::from_be_bytes([png[i], png[i + 1], png[i + 2], png[i + 3]]);
    (be(16), be(20))
}

fn svg_root(svg: &str) -> (f64, f64, [f64; 4]) {
    let attr =
        |name: &str| -> String { svg.split(&format!("{name}=\"")).nth(1).unwrap().split('"').next().unwrap().into() };
    let pt = |name: &str| attr(name).trim_end_matches("pt").parse::<f64>().unwrap();
    let vb: Vec<f64> = attr("viewBox").split(' ').map(|v| v.parse().unwrap()).collect();
    (pt("width"), pt("height"), [vb[0], vb[1], vb[2], vb[3]])
}

#[test]
fn exports_follow_the_margin_mode_in_every_format() {
    let (p, g) = project();
    let page = p.graph(g).unwrap().page.clone();
    let layout = |margin| ExportLayout { margin, border_px: None };
    let by = |ext: &str, margin: Margin, size: ExportSize| {
        export_bytes_with(&p, g, ext, 100.0, size, layout(margin)).unwrap()
    };

    // Page: the same as the plain export.
    let plain = export_bytes(&p, g, "svg", 100.0, ExportSize::default()).unwrap();
    assert_eq!(by("svg", Margin::Page, ExportSize::default()), plain);

    let area = export_area(&p, g, layout(Margin::Tight), None).unwrap();
    assert!(area.w > 0.0 && area.w < page.width_pt && area.h < page.height_pt, "{area:?}");
    let border = export_area(&p, g, layout(Margin::Border), None).unwrap();
    assert!((border.w - area.w - 10.0).abs() < 1e-6 && (border.h - area.h - 10.0).abs() < 1e-6);

    // SVG: size and viewBox are the area.
    let svg = String::from_utf8(by("svg", Margin::Tight, ExportSize::default())).unwrap();
    let (w, h, vb) = svg_root(&svg);
    assert!((w - area.w).abs() < 0.01 && (h - area.h).abs() < 0.01, "{w} x {h} vs {area:?}");
    assert!((vb[0] - area.x).abs() < 0.01 && (vb[1] - area.y).abs() < 0.01 && (vb[2] - area.w).abs() < 0.01);
    // The text of the graph is still there.
    assert!(svg.contains("<text"));

    // PNG: pixel size follows the area at the resolution.
    let (pw, ph) = png_size(&by("png", Margin::Tight, ExportSize::default()));
    assert_eq!(f64::from(pw), (area.w / 72.0 * 100.0).ceil());
    assert_eq!(f64::from(ph), (area.h / 72.0 * 100.0).ceil());

    // A size in pixels is the pixel size of the picture: 600 px wide at 100 dpi is 6 in.
    let size = ExportSize::from_unit(Some(600.0), None, SizeUnit::Pixel, 100.0);
    let (pw, ph) = png_size(&by("png", Margin::Tight, size));
    assert_eq!(pw, 600);
    assert!((f64::from(ph) - 600.0 * area.h / area.w).abs() <= 1.0, "aspect kept: {ph}");

    // A page ratio of 50 % halves the exported area, not the page.
    let half = ExportSize::from_unit(Some(50.0), None, SizeUnit::PageRatio, 100.0);
    let (pw, _) = png_size(&by("png", Margin::Tight, half));
    assert_eq!(f64::from(pw), (area.w / 72.0 * 100.0 * 0.5).ceil());

    // PDF page size.
    let pdf = String::from_utf8_lossy(&by("pdf", Margin::Tight, ExportSize::default())).into_owned();
    let media = pdf.split("MediaBox [0 0 ").nth(1).expect("a MediaBox");
    let mut nums = media.split_whitespace();
    let (mw, mh): (f64, f64) =
        (nums.next().unwrap().parse().unwrap(), nums.next().unwrap().trim_end_matches(']').parse().unwrap());
    assert!((mw - area.w).abs() < 0.05 && (mh - area.h).abs() < 0.05, "{mw} x {mh} vs {area:?}");

    // EPS: bounding box is the area, the drawing is moved by its corner.
    let eps = String::from_utf8(by("eps", Margin::Tight, ExportSize::default())).unwrap();
    assert!(eps.contains(&format!("%%BoundingBox: 0 0 {} {}\n", area.w.ceil(), area.h.ceil())), "{}", &eps[..220]);
    assert!(eps.contains(" translate\n"));
    let full = String::from_utf8(by("eps", Margin::Page, ExportSize::default())).unwrap();
    assert!(!full.contains(" translate\n"), "the whole page needs no move");

    // EMF: the header bounds are the area (0.01 mm units: 100 / 2.834645 per pt).
    let emf = by("emf", Margin::Tight, ExportSize::default());
    let emf_page = by("emf", Margin::Page, ExportSize::default());
    let right = |b: &[u8]| i32::from_le_bytes(b[16..20].try_into().unwrap());
    let bottom = |b: &[u8]| i32::from_le_bytes(b[20..24].try_into().unwrap());
    let ratio_w = f64::from(right(&emf) + 1) / f64::from(right(&emf_page) + 1);
    let ratio_h = f64::from(bottom(&emf) + 1) / f64::from(bottom(&emf_page) + 1);
    assert!((ratio_w - area.w / page.width_pt).abs() < 0.005, "{ratio_w}");
    assert!((ratio_h - area.h / page.height_pt).abs() < 0.005, "{ratio_h}");
    // And it starts with a window origin record (type 10) after the map mode.
    let types: Vec<u32> = {
        let mut v = Vec::new();
        let mut at = 0;
        while at + 8 <= emf.len() {
            v.push(u32::from_le_bytes(emf[at..at + 4].try_into().unwrap()));
            at += u32::from_le_bytes(emf[at + 4..at + 8].try_into().unwrap()) as usize;
        }
        v
    };
    assert!(types.contains(&10), "SETWINDOWORGEX present: {types:?}");
}

#[test]
fn a_border_in_pixels_uses_the_export_resolution_for_rasters_only() {
    let (p, g) = project();
    let lay = ExportLayout { margin: Margin::Border, border_px: Some(30.0) };
    let tight = export_area(&p, g, ExportLayout { margin: Margin::Tight, border_px: None }, None).unwrap();
    let raster = export_area(&p, g, lay, Some(300.0)).unwrap();
    assert!((raster.w - tight.w - 2.0 * 30.0 * 72.0 / 300.0).abs() < 1e-6);
    let vector = export_area(&p, g, lay, None).unwrap();
    assert!((vector.w - tight.w - 60.0).abs() < 1e-6);
}
