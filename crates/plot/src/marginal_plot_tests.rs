//! Marginal histograms and box charts (T110, graphing spec "Marginal plots").

use super::*;
use sheaf_core::template::Template;
use sheaf_core::{Color, ColumnData, Designation};

/// Origin's probe data: twelve points.
fn project(t: Template) -> (Project, WindowId) {
    let mut p = Project::new();
    let b = p.add_workbook("");
    let xs = vec![1.2, 2.5, 2.9, 3.1, 3.8, 4.4, 4.9, 5.5, 6.1, 7.3, 2.2, 3.5];
    let ys = vec![2.1, 3.9, 3.2, 5.5, 4.1, 6.3, 5.2, 7.7, 6.4, 8.8, 2.9, 4.6];
    p.add_column(b, 0, Designation::X, ColumnData::numeric(xs)).unwrap();
    let y = p.add_column(b, 0, Designation::Y, ColumnData::numeric(ys)).unwrap();
    let g = p.plot_columns(t, &[y]).unwrap();
    (p, g)
}

fn geometries(p: &Project, g: WindowId) -> Vec<LayerGeometry> {
    let graph = p.graph(g).unwrap();
    let pds: Vec<data::LayerData> = graph.layers.iter().map(|l| data::collect(p, l)).collect();
    layout::graph_geometry(p, graph, &pds, &FixedMetrics)
}

#[test]
fn the_margins_share_the_scatters_scales_and_hold_the_counts() {
    let (p, g) = project(Template::MarginalHistograms);
    let geoms = geometries(&p, g);
    let (scatter, top, right) = (&geoms[0], &geoms[1], &geoms[2]);
    assert_eq!((top.x.min, top.x.max), (scatter.x.min, scatter.x.max));
    assert_eq!((right.y.min, right.y.max), (scatter.y.min, scatter.y.max));
    // Above and right of the scatter, touching it.
    assert!((top.frame.y + top.frame.h - scatter.frame.y).abs() < 0.01);
    assert!((right.frame.x - (scatter.frame.x + scatter.frame.w)).abs() < 0.01);
    assert!(top.y.min <= 0.0 && top.y.max >= 1.0);
    let s = layout::build_scene(&p, g, &FixedMetrics).unwrap();
    let bars = |l: usize| {
        s.layers[l]
            .plots
            .iter()
            .filter(|it| matches!(it, scene::Item::Path { fill: Some(c), .. } if *c == Color::rgb(192, 192, 192)))
            .count()
    };
    assert!(bars(1) >= 3 && bars(2) >= 3, "{} {}", bars(1), bars(2));
}

#[test]
fn a_marginal_box_lies_on_top_and_stands_on_the_right() {
    let (p, g) = project(Template::MarginalBoxCharts);
    let geoms = geometries(&p, g);
    let s = layout::build_scene(&p, g, &FixedMetrics).unwrap();
    let boxes: Vec<Vec<f64>> = [1, 2]
        .iter()
        .map(|&l| {
            let d = s.layers[l]
                .plots
                .iter()
                .find_map(|it| match it {
                    scene::Item::Path { d, fill: None, stroke: Some(st) }
                        if st.color == Color::RED && d.ends_with('Z') =>
                    {
                        Some(d.clone())
                    }
                    _ => None,
                })
                .unwrap();
            d.split(['M', 'L', ' ', 'Z']).filter_map(|t| t.parse().ok()).collect()
        })
        .collect();
    // Top: wider than tall; right: taller than wide.
    let (w, h) = ((boxes[0][2] - boxes[0][0]).abs(), (boxes[0][5] - boxes[0][1]).abs());
    assert!(w > h, "{:?}", boxes[0]);
    let (w, h) = ((boxes[1][2] - boxes[1][0]).abs(), (boxes[1][5] - boxes[1][1]).abs());
    assert!(h > w, "{:?}", boxes[1]);
    // The top box runs from Q1 to Q3 of the X values on the scatter's X scale.
    let q1 = sheaf_calc::stats::quantile_origin(&[1.2, 2.2, 2.5, 2.9, 3.1, 3.5, 3.8, 4.4, 4.9, 5.5, 6.1, 7.3], 0.25);
    let (x, _) = geoms[0].data_to_page(q1, 0.0);
    assert!((boxes[0][0] - x).abs() < 0.01, "{} {x}", boxes[0][0]);
}
