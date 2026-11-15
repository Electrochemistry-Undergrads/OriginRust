//! Hexbin plots (T45, graphing spec 14.24): regular hexagons on the page, colours by count, the colour scale,
//! autoscale and the other kinds left alone.

use super::*;
use crate::tests::{doc, svg_of, texts};
use sheaf_core::graph::{Frame, Page, PlotKind};
use sheaf_core::template::Template;
use sheaf_core::{ColumnData, Designation};

/// A Hexbin graph of the points, on a `w` x `h` inch page.
fn hex(x: &[f64], y: &[f64], page: (f64, f64)) -> (Project, WindowId) {
    let mut p = Project::new();
    let b = p.add_workbook("data");
    p.add_column(b, 0, Designation::X, ColumnData::numeric(x.to_vec())).unwrap();
    let yc = p.add_column(b, 0, Designation::Y, ColumnData::numeric(y.to_vec())).unwrap();
    let g = p.plot_columns(Template::Hexbin, &[yc]).unwrap();
    let gr = p.graph_mut(g).unwrap();
    gr.page = Page::inches(page.0, page.1);
    gr.layers[0].frame = Frame { left: 0.2, top: 0.1, width: 0.5, height: 0.5 };
    (p, g)
}

fn points(n: usize) -> (Vec<f64>, Vec<f64>) {
    let x: Vec<f64> = (0..n).map(|i| ((i * 37 % 101) as f64) / 10.0).collect();
    let y: Vec<f64> = (0..n).map(|i| ((i * 53 % 97) as f64) / 7.0).collect();
    (x, y)
}

fn numbers(d: &str) -> Vec<(f64, f64)> {
    let n: Vec<f64> = d.split(['M', 'L', 'Z', ' ']).filter(|t| !t.is_empty()).filter_map(|t| t.parse().ok()).collect();
    n.as_chunks::<2>().0.iter().map(|c| (c[0], c[1])).collect()
}

fn hexagons(p: &Project, g: WindowId) -> Vec<(Vec<(f64, f64)>, sheaf_core::Color)> {
    layout::build_scene(p, g, &FixedMetrics).unwrap().layers[0]
        .plots
        .iter()
        .filter_map(|it| match it {
            scene::Item::Path { d, fill: Some(c), .. } => Some((numbers(d), *c)),
            _ => None,
        })
        .collect()
}

fn side(a: (f64, f64), b: (f64, f64)) -> f64 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
}

#[test]
fn hexagons_are_regular_on_the_page_for_a_square_frame_and_for_a_wide_one() {
    let (x, y) = points(400);
    // The frame is 0.5 of the page on both sides: square on a square page, 1.5:1 on a 3:2 page.
    for page in [(4.0, 4.0), (6.0, 4.0)] {
        let (p, g) = hex(&x, &y, page);
        let hs = hexagons(&p, g);
        assert!(hs.len() > 30, "{page:?}");
        for (pts, _) in &hs {
            assert_eq!(pts.len(), 6);
            let sides: Vec<f64> = (0..6).map(|i| side(pts[i], pts[(i + 1) % 6])).collect();
            for s in &sides {
                assert!((s - sides[0]).abs() < 0.02, "{page:?}: {sides:?}");
            }
            // Pointy top: the first corner is the topmost, straight above the centre.
            assert!(pts[0].1 < pts[1].1 && (pts[0].0 - (pts[1].0 + pts[5].0) / 2.0).abs() < 2e-3);
        }
    }
}

#[test]
fn colours_follow_the_count_over_the_largest_count() {
    // Ten points in one hexagon, one far away, in a lattice of 5 bins.
    let mut x = vec![0.0; 10];
    let mut y = vec![0.0; 10];
    x.push(10.0);
    y.push(10.0);
    let (mut p, g) = hex(&x, &y, (4.0, 4.0));
    for pl in &mut p.graph_mut(g).unwrap().layers[0].plots {
        pl.hexbin.x.by = sheaf_core::graph::HexbinBy::Bins(5);
    }
    let hs = hexagons(&p, g);
    assert_eq!(hs.len(), 2, "empty bins are not drawn");
    let colors: Vec<_> = hs.iter().map(|h| h.1).collect();
    assert!(colors.contains(&plots::map_color(1.0)), "the fullest bin is the top of the ramp");
    assert!(colors.contains(&plots::map_color(0.1)), "one of ten");
    assert_eq!(plots::map_color(1.0), sheaf_core::Color::rgb(255, 0, 0));
}

#[test]
fn the_colour_scale_has_a_title_nice_ticks_and_a_gradient_right_of_the_frame() {
    assert_eq!(hexbin::scale_ticks(0), vec![0]);
    assert_eq!(hexbin::scale_ticks(1), vec![0, 1]);
    assert_eq!(hexbin::scale_ticks(7), vec![0, 2, 4, 6]);
    assert_eq!(hexbin::scale_ticks(150), vec![0, 50, 100, 150]);
    assert_eq!(hexbin::scale_ticks(23), vec![0, 10, 20]);
    let (x, y) = points(400);
    let (p, g) = hex(&x, &y, (4.0, 4.0));
    let scene = layout::build_scene(&p, g, &FixedMetrics).unwrap();
    let layer = &scene.layers[0];
    let svg = svg_of(&p, g);
    let t = texts(&doc(&svg));
    assert!(t.iter().any(|s| s == "Count"), "{t:?}");
    assert!(t.iter().any(|s| s == "0"));
    let frame =
        layout::graph_geometry(&p, p.graph(g).unwrap(), &data::collect_graph(&p, p.graph(g).unwrap()), &FixedMetrics)
            [0]
        .frame;
    let strips: Vec<&Rect> = layer
        .overlay
        .iter()
        .filter_map(|it| match it {
            scene::Item::Rect { rect, fill: Some(_), .. } => Some(rect),
            _ => None,
        })
        .filter(|r| r.x >= frame.x + frame.w)
        .collect();
    assert_eq!(strips.len(), 64);
    assert!(strips.iter().all(|r| r.x >= frame.x + frame.w + hexbin::BAR_GAP_PT - 1e-9));
    // The bar and its labels stay on the page.
    assert!(strips[0].x + strips[0].w < scene.width);
}

#[test]
fn autoscale_covers_every_hexagon_and_the_legend_is_hidden() {
    let (x, y) = points(400);
    let (p, g) = hex(&x, &y, (4.0, 4.0));
    let gr = p.graph(g).unwrap();
    assert!(!gr.layers[0].legend.visible);
    let geom = &layout::graph_geometry(&p, gr, &data::collect_graph(&p, gr), &FixedMetrics)[0];
    let d = data::collect_graph(&p, gr);
    let (x0, x1, y0, y1) = d[0].plots[0].hex.as_ref().unwrap().extent().unwrap();
    assert!(geom.x.min <= x0 && geom.x.max >= x1, "{} {} vs {x0} {x1}", geom.x.min, geom.x.max);
    assert!(geom.y.min <= y0 && geom.y.max >= y1, "{} {} vs {y0} {y1}", geom.y.min, geom.y.max);
}

#[test]
fn other_kinds_get_no_colour_scale_and_no_hexagons() {
    let (x, y) = points(50);
    let mut p = Project::new();
    let b = p.add_workbook("data");
    p.add_column(b, 0, Designation::X, ColumnData::numeric(x)).unwrap();
    let yc = p.add_column(b, 0, Designation::Y, ColumnData::numeric(y)).unwrap();
    let g = p.plot_columns(Template::Scatter, &[yc]).unwrap();
    let t = texts(&doc(&svg_of(&p, g)));
    assert!(!t.iter().any(|s| s == "Count"));
    let d = data::collect_graph(&p, p.graph(g).unwrap());
    assert!(d[0].plots[0].hex.is_none());
    assert_eq!(p.graph(g).unwrap().layers[0].plots[0].kind, PlotKind::Scatter);
}

#[test]
fn a_single_point_and_a_missing_value_still_draw() {
    let (p, g) = hex(&[1.0, f64::NAN], &[2.0, 3.0], (4.0, 4.0));
    assert_eq!(hexagons(&p, g).len(), 1);
}
