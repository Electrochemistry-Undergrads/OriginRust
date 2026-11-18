//! The 3Ys and 4Ys templates and Multiple Y Axes drawn (T61, graphing spec 21.5 and 21.6).

use super::*;
use sheaf_core::column::{ColumnData, Designation};
use sheaf_core::multi_y::MultiYOptions;
use sheaf_core::template::Template;

/// A book with X and `n` Y columns of the golden curves, and the Y column ids.
fn book(n: usize) -> (Project, Vec<sheaf_core::ColumnId>) {
    let (x, shape) = super::tests::golden_data();
    let mut p = Project::new();
    let b = p.add_workbook("data");
    p.add_column(b, 0, Designation::X, ColumnData::numeric(x)).unwrap();
    let amps = [30.0, 3.0, 0.3, 21.5, 12.25];
    let ys = (0..n)
        .map(|i| {
            let c = p.add_column(b, 0, Designation::Y, ColumnData::numeric(shape(amps[i % amps.len()]))).unwrap();
            p.column_mut(c).unwrap().long_name = format!("Curve {}", i + 1);
            c
        })
        .collect();
    (p, ys)
}

fn golden(name: &str, template: Template, n: usize) {
    let (mut p, ys) = book(n);
    let g = p.plot_columns(template, &ys).unwrap();
    super::tests::compare_golden(name, &super::tests::svg_of(&p, g));
}

#[test]
fn golden_3ys_y_yy() {
    golden("multi-y-3ys-y-yy", Template::Multi3YYy, 3);
}

#[test]
fn golden_3ys_y_y_y() {
    golden("multi-y-3ys-y-y-y", Template::Multi3YYY, 3);
}

#[test]
fn golden_4ys_y_yyy() {
    golden("multi-y-4ys-y-yyy", Template::Multi4YYyy, 4);
}

#[test]
fn golden_4ys_yy_yy() {
    golden("multi-y-4ys-yy-yy", Template::Multi4YyYy, 4);
}

#[test]
fn golden_multiple_y_axes_with_two_plots_in_a_layer() {
    let (mut p, ys) = book(4);
    let o = MultiYOptions { per_layer: vec![2, 1, 1], ..MultiYOptions::default() };
    let g = p.plot_multi_y(&o, &ys).unwrap();
    super::tests::compare_golden("multi-y-multiple-axes", &super::tests::svg_of(&p, g));
}

/// The frame plus every displaced axis with its labels fits on the page for each template, also with fewer columns than
/// layers (the empty layers draw their axes too).
#[test]
fn every_axis_of_every_template_stays_on_the_page() {
    for (t, layers) in
        [(Template::Multi3YYy, 3), (Template::Multi3YYY, 3), (Template::Multi4YYyy, 4), (Template::Multi4YyYy, 4)]
    {
        for n in [1, layers, layers + 2] {
            let (mut p, ys) = book(n);
            let g = p.plot_columns(t, &ys).unwrap();
            let scene = layout::build_scene(&p, g, &FixedMetrics).unwrap();
            assert_eq!(scene.layers.len(), layers);
            let svg = super::tests::svg_of(&p, g);
            let d = super::tests::doc(&svg);
            let root = d.root_element();
            let w: f64 = root.attribute("width").unwrap().trim_end_matches("pt").parse().unwrap_or(f64::MAX);
            for n in d.descendants().filter(|n| n.has_tag_name("text")) {
                if let Some(x) = n.attribute("x").and_then(|v| v.parse::<f64>().ok()) {
                    assert!(x >= -1.0 && x <= w + 1.0, "{t:?} {n:?}: text at {x} outside the page of width {w}");
                }
            }
        }
    }
}
