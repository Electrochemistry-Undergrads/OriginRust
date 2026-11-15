//! Tests of the multi-page PDF writer (graphing.md 9.6).

use super::*;
use sheaf_core::template::Template;
use sheaf_core::{ColumnData, Designation};

/// A project with `n` line graphs; graph `i` is `(i + 1)` points wider than the first.
fn project(n: usize) -> (Project, Vec<WindowId>) {
    let mut p = Project::new();
    let book = p.add_workbook("data");
    let x: Vec<f64> = (0..=10).map(f64::from).collect();
    let xc = p.add_column(book, 0, Designation::X, ColumnData::numeric(x.clone())).unwrap();
    p.column_mut(xc).unwrap().long_name = "Potential".into();
    let mut graphs = Vec::new();
    for i in 0..n {
        let g = p.add_graph(Template::Line);
        let yc = p
            .add_column(book, 0, Designation::Y, ColumnData::numeric(x.iter().map(|v| v * (i + 1) as f64).collect()))
            .unwrap();
        p.add_plot(g, 0, yc).unwrap();
        let gr = p.graph_mut(g).unwrap();
        let (w, h) = (gr.page.width_pt + 36.0 * i as f64, gr.page.height_pt);
        gr.resize_page(w, h, true);
        graphs.push(g);
    }
    (p, graphs)
}

/// The text of the uncompressed parts of a PDF (object dictionaries).
fn text(pdf: &[u8]) -> String {
    String::from_utf8_lossy(pdf).into_owned()
}

/// The page objects' media boxes, in file order, as `[x0, y0, x1, y1]`.
fn media_boxes(pdf: &[u8]) -> Vec<[f64; 4]> {
    let t = text(pdf);
    t.match_indices("/MediaBox [")
        .map(|(i, m)| {
            let rest = &t[i + m.len()..];
            let end = rest.find(']').unwrap();
            let v: Vec<f64> = rest[..end].split_whitespace().map(|x| x.parse().unwrap()).collect();
            [v[0], v[1], v[2], v[3]]
        })
        .collect()
}

fn page_count(pdf: &[u8]) -> usize {
    let t = text(pdf);
    let i = t.find("/Count ").expect("page tree count");
    t[i + 7..].split(|c: char| !c.is_ascii_digit()).next().unwrap().parse().unwrap()
}

#[test]
fn one_page_per_graph_in_the_order_given_each_the_size_of_its_one_page_export() {
    let (p, g) = project(3);
    let order = [g[2], g[0], g[1]];
    let pdf = render_multipage_pdf(&p, &order, ExportSize::default(), ExportLayout::default()).unwrap();
    assert!(pdf.starts_with(b"%PDF"));
    assert!(text(&pdf[pdf.len() - 64..]).contains("%%EOF"));
    assert_eq!(page_count(&pdf), 3);
    let boxes = media_boxes(&pdf);
    assert_eq!(boxes.len(), 3, "{boxes:?}");
    for (b, &id) in boxes.iter().zip(&order) {
        let single = render_pdf(&p, id).unwrap();
        let one = media_boxes(&single);
        assert_eq!(one.len(), 1);
        for k in 0..4 {
            assert!((b[k] - one[0][k]).abs() < 1e-3, "page {b:?} vs one-page {:?}", one[0]);
        }
    }
    // The pages differ in width as the graphs do, in the order asked for.
    assert!(boxes[0][2] > boxes[2][2] && boxes[2][2] > boxes[1][2], "{boxes:?}");
}

#[test]
fn text_stays_text_on_every_page() {
    let (p, g) = project(2);
    let pdf = render_multipage_pdf(&p, &g, ExportSize::default(), ExportLayout::default()).unwrap();
    let t = text(&pdf);
    assert!(t.contains("/Type /Font") || t.contains("/Type/Font"), "fonts are embedded");
    // No page was rasterised into an image.
    assert!(!t.contains("/Subtype /Image"), "pages are vector");
}

#[test]
fn export_size_and_margin_apply_to_every_page() {
    let (p, g) = project(2);
    let size = ExportSize { width_in: Some(2.0), ..ExportSize::default() };
    let layout = ExportLayout { margin: Margin::Page, ..ExportLayout::default() };
    let pdf = render_multipage_pdf(&p, &g, size, layout).unwrap();
    for b in media_boxes(&pdf) {
        assert!((b[2] - b[0] - 144.0).abs() < 0.01, "2 in wide: {b:?}");
    }
    for (b, &id) in media_boxes(&pdf).iter().zip(&g) {
        let one = media_boxes(&export_bytes_with(&p, id, "pdf", 300.0, size, layout).unwrap())[0];
        assert!((b[3] - one[3]).abs() < 1e-3, "{b:?} vs {one:?}");
    }
}

#[test]
fn a_single_graph_gives_one_page_and_an_empty_list_or_unknown_graph_is_refused() {
    let (p, g) = project(1);
    let pdf = render_multipage_pdf(&p, &g, ExportSize::default(), ExportLayout::default()).unwrap();
    assert_eq!(page_count(&pdf), 1);
    assert!(matches!(
        render_multipage_pdf(&p, &[], ExportSize::default(), ExportLayout::default()),
        Err(PlotError::Pdf(_))
    ));
    // A workbook's id is not a graph.
    let book = p.workbooks().next().unwrap().id;
    assert!(matches!(
        render_multipage_pdf(&p, &[g[0], book], ExportSize::default(), ExportLayout::default()),
        Err(PlotError::NoSuchGraph(_))
    ));
}

#[test]
fn print_pages_are_paper_sized_and_placed_by_page_setup() {
    let (p, g) = project(2);
    let pdf = render_print_multipage_pdf(&p, &g).unwrap();
    assert_eq!(page_count(&pdf), 2);
    for (b, &id) in media_boxes(&pdf).iter().zip(&g) {
        let one = media_boxes(&render_print_pdf(&p, id).unwrap())[0];
        assert_eq!(b.map(|v| v.round()), one.map(|v| v.round()));
    }
}

#[test]
fn the_file_is_written() {
    let (p, g) = project(2);
    let dir = std::env::temp_dir().join(format!("sheaf-multipage-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("all.pdf");
    export_multipage_pdf_file(&p, &g, &path, ExportSize::default(), ExportLayout::default()).unwrap();
    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(page_count(&bytes), 2);
    std::fs::remove_dir_all(&dir).unwrap();
}
