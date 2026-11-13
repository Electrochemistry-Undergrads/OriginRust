//! Tests of the legend text, its substitutions and its columns.

use super::*;
use crate::scene::{Item, ObjectKind, Scene};
use sheaf_core::template::Template;
use sheaf_core::{ColumnData, Designation};

fn project(n: usize) -> (Project, WindowId) {
    let mut p = Project::new();
    let book = p.add_workbook("data");
    let x: Vec<f64> = (0..=10).map(f64::from).collect();
    p.add_column(book, 0, Designation::X, ColumnData::numeric(x.clone())).unwrap();
    let g = p.add_graph(Template::Line);
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
    let fs = 11.0;
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
