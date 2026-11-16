//! Straight lines at an axis value (T67, graphing spec "Straight lines").

use super::*;
use sheaf_core::graph::{Dash, ScaleKind};
use sheaf_core::straight_line::{StraightLine, StraightLineLabel};
use sheaf_core::template::Template;
use sheaf_core::{Color, ColumnData, Designation};

const INK: Color = Color::rgb(9, 8, 7);

/// A scatter graph with both axes fixed to 0..10.
fn graph() -> (Project, WindowId) {
    let mut p = Project::new();
    let b = p.add_workbook("data");
    let x = p.add_column(b, 0, Designation::X, ColumnData::numeric(vec![1.0, 9.0])).unwrap();
    let y = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![1.0, 9.0])).unwrap();
    let g = p.add_graph(Template::Scatter);
    p.add_plot_xy(g, 0, Some(x), y, None).unwrap();
    let l = &mut p.graph_mut(g).unwrap().layers[0];
    for a in [&mut l.x_axis, &mut l.y_axis] {
        a.min = Some(0.0);
        a.max = Some(10.0);
    }
    (p, g)
}

fn line(p: &mut Project, g: WindowId, vertical: bool, value: f64, location: StraightLineLabel) {
    let l = StraightLine { color: INK, location, ..StraightLine::new(vertical, value) };
    p.graph_mut(g).unwrap().layers[0].straight_lines.push(l);
}

fn scene(p: &Project, g: WindowId) -> scene::Scene {
    layout::build_scene(p, g, &FixedMetrics).unwrap()
}

fn paths(p: &Project, g: WindowId) -> Vec<(String, bool)> {
    scene(p, g).layers[0]
        .overlay
        .iter()
        .filter_map(|i| match i {
            scene::Item::Path { d, stroke: Some(s), .. } if s.color == INK => Some((d.clone(), !s.dash.is_empty())),
            _ => None,
        })
        .collect()
}

fn texts(p: &Project, g: WindowId) -> Vec<scene::TextItem> {
    scene(p, g).layers[0]
        .overlay
        .iter()
        .filter_map(|i| match i {
            scene::Item::Text(t) if t.color == INK => Some(t.clone()),
            _ => None,
        })
        .collect()
}

fn frame(p: &Project, g: WindowId) -> Rect {
    layout_geometry(p, g).unwrap()[0].frame
}

/// The label's centre: x of its middle, y from its baseline lifted by a third of the size.
fn centre(t: &scene::TextItem) -> (f64, f64) {
    (t.x, t.y - 0.35 * t.size)
}

fn near(a: f64, b: f64) {
    assert!((a - b).abs() < 2e-3, "{a} vs {b}");
}

#[test]
fn a_vertical_line_runs_across_the_frame_with_its_value_above() {
    let (mut p, g) = graph();
    line(&mut p, g, true, 4.0, StraightLineLabel::Top);
    let f = frame(&p, g);
    let ps = paths(&p, g);
    assert_eq!(ps.len(), 1);
    assert!(!ps[0].1, "solid");
    let pts = hit::path_points(&ps[0].0);
    near(pts[0].0, f.x + 0.4 * f.w);
    near(pts[1].0, f.x + 0.4 * f.w);
    near(pts[0].1.min(pts[1].1), f.y);
    near(pts[0].1.max(pts[1].1), f.y + f.h);
    let t = texts(&p, g);
    assert_eq!(t.len(), 1);
    assert_eq!(t[0].text, "4");
    let c = centre(&t[0]);
    near(c.0, f.x + 0.4 * f.w);
    near(c.1, f.y - 0.05 * f.h);
}

#[test]
fn a_vertical_line_has_four_label_places() {
    for (loc, want) in [
        (StraightLineLabel::Bottom, (0.4, 1.05)),
        (StraightLineLabel::Left, (0.35, 0.5)),
        (StraightLineLabel::Right, (0.45, 0.5)),
    ] {
        let (mut p, g) = graph();
        line(&mut p, g, true, 4.0, loc);
        let f = frame(&p, g);
        let c = centre(&texts(&p, g)[0]);
        near(c.0, f.x + want.0 * f.w);
        near(c.1, f.y + want.1 * f.h);
    }
}

#[test]
fn a_horizontal_line_has_four_label_places() {
    // Y = 7 stands at 30 % of the frame's height from the top.
    for (loc, want) in [
        (StraightLineLabel::Top, (0.5, 0.25)),
        (StraightLineLabel::Bottom, (0.5, 0.35)),
        (StraightLineLabel::Left, (-0.05, 0.3)),
        (StraightLineLabel::Right, (1.05, 0.3)),
    ] {
        let (mut p, g) = graph();
        line(&mut p, g, false, 7.0, loc);
        let f = frame(&p, g);
        let ps = paths(&p, g);
        let pts = hit::path_points(&ps[0].0);
        near(pts[0].1, f.y + 0.3 * f.h);
        near(pts[1].1, f.y + 0.3 * f.h);
        near(pts[0].0.min(pts[1].0), f.x);
        near(pts[0].0.max(pts[1].0), f.x + f.w);
        let t = texts(&p, g);
        assert_eq!(t[0].text, "7");
        let c = centre(&t[0]);
        near(c.0, f.x + want.0 * f.w);
        near(c.1, f.y + want.1 * f.h);
    }
}

#[test]
fn the_span_is_the_stored_range_clipped_to_the_frame() {
    let (mut p, g) = graph();
    line(&mut p, g, true, 5.0, StraightLineLabel::Top);
    p.graph_mut(g).unwrap().layers[0].straight_lines[0].span = Some((2.0, 20.0));
    let f = frame(&p, g);
    let pts = hit::path_points(&paths(&p, g)[0].0);
    near(pts[0].1.min(pts[1].1), f.y);
    near(pts[0].1.max(pts[1].1), f.y + 0.8 * f.h);
    // The ends stay where they were made when the axis changes afterwards.
    p.graph_mut(g).unwrap().layers[0].y_axis.max = Some(40.0);
    let f = frame(&p, g);
    let pts = hit::path_points(&paths(&p, g)[0].0);
    near(pts[0].1.max(pts[1].1), f.y + f.h * (1.0 - 2.0 / 40.0));
    near(pts[0].1.min(pts[1].1), f.y + f.h * (1.0 - 20.0 / 40.0));
}

#[test]
fn a_value_outside_the_axis_range_draws_nothing() {
    let (mut p, g) = graph();
    line(&mut p, g, true, 11.0, StraightLineLabel::Top);
    line(&mut p, g, false, -1.0, StraightLineLabel::Top);
    assert!(paths(&p, g).is_empty());
    assert!(texts(&p, g).is_empty());
    assert!(scene(&p, g).layers[0].objects.iter().all(|o| !matches!(o.kind, scene::ObjectKind::StraightLine(_))));
    // On the edge counts as inside.
    line(&mut p, g, true, 10.0, StraightLineLabel::Top);
    assert_eq!(paths(&p, g).len(), 1);
}

#[test]
fn a_log_axis_places_the_line_by_decade() {
    let (mut p, g) = graph();
    {
        let l = &mut p.graph_mut(g).unwrap().layers[0];
        l.x_axis.scale = ScaleKind::Log10;
        l.x_axis.min = Some(1.0);
        l.x_axis.max = Some(1000.0);
    }
    line(&mut p, g, true, 10.0, StraightLineLabel::Top);
    let f = frame(&p, g);
    let pts = hit::path_points(&paths(&p, g)[0].0);
    near(pts[0].0, f.x + f.w / 3.0);
    assert_eq!(texts(&p, g)[0].text, "10");
    line(&mut p, g, true, 0.5, StraightLineLabel::Top);
    assert_eq!(paths(&p, g).len(), 1, "below a log axis' range, not drawn");
}

#[test]
fn the_format_is_the_tick_format_or_fixed_decimals() {
    let (mut p, g) = graph();
    line(&mut p, g, true, 2.0, StraightLineLabel::Top);
    assert_eq!(texts(&p, g)[0].text, "2");
    // The value is not rounded to the tick step (2 here).
    p.graph_mut(g).unwrap().layers[0].straight_lines[0].value = 2.5;
    assert_eq!(texts(&p, g)[0].text, "2.5");
    p.graph_mut(g).unwrap().layers[0].straight_lines[0].value = 2.0;
    p.graph_mut(g).unwrap().layers[0].straight_lines[0].format = Some(".2".into());
    assert_eq!(texts(&p, g)[0].text, "2.00");
    p.graph_mut(g).unwrap().layers[0].straight_lines[0].value = 2.345;
    assert_eq!(texts(&p, g)[0].text, "2.35");
    p.graph_mut(g).unwrap().layers[0].straight_lines[0].format = None;
    p.graph_mut(g).unwrap().layers[0].straight_lines[0].value = 3.0;
    p.graph_mut(g).unwrap().layers[0].x_axis.tick_format = sheaf_core::graph::TickFormat::Decimal(1);
    assert_eq!(texts(&p, g)[0].text, "3.0");
    p.graph_mut(g).unwrap().layers[0].straight_lines[0].show_label = false;
    assert!(texts(&p, g).is_empty());
}

#[test]
fn style_and_width_are_drawn() {
    let (mut p, g) = graph();
    line(&mut p, g, false, 5.0, StraightLineLabel::Top);
    {
        let s = &mut p.graph_mut(g).unwrap().layers[0].straight_lines[0];
        s.dash = Dash::Dash;
        s.width_pt = 2.0;
    }
    let st = scene(&p, g).layers[0]
        .overlay
        .iter()
        .find_map(|i| match i {
            scene::Item::Path { stroke: Some(s), .. } if s.color == INK => Some(s.clone()),
            _ => None,
        })
        .unwrap();
    assert!(!st.dash.is_empty());
    near(st.width, 2.0);
}

#[test]
fn the_line_and_its_label_can_be_picked() {
    let (mut p, g) = graph();
    line(&mut p, g, true, 4.0, StraightLineLabel::Top);
    let s = scene(&p, g);
    let f = frame(&p, g);
    let want = hit::Hit::Object { layer: 0, kind: scene::ObjectKind::StraightLine(0) };
    assert_eq!(hit::hit_test(&s, (f.x + 0.4 * f.w + 1.0, f.y + f.h * 0.3), 3.0), want);
    // Away from the line, inside the frame, it is the layer.
    assert_eq!(hit::hit_test(&s, (f.x + 0.8 * f.w, f.y + f.h * 0.3), 3.0), hit::Hit::Layer(0));
    let t = texts(&p, g)[0].clone();
    assert_eq!(hit::hit_test(&s, (t.x, t.y - 0.3 * t.size), 1.0), want);
}

#[test]
fn lines_are_drawn_in_the_svg_export() {
    let (mut p, g) = graph();
    line(&mut p, g, true, 4.0, StraightLineLabel::Top);
    p.graph_mut(g).unwrap().layers[0].straight_lines[0].format = Some(".1".into());
    let svg = render_svg(&p, g).unwrap();
    assert!(svg.contains(">4.0<"), "{svg}");
}
