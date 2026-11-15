use super::*;
use crate::scene::{Item, Scene};
use sheaf_calc::trend::{self, TrendKind};
use sheaf_core::graph::{Dash, ScaleKind, Trendline};
use sheaf_core::template::Template;
use sheaf_core::{Color, ColumnData, Designation};

const CURVE: Color = Color { r: 10, g: 200, b: 30, a: 255 };
const XS: [f64; 6] = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
const YS: [f64; 6] = [2.1, 3.9, 6.2, 7.8, 10.1, 12.2];

fn project(xs: &[f64], ys: &[f64], kind: Template) -> (Project, WindowId) {
    let mut p = Project::new();
    let b = p.add_workbook("Data");
    let x = p.add_column(b, 0, Designation::X, ColumnData::numeric(xs.to_vec())).unwrap();
    let y = p.add_column(b, 0, Designation::Y, ColumnData::numeric(ys.to_vec())).unwrap();
    let g = p.add_graph(kind);
    p.add_plot_xy(g, 0, Some(x), y, None).unwrap();
    let l = &mut p.graph_mut(g).unwrap().layers[0];
    l.legend.visible = false;
    (p, g)
}

fn add(p: &mut Project, g: WindowId, kind: TrendKind) -> &mut Trendline {
    let plot = &mut p.graph_mut(g).unwrap().layers[0].plots[0];
    let mut t = Trendline::new(kind);
    t.color = Some(CURVE);
    plot.decor.trendlines.push(t);
    plot.decor.trendlines.last_mut().unwrap()
}

fn scene(p: &Project, g: WindowId) -> Scene {
    layout::build_scene(p, g, &FixedMetrics).unwrap()
}

fn curves(s: &Scene) -> Vec<Vec<(f64, f64)>> {
    s.layers[0]
        .plots
        .iter()
        .filter_map(|i| match i {
            Item::Path { d, fill: None, stroke: Some(st) } if st.color == CURVE => Some(hit::path_points(d)),
            _ => None,
        })
        .collect()
}

/// The labels in the curve colour: text without markup, x and y of the first run. A label is a
/// row of runs (superscripts are runs of their own) that advance to the right.
fn texts(s: &Scene) -> Vec<(String, f64, f64)> {
    let mut out: Vec<(String, f64, f64)> = Vec::new();
    let mut last_x = f64::INFINITY;
    for i in &s.layers[0].plots {
        if let Item::Text(t) = i
            && t.color == CURVE
        {
            if t.x > last_x {
                out.last_mut().unwrap().0.push_str(&t.text);
            } else {
                out.push((t.text.clone(), t.x, t.y));
            }
            last_x = t.x;
        }
    }
    out
}

#[test]
fn linear_curve_matches_the_fit_across_the_data_range_with_a_label() {
    let (mut p, g) = project(&XS, &YS, Template::Scatter);
    add(&mut p, g, TrendKind::Linear);
    let s = scene(&p, g);
    let geom = &layout_geometry(&p, g).unwrap()[0];
    let c = curves(&s);
    assert_eq!(c.len(), 1);
    assert_eq!(c[0].len(), 200);
    let fit = trend::fit(TrendKind::Linear, &XS, &YS).unwrap();
    for (k, pt) in c[0].iter().enumerate() {
        let x = 1.0 + 5.0 * k as f64 / 199.0;
        let want = geom.data_to_page(x, fit.eval(x));
        assert!((pt.0 - want.0).abs() < 0.02 && (pt.1 - want.1).abs() < 0.02, "{k}: {pt:?} {want:?}");
    }
    let t = texts(&s);
    assert_eq!(t.len(), 1);
    let want = format!("{}, R\\+(2) = {}", fit.equation(4), trend::sig_str(fit.r2.unwrap(), 4));
    assert_eq!(t[0].0, crate::markup::plain(&want));
    assert!(t[0].0.starts_with("y = 2.02x"), "{}", t[0].0);
    // Top left inside the frame.
    assert!(t[0].1 > geom.frame.x && t[0].1 < geom.frame.x + 20.0);
    assert!(t[0].2 > geom.frame.y && t[0].2 < geom.frame.y + 30.0);
}

#[test]
fn label_options_offset_and_one_line_per_trendline() {
    let (mut p, g) = project(&XS, &YS, Template::Scatter);
    add(&mut p, g, TrendKind::Linear).show_r2 = false;
    add(&mut p, g, TrendKind::Exponential).show_equation = false;
    let t = texts(&scene(&p, g));
    assert_eq!(t.len(), 2);
    assert!(t[0].0.starts_with("y = ") && !t[0].0.contains("R"));
    assert!(t[1].0.starts_with("R2 = ") && !t[1].0.contains("y ="));
    assert!(t[1].2 > t[0].2 + 5.0, "second line below the first");
    let before = t[0].clone();
    add(&mut p, g, TrendKind::Power).label_offset_pt = (10.0, 7.0);
    let t = texts(&scene(&p, g));
    assert_eq!(t.len(), 3);
    assert_eq!((t[0].1, t[0].2), (before.1, before.2));
    assert!((t[2].1 - t[0].1 - 10.0).abs() < 1e-9);
}

#[test]
fn moving_average_draws_its_points_and_no_label() {
    let (mut p, g) = project(&XS, &YS, Template::Scatter);
    add(&mut p, g, TrendKind::MovingAverage { window: 3 });
    let s = scene(&p, g);
    let geom = &layout_geometry(&p, g).unwrap()[0];
    let c = curves(&s);
    assert_eq!(c[0].len(), 4);
    let want = trend::moving_average(&XS, &YS, 3);
    for (pt, w) in c[0].iter().zip(want) {
        let q = geom.data_to_page(w.0, w.1);
        assert!((pt.0 - q.0).abs() < 0.02 && (pt.1 - q.1).abs() < 0.02);
    }
    assert!(texts(&s).is_empty());
}

#[test]
fn log_axis_samples_evenly_along_the_axis_and_reversed_axes_work() {
    let xs = [1.0, 10.0, 100.0, 1000.0];
    let ys = [3.0, 5.0, 8.0, 9.5];
    let (mut p, g) = project(&xs, &ys, Template::Scatter);
    p.graph_mut(g).unwrap().layers[0].x_axis.scale = ScaleKind::Log10;
    add(&mut p, g, TrendKind::Logarithmic);
    let s = scene(&p, g);
    let geom = &layout_geometry(&p, g).unwrap()[0];
    let c = &curves(&s)[0];
    assert_eq!(c.len(), 200);
    let step = c[1].0 - c[0].0;
    assert!(step > 0.0);
    assert!(c.windows(2).all(|w| ((w[1].0 - w[0].0) - step).abs() < 0.05), "even page spacing on a log axis");
    let fit = trend::fit(TrendKind::Logarithmic, &xs, &ys).unwrap();
    let want = geom.data_to_page(1000.0, fit.eval(1000.0));
    let last = c[199];
    assert!((last.0 - want.0).abs() < 0.02 && (last.1 - want.1).abs() < 0.02);
    // The logarithmic fit is a straight line on a log X axis.
    let slope = (c[199].1 - c[0].1) / (c[199].0 - c[0].0);
    assert!(c.iter().all(|q| (q.1 - (c[0].1 + slope * (q.0 - c[0].0))).abs() < 0.05));
    // Reversed X: the same curve mirrored.
    p.graph_mut(g).unwrap().layers[0].x_axis.reverse = true;
    let s2 = scene(&p, g);
    let c2 = &curves(&s2)[0];
    assert_eq!(c2.len(), 200);
    assert!(c2[0].0 > c2[199].0);
}

#[test]
fn a_fit_that_cannot_be_made_draws_nothing() {
    let (mut p, g) = project(&XS, &YS, Template::Scatter);
    add(&mut p, g, TrendKind::Linear);
    let clean = scene(&p, g);
    // A power fit of data with non-positive values uses the positive points only; a fit that
    // cannot be made draws nothing.
    let (mut q, h) = project(&[1.0], &[1.0], Template::Scatter);
    add(&mut q, h, TrendKind::Linear);
    let s = scene(&q, h);
    assert!(curves(&s).is_empty() && texts(&s).is_empty());
    assert_eq!(curves(&clean).len(), 1);
}

#[test]
fn style_and_hidden_kinds() {
    let (mut p, g) = project(&XS, &YS, Template::Scatter);
    {
        let t = add(&mut p, g, TrendKind::Polynomial { order: 2 });
        t.width_pt = 3.0;
        t.dash = Dash::Dash;
    }
    let s = scene(&p, g);
    let st = s.layers[0]
        .plots
        .iter()
        .find_map(|i| match i {
            Item::Path { stroke: Some(st), .. } if st.color == CURVE => Some(st.clone()),
            _ => None,
        })
        .unwrap();
    assert_eq!(st.width, 3.0);
    assert!(!st.dash.is_empty());
    // Auto colour is the plot's colour.
    let plot_color = Color { r: 1, g: 2, b: 3, a: 255 };
    let plot = &mut p.graph_mut(g).unwrap().layers[0].plots[0];
    plot.decor.trendlines[0].color = None;
    plot.symbol.color = plot_color;
    let s = scene(&p, g);
    assert!(s.layers[0].plots.iter().any(|i| matches!(i, Item::Text(t) if t.color == plot_color && t.text.starts_with("y"))));
}

#[test]
fn graphs_without_trendlines_export_byte_identically() {
    let (mut p, g) = project(&XS, &YS, Template::LineSymbol);
    let before = render_svg(&p, g).unwrap();
    add(&mut p, g, TrendKind::Linear);
    let with = render_svg(&p, g).unwrap();
    assert_ne!(with, before);
    p.graph_mut(g).unwrap().layers[0].plots[0].decor.trendlines.clear();
    assert_eq!(render_svg(&p, g).unwrap(), before);
}
