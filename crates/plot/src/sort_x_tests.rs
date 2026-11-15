use super::*;
use crate::scene::{Item, Scene};
use sheaf_core::graph::{Connect, Plot, PlotKind};
use sheaf_core::template::Template;
use sheaf_core::{Color, ColumnData, Designation};

const LINE: Color = Color::RED;
const FILL: Color = Color::BLUE;

fn project(xs: Vec<f64>, ys: Vec<f64>, kind: Template) -> (Project, WindowId) {
    let mut p = Project::new();
    let b = p.add_workbook("Data");
    let x = p.add_column(b, 0, Designation::X, ColumnData::numeric(xs)).unwrap();
    let y = p.add_column(b, 0, Designation::Y, ColumnData::numeric(ys)).unwrap();
    let g = p.add_graph(kind);
    p.add_plot_xy(g, 0, Some(x), y, None).unwrap();
    let l = &mut p.graph_mut(g).unwrap().layers[0];
    l.legend.visible = false;
    l.x_axis.min = Some(0.0);
    l.x_axis.max = Some(10.0);
    l.y_axis.min = Some(0.0);
    l.y_axis.max = Some(10.0);
    let plot = &mut l.plots[0];
    plot.line.color = LINE;
    plot.line.width_pt = 2.0;
    plot.symbol.color = Color::rgb(0, 160, 0);
    (p, g)
}

fn plot_mut(p: &mut Project, g: WindowId) -> &mut Plot {
    &mut p.graph_mut(g).unwrap().layers[0].plots[0]
}

fn scene(p: &Project, g: WindowId) -> Scene {
    layout::build_scene(p, g, &FixedMetrics).unwrap()
}

/// Vertices of every stroked, unfilled path in the line colour, one list per path.
fn lines(s: &Scene) -> Vec<Vec<(f64, f64)>> {
    s.layers[0]
        .plots
        .iter()
        .filter_map(|item| match item {
            Item::Path { d, fill: None, stroke: Some(st) } if st.color == LINE => Some(hit::path_points(d)),
            _ => None,
        })
        .collect()
}

fn near(a: &[(f64, f64)], b: &[(f64, f64)]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(p, q)| (p.0 - q.0).abs() < 0.01 && (p.1 - q.1).abs() < 0.01)
}

const XS: [f64; 5] = [3.0, 1.0, 2.0, 5.0, 4.0];
const YS: [f64; 5] = [2.0, 6.0, 4.0, 8.0, 1.0];

fn expected(order: &[usize], geom: &LayerGeometry, xs: &[f64], ys: &[f64]) -> Vec<(f64, f64)> {
    order.iter().map(|&i| geom.data_to_page(xs[i], ys[i])).collect()
}

#[test]
fn flag_off_keeps_worksheet_order_and_renders_exactly_as_before() {
    let (mut p, g) = project(XS.to_vec(), YS.to_vec(), Template::LineSymbol);
    let before = render_svg(&p, g).unwrap();
    let geom = layout_geometry(&p, g).unwrap();
    let l = lines(&scene(&p, g));
    assert_eq!(l.len(), 1);
    assert!(near(&l[0], &expected(&[0, 1, 2, 3, 4], &geom[0], &XS, &YS)));
    plot_mut(&mut p, g).line.sort_x = true;
    assert_ne!(render_svg(&p, g).unwrap(), before, "the flag changes the drawing");
    plot_mut(&mut p, g).line.sort_x = false;
    assert_eq!(render_svg(&p, g).unwrap(), before);
}

#[test]
fn sort_x_connects_in_increasing_x_and_keeps_per_point_items() {
    let (mut p, g) = project(XS.to_vec(), YS.to_vec(), Template::LineSymbol);
    let off = scene(&p, g);
    plot_mut(&mut p, g).line.sort_x = true;
    plot_mut(&mut p, g).decor.labels.enable = true;
    let on = scene(&p, g);
    let geom = layout_geometry(&p, g).unwrap();
    let l = lines(&on);
    assert_eq!(l.len(), 1);
    assert!(near(&l[0], &expected(&[1, 2, 0, 4, 3], &geom[0], &XS, &YS)), "{:?}", l[0]);
    // Everything but the line path (symbols) is identical, in worksheet order.
    let rest = |s: &Scene| -> Vec<String> {
        s.layers[0]
            .plots
            .iter()
            .filter(|i| !matches!(i, Item::Path { fill: None, stroke: Some(st), .. } if st.color == LINE))
            .filter(|i| !matches!(i, Item::Text(_)))
            .map(|i| format!("{i:?}"))
            .collect()
    };
    assert_eq!(rest(&off), rest(&on));
    assert!(!rest(&on).is_empty());
}

#[test]
fn ties_keep_worksheet_order_and_unplotted_kinds_are_unchanged() {
    let xs = [2.0, 1.0, 2.0, 1.0];
    let ys = [1.0, 2.0, 3.0, 4.0];
    let (mut p, g) = project(xs.to_vec(), ys.to_vec(), Template::Line);
    plot_mut(&mut p, g).line.sort_x = true;
    let geom = layout_geometry(&p, g).unwrap();
    let l = lines(&scene(&p, g));
    assert!(near(&l[0], &expected(&[1, 3, 0, 2], &geom[0], &xs, &ys)));
    // Columns ignore the flag.
    let (mut c, cg) = project(XS.to_vec(), YS.to_vec(), Template::Column);
    let before = render_svg(&c, cg).unwrap();
    plot_mut(&mut c, cg).line.sort_x = true;
    assert_eq!(plot_mut(&mut c, cg).kind, PlotKind::Column);
    assert_eq!(render_svg(&c, cg).unwrap(), before);
}

#[test]
fn nan_x_is_left_out_without_breaking_and_missing_y_breaks_between_sorted_neighbours() {
    let xs = [3.0, f64::NAN, 1.0, 2.0, 5.0, 4.0];
    let ys = [2.0, 9.0, 6.0, 4.0, 8.0, 1.0];
    let (mut p, g) = project(xs.to_vec(), ys.to_vec(), Template::Line);
    plot_mut(&mut p, g).line.sort_x = true;
    let geom = layout_geometry(&p, g).unwrap();
    let l = lines(&scene(&p, g));
    assert_eq!(l.len(), 1, "no break at the NaN X");
    assert!(near(&l[0], &expected(&[2, 3, 0, 5, 4], &geom[0], &xs, &ys)));
    // Row 3 (x = 2) has no Y: the line breaks there, in X order.
    let ys = [2.0, 9.0, 6.0, f64::NAN, 8.0, 1.0];
    let (mut p, g) = project(xs.to_vec(), ys.to_vec(), Template::Line);
    plot_mut(&mut p, g).line.sort_x = true;
    let geom = layout_geometry(&p, g).unwrap();
    let l = lines(&scene(&p, g));
    assert_eq!(l.len(), 1, "the lone point 1 has no segment");
    assert!(near(&l[0], &expected(&[0, 5, 4], &geom[0], &xs, &ys)));
}

#[test]
fn steps_and_splines_follow_the_sorted_path() {
    let (mut p, g) = project(XS.to_vec(), YS.to_vec(), Template::Line);
    plot_mut(&mut p, g).line.sort_x = true;
    plot_mut(&mut p, g).line.connect = Connect::StepHorizontal;
    let l = lines(&scene(&p, g));
    assert!(l[0].windows(2).all(|w| w[0].0 <= w[1].0 + 1e-9), "step path runs left to right");
    assert_eq!(l[0].len(), 9);
    plot_mut(&mut p, g).line.connect = Connect::Spline;
    let l = lines(&scene(&p, g));
    assert!(l[0].len() > 5, "sorted unique X makes a real spline");
    assert!(l[0].windows(2).all(|w| w[0].0 <= w[1].0 + 1e-9));
    // Without the flag the unsorted X draws straight segments.
    plot_mut(&mut p, g).line.sort_x = false;
    assert_eq!(lines(&scene(&p, g))[0].len(), 5);
}

#[test]
fn fill_under_follows_the_sorted_path() {
    let (mut p, g) = project(XS.to_vec(), YS.to_vec(), Template::Line);
    plot_mut(&mut p, g).line.sort_x = true;
    plot_mut(&mut p, g).decor.fill_under = Some(FILL);
    let s = scene(&p, g);
    let geom = layout_geometry(&p, g).unwrap();
    let d = s.layers[0]
        .plots
        .iter()
        .find_map(|i| match i {
            Item::Path { d, fill: Some(c), .. } if *c == FILL => Some(d.clone()),
            _ => None,
        })
        .unwrap();
    let pts = hit::path_points(&d);
    let mut want = vec![(geom[0].data_to_page(1.0, 0.0).0, geom[0].frame.y + geom[0].frame.h)];
    want.extend(expected(&[1, 2, 0, 4, 3], &geom[0], &XS, &YS));
    want.push((geom[0].data_to_page(5.0, 0.0).0, geom[0].frame.y + geom[0].frame.h));
    assert!(near(&pts[..want.len()], &want), "{pts:?}");
}

#[test]
fn gap_to_symbol_uses_the_sorted_neighbours() {
    let (mut p, g) = project(vec![1.0, 9.0, 5.0], vec![5.0, 5.0, 5.0], Template::LineSymbol);
    plot_mut(&mut p, g).symbol.size_pt = 8.0;
    plot_mut(&mut p, g).decor.symbol_line.gap_to_symbol = true;
    plot_mut(&mut p, g).line.sort_x = true;
    let l = lines(&scene(&p, g));
    // 1 -> 5 and 5 -> 9: two clipped segments, none crossing from 1 to 9 over the middle symbol.
    assert_eq!(l.iter().map(Vec::len).sum::<usize>(), 4, "{l:?}");
    let geom = layout_geometry(&p, g).unwrap();
    let mid = geom[0].data_to_page(5.0, 5.0).0;
    assert!(l.iter().all(|seg| seg.iter().all(|pt| (pt.0 - mid).abs() > 3.0)));
}
