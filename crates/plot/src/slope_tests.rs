//! Slope annotations (T52, graphing spec 17.6).

use super::*;
use sheaf_core::graph::{PlotKind, ScaleKind, SlopeAnnotation};
use sheaf_core::template::Template;
use sheaf_core::{Color, ColumnData, ColumnId, Designation};

const INK: Color = Color::rgb(9, 8, 7);

/// A scatter graph of `ys` against `xs` with the given column units, and the Y column.
fn graph(xs: &[f64], ys: &[f64], units: (&str, &str)) -> (Project, WindowId, ColumnId) {
    let mut p = Project::new();
    let b = p.add_workbook("data");
    let x = p.add_column(b, 0, Designation::X, ColumnData::numeric(xs.to_vec())).unwrap();
    let y = p.add_column(b, 0, Designation::Y, ColumnData::numeric(ys.to_vec())).unwrap();
    p.column_mut(x).unwrap().units = units.0.to_string();
    p.column_mut(y).unwrap().units = units.1.to_string();
    let g = p.add_graph(Template::Scatter);
    p.add_plot_xy(g, 0, Some(x), y, None).unwrap();
    (p, g, y)
}

fn annotate(p: &mut Project, g: WindowId, y: ColumnId, a: usize, b: usize) {
    let mut s = SlopeAnnotation::new(y, a, b);
    s.color = Some(INK);
    p.graph_mut(g).unwrap().layers[0].slopes.push(s);
}

fn scales(p: &mut Project, g: WindowId, x: ScaleKind, y: ScaleKind) {
    let l = &mut p.graph_mut(g).unwrap().layers[0];
    l.x_axis.scale = x;
    l.y_axis.scale = y;
}

fn scene(p: &Project, g: WindowId) -> scene::Scene {
    layout::build_scene(p, g, &FixedMetrics).unwrap()
}

/// The text of the slope label(s) of the first layer.
fn labels(p: &Project, g: WindowId) -> Vec<String> {
    scene(p, g).layers[0]
        .overlay
        .iter()
        .filter_map(|i| match i {
            scene::Item::Text(t) if t.color == INK => Some(t.text.clone()),
            _ => None,
        })
        .collect()
}

fn text(p: &Project, g: WindowId) -> String {
    let l = labels(p, g);
    assert_eq!(l.len(), 1, "{l:?}");
    l[0].clone()
}

#[test]
fn a_linear_slope_is_dy_over_dx_in_data_units() {
    let (mut p, g, y) = graph(&[1.0, 2.0, 3.0, 4.0], &[3.0, 5.0, 7.0, 9.0], ("", ""));
    annotate(&mut p, g, y, 0, 3);
    assert_eq!(text(&p, g), "slope = 2");
    // Rows in the other order give the same slope.
    p.graph_mut(g).unwrap().layers[0].slopes[0].row_a = 3;
    p.graph_mut(g).unwrap().layers[0].slopes[0].row_b = 0;
    assert_eq!(text(&p, g), "slope = 2");
}

#[test]
fn the_value_has_four_significant_digits() {
    let (mut p, g, y) = graph(&[0.0, 3.0], &[0.0, 1.0], ("", ""));
    annotate(&mut p, g, y, 0, 1);
    assert_eq!(text(&p, g), "slope = 0.3333");
}

#[test]
fn plot_multipliers_and_offsets_count_as_drawn() {
    let (mut p, g, y) = graph(&[1.0, 2.0], &[1.0, 2.0], ("", ""));
    annotate(&mut p, g, y, 0, 1);
    {
        let l = &mut p.graph_mut(g).unwrap().layers[0];
        l.stack = sheaf_core::graph::Stack::Individual;
        let o = l.plot_offset_mut(y);
        (o.x_multiplier, o.y_multiplier, o.x, o.y) = (2.0, 10.0, 5.0, 7.0);
    }
    // Drawn points (7, 17) and (9, 27): slope 5.
    assert_eq!(text(&p, g), "slope = 5");
}

#[test]
fn a_log10_x_axis_gives_the_slope_per_decade_with_units() {
    // A Tafel line: 120 mV per decade of current.
    let i = [1e-6, 1e-5, 1e-4, 1e-3];
    let e: Vec<f64> = i.iter().map(|v| 300.0 + 120.0 * f64::log10(*v / 1e-6)).collect();
    let (mut p, g, y) = graph(&i, &e, ("A", "mV"));
    scales(&mut p, g, ScaleKind::Log10, ScaleKind::Linear);
    annotate(&mut p, g, y, 0, 3);
    assert_eq!(text(&p, g), "slope = 120 mV/dec");
    // Two points one decade apart.
    p.graph_mut(g).unwrap().layers[0].slopes[0].row_b = 1;
    assert_eq!(text(&p, g), "slope = 120 mV/dec");
    // The same points on a linear X axis: the plain quotient.
    scales(&mut p, g, ScaleKind::Linear, ScaleKind::Linear);
    let joined = labels(&p, g).concat();
    assert!(joined.starts_with("slope = 1.333") && joined.ends_with(" mV/A"), "{joined}");
}

fn sig(v: f64) -> String {
    sheaf_calc::trend::sig_str(v, 4)
}

#[test]
fn a_log_x_axis_needs_only_the_y_unit_and_no_units_means_no_unit_text() {
    let (mut p, g, y) = graph(&[1.0, 10.0, 100.0], &[0.0, 50.0, 100.0], ("", "mV"));
    scales(&mut p, g, ScaleKind::Log10, ScaleKind::Linear);
    annotate(&mut p, g, y, 0, 2);
    assert_eq!(text(&p, g), "slope = 50 mV/dec");
    // A linear X axis without an X unit: the value only.
    scales(&mut p, g, ScaleKind::Linear, ScaleKind::Linear);
    assert_eq!(text(&p, g), format!("slope = {}", sig(100.0 / 99.0)));
    // No unit on Y either.
    scales(&mut p, g, ScaleKind::Log10, ScaleKind::Linear);
    let ycol = p.column_mut(y).unwrap();
    ycol.units.clear();
    assert_eq!(text(&p, g), "slope = 50");
}

#[test]
fn a_log_y_axis_gives_decades_of_y_per_x_unit_and_ln_and_log2_use_their_base() {
    let xs = [0.0, 1.0, 2.0];
    let ys = [1.0, 100.0, 10000.0];
    let (mut p, g, y) = graph(&xs, &ys, ("V", "A"));
    scales(&mut p, g, ScaleKind::Linear, ScaleKind::Log10);
    annotate(&mut p, g, y, 0, 2);
    assert_eq!(text(&p, g), "slope = 2 dec/V");
    scales(&mut p, g, ScaleKind::Linear, ScaleKind::Ln);
    assert_eq!(text(&p, g), format!("slope = {} e-fold/V", sig(f64::ln(10000.0) / 2.0)));
    scales(&mut p, g, ScaleKind::Linear, ScaleKind::Log2);
    assert_eq!(text(&p, g), format!("slope = {} oct/V", sig(f64::log2(10000.0) / 2.0)));
    // Both axes logarithmic: a power law y = x^3 has slope 3 dec/dec.
    let (mut p, g, y) = graph(&[1.0, 10.0, 100.0], &[1.0, 1e3, 1e6], ("", ""));
    scales(&mut p, g, ScaleKind::Log10, ScaleKind::Log10);
    annotate(&mut p, g, y, 0, 2);
    assert_eq!(text(&p, g), "slope = 3 dec/dec");
}

#[test]
fn a_missing_row_or_no_run_draws_nothing() {
    let (mut p, g, y) = graph(&[1.0, 2.0, 2.0, 4.0, f64::NAN], &[1.0, f64::NAN, 5.0, 7.0, 9.0], ("", ""));
    for (a, b) in [(0, 1), (0, 4), (0, 99), (2, 2), (1, 2)] {
        p.graph_mut(g).unwrap().layers[0].slopes.clear();
        annotate(&mut p, g, y, a, b);
        assert!(labels(&p, g).is_empty(), "rows {a} and {b}");
        assert!(scene(&p, g).layers[0].objects.iter().all(|o| !matches!(o.kind, scene::ObjectKind::Slope(_))));
    }
    // Two points with the same X (rows 1 and 2 of the plot, 0-based 1 is NaN: use rows 2 and 3 for a valid one).
    p.graph_mut(g).unwrap().layers[0].slopes.clear();
    annotate(&mut p, g, y, 2, 3);
    assert_eq!(text(&p, g), "slope = 1");
    // A non-positive X on a log axis cannot be placed.
    scales(&mut p, g, ScaleKind::Log10, ScaleKind::Linear);
    p.column_mut(ColumnId(y.0 - 1)).unwrap().data = ColumnData::numeric(vec![-1.0, 2.0, 2.0, 4.0, 5.0]);
    p.graph_mut(g).unwrap().layers[0].slopes[0].row_a = 0;
    p.graph_mut(g).unwrap().layers[0].slopes[0].row_b = 3;
    assert!(labels(&p, g).is_empty());
}

#[test]
fn masked_rows_draw_nothing() {
    let (mut p, g, y) = graph(&[1.0, 2.0, 3.0], &[1.0, 2.0, 3.0], ("", ""));
    annotate(&mut p, g, y, 0, 2);
    assert_eq!(text(&p, g), "slope = 1");
    p.column_mut(y).unwrap().mask.insert(2);
    assert!(labels(&p, g).is_empty());
}

#[test]
fn changing_the_data_changes_the_slope() {
    let (mut p, g, y) = graph(&[0.0, 1.0], &[0.0, 1.0], ("", ""));
    annotate(&mut p, g, y, 0, 1);
    assert_eq!(text(&p, g), "slope = 1");
    p.column_mut(y).unwrap().data = ColumnData::numeric(vec![0.0, 4.5]);
    assert_eq!(text(&p, g), "slope = 4.5");
}

#[test]
fn the_annotation_follows_its_plot_by_column_not_by_position() {
    let mut p = Project::new();
    let b = p.add_workbook("data");
    let x = p.add_column(b, 0, Designation::X, ColumnData::numeric(vec![0.0, 1.0])).unwrap();
    let y1 = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![0.0, 1.0])).unwrap();
    let y2 = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![0.0, 3.0])).unwrap();
    let g = p.add_graph(Template::Scatter);
    p.add_plot_xy(g, 0, Some(x), y1, None).unwrap();
    p.add_plot_xy(g, 0, Some(x), y2, None).unwrap();
    annotate(&mut p, g, y2, 0, 1);
    assert_eq!(text(&p, g), "slope = 3");
    // The plot of the annotation is no longer in the layer: nothing is drawn.
    p.graph_mut(g).unwrap().layers[0].plots.retain(|pl| pl.y != y2);
    assert!(labels(&p, g).is_empty());
}

#[test]
fn legs_are_dashed_and_optional_and_the_annotation_can_be_picked() {
    let (mut p, g, y) = graph(&[0.0, 10.0], &[0.0, 10.0], ("", ""));
    annotate(&mut p, g, y, 0, 1);
    let paths = |p: &Project| -> Vec<(String, bool)> {
        scene(p, g).layers[0]
            .overlay
            .iter()
            .filter_map(|i| match i {
                scene::Item::Path { d, stroke: Some(s), .. } if s.color == INK => Some((d.clone(), !s.dash.is_empty())),
                _ => None,
            })
            .collect()
    };
    let with = paths(&p);
    assert_eq!(with.len(), 2, "{with:?}");
    assert!(with[0].1 && !with[1].1, "legs dashed, segment solid");
    // The legs run horizontally from the first point, then vertically to the second.
    assert_eq!(with[0].0.matches('L').count(), 2);
    p.graph_mut(g).unwrap().layers[0].slopes[0].show_legs = false;
    let without = paths(&p);
    assert_eq!(without.len(), 1);
    assert!(!without[0].1);
    // A click on the segment picks it.
    let s = scene(&p, g);
    let pts = hit::path_points(&without[0].0);
    let mid = ((pts[0].0 + pts[1].0) / 2.0, (pts[0].1 + pts[1].1) / 2.0);
    assert_eq!(hit::hit_test(&s, mid, 3.0), hit::Hit::Object { layer: 0, kind: scene::ObjectKind::Slope(0) });
    // The label counts as part of it.
    let t = s.layers[0]
        .overlay
        .iter()
        .find_map(|i| match i {
            scene::Item::Text(t) if t.color == INK => Some(t.clone()),
            _ => None,
        })
        .unwrap();
    assert_eq!(
        hit::hit_test(&s, (t.x, t.y - 3.0), 1.0),
        hit::Hit::Object { layer: 0, kind: scene::ObjectKind::Slope(0) }
    );
}

#[test]
fn a_colour_of_none_is_the_plot_colour_and_the_width_is_used() {
    let (mut p, g, y) = graph(&[0.0, 10.0], &[0.0, 10.0], ("", ""));
    p.graph_mut(g).unwrap().layers[0].slopes.push(SlopeAnnotation { width_pt: 2.5, ..SlopeAnnotation::new(y, 0, 1) });
    let plot_color = p.graph(g).unwrap().layers[0].plots[0].symbol.color;
    let s = scene(&p, g);
    let seg = s.layers[0]
        .overlay
        .iter()
        .filter_map(|i| match i {
            scene::Item::Path { stroke: Some(st), .. } if st.dash.is_empty() && (st.width - 2.5).abs() < 1e-9 => {
                Some(st.color)
            }
            _ => None,
        })
        .count();
    assert_eq!(seg, 1);
    assert!(
        s.layers[0]
            .overlay
            .iter()
            .any(|i| matches!(i, scene::Item::Text(t) if t.text.starts_with("slope") && t.color == plot_color))
    );
}

#[test]
fn plots_sharing_a_y_column_are_told_apart_by_their_x_column() {
    let mut p = Project::new();
    let b = p.add_workbook("data");
    let x1 = p.add_column(b, 0, Designation::X, ColumnData::numeric(vec![0.0, 1.0])).unwrap();
    let x2 = p.add_column(b, 0, Designation::X, ColumnData::numeric(vec![0.0, 2.0])).unwrap();
    let y = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![0.0, 4.0])).unwrap();
    let g = p.add_graph(Template::Scatter);
    p.add_plot_xy(g, 0, Some(x1), y, None).unwrap();
    p.add_plot_xy(g, 0, Some(x2), y, None).unwrap();
    for (plot, want) in [(0, "slope = 4"), (1, "slope = 2")] {
        let l = &mut p.graph_mut(g).unwrap().layers[0];
        l.slopes.clear();
        let mut s = SlopeAnnotation::for_plot(&l.plots[plot], 0, 1);
        s.color = Some(INK);
        l.slopes.push(s);
        assert_eq!(text(&p, g), want, "plot {plot}");
    }
    // Without a stored X the first plot of that Y is used.
    let l = &mut p.graph_mut(g).unwrap().layers[0];
    l.slopes[0].x = None;
    assert_eq!(text(&p, g), "slope = 4");
}

#[test]
fn a_slope_moves_with_its_plot_to_the_right_y_layer() {
    let (mut p, g, y) = graph(&[1.0, 2.0, 3.0], &[1.0, 3.0, 5.0], ("", ""));
    annotate(&mut p, g, y, 0, 2);
    assert_eq!(text(&p, g), "slope = 2");
    let (layer, _) = p.graph_mut(g).unwrap().set_plot_y_axis(0, 0, true).unwrap();
    assert_eq!(layer, 1);
    let gr = p.graph(g).unwrap();
    assert!(gr.layers[0].slopes.is_empty());
    assert_eq!(gr.layers[1].slopes.len(), 1);
    let drawn: Vec<String> = scene(&p, g).layers[1]
        .overlay
        .iter()
        .filter_map(|i| match i {
            scene::Item::Text(t) if t.color == INK => Some(t.text.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(drawn, vec!["slope = 2".to_string()]);
}

#[test]
fn horizontal_bars_use_the_units_of_the_columns_on_the_drawn_axes() {
    // Bars: the drawn X axis shows the Y column (seconds), the drawn Y axis the X column (millimetres).
    let (mut p, g, y) = graph(&[1.0, 2.0, 3.0], &[10.0, 20.0, 40.0], ("mm", "s"));
    p.graph_mut(g).unwrap().layers[0].plots[0].kind = PlotKind::Bar;
    annotate(&mut p, g, y, 0, 1);
    // dy (drawn Y: rows 1 to 2) over dx (drawn X: 10 to 20 s).
    assert_eq!(text(&p, g), "slope = 0.1 mm/s");
}

#[test]
fn the_label_of_a_steep_segment_does_not_cover_it() {
    // Nearly vertical, up and down, and a diagonal.
    for ys in [[0.0, 100.0], [100.0, 0.0], [0.0, 1.0]] {
        let (mut p, g, y) = graph(&[0.0, 1.0], &ys, ("", ""));
        annotate(&mut p, g, y, 0, 1);
        let s = scene(&p, g);
        let o = &s.layers[0].overlay;
        let seg = o
            .iter()
            .find_map(|i| match i {
                scene::Item::Path { d, stroke: Some(st), .. } if st.color == INK && st.dash.is_empty() => {
                    Some(hit::path_points(d))
                }
                _ => None,
            })
            .unwrap();
        let t = o
            .iter()
            .find_map(|i| match i {
                scene::Item::Text(t) if t.color == INK => Some(t.clone()),
                _ => None,
            })
            .unwrap();
        let w = t.text.chars().count() as f64 * 0.55 * t.size;
        let x0 = match t.anchor {
            scene::Anchor::Start => t.x,
            scene::Anchor::End => t.x - w,
            scene::Anchor::Middle => t.x - w / 2.0,
        };
        let (y0, y1) = (t.y - t.size, t.y);
        // Sample the segment: no sample lies inside the label's box.
        for k in 0..=200 {
            let f = k as f64 / 200.0;
            let (px, py) = (seg[0].0 + (seg[1].0 - seg[0].0) * f, seg[0].1 + (seg[1].1 - seg[0].1) * f);
            assert!(!(px >= x0 && px <= x0 + w && py >= y0 && py <= y1), "{ys:?}: the label covers the segment");
        }
    }
}
