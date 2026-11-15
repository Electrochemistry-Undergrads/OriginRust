//! Sunburst (T51, graphing spec 14.27): angles, start angle and direction, the hole, colours, unfilled missing
//! categories, labels hidden when they do not fit and the other plot kinds left alone.

use super::*;
use crate::tests::doc;
use crate::tests::{svg_of, texts};
use sheaf_core::graph::{PlotKind, SunburstOrder, SunburstStyle};
use sheaf_core::template::Template;
use sheaf_core::{Color, ColumnData, ColumnId, Designation, WindowId};

const TOL: f64 = 1e-6;

/// Region, country, value; the second Asia row repeats a country (summed), one country has no name.
fn world() -> (Project, WindowId, ColumnId) {
    let rows = [
        ("Asia", "China", 40.0),
        ("Asia", "India", 30.0),
        ("Europe", "France", 10.0),
        ("Europe", "", 20.0),
        ("Africa", "Egypt", 0.5),
    ];
    let mut p = Project::new();
    let b = p.add_workbook("data");
    let text = |p: &mut Project, f: &dyn Fn(&(&str, &str, f64)) -> String| {
        p.add_column(b, 0, Designation::Y, ColumnData::text(rows.iter().map(f).collect())).unwrap()
    };
    let region = text(&mut p, &|r| r.0.to_string());
    let country = text(&mut p, &|r| r.1.to_string());
    let v = p.add_column(b, 0, Designation::Y, ColumnData::numeric(rows.iter().map(|r| r.2).collect())).unwrap();
    let g = p.plot_sunburst(&[region, country], v).unwrap();
    (p, g, v)
}

fn set(p: &mut Project, g: WindowId, f: impl Fn(&mut SunburstStyle)) {
    f(&mut p.graph_mut(g).unwrap().layers[0].plots[0].sunburst);
}

fn frame(p: &Project, g: WindowId) -> Rect {
    let graph = p.graph(g).unwrap();
    let pds = data::collect_graph(p, graph);
    layout::graph_geometry(p, graph, &pds, &FixedMetrics)[0].frame
}

fn layer(p: &Project, g: WindowId) -> scene::LayerScene {
    layout::build_scene(p, g, &FixedMetrics).unwrap().layers.remove(0)
}

/// The path items of the plot (the wedges, in drawing order) with their fills.
fn wedge_paths(l: &scene::LayerScene) -> Vec<(String, Option<Color>)> {
    l.plots
        .iter()
        .filter_map(|it| match it {
            scene::Item::Path { d, fill, .. } => Some((d.clone(), *fill)),
            _ => None,
        })
        .collect()
}

/// The vertices of a path as (x, y).
fn points(d: &str) -> Vec<(f64, f64)> {
    let mut nums = Vec::new();
    let mut cur = String::new();
    for ch in d.chars().chain(std::iter::once('Z')) {
        if ch.is_ascii_digit() || ch == '.' || ch == '-' {
            cur.push(ch);
        } else {
            if !cur.is_empty() {
                nums.push(cur.parse::<f64>().unwrap());
                cur.clear();
            }
        }
    }
    nums.chunks(2).filter(|c| c.len() == 2).map(|c| (c[0], c[1])).collect()
}

fn data_of(p: &Project, g: WindowId) -> sunburst::SunburstData {
    let graph = p.graph(g).unwrap();
    let ld = data::collect(p, &graph.layers[0]);
    ld.plots[0].sunburst.as_deref().unwrap().clone()
}

/// Radius and angle (degrees counter-clockwise from 3 o'clock) of a point round the centre.
fn polar(p: (f64, f64), c: (f64, f64)) -> (f64, f64) {
    let (dx, dy) = (p.0 - c.0, c.1 - p.1);
    (dx.hypot(dy), dy.atan2(dx).to_degrees())
}

#[test]
fn wedge_angles_are_the_shares_of_their_parent() {
    let (p, g, _) = world();
    let sb = data_of(&p, g);
    // Top level: Asia 70, Europe 30, Africa 0.5 of 100.5.
    let top: Vec<_> = sb.wedges.iter().filter(|w| w.level == 0).collect();
    assert_eq!(top.iter().map(|w| w.name.as_str()).collect::<Vec<_>>(), ["Asia", "Europe", "Africa"]);
    assert!((top[0].span - 70.0 / 100.5).abs() < TOL);
    for level in 0..2 {
        let sum: f64 = sb.wedges.iter().filter(|w| w.level == level).map(|w| w.span).sum();
        assert!((sum - 1.0).abs() < TOL, "level {level}: {sum}");
    }
    // The children of Asia fill exactly Asia's angle.
    let asia: f64 = sb.wedges.iter().filter(|w| w.level == 1 && w.from < top[0].span - 1e-9).map(|w| w.span).sum();
    assert!((asia - top[0].span).abs() < TOL);
    // And the drawn outline: the first wedge's outer arc runs over its angle.
    let l = layer(&p, g);
    let paths = wedge_paths(&l);
    let f = frame(&p, g);
    let c = (f.x + f.w / 2.0, f.y + f.h / 2.0);
    let pts = points(&paths[0].0);
    let (_, a_first) = polar(pts[0], c);
    assert!((a_first - 90.0).abs() < 1e-3, "starts at 12 o'clock: {a_first}");
    let outer: Vec<f64> = pts
        .iter()
        .map(|q| polar(*q, c))
        .filter(|(r, _)| (r - (0.3 + 0.35) * sb_outer(&p, g)).abs() < 0.05)
        .map(|(_, a)| a)
        .collect();
    let swept = outer.first().unwrap() - outer.last().unwrap();
    assert!((swept - 360.0 * top[0].span).abs() < 0.1, "{swept}");
}

fn sb_outer(p: &Project, g: WindowId) -> f64 {
    let f = frame(p, g);
    0.5 * f.w.min(f.h)
}

#[test]
fn the_start_angle_and_the_direction() {
    let (mut p, g, _) = world();
    let f = frame(&p, g);
    let c = (f.x + f.w / 2.0, f.y + f.h / 2.0);
    let first_point = |p: &Project| points(&wedge_paths(&layer(p, g))[0].0)[0];
    let q = first_point(&p);
    assert!((q.0 - c.0).abs() < 1e-3 && q.1 < c.1, "default: 12 o'clock {q:?}");
    set(&mut p, g, |s| s.start_angle_deg = 0.0);
    let q = first_point(&p);
    assert!((q.1 - c.1).abs() < 1e-3 && q.0 > c.0, "0 degrees: 3 o'clock {q:?}");
    // Clockwise: Asia (69.7 % of the turn) from 12 o'clock ends past 6 o'clock on the left, so its end is at x < cx;
    // Europe starts there. A quarter-turn start moves everything by 90 degrees the other way round.
    set(&mut p, g, |s| s.start_angle_deg = 90.0);
    let sb = data_of(&p, g);
    let paths = wedge_paths(&layer(&p, g));
    let europe = sb.wedges.iter().position(|w| w.name == "Europe").unwrap();
    let (_, a) = polar(points(&paths[europe].0)[0], c);
    let want = 90.0 - 360.0 * sb.wedges[europe].from;
    let diff = (a - want).rem_euclid(360.0);
    assert!(diff < 1e-3 || diff > 360.0 - 1e-3, "{a} vs {want}");
}

#[test]
fn the_hole_and_the_rings_have_the_radii_of_the_style() {
    let (mut p, g, _) = world();
    set(&mut p, g, |s| s.hole = 0.4);
    let f = frame(&p, g);
    let c = (f.x + f.w / 2.0, f.y + f.h / 2.0);
    let r = sb_outer(&p, g);
    let sb = data_of(&p, g);
    let paths = wedge_paths(&layer(&p, g));
    for (w, (d, _)) in sb.wedges.iter().zip(&paths) {
        let radii: Vec<f64> = points(d).iter().map(|q| polar(*q, c).0).collect();
        let (lo, hi) = (radii.iter().cloned().fold(f64::MAX, f64::min), radii.iter().cloned().fold(0.0, f64::max));
        let ring = 0.6 * r / 2.0;
        let want = (0.4 * r + w.level as f64 * ring, 0.4 * r + (w.level + 1) as f64 * ring);
        assert!((lo - want.0).abs() < 1e-2 && (hi - want.1).abs() < 1e-2, "{} {lo} {hi} vs {want:?}", w.name);
    }
    // A ring gap takes half from each side where there is a neighbouring ring.
    set(&mut p, g, |s| s.ring_gap_pt = 4.0);
    let paths = wedge_paths(&layer(&p, g));
    let level1 = sb.wedges.iter().position(|w| w.level == 1).unwrap();
    let lo = points(&paths[level1].0).iter().map(|q| polar(*q, c).0).fold(f64::MAX, f64::min);
    assert!((lo - (0.4 * r + 0.3 * r + 2.0)).abs() < 1e-2, "{lo}");
}

#[test]
fn arcs_are_polylines_of_at_most_two_degrees_and_the_circle_stays_round() {
    let (mut p, g, _) = world();
    // A wide, short frame: the circle is still a circle in its shorter side.
    p.graph_mut(g).unwrap().page.width_pt = 800.0;
    p.graph_mut(g).unwrap().page.height_pt = 300.0;
    let f = frame(&p, g);
    assert!(f.w > 1.5 * f.h);
    let c = (f.x + f.w / 2.0, f.y + f.h / 2.0);
    let r = 0.5 * f.h;
    let paths = wedge_paths(&layer(&p, g));
    for (d, _) in &paths {
        let pts = points(d);
        for q in &pts {
            assert!(polar(*q, c).0 <= r + 1e-2, "inside the circle of the shorter side");
        }
        // Consecutive vertices on one arc are at most 2 degrees apart.
        for w in pts.windows(2) {
            let ((r0, a0), (r1, a1)) = (polar(w[0], c), polar(w[1], c));
            if (r0 - r1).abs() < 1e-6 {
                let da = (a0 - a1).abs();
                assert!(da <= 2.0 + 1e-6 || (360.0 - da) <= 2.0 + 1e-6, "{da}");
            }
        }
    }
}

#[test]
fn colours_follow_the_leaf_value_and_the_mean_of_a_category() {
    let (p, g, _) = world();
    let sb = data_of(&p, g);
    let paths = wedge_paths(&layer(&p, g));
    let fill_of = |name: &str, level: usize| {
        let i = sb.wedges.iter().position(|w| w.name == name && w.level == level).unwrap();
        paths[i].1.unwrap()
    };
    // Leaf range 0.5 (Egypt) to 40 (China): blue to red of the shared ramp.
    assert_eq!(fill_of("Egypt", 1), plots::map_color(0.0));
    assert_eq!(fill_of("China", 1), plots::map_color(1.0));
    let t = |v: f64| (v - 0.5) / (40.0 - 0.5);
    assert_eq!(fill_of("India", 1), plots::map_color(t(30.0)));
    // A category: the mean of its leaves (Asia: 40 and 30).
    assert_eq!(fill_of("Asia", 0), plots::map_color(t(35.0)));
    assert_eq!(fill_of("France", 1), plots::map_color(t(10.0)));
}

#[test]
fn a_missing_category_is_unfilled() {
    let (p, g, _) = world();
    let sb = data_of(&p, g);
    let paths = wedge_paths(&layer(&p, g));
    let blank = sb.wedges.iter().position(|w| w.blank).unwrap();
    assert_eq!(sb.wedges[blank].name, "(blank)");
    assert!(paths[blank].1.is_none());
    assert_eq!(paths.iter().filter(|(_, f)| f.is_none()).count(), 1);
}

#[test]
fn labels_are_hidden_when_the_wedge_is_too_small_and_stay_upright() {
    let (p, g, _) = world();
    let t = texts(&doc(&svg_of(&p, g)));
    assert!(t.contains(&"Asia".to_string()) && t.contains(&"China".to_string()), "{t:?}");
    assert!(!t.contains(&"Africa".to_string()) && !t.contains(&"Egypt".to_string()), "0.5 % wedges: {t:?}");
    assert!(!t.iter().any(|s| s == "15" || s == "70"), "values are off by default");
    let l = layer(&p, g);
    for it in &l.plots {
        if let scene::Item::Text(tx) = it {
            let r = tx.rotate_ccw.rem_euclid(360.0);
            assert!(r <= 90.0 + TOL || r >= 270.0 - TOL, "upright: {r}");
        }
    }
    // Values on: written below the names where they fit.
    let (mut p, g, _) = world();
    set(&mut p, g, |s| s.show_values = true);
    let t = texts(&doc(&svg_of(&p, g)));
    assert!(t.contains(&"70".to_string()) && t.contains(&"40".to_string()), "{t:?}");
    set(&mut p, g, |s| {
        s.show_names = false;
        s.show_values = false;
    });
    let t = texts(&doc(&svg_of(&p, g)));
    assert!(!t.contains(&"Asia".to_string()) && !t.contains(&"70".to_string()), "{t:?}");
}

/// Twenty equal one-ring wedges (18 degrees) without a hole, named `name`; the labels written.
fn narrow_wedges(name: &str) -> Vec<String> {
    let mut p = Project::new();
    let b = p.add_workbook("data");
    let names: Vec<String> = (0..20).map(|i| format!("{name}{}", (b'a' + i as u8) as char)).collect();
    let c = p.add_column(b, 0, Designation::Y, ColumnData::text(names)).unwrap();
    let v = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![1.0; 20])).unwrap();
    let g = p.plot_sunburst(&[c], v).unwrap();
    set(&mut p, g, |s| s.hole = 0.0);
    texts(&doc(&svg_of(&p, g)))
}

#[test]
fn a_radial_label_must_fit_the_wedge_at_its_inner_end() {
    // Short names fit everywhere; a name nearly as long as the ring fits the chord at the middle radius but
    // overflows the narrow wedge at its inner end, so it is hidden.
    let short = narrow_wedges("");
    assert!(short.iter().any(|s| s == "a"), "{short:?}");
    let shown = |n: usize| narrow_wedges(&"W".repeat(n)).iter().filter(|s| s.starts_with('W')).count();
    assert_eq!(shown(5), 20);
    // 35 W's are shorter than the ring (the middle-radius test alone let them through) but not the inner chord.
    assert_eq!(shown(35), 0, "overflows into the neighbouring wedges");
}

#[test]
fn the_colour_scale_is_titled_with_the_value_column() {
    let (mut p, g, v) = world();
    p.column_mut(v).unwrap().long_name = "Population".to_string();
    let t = texts(&doc(&svg_of(&p, g)));
    assert!(t.contains(&"Population".to_string()), "{t:?}");
    assert!(t.contains(&"10".to_string()) && t.contains(&"40".to_string()), "ticks: {t:?}");
    assert_eq!(sunburst::scale_ticks(0.5, 40.0).first().map(|t| t.0 > 0.0), Some(true));
}

#[test]
fn by_size_order_and_single_ring_and_full_turn() {
    let (mut p, g, _) = world();
    set(&mut p, g, |s| s.order = SunburstOrder::BySize);
    let sb = data_of(&p, g);
    let top: Vec<_> = sb.wedges.iter().filter(|w| w.level == 0).map(|w| w.name.as_str()).collect();
    assert_eq!(top, ["Asia", "Europe", "Africa"]);
    // One category column with one category: a full circle ring still draws (two circles).
    let mut q = Project::new();
    let b = q.add_workbook("d");
    let c = q.add_column(b, 0, Designation::Y, ColumnData::text(vec!["a".into()])).unwrap();
    let v = q.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![3.0])).unwrap();
    let s = q.plot_sunburst(&[c], v).unwrap();
    let paths = wedge_paths(&layer(&q, s));
    assert_eq!(paths.len(), 1);
    assert_eq!(paths[0].0.matches('Z').count(), 2, "outer and inner circle");
}

#[test]
fn a_sunburst_without_category_columns_draws_nothing_and_other_kinds_are_unchanged() {
    let (mut p, g, _) = world();
    set(&mut p, g, |s| s.categories.clear());
    assert!(wedge_paths(&layer(&p, g)).is_empty());
    let mut q = Project::new();
    let b = q.add_workbook("d");
    let y = q.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![3.0, 1.0, 2.0])).unwrap();
    let s = q.plot_columns(Template::Scatter, &[y]).unwrap();
    let alone = svg_of(&q, s);
    let label = q.add_column(b, 0, Designation::Y, ColumnData::text(vec!["a".into(), "b".into(), "c".into()])).unwrap();
    q.plot_sunburst(&[label], y).unwrap();
    assert_eq!(svg_of(&q, s), alone);
    assert_eq!(q.graph(s).unwrap().layers[0].default_kind, PlotKind::Scatter);
}
