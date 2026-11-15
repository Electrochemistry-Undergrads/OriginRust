//! Tests of the legend text, its substitutions and its columns.

use super::*;
use crate::scene::{Item, ObjectKind, Scene};
use sheaf_core::template::Template;
use sheaf_core::{ColumnData, Designation};

fn project(n: usize) -> (Project, WindowId) {
    project_with_template(n, Template::Line)
}

fn project_with_template(n: usize, template: Template) -> (Project, WindowId) {
    let mut p = Project::new();
    let book = p.add_workbook("data");
    let x: Vec<f64> = (0..=10).map(f64::from).collect();
    p.add_column(book, 0, Designation::X, ColumnData::numeric(x.clone())).unwrap();
    let g = p.add_graph(template);
    for k in 0..n {
        let yc = p
            .add_column(book, 0, Designation::Y, ColumnData::numeric(x.iter().map(|v| v * (k + 1) as f64).collect()))
            .unwrap();
        p.column_mut(yc).unwrap().long_name = format!("Series {}", k + 1);
        p.add_plot(g, 0, yc).unwrap();
    }
    (p, g)
}

fn scene(p: &Project, g: WindowId) -> Scene {
    layout::build_scene(p, g, &FixedMetrics).unwrap()
}

/// The text items of the legend: (x, y, text), and the number of line samples.
fn legend_items(s: &Scene) -> (Vec<(f64, f64, String)>, usize) {
    let layer = &s.layers[0];
    let obj = layer.objects.iter().find(|o| o.kind == ObjectKind::Legend).expect("a legend");
    let items = &layer.overlay[obj.items.clone()];
    let texts = items
        .iter()
        .filter_map(|i| if let Item::Text(t) = i { Some((t.x, t.y, t.text.clone())) } else { None })
        .collect();
    let samples = items.iter().filter(|i| matches!(i, Item::Path { fill: None, .. })).count();
    (texts, samples)
}

#[test]
fn the_automatic_text_lists_every_plot_with_its_symbol() {
    let (p, g) = project(3);
    assert_eq!(legend_automatic_text(&p, g, 0).unwrap(), "\\l(1) %(1)\n\\l(2) %(2)\n\\l(3) %(3)");
    assert_eq!(legend_text(&p, g, 0).unwrap(), legend_automatic_text(&p, g, 0).unwrap());
    let (texts, samples) = legend_items(&scene(&p, g));
    let names: Vec<&str> = texts.iter().map(|t| t.2.as_str()).collect();
    assert_eq!(names, ["Series 1", "Series 2", "Series 3"]);
    assert_eq!(samples, 3);
    assert!(legend_text(&p, g, 5).is_err());
}

#[test]
fn custom_text_reorders_substitutes_and_adds_literal_text() {
    let (mut p, g) = project(2);
    p.graph_mut(g).unwrap().layers[0].legend.text = Some("\\l(2) T = 300 K, %(2)\r\nonly text\n\\l(1)".into());
    assert_eq!(legend_text(&p, g, 0).unwrap(), "\\l(2) T = 300 K, %(2)\r\nonly text\n\\l(1)");
    let (texts, samples) = legend_items(&scene(&p, g));
    let names: Vec<&str> = texts.iter().map(|t| t.2.as_str()).collect();
    assert_eq!(names, ["T = 300 K, Series 2", "only text"]);
    assert_eq!(samples, 2, "two symbol samples: plots 2 and 1");
    // The text after a sample starts one gap after it; a line without sample starts at the left.
    let fs = 22.0;
    assert!((texts[0].0 - texts[1].0 - (2.2 * fs + 0.5 * fs)).abs() < 1e-9, "{texts:?}");
    // Rows are one after the other.
    assert!((texts[1].1 - texts[0].1 - 1.3 * fs).abs() < 1e-9);
}

#[test]
fn an_unknown_plot_number_draws_nothing() {
    let (mut p, g) = project(1);
    p.graph_mut(g).unwrap().layers[0].legend.text = Some("\\l(7) %(7) x".into());
    let (texts, samples) = legend_items(&scene(&p, g));
    assert_eq!(samples, 0);
    assert_eq!(texts.len(), 1);
    assert_eq!(texts[0].2, "x");
}

#[test]
fn columns_fill_top_to_bottom_and_widen_the_legend() {
    let (mut p, g) = project(5);
    p.graph_mut(g).unwrap().layers[0].legend.position = sheaf_core::graph::LegendPosition::TopRight;
    let one = legend_items(&scene(&p, g));
    p.graph_mut(g).unwrap().layers[0].legend.columns = 2;
    let (two, samples) = legend_items(&scene(&p, g));
    assert_eq!(samples, 5);
    // Three rows in the first column, two in the second.
    let xs: Vec<f64> = two.iter().map(|t| t.0).collect();
    assert!(xs[0] == xs[1] && xs[1] == xs[2] && xs[3] == xs[4] && xs[3] > xs[0], "{xs:?}");
    assert_eq!(two[3].1, two[0].1, "the second column starts at the top");
    assert!(two[0].1 < two[2].1);
    // The legend is anchored at the top right: more width moves its left side left.
    assert!(two[0].0 < one.0[0].0);
    // More columns than entries is the same as one entry per column.
    p.graph_mut(g).unwrap().layers[0].legend.columns = 99;
    let (many, _) = legend_items(&scene(&p, g));
    assert!(many.iter().all(|t| t.1 == many[0].1), "one row");
}

#[test]
fn the_legend_of_a_double_y_graph_names_plots_by_layer_and_plot() {
    let mut p = Project::new();
    let book = p.add_workbook("data");
    let x: Vec<f64> = (0..=10).map(f64::from).collect();
    p.add_column(book, 0, Designation::X, ColumnData::numeric(x.clone())).unwrap();
    let a = p.add_column(book, 0, Designation::Y, ColumnData::numeric(x.clone())).unwrap();
    let b = p.add_column(book, 0, Designation::Y, ColumnData::numeric(x.clone())).unwrap();
    let g = p.plot_columns_double_y(Template::DoubleY, &[a], &[b]).unwrap();
    p.graph_mut(g).unwrap().layers[0].legend.scope = sheaf_core::graph::LegendScope::Graph;
    assert_eq!(legend_automatic_text(&p, g, 0).unwrap(), "\\l(1.1) %(1.1)\n\\l(2.1) %(2.1)");
}

/// The frame rectangle, excluding its stroke, so this checks the placement anchor rather than
/// font-dependent hit bounds. Reference values are the 2016 stock Line object's x1/y1.
fn automatic_legend_box(p: &Project, g: WindowId) -> Rect {
    let s = scene(p, g);
    let l = &s.layers[0];
    let object = l.objects.iter().find(|o| o.kind == ObjectKind::Legend).unwrap();
    l.overlay[object.items.clone()]
        .iter()
        .find_map(|item| match item {
            Item::Rect { rect, fill, .. } => {
                assert_eq!(*fill, Some(sheaf_core::color::Color::WHITE), "opaque stock legend");
                Some(*rect)
            }
            _ => None,
        })
        .unwrap()
}

fn check_origin_anchor(p: &Project, g: WindowId) -> Rect {
    let rect = automatic_legend_box(p, g);
    let frame = layout_geometry(p, g).unwrap()[0].frame;
    // x=4.933850365 on 0..6, y=12.043552036 on 1..11; the tolerance includes
    // Origin's page/frame rounding and the subpixel border of its exported rectangle.
    let expected_x = frame.x + (4.933_850_365 / 6.0) * frame.w;
    let expected_y = frame.y + (1.0 - (12.043_552_036 - 1.0) / 10.0) * frame.h;
    assert!((rect.x - expected_x).abs() < 0.0005 * frame.w, "{rect:?}, {frame:?}");
    assert!((rect.y - expected_y).abs() < 0.0005 * frame.h, "{rect:?}, {frame:?}");
    rect
}

#[test]
fn automatic_legend_anchor_matches_the_four_measured_stock_templates() {
    use sheaf_core::graph::LegendPosition;
    for template in [Template::Line, Template::Scatter, Template::LineSymbol, Template::Column] {
        let (p, g) = project_with_template(2, template);
        assert_eq!(p.graph(g).unwrap().layers[0].legend.position, LegendPosition::Automatic);
        check_origin_anchor(&p, g);
    }
    assert_eq!(
        Template::CyclicVoltammetry.new_graph(WindowId(1), "CV").layers[0].legend.position,
        LegendPosition::BottomRight
    );
    assert_eq!(Template::DoubleY.new_graph(WindowId(1), "DY").layers[0].legend.position, LegendPosition::TopRight);
}

#[test]
fn automatic_legend_anchor_stays_fixed_as_content_and_geometry_change() {
    use sheaf_core::graph::{Frame, Page};
    let (mut p, g) = project(4);
    let initial = check_origin_anchor(&p, g);
    {
        let legend = &mut p.graph_mut(g).unwrap().layers[0].legend;
        legend.text = Some("A much longer literal legend row\nsecond\nthird\nfourth".into());
    }
    let longer = check_origin_anchor(&p, g);
    assert!(longer.w > initial.w);
    assert_eq!((longer.x, longer.y), (initial.x, initial.y));
    p.graph_mut(g).unwrap().layers[0].legend.columns = 2;
    let columns = check_origin_anchor(&p, g);
    assert!(columns.h < longer.h && columns.w > longer.w);
    assert_eq!((columns.x, columns.y), (initial.x, initial.y));
    p.graph_mut(g).unwrap().layers[0].legend.font.size_pt = 30.0;
    let larger_font = check_origin_anchor(&p, g);
    assert_eq!((larger_font.x, larger_font.y), (initial.x, initial.y));
    p.graph_mut(g).unwrap().layers[0].frame = Frame { left: 0.25, top: 0.20, width: 0.55, height: 0.55 };
    check_origin_anchor(&p, g);
    p.graph_mut(g).unwrap().page = Page::inches(15.0, 7.5);
    check_origin_anchor(&p, g);
}
