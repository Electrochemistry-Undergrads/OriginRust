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

/// A deterministic generator in `[0, 1)`.
struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> f64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((self.0 >> 11) as f64) / (1u64 << 53) as f64
    }
    fn normal(&mut self) -> f64 {
        let (a, b) = (self.next().max(1e-12), self.next());
        (-2.0 * a.ln()).sqrt() * (std::f64::consts::TAU * b).cos()
    }
}

fn set_bins(p: &mut Project, g: WindowId, n: u32) {
    for pl in &mut p.graph_mut(g).unwrap().layers[0].plots {
        pl.hexbin.x.by = sheaf_core::graph::HexbinBy::Bins(n);
    }
}

fn assert_inside_clip(p: &Project, g: WindowId) -> usize {
    let scene = layout::build_scene(p, g, &FixedMetrics).unwrap();
    let c = scene.layers[0].clip;
    let mut n = 0;
    for it in &scene.layers[0].plots {
        if let scene::Item::Path { d, fill: Some(_), .. } = it {
            n += 1;
            for (x, y) in numbers(d) {
                let e = 1e-6;
                assert!(
                    x.is_finite()
                        && y.is_finite()
                        && x >= c.x - e
                        && x <= c.x + c.w + e
                        && y >= c.y - e
                        && y <= c.y + c.h + e,
                    "corner ({x}, {y}) outside the clip {c:?}"
                );
            }
        }
    }
    n
}

#[test]
fn every_corner_of_the_drawn_hexagons_is_inside_the_layer_clip() {
    for seed in 1..=12u64 {
        let mut r = Lcg(seed * 7919);
        let x: Vec<f64> = (0..3000).map(|_| 3.0 + r.next()).collect();
        let y: Vec<f64> = (0..3000).map(|_| 5.0 * r.normal()).collect();
        for bins in 2..=6 {
            for page in [(4.0, 4.0), (3.0, 5.0), (7.0, 3.0)] {
                let (mut p, g) = hex(&x, &y, page);
                set_bins(&mut p, g, bins);
                assert!(assert_inside_clip(&p, g) > 0, "seed {seed} bins {bins} page {page:?}");
            }
        }
    }
}

#[test]
fn on_a_log_axis_no_bin_is_dropped_and_the_hexagons_stay_in_the_domain() {
    // 0.1 .. 1000 geometrically, plus values the log axis cannot place.
    let mut x: Vec<f64> = (0..400).map(|i| 0.1 * 10f64.powf(4.0 * i as f64 / 399.0)).collect();
    let positive = x.len();
    x.extend([0.0, -3.0, -0.5]);
    let y: Vec<f64> = (0..x.len()).map(|i| ((i * 29) % 83) as f64).collect();
    let (mut p, g) = hex(&x, &y, (4.0, 4.0));
    p.graph_mut(g).unwrap().layers[0].x_axis.scale = sheaf_core::graph::ScaleKind::Log10;
    set_bins(&mut p, g, 20);
    let drawn = assert_inside_clip(&p, g);
    let gr = p.graph(g).unwrap();
    let d = data::collect_graph(&p, gr);
    let hex = d[0].plots[0].hex.as_ref().unwrap();
    assert_eq!(hex.excluded, 3);
    let geom = &layout::graph_geometry(&p, gr, &d, &FixedMetrics)[0];
    let f = geom.frame;
    let h = hex.bin_for(&d[0].plots[0].plot.hexbin, f.w, f.h, geom.x.scale_span(), geom.y.scale_span()).unwrap();
    assert_eq!(drawn, h.bins.len(), "no bin is dropped");
    assert_eq!(h.counted, positive);
    assert_eq!(h.bins.iter().map(|b| b.count).sum::<usize>(), positive);
    for b in &h.bins {
        for (cx, cy) in hex.corners(&h, b) {
            assert!(cx > 0.0 && cx.is_finite() && cy.is_finite());
        }
    }
    // Regular on the page: the hexagons are as wide as they are tall by the lattice, whatever the decade.
    for (pts, _) in hexagons(&p, g) {
        let sides: Vec<f64> = (0..6).map(|i| side(pts[i], pts[(i + 1) % 6])).collect();
        for s in &sides {
            assert!((s - sides[0]).abs() < 0.05 * sides[0] + 0.02, "{sides:?}");
        }
    }
}

#[test]
fn several_hexbin_plots_share_one_colour_scale() {
    let (x, y) = points(400);
    let (mut p, g) = hex(&x, &y, (4.0, 4.0));
    set_bins(&mut p, g, 20);
    let mut second = p.graph(g).unwrap().layers[0].plots[0].clone();
    second.hexbin.x.by = sheaf_core::graph::HexbinBy::Bins(3);
    p.graph_mut(g).unwrap().layers[0].plots.push(second);
    let gr = p.graph(g).unwrap();
    let d = data::collect_graph(&p, gr);
    assert_eq!(d[0].plots.len(), 2);
    let geom = &layout::graph_geometry(&p, gr, &d, &FixedMetrics)[0];
    let f = geom.frame;
    let bin = |k: usize| {
        let pd = &d[0].plots[k];
        pd.hex.as_ref().unwrap().bin_for(&pd.plot.hexbin, f.w, f.h, geom.x.scale_span(), geom.y.scale_span()).unwrap()
    };
    let (a, b) = (bin(0), bin(1));
    assert!(a.max_count < b.max_count, "the coarse plot has the larger counts");
    let max = b.max_count;
    let hs = hexagons(&p, g);
    assert_eq!(hs.len(), a.bins.len() + b.bins.len());
    for (hexagon, bin) in hs.iter().zip(a.bins.iter().chain(&b.bins)) {
        assert_eq!(hexagon.1, hexbin::bin_color(bin.count, max));
    }
    // The scale ends at the shared maximum.
    let t = texts(&doc(&svg_of(&p, g)));
    let last = hexbin::scale_ticks(max).last().unwrap().to_string();
    assert!(t.contains(&last), "{t:?} lacks {last}");
}
