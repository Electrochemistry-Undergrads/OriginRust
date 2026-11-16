//! Colour scales and bubble scales of a layer (T80, graphing spec 27).

use super::*;
use crate::tests::{doc, svg_of, texts};
use sheaf_core::layer_scale::{LayerScale, LayerScaleKind};
use sheaf_core::template::Template;
use sheaf_core::{ColumnData, Designation};

/// A scatter graph of three points whose symbols take colour and size from a Z column "Depth" (1, 5, 9).
fn graph(color: bool, size: bool) -> (Project, WindowId) {
    let mut p = Project::new();
    let b = p.add_workbook("data");
    let x = p.add_column(b, 0, Designation::X, ColumnData::numeric(vec![1.0, 5.0, 9.0])).unwrap();
    let y = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![2.0, 4.0, 8.0])).unwrap();
    let z = p.add_column(b, 0, Designation::Z, ColumnData::numeric(vec![1.0, 5.0, 9.0])).unwrap();
    p.column_mut(z).unwrap().long_name = "Depth".into();
    let g = p.add_graph(Template::Scatter);
    p.add_plot_xy(g, 0, Some(x), y, None).unwrap();
    let plot = &mut p.graph_mut(g).unwrap().layers[0].plots[0];
    plot.color_from = color.then_some(z);
    plot.size_from = size.then_some(z);
    (p, g)
}

fn add(p: &mut Project, g: WindowId, kind: LayerScaleKind) {
    p.graph_mut(g).unwrap().layers[0].scales.push(LayerScale::new(kind));
}

fn frame(p: &Project, g: WindowId) -> Rect {
    let gr = p.graph(g).unwrap();
    layout::graph_geometry(p, gr, &data::collect_graph(p, gr), &FixedMetrics)[0].frame
}

fn strips(scene: &scene::Scene, f: Rect) -> usize {
    scene.layers[0]
        .overlay
        .iter()
        .filter(|it| matches!(it, scene::Item::Rect { rect, fill: Some(_), .. } if rect.x >= f.x + f.w))
        .count()
}

#[test]
fn a_colour_mapped_plot_draws_no_scale_until_one_is_added() {
    let (mut p, g) = graph(true, false);
    let f = frame(&p, g);
    let before = layout::build_scene(&p, g, &FixedMetrics).unwrap();
    assert_eq!(strips(&before, f), 0);
    add(&mut p, g, LayerScaleKind::Color);
    let f2 = frame(&p, g);
    let _ = (f, &before);
    let scene = layout::build_scene(&p, g, &FixedMetrics).unwrap();
    assert_eq!(strips(&scene, f2), 64, "the same 64 strip ramp as the hexbin scale");
    let t = texts(&doc(&svg_of(&p, g)));
    assert!(t.iter().any(|s| s == "Depth"), "titled with the Z column: {t:?}");
    assert!(t.iter().any(|s| s == "8"), "ticks over 1..9: {t:?}");
    let obj = &scene.layers[0].objects;
    assert!(obj.iter().any(|o| o.kind == scene::ObjectKind::Scale(0)));
    let o = obj.iter().find(|o| o.kind == scene::ObjectKind::Scale(0)).unwrap();
    assert!(o.bounds.x >= f2.x + f2.w && o.bounds.x + o.bounds.w < scene.width);
}

#[test]
fn a_bubble_scale_shows_four_circles_from_the_smallest_to_the_largest_value() {
    let (mut p, g) = graph(false, true);
    add(&mut p, g, LayerScaleKind::Bubble);
    let t = texts(&doc(&svg_of(&p, g)));
    for label in ["1.00", "3.67", "6.33", "9.00", "Depth"] {
        assert!(t.iter().any(|s| s == label), "{label} in {t:?}");
    }
    let scene = layout::build_scene(&p, g, &FixedMetrics).unwrap();
    let o = scene.layers[0].objects.iter().find(|o| o.kind == scene::ObjectKind::Scale(0)).unwrap();
    let f = frame(&p, g);
    assert!(o.bounds.x >= f.x + f.w && o.bounds.x + o.bounds.w < scene.width);
}

#[test]
fn a_scale_without_a_mapped_plot_draws_nothing_and_takes_no_room() {
    let (mut p, g) = graph(false, false);
    let f = frame(&p, g);
    add(&mut p, g, LayerScaleKind::Color);
    add(&mut p, g, LayerScaleKind::Bubble);
    assert_eq!(frame(&p, g), f);
    let scene = layout::build_scene(&p, g, &FixedMetrics).unwrap();
    assert!(!scene.layers[0].objects.iter().any(|o| matches!(o.kind, scene::ObjectKind::Scale(_))));
}

#[test]
fn two_scales_stand_side_by_side() {
    let (mut p, g) = graph(true, true);
    add(&mut p, g, LayerScaleKind::Color);
    add(&mut p, g, LayerScaleKind::Bubble);
    let scene = layout::build_scene(&p, g, &FixedMetrics).unwrap();
    let find = |k| scene.layers[0].objects.iter().find(|o| o.kind == scene::ObjectKind::Scale(k)).unwrap().bounds;
    let (a, b) = (find(0), find(1));
    assert!(b.x >= a.x + a.w - 1e-9, "{a:?} {b:?}");
    assert!(b.x + b.w < scene.width);
}

#[test]
fn a_click_on_the_scale_picks_it() {
    let (mut p, g) = graph(true, false);
    add(&mut p, g, LayerScaleKind::Color);
    let scene = layout::build_scene(&p, g, &FixedMetrics).unwrap();
    let o = scene.layers[0].objects.iter().find(|o| o.kind == scene::ObjectKind::Scale(0)).unwrap();
    let c = (o.bounds.x + o.bounds.w / 2.0, o.bounds.y + o.bounds.h / 2.0);
    match hit::hit_test(&scene, c, 2.0) {
        hit::Hit::Object { kind, .. } => assert_eq!(kind, scene::ObjectKind::Scale(0)),
        other => panic!("{other:?}"),
    }
}
