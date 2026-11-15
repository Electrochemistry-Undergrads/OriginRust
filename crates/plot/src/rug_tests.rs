//! Rug strips beside the axes (T41, graphing spec 18.10).

use super::*;
use sheaf_core::graph::{AxisSide, Rug, ScaleKind};
use sheaf_core::template::Template;
use sheaf_core::{Color, ColumnData, Designation};

const INK: Color = Color::rgb(1, 2, 3);

fn graph(xs: &[f64], ys: &[&[f64]]) -> (Project, WindowId) {
    let mut p = Project::new();
    let b = p.add_workbook("data");
    let x = p.add_column(b, 0, Designation::X, ColumnData::numeric(xs.to_vec())).unwrap();
    let g = p.add_graph(Template::Scatter);
    for y in ys {
        let y = p.add_column(b, 0, Designation::Y, ColumnData::numeric(y.to_vec())).unwrap();
        p.add_plot_xy(g, 0, Some(x), y, None).unwrap();
    }
    (p, g)
}

fn rug() -> Rug {
    Rug { show: true, color: Some(INK), ..Rug::default() }
}

fn set(p: &mut Project, g: WindowId, f: impl FnOnce(&mut sheaf_core::graph::Layer)) {
    f(&mut p.graph_mut(g).unwrap().layers[0]);
}

/// The rug's segments (x0, y0, x1, y1), found by their colour.
fn segs(p: &Project, g: WindowId) -> (Vec<(f64, f64, f64, f64)>, Rect) {
    let s = layout::build_scene(p, g, &FixedMetrics).unwrap();
    let mut out = Vec::new();
    for item in &s.layers[0].overlay {
        if let scene::Item::Path { d, fill: None, stroke: Some(st) } = item
            && st.color == INK
        {
            let nums: Vec<f64> =
                d.split(['M', 'L', ' ']).filter(|t| !t.is_empty()).filter_map(|t| t.parse().ok()).collect();
            for c in nums.as_chunks::<4>().0 {
                out.push((c[0], c[1], c[2], c[3]));
            }
        }
    }
    (out, s.layers[0].clip)
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 0.05
}

fn fixed_x(l: &mut sheaf_core::graph::Layer, lo: f64, hi: f64) {
    l.x_axis.min = Some(lo);
    l.x_axis.max = Some(hi);
}

#[test]
fn a_bottom_rug_has_one_vertical_line_per_point_below_the_frame() {
    let (mut p, g) = graph(&[2.0, 5.0, 8.0], &[&[1.0, 2.0, 3.0]]);
    set(&mut p, g, |l| {
        fixed_x(l, 0.0, 10.0);
        l.x_axis.rug = rug();
    });
    let (s, f) = segs(&p, g);
    assert_eq!(s.len(), 3, "{s:?}");
    let size = 0.05 * (f.w + f.h) / 2.0;
    let tick = p.graph(g).unwrap().layers[0].x_axis.major_tick_len_pt;
    for (seg, x) in s.iter().zip([2.0, 5.0, 8.0]) {
        assert!(near(seg.0, f.x + x / 10.0 * f.w) && near(seg.2, seg.0), "{seg:?}");
        assert!(near(seg.1, f.y + f.h + tick) && near(seg.3, f.y + f.h + tick + size), "{seg:?} {f:?}");
    }
}

#[test]
fn a_left_rug_has_one_horizontal_line_per_point_left_of_the_frame() {
    let (mut p, g) = graph(&[1.0, 2.0, 3.0], &[&[2.0, 5.0, 8.0]]);
    set(&mut p, g, |l| {
        l.y_axis.min = Some(0.0);
        l.y_axis.max = Some(10.0);
        l.y_axis.rug = rug();
    });
    let (s, f) = segs(&p, g);
    assert_eq!(s.len(), 3);
    let size = 0.05 * (f.w + f.h) / 2.0;
    for (seg, y) in s.iter().zip([2.0, 5.0, 8.0]) {
        assert!(near(seg.1, f.y + (1.0 - y / 10.0) * f.h) && near(seg.3, seg.1), "{seg:?}");
        assert!(seg.0 < f.x && seg.2 < seg.0 && near(seg.0 - seg.2, size), "{seg:?}");
    }
}

#[test]
fn the_top_and_right_rugs_lie_beyond_the_opposite_edges() {
    let (mut p, g) = graph(&[1.0, 2.0], &[&[1.0, 2.0]]);
    set(&mut p, g, |l| {
        l.x_axis.rug_opposite = rug();
        l.y_axis.rug_opposite = rug();
    });
    let (s, f) = segs(&p, g);
    assert_eq!(s.len(), 4);
    assert!(s.iter().filter(|c| near(c.0, c.2)).all(|c| c.1 < f.y && c.3 < c.1));
    assert!(s.iter().filter(|c| near(c.1, c.3)).all(|c| c.0 > f.x + f.w && c.2 > c.0));
}

#[test]
fn a_rug_beside_an_axis_on_the_top_edge_goes_up() {
    let (mut p, g) = graph(&[1.0, 2.0], &[&[1.0, 2.0]]);
    set(&mut p, g, |l| {
        l.x_axis.side = AxisSide::Opposite;
        l.x_axis.rug = rug();
    });
    let (s, f) = segs(&p, g);
    assert_eq!(s.len(), 2);
    assert!(s.iter().all(|c| c.1 < f.y && c.3 < c.1));
}

#[test]
fn a_log_axis_places_the_lines_on_the_log_scale() {
    let (mut p, g) = graph(&[1.0, 10.0, 100.0], &[&[1.0, 2.0, 3.0]]);
    set(&mut p, g, |l| {
        l.x_axis.scale = ScaleKind::Log10;
        fixed_x(l, 1.0, 100.0);
        l.x_axis.rug = rug();
    });
    let (s, f) = segs(&p, g);
    let xs: Vec<f64> = s.iter().map(|c| c.0).collect();
    assert!(near(xs[0], f.x) && near(xs[1], f.x + f.w / 2.0) && near(xs[2], f.x + f.w), "{xs:?}");
}

#[test]
fn a_reversed_axis_mirrors_the_lines() {
    let (mut p, g) = graph(&[2.0, 8.0], &[&[1.0, 2.0]]);
    set(&mut p, g, |l| {
        fixed_x(l, 0.0, 10.0);
        l.x_axis.reverse = true;
        l.x_axis.rug = rug();
    });
    let (s, f) = segs(&p, g);
    assert!(near(s[0].0, f.x + 0.8 * f.w) && near(s[1].0, f.x + 0.2 * f.w), "{s:?}");
}

#[test]
fn missing_masked_and_out_of_range_points_get_no_line() {
    let (mut p, g) = graph(&[1.0, f64::NAN, 3.0, 4.0, 50.0, 6.0], &[&[1.0, 2.0, 3.0, 4.0, 5.0, f64::NAN]]);
    let y = p.graph(g).unwrap().layers[0].plots[0].y;
    p.column_mut(y).unwrap().mask.insert(2);
    set(&mut p, g, |l| {
        fixed_x(l, 0.0, 10.0);
        l.x_axis.rug = rug();
    });
    let (s, f) = segs(&p, g);
    // Rows 1 (X missing), 2 (masked), 4 (X out of range) and 5 (Y missing) are left out.
    assert_eq!(s.len(), 2, "{s:?}");
    assert!(near(s[0].0, f.x + 0.1 * f.w) && near(s[1].0, f.x + 0.4 * f.w));
}

#[test]
fn one_plot_can_be_chosen_and_auto_colour_follows_each_plot() {
    let (mut p, g) = graph(&[1.0, 2.0], &[&[1.0, 2.0], &[3.0, 4.0]]);
    set(&mut p, g, |l| {
        l.plots[0].symbol.color = Color::rgb(10, 0, 0);
        l.plots[1].symbol.color = Color::rgb(0, 10, 0);
        l.y_axis.rug = Rug { show: true, ..Rug::default() };
    });
    let colours = |p: &Project| {
        let s = layout::build_scene(p, g, &FixedMetrics).unwrap();
        s.layers[0]
            .overlay
            .iter()
            .filter_map(|i| match i {
                scene::Item::Path { fill: None, stroke: Some(st), .. }
                    if st.color == Color::rgb(10, 0, 0) || st.color == Color::rgb(0, 10, 0) =>
                {
                    Some(st.color)
                }
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(colours(&p), vec![Color::rgb(10, 0, 0), Color::rgb(0, 10, 0)]);
    set(&mut p, g, |l| l.y_axis.rug.plot = Some(1));
    assert_eq!(colours(&p), vec![Color::rgb(0, 10, 0)]);
}

#[test]
fn transparency_lowers_the_alpha_of_the_line_colour() {
    let (mut p, g) = graph(&[1.0], &[&[1.0]]);
    set(&mut p, g, |l| l.x_axis.rug = Rug { transparency_pct: 40.0, ..rug() });
    let s = layout::build_scene(&p, g, &FixedMetrics).unwrap();
    let a = s.layers[0].overlay.iter().find_map(|i| match i {
        scene::Item::Path { fill: None, stroke: Some(st), .. } if (st.color.r, st.color.g, st.color.b) == (1, 2, 3) => {
            Some(st.color.a)
        }
        _ => None,
    });
    assert_eq!(a, Some(153));
}

#[test]
fn without_overlap_each_plot_gets_its_own_sub_strip_separated_by_the_gap() {
    let (mut p, g) = graph(&[1.0, 2.0], &[&[1.0, 2.0], &[3.0, 4.0]]);
    set(&mut p, g, |l| l.x_axis.rug = Rug { overlap: false, gap_pct: 20.0, ..rug() });
    let (s, f) = segs(&p, g);
    assert_eq!(s.len(), 4);
    let size = 0.05 * (f.w + f.h) / 2.0;
    let gap = 0.2 * size;
    let sub = (size - gap) / 2.0;
    let (first, second) = (s[0], s[2]);
    assert!(near(first.3 - first.1, sub) && near(second.3 - second.1, sub), "{s:?}");
    assert!(near(second.1 - first.3, gap), "{s:?}");
    // With overlap the plots share the whole strip.
    set(&mut p, g, |l| l.x_axis.rug.overlap = true);
    let (s, _) = segs(&p, g);
    assert!(near(s[0].3 - s[0].1, size) && near(s[2].1, s[0].1));
}

#[test]
fn margin_shortens_the_lines_at_both_ends_and_borders_draw() {
    let (mut p, g) = graph(&[1.0, 2.0], &[&[1.0, 2.0]]);
    set(&mut p, g, |l| l.x_axis.rug = Rug { margin_pct: 20.0, ..rug() });
    let (s, f) = segs(&p, g);
    let size = 0.05 * (f.w + f.h) / 2.0;
    assert!(near(s[0].3 - s[0].1, 0.6 * size), "{s:?}");
    let count = |p: &Project| layout::build_scene(p, g, &FixedMetrics).unwrap().layers[0].overlay.len();
    let plain = count(&p);
    set(&mut p, g, |l| {
        l.x_axis.rug.parallel_border = true;
        l.x_axis.rug.perpendicular_border = true;
    });
    assert_eq!(count(&p), plain + 1, "the four border lines are one path");
}

#[test]
fn the_tick_labels_move_out_so_the_strip_does_not_overlap_them() {
    let (mut p, g) = graph(&[1.0, 2.0], &[&[1.0, 2.0]]);
    let label_top = |p: &Project| {
        let s = layout::build_scene(p, g, &FixedMetrics).unwrap();
        let f = s.layers[0].clip;
        let ys: Vec<f64> = s.layers[0]
            .overlay
            .iter()
            .filter_map(|i| match i {
                scene::Item::Text(t) if t.rotate_ccw == 0.0 && t.y > f.y + f.h => Some(t.y - t.size),
                _ => None,
            })
            .collect();
        (ys.iter().cloned().fold(f64::INFINITY, f64::min), f)
    };
    let (before, _) = label_top(&p);
    set(&mut p, g, |l| l.x_axis.rug = rug());
    let (after, f) = label_top(&p);
    let size = 0.05 * (f.w + f.h) / 2.0;
    assert!(after >= before + size - 1.0, "labels at {before} then {after}, strip {size}");
    let (s, _) = segs(&p, g);
    assert!(s.iter().all(|c| c.3 <= after + 0.5), "strip ends at {} but labels start at {after}", s[0].3);
}

#[test]
fn a_hidden_rug_changes_nothing_in_the_export() {
    let (mut p, g) = graph(&[1.0, 2.0, 3.0], &[&[1.0, 2.0, 3.0]]);
    let plain = render_svg_with(&p, g, &FixedMetrics).unwrap();
    set(&mut p, g, |l| {
        l.x_axis.rug = Rug { show: false, size_pct: 30.0, overlap: false, ..Rug::default() };
        l.y_axis.rug_opposite = Rug { show: false, margin_pct: 10.0, ..Rug::default() };
    });
    assert_eq!(render_svg_with(&p, g, &FixedMetrics).unwrap(), plain);
    set(&mut p, g, |l| l.x_axis.rug.show = true);
    assert_ne!(render_svg_with(&p, g, &FixedMetrics).unwrap(), plain);
}

#[test]
fn the_scatter_rug_template_has_rugs_on_the_bottom_and_left_axes() {
    let mut p = Project::new();
    let b = p.add_workbook("d");
    let x = p.add_column(b, 0, Designation::X, ColumnData::numeric(vec![1.0, 2.0, 3.0])).unwrap();
    let y = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![3.0, 1.0, 2.0])).unwrap();
    let g = p.add_graph(Template::ScatterRug);
    p.add_plot_xy(g, 0, Some(x), y, None).unwrap();
    let l = &p.graph(g).unwrap().layers[0];
    assert!(l.x_axis.rug.show && l.y_axis.rug.show && l.x_axis.rug.overlap);
    assert!(!l.x_axis.rug_opposite.show && !l.y_axis.rug_opposite.show);
    let svg = render_svg_with(&p, g, &FixedMetrics).unwrap();
    assert!(svg.contains("<path"));
}
