//! Parallel Plot and Parallel Index (T49, graphing spec 14.25): axis places, per-axis and shared scaling, lines
//! that break at missing cells, colour by a column or by group, axis titles, the colour scale and the legend.

use super::*;
use crate::tests::{doc, svg_of, texts};
use sheaf_core::graph::PlotKind;
use sheaf_core::template::Template;
use sheaf_core::{Color, ColumnData, ColumnId, Designation};

const NAN: f64 = f64::NAN;
/// The scene writes coordinates to two decimals.
const TOL: f64 = 0.02;

/// A project with numeric columns `cols` (long names `Col k`, unit `cm`) and a text column `g`.
fn sheet(cols: &[Vec<f64>], groups: &[&str]) -> (Project, Vec<ColumnId>, ColumnId) {
    let mut p = Project::new();
    let b = p.add_workbook("data");
    let ys: Vec<ColumnId> = cols
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let id = p.add_column(b, 0, Designation::Y, ColumnData::numeric(c.clone())).unwrap();
            let col = p.column_mut(id).unwrap();
            col.long_name = format!("Col {}", i + 1);
            col.units = "cm".into();
            id
        })
        .collect();
    let g = p
        .add_column(b, 0, Designation::Group, ColumnData::text(groups.iter().map(|s| s.to_string()).collect()))
        .unwrap();
    (p, ys, g)
}

fn four() -> Vec<Vec<f64>> {
    vec![
        vec![0.0, 0.25, 0.5, 0.75, 1.0],
        vec![0.0, 250.0, 500.0, 750.0, 1000.0],
        vec![10.0, 20.0, 30.0, 40.0, 50.0],
        vec![5.0, 4.0, 3.0, 2.0, 1.0],
    ]
}

fn parallel(cols: &[Vec<f64>]) -> (Project, WindowId, Vec<ColumnId>) {
    let (mut p, ys, _) = sheet(cols, &["a", "b", "a", "b", "c"]);
    let g = p.plot_columns(Template::Parallel, &ys).unwrap();
    (p, g, ys)
}

fn index(cols: &[Vec<f64>]) -> (Project, WindowId) {
    let (mut p, ys, _) = sheet(cols, &["a", "b", "a", "b", "c"]);
    let g = p.plot_columns(Template::ParallelIndex, &ys).unwrap();
    (p, g)
}

fn style(p: &mut Project, g: WindowId, f: impl Fn(&mut sheaf_core::graph::ParallelStyle)) {
    for pl in &mut p.graph_mut(g).unwrap().layers[0].plots {
        f(&mut pl.parallel);
    }
}

fn numbers(d: &str) -> Vec<(f64, f64)> {
    let n: Vec<f64> = d.split(['M', 'L', 'Z', ' ']).filter(|t| !t.is_empty()).filter_map(|t| t.parse().ok()).collect();
    n.as_chunks::<2>().0.iter().map(|c| (c[0], c[1])).collect()
}

fn layer(p: &Project, g: WindowId) -> scene::LayerScene {
    layout::build_scene(p, g, &FixedMetrics).unwrap().layers.remove(0)
}

fn frame(p: &Project, g: WindowId) -> Rect {
    let graph = p.graph(g).unwrap();
    let pds = data::collect_graph(p, graph);
    layout::graph_geometry(p, graph, &pds, &FixedMetrics)[0].frame
}

/// The lines of the rows: `(path data, stroke)` in drawing order.
fn rows(l: &scene::LayerScene) -> Vec<(String, scene::Stroke)> {
    l.plots
        .iter()
        .filter_map(|it| match it {
            scene::Item::Path { d, fill: None, stroke: Some(s) } if d.starts_with('M') => Some((d.clone(), s.clone())),
            _ => None,
        })
        .collect()
}

/// The pieces of a row line, each a list of page points.
fn pieces(d: &str) -> Vec<Vec<(f64, f64)>> {
    d.split('M').filter(|s| !s.trim().is_empty()).map(|s| numbers(&format!("M{s}"))).collect()
}

#[test]
fn the_axes_stand_evenly_across_the_frame_over_its_full_height() {
    let (p, g, _) = parallel(&four());
    let f = frame(&p, g);
    let l = layer(&p, g);
    let mut xs: Vec<f64> = l
        .overlay
        .iter()
        .filter_map(|it| match it {
            scene::Item::Path { d, .. } => {
                let pts = numbers(d);
                (pts.len() == 2
                    && pts[0].0 == pts[1].0
                    && (pts[0].1 - f.y).abs() < TOL
                    && (pts[1].1 - (f.y + f.h)).abs() < TOL)
                    .then_some(pts[0].0)
            }
            _ => None,
        })
        .collect();
    xs.sort_by(f64::total_cmp);
    assert_eq!(xs.len(), 4);
    for (i, x) in xs.iter().enumerate() {
        assert!((x - parallel::axis_x(f, 4, i)).abs() < TOL, "{i}: {x}");
        assert!((x - (f.x + f.w * (i as f64 + 0.5) / 4.0)).abs() < TOL);
    }
    // One line per row, through all four axes.
    let lines = rows(&l);
    assert_eq!(lines.len(), 5);
    for (d, s) in &lines {
        let pts = numbers(d);
        assert_eq!(pts.len(), 4);
        for (i, (x, _)) in pts.iter().enumerate() {
            assert!((x - xs[i]).abs() < TOL);
        }
        assert_eq!(s.width, 0.5);
    }
}

#[test]
fn each_axis_has_its_own_range_unless_rescaling_is_cleared() {
    let (mut p, g, _) = parallel(&four());
    let ys = |p: &Project| -> Vec<Vec<f64>> {
        rows(&layer(p, g)).iter().map(|(d, _)| numbers(d).iter().map(|q| q.1).collect()).collect()
    };
    // Rescale on (the default): column 2 is 1000 times column 1, and both fill their axis the same way.
    for row in ys(&p) {
        assert!((row[0] - row[1]).abs() < TOL, "{row:?}");
    }
    // Rescale off: one range for every axis, the union of the columns' (0 to 1000): column 1 stays at the bottom
    // (a larger page y), column 2 climbs to the top.
    style(&mut p, g, |s| s.rescale_each_axis = false);
    let off = ys(&p);
    let (first, last) = (&off[0], &off[4]);
    assert!((first[0] - first[1]).abs() < TOL, "both start at 0: {first:?}");
    assert!(last[1] < last[0] - 100.0, "column 2 reaches the top, column 1 stays near the bottom: {last:?}");
    // The shared ticks: every axis shows the labels of the one range.
    let labels = |p: &Project| {
        let l = layer(p, g);
        l.overlay.iter().filter(|it| matches!(it, scene::Item::Text(t) if t.text == "1000")).count()
    };
    assert_eq!(labels(&p), 4);
}

#[test]
fn a_line_breaks_at_a_missing_cell() {
    let mut cols = vec![vec![1.0, 2.0, 3.0, 4.0, 5.0]; 5];
    cols[1][0] = NAN; // row 0: axis 0 alone is left of the gap -> one piece, axes 2 to 4
    cols[2][1] = NAN; // row 1: pieces on axes 0-1 and 3-4
    cols[1][2] = NAN;
    cols[3][2] = NAN; // row 2: pieces 0 (alone, dropped), 2 (alone, dropped), 4 (alone, dropped): nothing
    for c in &mut cols {
        c[4] = NAN; // row 4: nothing at all
    }
    let (p, g, _) = parallel(&cols);
    let f = frame(&p, g);
    let lines = rows(&layer(&p, g));
    // Rows 0, 1 and 3 draw; rows 2 and 4 have no run of two cells.
    assert_eq!(lines.len(), 3);
    let on = |x: f64| (0..5).find(|&i| (x - parallel::axis_x(f, 5, i)).abs() < TOL).unwrap();
    let axes =
        |d: &str| -> Vec<Vec<usize>> { pieces(d).iter().map(|pc| pc.iter().map(|q| on(q.0)).collect()).collect() };
    assert_eq!(axes(&lines[0].0), vec![vec![2, 3, 4]]);
    assert_eq!(axes(&lines[1].0), vec![vec![0, 1], vec![3, 4]]);
    assert_eq!(axes(&lines[2].0), vec![vec![0, 1, 2, 3, 4]]);
    assert!(!lines[1].0.contains('Z'));
}

#[test]
fn a_masked_cell_is_missing_too() {
    let (mut p, g, ys) = parallel(&four());
    p.column_mut(ys[1]).unwrap().mask.insert(2);
    let lines = rows(&layer(&p, g));
    // Row 2 has no cell on axis 1: axis 0 is left alone, a line joins axes 2 and 3.
    let f = frame(&p, g);
    let pc = pieces(&lines[2].0);
    assert_eq!(pc.len(), 1, "{}", lines[2].0);
    assert_eq!(pc[0].len(), 2);
    assert!((pc[0][0].0 - parallel::axis_x(f, 4, 2)).abs() < TOL);
}

#[test]
fn a_parallel_plot_colours_its_lines_by_the_last_column_through_the_ramp_with_a_colour_scale() {
    let (p, g, ys) = parallel(&four());
    let graph = p.graph(g).unwrap();
    assert_eq!(graph.layers[0].plots[0].parallel.color, Some(*ys.last().unwrap()));
    let l = layer(&p, g);
    let lines = rows(&l);
    // The last column runs 5, 4, 3, 2, 1: the first row is the top of the ramp, the last the bottom.
    assert_eq!(lines[0].1.color, plots::map_color(1.0));
    assert_eq!(lines[4].1.color, plots::map_color(0.0));
    assert_eq!(lines[2].1.color, plots::map_color(0.5));
    let svg = svg_of(&p, g);
    let d = doc(&svg);
    let t = texts(&d);
    // The title of the colour scale is the column's name, followed by the labels of the ramp's ticks (the
    // data runs from 1 to 5, so the nice ticks inside are 2 and 4).
    let k = t.iter().position(|s| s == "Col 4").expect("the colour scale title");
    assert_eq!(&t[k + 1..], ["2", "4"], "{t:?}");
}

#[test]
fn without_a_colour_column_every_line_takes_the_plot_colour_and_there_is_no_colour_scale() {
    let (mut p, g, _) = parallel(&four());
    style(&mut p, g, |s| s.color = None);
    let l = layer(&p, g);
    let plain = p.graph(g).unwrap().layers[0].plots[0].line.color;
    for (_, s) in rows(&l) {
        assert_eq!(s.color, plain);
    }
    let strips = l.overlay.iter().filter(|it| matches!(it, scene::Item::Rect { fill: Some(_), .. })).count();
    assert_eq!(strips, 0);
}

#[test]
fn a_parallel_index_colours_by_group_with_one_legend_entry_per_group() {
    let (p, g) = index(&four());
    let graph = p.graph(g).unwrap();
    assert!(graph.layers[0].plots.iter().all(|pl| pl.kind == PlotKind::ParallelIndex));
    let l = layer(&p, g);
    let lines = rows(&l);
    let pal = &graph.layers[0].palette;
    let c = |i: usize| pal.color(i, 3);
    let colours: Vec<Color> = lines.iter().map(|(_, s)| s.color).collect();
    assert_eq!(colours, vec![c(0), c(1), c(0), c(1), c(2)]);
    assert!(c(0) != c(1) && c(1) != c(2));
    // The legend lists the groups in order of first appearance, right of the frame, and not the columns.
    let svg = svg_of(&p, g);
    let d = doc(&svg);
    let t = texts(&d);
    for name in ["a", "b", "c"] {
        assert_eq!(t.iter().filter(|s| s.as_str() == name).count(), 1, "{name}: {t:?}");
    }
    let f = frame(&p, g);
    let sample: Vec<(f64, f64)> = l
        .overlay
        .iter()
        .filter_map(|it| match it {
            scene::Item::Path { d, stroke: Some(s), .. } if colours.contains(&s.color) && numbers(d).len() == 2 => {
                let q = numbers(d);
                (q[0].0 > f.x + f.w).then_some(q[0])
            }
            _ => None,
        })
        .collect();
    assert_eq!(sample.len(), 3);
    // Hiding the legend removes it.
    let mut p2 = p.clone();
    p2.graph_mut(g).unwrap().layers[0].legend.visible = false;
    let t2 = texts(&doc(&svg_of(&p2, g)));
    assert!(!t2.iter().any(|s| s == "a" || s == "c"), "{t2:?}");
}

#[test]
fn axis_titles_are_the_long_name_and_the_unit() {
    let (p, g, _) = parallel(&four());
    let t = texts(&doc(&svg_of(&p, g)));
    for k in 1..=4 {
        assert!(t.contains(&format!("Col {k} (cm)")), "{t:?}");
    }
    // The tick labels of the axes of the first column: 0 to 1.
    assert!(t.contains(&"0.5".to_string()), "{t:?}");
}

#[test]
fn width_and_transparency_apply_to_every_line() {
    let (mut p, g, _) = parallel(&four());
    style(&mut p, g, |s| {
        s.width_pt = 2.0;
        s.transparency_pct = 50.0;
    });
    for (_, s) in rows(&layer(&p, g)) {
        assert_eq!(s.width, 2.0);
        assert!((127..=128).contains(&s.color.a), "{}", s.color.a);
    }
    style(&mut p, g, |s| s.transparency_pct = 0.0);
    assert!(rows(&layer(&p, g)).iter().all(|(_, s)| s.color.a == 255));
}

#[test]
fn other_plot_kinds_are_drawn_as_before() {
    // The line graph has no vertical axis of its own per column: its scene has the ordinary frame and ticks.
    let mut p = Project::new();
    let b = p.add_workbook("data");
    p.add_column(b, 0, Designation::X, ColumnData::numeric(vec![1.0, 2.0, 3.0])).unwrap();
    let y = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![1.0, 4.0, 9.0])).unwrap();
    let g = p.plot_columns(Template::Line, &[y]).unwrap();
    let t = texts(&doc(&svg_of(&p, g)));
    assert!(t.contains(&"9".to_string()) || t.contains(&"8".to_string()) || t.iter().any(|s| s == "2"), "{t:?}");
    assert!(layer(&p, g).plots.iter().all(|it| !matches!(it, scene::Item::Path { d, .. } if d.is_empty())));
}
