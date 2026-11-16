//! Displaced Y axes (T61 slice 2, graphing spec 21.3 and 21.5): an extra Y axis stands outward from the
//! frame by a percentage of its width and takes part in the automatic margins, hit testing and linked
//! ranges like any other axis.

use super::*;
use sheaf_core::graph::{AxisSide, ScaleLink};
use sheaf_core::layers::NewLayerKind;
use sheaf_core::template::Template;

/// Three layers: the left axis at the frame, a right axis at the frame (layer 1) and a second right
/// axis (layer 2) displaced by `offset` percent.
fn three_axes(offset: f64) -> (Project, WindowId) {
    let (x, shape) = super::tests::golden_data();
    let (mut p, g) = super::tests::project(Template::Line, x, vec![shape(30.0), shape(1.0), shape(0.01)]);
    let graph = p.graph_mut(g).unwrap();
    graph.set_plot_y_axis(0, 1, true).unwrap();
    graph.add_layer(NewLayerKind::RightY, 0).unwrap();
    graph.move_plot_to_layer(0, 1, 2).unwrap();
    // The multi-Y templates draw no Y titles (graphing spec 21.5).
    graph.layers.iter_mut().for_each(|l| l.y_axis.show_title = false);
    graph.layers[2].y_axis.offset_pct = offset;
    assert_eq!(graph.layers[2].y_axis.side, AxisSide::Opposite);
    (p, g)
}

fn geoms(p: &Project, g: WindowId) -> Vec<LayerGeometry> {
    let graph = p.graph(g).unwrap();
    let pds: Vec<data::LayerData> = graph.layers.iter().map(|l| data::collect(p, l)).collect();
    layout::graph_geometry(p, graph, &pds, &FixedMetrics)
}

/// The X of every vertical straight line (axis lines, not ticks) of a layer's overlay.
fn vertical_lines(items: &[scene::Item], width: f64) -> Vec<f64> {
    items
        .iter()
        .filter_map(|it| match it {
            scene::Item::Path { d, stroke: Some(s), .. } if s.width == width => {
                let pts = hit::path_points(d);
                (pts.len() == 2 && (pts[0].0 - pts[1].0).abs() < 1e-9 && (pts[0].1 - pts[1].1).abs() > 10.0)
                    .then_some(pts[0].0)
            }
            _ => None,
        })
        .collect()
}

#[test]
fn the_axis_line_stands_outward_by_a_percentage_of_the_frame_width() {
    let (p, g) = three_axes(12.0);
    let gs = geoms(&p, g);
    let f = gs[2].frame;
    assert_eq!(gs[0].frame, f, "the frame itself does not move");
    let scene = layout::build_scene(&p, g, &FixedMetrics).unwrap();
    let width = p.graph(g).unwrap().layers[2].y_axis.line_width_pt;
    let xs = vertical_lines(&scene.layers[2].overlay, width);
    assert_eq!(xs.len(), 1, "{xs:?}");
    assert!((xs[0] - (f.x + f.w + 0.12 * f.w)).abs() < 1e-2, "{} {f:?}", xs[0]);
    // The tick labels come with it: they are right of the displaced line (the legend is drawn in this overlay too).
    let right_of_line = scene.layers[2]
        .overlay
        .iter()
        .filter(|i| matches!(i, scene::Item::Text(t) if t.x > xs[0] && t.anchor == scene::Anchor::Start))
        .count();
    assert!(right_of_line >= 3, "{right_of_line}");
}

#[test]
fn a_negative_offset_puts_the_axis_inside_the_frame() {
    let (mut p, g) = three_axes(0.0);
    {
        let l = &mut p.graph_mut(g).unwrap().layers[2];
        l.y_axis.side = AxisSide::Normal;
        l.y_axis.offset_pct = -50.0;
    }
    let f = geoms(&p, g)[2].frame;
    let scene = layout::build_scene(&p, g, &FixedMetrics).unwrap();
    let width = p.graph(g).unwrap().layers[2].y_axis.line_width_pt;
    let xs = vertical_lines(&scene.layers[2].overlay, width);
    assert_eq!(xs.len(), 1);
    assert!((xs[0] - (f.x + 0.5 * f.w)).abs() < 1e-2, "{} {f:?}", xs[0]);
}

#[test]
fn a_displaced_axis_makes_room_in_the_automatic_margins() {
    let (p0, g0) = three_axes(0.0);
    let (p1, g1) = three_axes(40.0);
    let (a, b) = (geoms(&p0, g0)[0].frame, geoms(&p1, g1)[0].frame);
    assert!(b.w < a.w - 1.0, "the frame narrows: {a:?} {b:?}");
    assert!((a.x - b.x).abs() < 1e-6, "the left side stays");
    // Everything the axis draws stays on the page, 3 pt inside.
    let scene = layout::build_scene(&p1, g1, &FixedMetrics).unwrap();
    let page = p1.graph(g1).unwrap().page.width_pt;
    for it in &scene.layers[2].overlay {
        if let scene::Item::Text(t) = it
            && t.rotate_ccw == 0.0
        {
            assert!(t.x < page - 3.0, "{t:?}");
        }
    }
    let view = margin::drawn_bounds(&scene, &FixedMetrics, None).unwrap();
    assert!(view.x + view.w <= page + 1e-6, "{view:?}");
}

#[test]
fn the_displaced_axis_draws_outside_the_clip_and_is_a_hit_target_of_its_own_layer() {
    let (p, g) = three_axes(12.0);
    let gs = geoms(&p, g);
    let f = gs[2].frame;
    let scene = layout::build_scene(&p, g, &FixedMetrics).unwrap();
    assert_eq!(scene.layers[2].clip, f, "data stay clipped to the frame");
    let line_x = f.x + 1.12 * f.w;
    assert!(line_x > scene.layers[2].clip.x + scene.layers[2].clip.w);
    let mid = f.y + f.h / 2.0;
    match hit::hit_test(&scene, (line_x, mid), 3.0) {
        hit::Hit::Object { layer: 2, kind: scene::ObjectKind::YAxis } => {}
        other => panic!("{other:?}"),
    }
    // The axis at the frame still belongs to layer 1.
    match hit::hit_test(&scene, (f.x + f.w, mid), 3.0) {
        hit::Hit::Object { layer: 1, kind: scene::ObjectKind::YAxis } => {}
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_linked_scale_stays_the_parents_whatever_the_offset() {
    let (mut p, g) = three_axes(12.0);
    {
        let l = &mut p.graph_mut(g).unwrap().layers[2];
        l.link.as_mut().unwrap().y = ScaleLink::Straight;
    }
    let gs = geoms(&p, g);
    assert_eq!(gs[2].y, gs[0].y);
    assert_eq!(gs[2].x, gs[0].x);
}

#[test]
fn golden_three_y_axes_with_an_offset_axis() {
    let (p, g) = three_axes(12.0);
    super::tests::compare_golden("multi-y-offset-axis", &super::tests::svg_of(&p, g));
}
