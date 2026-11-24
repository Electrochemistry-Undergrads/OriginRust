//! Displaced Y axes (T61 slice 2, graphing spec 21.3 and 21.5): an extra Y axis stands outward from the
//! frame by a percentage of its width and takes part in the automatic margins, hit testing and linked
//! ranges like any other axis.

use super::*;
use sheaf_core::graph::{AxisSide, OppositeAxisStyle, ScaleLink, TickDirection};
use sheaf_core::layers::{NewLayerKind, UserDefinedLayer};
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

/// The Y of every horizontal straight line (axis lines, not ticks) of a layer's overlay.
fn horizontal_lines(items: &[scene::Item], width: f64) -> Vec<f64> {
    items
        .iter()
        .filter_map(|it| match it {
            scene::Item::Path { d, stroke: Some(s), .. } if s.width == width => {
                let pts = hit::path_points(d);
                (pts.len() == 2 && (pts[0].1 - pts[1].1).abs() < 1e-9 && (pts[0].0 - pts[1].0).abs() > 10.0)
                    .then_some(pts[0].1)
            }
            _ => None,
        })
        .collect()
}

#[test]
fn paired_axes_draw_pickable_labels_and_titles_at_separate_offsets() {
    let (mut p, g) = three_axes(0.0);
    let graph = p.graph_mut(g).unwrap();
    let i = graph
        .add_user_defined_layer(
            UserDefinedLayer {
                top: true,
                right: true,
                parent: Some(0),
                x_link: ScaleLink::Straight,
                ..Default::default()
            },
            0,
            false,
        )
        .unwrap();
    let layer = &mut graph.layers[i];
    layer.x_axis.offset_pct = 5.0;
    layer.x_axis.opposite_offset_pct = Some(12.0);
    layer.x_axis.show_title = true;
    layer.x_axis.title_on_top = true;
    layer.y_axis.offset_pct = 8.0;
    layer.y_axis.opposite_offset_pct = Some(16.0);
    layer.y_axis.show_title = true;
    let (x_width, y_width) = (layer.x_axis.line_width_pt, layer.y_axis.line_width_pt);
    let geometry = geoms(&p, g);
    let f = geometry[i].frame;
    assert_eq!(geometry[0].frame, f);
    assert_eq!(geometry[0].x, geometry[i].x);

    let scene = layout::build_scene(&p, g, &FixedMetrics).unwrap();
    let overlay = &scene.layers[i].overlay;
    assert!(
        overlay[overlay.len() - 2..]
            .iter()
            .all(|item| matches!(item, scene::Item::Text(t) if t.text == "X Axis Title"))
    );
    let mut xs = vertical_lines(overlay, y_width);
    let mut ys = horizontal_lines(overlay, x_width);
    xs.sort_by(f64::total_cmp);
    ys.sort_by(f64::total_cmp);
    let (left, right) = (f.x - 0.08 * f.w, f.x + 1.16 * f.w);
    let (top, bottom) = (f.y - 0.12 * f.h, f.y + 1.05 * f.h);
    assert_eq!((xs.len(), ys.len()), (2, 2), "{xs:?} {ys:?}");
    for (actual, expected) in xs.into_iter().chain(ys).zip([left, right, top, bottom]) {
        assert!((actual - expected).abs() < 0.01, "{actual} vs {expected}");
    }
    let texts: Vec<_> =
        overlay.iter().filter_map(|item| if let scene::Item::Text(t) = item { Some(t) } else { None }).collect();
    for (title, past_face) in [
        ("X Axis Title", texts.iter().any(|t| t.text == "X Axis Title" && t.y < top)),
        ("X Axis Title", texts.iter().any(|t| t.text == "X Axis Title" && t.y > bottom)),
        ("Y Axis Title", texts.iter().any(|t| t.text == "Y Axis Title" && t.x < left)),
        ("Y Axis Title", texts.iter().any(|t| t.text == "Y Axis Title" && t.x > right)),
    ] {
        assert!(past_face, "{title}: {texts:?}");
    }
    assert!(texts.iter().any(|t| t.text.parse::<f64>().is_ok() && t.y < top));
    assert!(texts.iter().any(|t| t.text.parse::<f64>().is_ok() && t.x > right));
    let (mid_x, mid_y) = (f.x + f.w / 2.0, f.y + f.h / 2.0);
    for (xy, kind) in [((mid_x, top), scene::ObjectKind::XAxis), ((right, mid_y), scene::ObjectKind::YAxis)] {
        assert_eq!(hit::hit_test(&scene, xy, 3.0), hit::Hit::Object { layer: i, kind });
    }
    let b = margin::drawn_bounds(&scene, &FixedMetrics, None).unwrap();
    assert!(b.x < left && b.x + b.w > right && b.y < top && b.y + b.h > bottom, "{b:?}");
}

#[test]
fn opposite_axis_style_sets_its_own_tick_direction_and_labels() {
    let (x, shape) = super::tests::golden_data();
    let (mut p, g) = super::tests::project(Template::Line, x, vec![shape(1.0)]);
    let axis = &mut p.graph_mut(g).unwrap().layers[0].x_axis;
    axis.min = Some(0.0);
    axis.max = Some(5.0);
    axis.major_step = Some(1.0);
    axis.minor_ticks = 0;
    axis.major_tick = TickDirection::Out;
    axis.tick_color = Some(sheaf_core::Color::RED);
    axis.show_opposite = true;
    axis.opposite_ticks = true;
    axis.title = Some("Facing Title".into());
    axis.show_title = false;
    axis.opposite_style = OppositeAxisStyle {
        tick_direction: Some(TickDirection::In),
        show_title: Some(true),
        show_tick_labels: Some(true),
        tick_label_formula: Some("10*x+100".into()),
    };

    for show_line in [true, false] {
        p.graph_mut(g).unwrap().layers[0].x_axis.show_opposite = show_line;
        let scene = layout::build_scene(&p, g, &FixedMetrics).unwrap();
        let layer = &scene.layers[0];
        let frame = layer.clip;
        let bottom = frame.y + frame.h;
        assert!(layer.overlay.iter().any(|i| matches!(i, scene::Item::Text(t) if t.text == "100" && t.y < frame.y)));
        assert!(layer.overlay.iter().any(|i| matches!(i, scene::Item::Text(t) if t.text == "0" && t.y > bottom)));
        let titles: Vec<_> = layer
            .overlay
            .iter()
            .filter_map(|i| match i {
                scene::Item::Text(t) if t.text == "Facing Title" => Some(t.y),
                _ => None,
            })
            .collect();
        assert_eq!(titles.len(), 1);
        assert!(titles[0] < frame.y);

        let ticks: Vec<_> = layer
            .overlay
            .iter()
            .filter_map(|i| match i {
                scene::Item::Path { d, stroke: Some(s), .. } if s.color == sheaf_core::Color::RED => {
                    Some(hit::path_points(d))
                }
                _ => None,
            })
            .collect();
        assert_eq!(ticks.len(), if show_line { 2 } else { 1 });
        for [(ax, ay), (bx, by)] in ticks[0].as_chunks::<2>().0 {
            assert_eq!(ax, bx);
            assert!((*ay).min(*by) >= bottom - 0.002 && (*ay).max(*by) > bottom + 0.002);
        }
        if show_line {
            for [(ax, ay), (bx, by)] in ticks[1].as_chunks::<2>().0 {
                assert_eq!(ax, bx);
                assert!((*ay).min(*by) >= frame.y - 0.002 && (*ay).max(*by) > frame.y + 0.002);
            }
        }
    }

    let axis = &mut p.graph_mut(g).unwrap().layers[0].x_axis;
    axis.tick_label_formula = "x+20".into();
    axis.tick_prefix = "[".into();
    axis.tick_suffix = "]".into();
    axis.opposite_style.tick_label_formula = Some(String::new());
    let scene = layout::build_scene(&p, g, &FixedMetrics).unwrap();
    let layer = &scene.layers[0];
    assert!(layer.overlay.iter().any(|i| matches!(i, scene::Item::Text(t) if t.text == "[0]" && t.y < layer.clip.y)));
    assert!(
        layer
            .overlay
            .iter()
            .any(|i| matches!(i, scene::Item::Text(t) if t.text == "[20]" && t.y > layer.clip.y + layer.clip.h))
    );
}

#[test]
fn scatter_central_draws_both_axes_in_the_middle_of_a_boxed_frame() {
    let (x, shape) = super::tests::golden_data();
    let (p, g) = super::tests::project(Template::ScatterCentral, x, vec![shape(1.0)]);
    let layer = &p.graph(g).unwrap().layers[0];
    assert_eq!((layer.x_axis.offset_pct, layer.y_axis.offset_pct), (-50.0, -50.0));
    let f = geoms(&p, g)[0].frame;
    let scene = layout::build_scene(&p, g, &FixedMetrics).unwrap();
    let width = layer.x_axis.line_width_pt;
    let ys = horizontal_lines(&scene.layers[0].overlay, width);
    let xs = vertical_lines(&scene.layers[0].overlay, width);
    let near = |v: &[f64], t: f64| v.iter().any(|a| (a - t).abs() < 1e-2);
    assert!(near(&ys, f.y + 0.5 * f.h), "X axis at the vertical middle: {ys:?} {f:?}");
    assert!(near(&xs, f.x + 0.5 * f.w), "Y axis at the horizontal middle: {xs:?} {f:?}");
    // The frame is a box: the four edges are drawn too.
    assert!(near(&ys, f.y) && near(&ys, f.y + f.h), "{ys:?}");
    assert!(near(&xs, f.x) && near(&xs, f.x + f.w), "{xs:?}");
}

#[test]
fn an_x_axis_offset_moves_line_labels_and_title_and_its_hit_box() {
    let (x, shape) = super::tests::golden_data();
    let (mut p, g) = super::tests::project(Template::Scatter, x, vec![shape(1.0)]);
    let base = layout::build_scene(&p, g, &FixedMetrics).unwrap();
    p.graph_mut(g).unwrap().layers[0].x_axis.offset_pct = -50.0;
    let f = geoms(&p, g)[0].frame;
    let scene = layout::build_scene(&p, g, &FixedMetrics).unwrap();
    let width = p.graph(g).unwrap().layers[0].x_axis.line_width_pt;
    let ys = horizontal_lines(&scene.layers[0].overlay, width);
    assert!(ys.iter().any(|y| (y - (f.y + 0.5 * f.h)).abs() < 1e-2), "{ys:?}");
    assert!(!ys.iter().any(|y| (y - (f.y + f.h)).abs() < 1e-2), "the bottom edge has no line: {ys:?}");
    // Tick labels and title move up with the axis: none of the horizontal text is below the frame.
    let below = scene.layers[0]
        .overlay
        .iter()
        .filter(|i| matches!(i, scene::Item::Text(t) if t.rotate_ccw == 0.0 && t.y > f.y + f.h))
        .count();
    let was_below = base.layers[0]
        .overlay
        .iter()
        .filter(|i| matches!(i, scene::Item::Text(t) if t.rotate_ccw == 0.0 && t.y > f.y + f.h))
        .count();
    assert!(was_below > 0 && below < was_below, "{below} {was_below}");
    // A click on the moved axis line selects the X axis.
    match hit::hit_test(&scene, (f.x + 0.5 * f.w, f.y + 0.5 * f.h), 3.0) {
        hit::Hit::Object { layer: 0, kind: scene::ObjectKind::XAxis } => {}
        other => panic!("{other:?}"),
    }
}

#[test]
fn scatter_central_titles_stand_at_the_ends_of_the_moved_axes() {
    let (x, shape) = super::tests::golden_data();
    let (p, g) = super::tests::project(Template::ScatterCentral, x, vec![shape(1.0)]);
    let f = geoms(&p, g)[0].frame;
    let scene = layout::build_scene(&p, g, &FixedMetrics).unwrap();
    let texts: Vec<&scene::TextItem> = scene.layers[0]
        .overlay
        .iter()
        .filter_map(|i| match i {
            scene::Item::Text(t) => Some(t),
            _ => None,
        })
        .collect();
    // Y title: unrotated, above the frame top, at the Y axis line's x (the horizontal middle).
    let y_title = texts.iter().find(|t| {
        t.rotate_ccw == 0.0 && t.y < f.y && (t.x - (f.x + 0.5 * f.w)).abs() < 1e-2 && t.anchor == scene::Anchor::Middle
    });
    assert!(y_title.is_some(), "no horizontal title above the frame: {f:?}");
    // X title: rotated, left of the frame, at the X axis line's y (the vertical middle).
    let x_title = texts.iter().find(|t| t.rotate_ccw != 0.0 && t.x < f.x && (t.y - (f.y + 0.5 * f.h)).abs() < 1e-2);
    assert!(x_title.is_some(), "no rotated title left of the frame: {f:?}");
}

#[test]
fn an_axis_at_a_position_stands_where_the_other_axis_has_that_value() {
    // T243: Origin's Axis Position "At Position =". Y from -1 to 3: an X axis at Y = 0 stands a quarter
    // of the way up; a Y axis at X = the middle of X stands in the middle.
    let (x, shape) = super::tests::golden_data();
    let (mut p, g) = super::tests::project(Template::Scatter, x, vec![shape(1.0)]);
    {
        let l = &mut p.graph_mut(g).unwrap().layers[0];
        (l.y_axis.min, l.y_axis.max) = (Some(-1.0), Some(3.0));
        l.x_axis.at_position = Some(0.0);
    }
    let f = geoms(&p, g)[0].frame;
    let width = p.graph(g).unwrap().layers[0].x_axis.line_width_pt;
    let ys =
        |p: &Project| horizontal_lines(&layout::build_scene(p, g, &FixedMetrics).unwrap().layers[0].overlay, width);
    let near = |v: &[f64], t: f64| v.iter().any(|a| (a - t).abs() < 1e-2);
    let got = ys(&p);
    assert!(near(&got, f.y + 0.75 * f.h), "X axis at Y = 0: {got:?} {f:?}");
    assert!(!near(&got, f.y + f.h), "the bottom edge has no line: {got:?}");
    // It follows the other axis: Y from 0 to 4 puts Y = 0 on the bottom edge.
    {
        let l = &mut p.graph_mut(g).unwrap().layers[0];
        (l.y_axis.min, l.y_axis.max) = (Some(0.0), Some(4.0));
    }
    assert!(near(&ys(&p), f.y + f.h), "{:?}", ys(&p));
    // A value beyond the range puts the axis outside the frame where it would be (Origin 2016, T266):
    // Y = 3.5 on -1..3 is an eighth of the height above the top, Y = -1.5 an eighth below the bottom.
    {
        let l = &mut p.graph_mut(g).unwrap().layers[0];
        (l.y_axis.min, l.y_axis.max) = (Some(-1.0), Some(3.0));
        l.x_axis.at_position = Some(3.5);
    }
    let f = geoms(&p, g)[0].frame;
    assert!(near(&ys(&p), f.y - 0.125 * f.h), "above the top: {:?} {f:?}", ys(&p));
    p.graph_mut(g).unwrap().layers[0].x_axis.at_position = Some(-1.5);
    let f = geoms(&p, g)[0].frame;
    assert!(near(&ys(&p), f.y + 1.125 * f.h), "below the bottom: {:?} {f:?}", ys(&p));
    {
        let l = &mut p.graph_mut(g).unwrap().layers[0];
        l.x_axis.at_position = Some(2.0);
        l.x_axis.side = AxisSide::Opposite;
    }
    let f = geoms(&p, g)[0].frame;
    assert!(near(&ys(&p), f.y + 0.25 * f.h), "{:?} {f:?}", ys(&p));
    // A Y axis at an X value.
    let x_mid = {
        let l = &p.graph(g).unwrap().layers[0];
        let geom = &geoms(&p, g)[0];
        let _ = l;
        0.5 * (geom.x.min + geom.x.max)
    };
    p.graph_mut(g).unwrap().layers[0].y_axis.at_position = Some(x_mid);
    let f = geoms(&p, g)[0].frame;
    let scene = layout::build_scene(&p, g, &FixedMetrics).unwrap();
    let xs = vertical_lines(&scene.layers[0].overlay, p.graph(g).unwrap().layers[0].y_axis.line_width_pt);
    assert!(near(&xs, f.x + 0.5 * f.w), "Y axis at the middle X: {xs:?} {f:?}");
    match hit::hit_test(&scene, (f.x + 0.5 * f.w, f.y + 0.6 * f.h), 3.0) {
        hit::Hit::Object { layer: 0, kind: scene::ObjectKind::YAxis } => {}
        other => panic!("a click on the moved line selects the Y axis: {other:?}"),
    }
}

/// The filled paths without a stroke, as their first point (the tip of an arrowhead).
fn filled_tips(items: &[scene::Item]) -> Vec<(f64, f64)> {
    items
        .iter()
        .filter_map(|i| match i {
            scene::Item::Path { d, fill: Some(_), stroke: None } => {
                let nums: Vec<f64> = d
                    .split(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-' || c == 'e'))
                    .filter_map(|t| t.parse().ok())
                    .collect();
                (nums.len() >= 6 && d.ends_with('Z')).then(|| (nums[0], nums[1]))
            }
            _ => None,
        })
        .collect()
}

#[test]
fn axis_arrows_point_out_of_the_chosen_ends_in_the_line_colour() {
    // T245: Arrow at Begin / Arrow at End.
    let (x, shape) = super::tests::golden_data();
    let (mut p, g) = super::tests::project(Template::Scatter, x, vec![shape(1.0)]);
    let base = layout::build_scene(&p, g, &FixedMetrics).unwrap();
    let f = geoms(&p, g)[0].frame;
    {
        let l = &mut p.graph_mut(g).unwrap().layers[0];
        l.x_axis.arrow_end = true;
        l.y_axis.arrow_end = true;
        l.y_axis.arrow_begin = true;
    }
    let scene = layout::build_scene(&p, g, &FixedMetrics).unwrap();
    let tips = filled_tips(&scene.layers[0].overlay);
    let before = filled_tips(&base.layers[0].overlay);
    assert_eq!(tips.len(), before.len() + 3, "{tips:?}");
    let (len, _) = crate::layout::arrow_size(p.graph(g).unwrap().layers[0].x_axis.line_width_pt);
    let near = |a: (f64, f64), b: (f64, f64)| (a.0 - b.0).abs() < 1e-2 && (a.1 - b.1).abs() < 1e-2;
    let has = |t: (f64, f64)| tips.iter().any(|&p| near(p, t));
    assert!(has((f.x + f.w + len, f.y + f.h)), "X end arrow right of the frame: {tips:?} {f:?}");
    assert!(has((f.x, f.y - len)), "Y end arrow above the frame: {tips:?}");
    assert!(has((f.x, f.y + f.h + len)), "Y begin arrow below the frame: {tips:?}");
    assert!(!has((f.x - len, f.y + f.h)), "no X begin arrow: {tips:?}");
    // The arrows move with the axis.
    p.graph_mut(g).unwrap().layers[0].x_axis.offset_pct = -50.0;
    let scene = layout::build_scene(&p, g, &FixedMetrics).unwrap();
    let f = geoms(&p, g)[0].frame;
    let tips = filled_tips(&scene.layers[0].overlay);
    assert!(tips.iter().any(|&t| near(t, (f.x + f.w + len, f.y + 0.5 * f.h))), "{tips:?}");
    assert_eq!(crate::layout::arrow_size(0.5), (6.0, 2.0), "small lines get the smallest head");
    assert_eq!(crate::layout::arrow_size(2.0), (10.0, 3.5));
}

#[test]
fn axis_token_options_give_what_origin_2016_draws() {
    // T281: measured with yl.text$ = "[%(?Y,option)]" on column B of Book1 (re-workspace/scratch/t281).
    use sheaf_core::{ColumnData, Project, column::Designation};
    let mut p = Project::new();
    let book = p.add_workbook("Book1");
    let b = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![1.0])).unwrap();
    let labels = |p: &mut Project, long: &str, units: &str, comments: &str| {
        let c = p.column_mut(b).unwrap();
        (c.long_name, c.units, c.comments) = (long.into(), units.into(), comments.into());
    };
    let show = |p: &Project, option: &str| layout::token_option_text(p, p.column(b).unwrap(), option);
    let short = p.column(b).unwrap().short_name.clone();
    // Long name Volt, units mV, comments cmt.
    labels(&mut p, "Volt", "mV", "cmt");
    for (option, want) in [
        ("", "Volt (mV)"),
        ("@LG", "Volt (mV)"),
        ("@U", "Volt (mV)"),
        ("@LL", "Volt"),
        ("@L", "Volt"),
        ("@LU", "mV"),
        ("@LC", "cmt"),
        ("@LA", "Volt"),
        ("@LN", "cmt (mV)"),
        ("@WS", "Sheet1"),
        ("@W", "Book1"),
        ("@LP", ""),
        ("@X", "###"),
    ] {
        assert_eq!(show(&p, option), want, "{option}");
    }
    assert_eq!(show(&p, "@LS"), short);
    assert_eq!(show(&p, "@C"), short);
    assert_eq!(show(&p, "@D"), format!("Book1_{short}"));
    // No labels: the short name where Origin falls back to it, nothing where it does not.
    labels(&mut p, "", "", "");
    for (option, want) in
        [("", &short[..]), ("@LG", &short), ("@LL", ""), ("@LU", ""), ("@LA", &short), ("@LN", &short)]
    {
        assert_eq!(show(&p, option), want, "no labels, {option}");
    }
    // Comments only: @LN shows them, @LA does not.
    labels(&mut p, "", "", "cmt");
    assert_eq!((show(&p, "@LN"), show(&p, "@LA"), show(&p, "")), ("cmt".into(), short.clone(), short.clone()));
}

#[test]
fn a_turned_title_keeps_its_middle_and_moves_by_its_offset() {
    // T291: Origin turns a title about its middle (measured in Origin 2016: yl.rotate = 45 leaves
    // the title's centre where it was).
    let (x, shape) = super::tests::golden_data();
    let (p, g) = super::tests::project(Template::Scatter, x, vec![shape(1.0)]);
    let mut axis = p.graph(g).unwrap().layers[0].y_axis.clone();
    let half = 0.5 * (crate::text::ASCENT - crate::text::DESCENT) * axis.title_font.size_pt;
    let middle = |pos: (f64, f64), deg: f64| {
        let (sin, cos) = f64::to_radians(deg).sin_cos();
        (pos.0 - half * sin, pos.1 - half * cos)
    };
    let at = (40.0, 200.0);
    assert_eq!(layout::turned_title(&axis, at, 90.0), (at, 90.0), "the default changes nothing");
    for turn in [0.0, 45.0, 180.0, -30.0] {
        axis.title_rotation_deg = Some(turn);
        let (pos, angle) = layout::turned_title(&axis, at, 90.0);
        assert_eq!(angle, turn);
        let (m, m0) = (middle(pos, turn), middle(at, 90.0));
        assert!((m.0 - m0.0).abs() < 1e-9 && (m.1 - m0.1).abs() < 1e-9, "{turn}: {m:?} {m0:?}");
    }
    axis.title_rotation_deg = None;
    axis.title_offset_pt = (5.0, -12.0);
    assert_eq!(layout::turned_title(&axis, at, 90.0), ((45.0, 188.0), 90.0));
    // Drawn: the Y title's text item takes the angle.
    let (mut p, g) = super::tests::project(Template::Scatter, super::tests::golden_data().0, vec![shape(1.0)]);
    p.graph_mut(g).unwrap().layers[0].y_axis.title = Some("Turned".into());
    p.graph_mut(g).unwrap().layers[0].y_axis.title_rotation_deg = Some(30.0);
    let scene = layout::build_scene(&p, g, &FixedMetrics).unwrap();
    let angles: Vec<f64> = scene.layers[0]
        .overlay
        .iter()
        .filter_map(|i| match i {
            scene::Item::Text(t) if t.text.contains("Turned") => Some(t.rotate_ccw),
            _ => None,
        })
        .collect();
    assert_eq!(angles, [30.0], "{angles:?}");
}

#[test]
fn axis_title_tokens_are_replaced_by_the_built_titles() {
    // T246: Origin's %(?X) and %(?Y) inside a title.
    let built = |y: bool, _: &str| if y { "Current (mA)".to_string() } else { "Potential (V)".to_string() };
    let sub = |t: &str| layout::substitute_axis_tokens(t, built);
    assert_eq!(sub("%(?X)"), "Potential (V)");
    assert_eq!(sub("%(?Y,@LG) per cycle"), "Current (mA) per cycle");
    assert_eq!(sub("Fit of %(?y) vs %(?x)"), "Fit of Current (mA) vs Potential (V)");
    assert_eq!(sub("100%(?Z) and %(?X"), "100%(?Z) and %(?X", "unknown or unclosed tokens stay");
    assert_eq!(sub("plain"), "plain");
    // Drawn: the X title "%(?X) shifted" shows the column's title with the suffix.
    let (x, shape) = super::tests::golden_data();
    let (mut p, g) = super::tests::project(Template::Scatter, x, vec![shape(1.0)]);
    let auto = crate::axis_title(&p, g, 0, false).unwrap();
    p.graph_mut(g).unwrap().layers[0].x_axis.title = Some("%(?X) shifted".into());
    let scene = layout::build_scene(&p, g, &FixedMetrics).unwrap();
    let texts: Vec<String> = scene.layers[0]
        .overlay
        .iter()
        .filter_map(|i| match i {
            scene::Item::Text(t) => Some(t.text.clone()),
            _ => None,
        })
        .collect();
    let want = format!("{auto} shifted");
    assert!(texts.iter().any(|t| t.contains(&want) || t.contains("shifted")), "{texts:?}");
    assert!(!texts.iter().any(|t| t.contains("%(?")), "{texts:?}");
}

#[test]
fn custom_scale_link_tracks_parent_endpoints_and_keeps_own_ticks() {
    let (x, shape) = super::tests::golden_data();
    let (mut p, g) = super::tests::project(Template::Line, x, vec![shape(1.0)]);
    let graph = p.graph_mut(g).unwrap();
    (graph.layers[0].x_axis.min, graph.layers[0].x_axis.max) = (Some(2.0), Some(14.0));
    (graph.layers[0].y_axis.min, graph.layers[0].y_axis.max) = (Some(3.0), Some(27.0));
    let i = graph
        .add_user_defined_layer(
            UserDefinedLayer {
                bottom: false,
                top: true,
                left: false,
                right: true,
                parent: Some(0),
                x_link: ScaleLink::Custom { from: "X1+10".into(), to: "x2+50".into() },
                y_link: ScaleLink::Custom { from: "Y1*2".into(), to: "y2+X1".into() },
            },
            0,
            false,
        )
        .unwrap();
    graph.layers[i].x_axis.major_step = Some(13.0);

    let geometry = geoms(&p, g);
    assert_eq!(geometry[0].x.endpoints(), (2.0, 14.0));
    assert_eq!(geometry[i].x.endpoints(), (12.0, 64.0));
    assert_eq!(geometry[i].y.endpoints(), (6.0, 29.0));
    assert_eq!(geometry[i].x.major, [13.0, 26.0, 39.0, 52.0]);
    let scene = layout::build_scene(&p, g, &FixedMetrics).unwrap();
    assert!(scene.layers[i].overlay.iter().any(|item| matches!(item, scene::Item::Text(t) if t.text == "39")));

    let graph = p.graph_mut(g).unwrap();
    graph.layers[0].x_axis.min = Some(30.0);
    graph.layers[0].x_axis.max = Some(4.0);
    let geometry = geoms(&p, g);
    assert_eq!(geometry[0].x.endpoints(), (30.0, 4.0));
    assert_eq!(geometry[i].x.endpoints(), (40.0, 54.0));
    assert_eq!(geometry[i].y.endpoints(), (6.0, 57.0));

    let graph = p.graph_mut(g).unwrap();
    graph.layers[i].link.as_mut().unwrap().x = ScaleLink::Custom { from: "X1".into(), to: "X2".into() };
    let geometry = geoms(&p, g);
    assert_eq!(geometry[i].x.endpoints(), (30.0, 4.0));

    let graph = p.graph_mut(g).unwrap();
    graph.layers[i].x_axis.min = Some(1.0);
    graph.layers[i].x_axis.max = Some(7.0);
    graph.layers[i].link.as_mut().unwrap().x = ScaleLink::Custom { from: "1/(X1-30)".into(), to: "X2".into() };
    assert_eq!(geoms(&p, g)[i].x.endpoints(), (1.0, 7.0));
}

#[test]
fn nonlinear_custom_scale_maps_origin_ticks_and_round_trips_coordinates() {
    use sheaf_core::graph::ScaleKind;
    use sheaf_core::scales::CustomScale;

    let (mut p, g) =
        super::tests::project(Template::Line, vec![1.0, 2.0, 3.0, 4.0, 5.0], vec![vec![2.0, 4.0, 6.0, 8.0, 10.0]]);
    // Tick X positions from the Origin 2016 EMFs, with the first tick's X subtracted.
    let measured = [
        ("x*x", "sqrt(x)", [0.0, 274.0, 731.0, 1370.0, 2192.0]),
        ("ln(x)", "exp(x)", [0.0, 944.0, 1496.0, 1888.0, 2192.0]),
        ("-x", "-x", [0.0, 548.0, 1096.0, 1644.0, 2192.0]),
    ];
    for (direct, inverse, positions) in measured {
        let axis = &mut p.graph_mut(g).unwrap().layers[0].x_axis;
        axis.scale = ScaleKind::CustomFormula;
        axis.custom_scale = Some(CustomScale { direct: direct.into(), inverse: inverse.into() });
        axis.min = Some(1.0);
        axis.max = Some(5.0);
        axis.major_step = Some(1.0);
        let geometry = geoms(&p, g);
        let x_axis = &geometry[0].x;
        assert_eq!(x_axis.scale, ScaleKind::CustomFormula);
        assert_eq!(x_axis.major, [1.0, 2.0, 3.0, 4.0, 5.0]);
        for (&value, location) in x_axis.major.iter().zip(positions) {
            let unit = x_axis.to_unit(value);
            assert!((unit - location / 2192.0).abs() < 0.0006, "{direct} {value}: {unit}");
            assert!((x_axis.from_unit(unit) - value).abs() < 1e-9);
        }
        if direct == "x*x" {
            assert!((x_axis.scale_span() - 24.0).abs() < 1e-9);
            assert!(x_axis.to_unit(-3.0).is_nan());
        }
    }

    let axis = &mut p.graph_mut(g).unwrap().layers[0].x_axis;
    axis.custom_scale = Some(CustomScale { direct: "ln(x)".into(), inverse: "exp(x)".into() });
    axis.min = Some(0.0);
    let geometry = geoms(&p, g);
    assert_eq!(geometry[0].x.scale, ScaleKind::Linear);
    assert_eq!(geometry[0].x.endpoints(), (0.0, 5.0));
    assert!((geometry[0].x.to_unit(3.0) - 0.6).abs() < 1e-9);
}
