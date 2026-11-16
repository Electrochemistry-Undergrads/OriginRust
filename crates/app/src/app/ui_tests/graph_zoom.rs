use super::*;
use crate::actions::GraphTool;

fn factor(h: &Harness<'static, SheafApp>) -> f32 {
    graph_ui(h).zoom.factor
}

/// A screen point on the page at fractions of its size.
fn at_page(h: &Harness<'static, SheafApp>, fx: f64, fy: f64) -> egui::Pos2 {
    let page = &graph(h).page;
    graph_ui(h).page_point(page.width_pt * fx, page.height_pt * fy).expect("the graph was drawn")
}

/// Zoom In from the View menu, then a click at `at`.
fn zoom_in_at(h: &mut Harness<'static, SheafApp>, at: egui::Pos2) {
    menu_path(h, &["View", "Zoom In"]);
    click_at(h, at);
}

fn wheel(h: &mut Harness<'static, SheafApp>, at: egui::Pos2, delta: egui::Vec2, modifiers: egui::Modifiers) {
    pause(h);
    h.event(egui::Event::PointerMoved(at));
    h.step();
    h.event(egui::Event::MouseWheel { unit: egui::MouseWheelUnit::Line, delta, modifiers });
    // egui smooths a wheel over several frames.
    pause(h);
}

#[test]
fn a_new_graph_window_shows_the_whole_page() {
    let h = start(vec![data_file("zoom-new")]);
    assert_eq!(factor(&h), 1.0);
    assert_eq!(graph_ui(&h).zoom.scroll, egui::Vec2::ZERO);
}

#[test]
fn zoom_in_is_a_mode_and_the_click_zooms_by_two_keeping_the_point_under_the_pointer() {
    let mut h = start(vec![data_file("zoom-in")]);
    menu_path(&mut h, &["View", "Zoom In"]);
    assert_eq!(graph_ui(&h).tool(), GraphTool::ZoomIn);
    assert_eq!(factor(&h), 1.0, "choosing Zoom In changes nothing until a click");
    let p = at_page(&h, 0.4, 0.45);
    let page = graph(&h).page.clone();
    click_at(&mut h, p);
    assert_eq!(factor(&h), 2.0);
    assert_eq!(graph_ui(&h).tool(), GraphTool::Pointer, "one click ends the mode");
    assert!(graph_ui(&h).selected.is_none(), "the click only zooms");
    let after = graph_ui(&h).page_point(page.width_pt * 0.4, page.height_pt * 0.45).unwrap();
    assert!(after.distance(p) < 2.0, "{p:?} moved to {after:?}");
    // Again: 400 %.
    let p = at_page(&h, 0.4, 0.45);
    zoom_in_at(&mut h, p);
    assert_eq!(factor(&h), 4.0);
}

#[test]
fn zoom_is_limited_to_3200_percent() {
    let mut h = start(vec![data_file("zoom-max")]);
    for _ in 0..7 {
        let p = at_page(&h, 0.5, 0.5);
        zoom_in_at(&mut h, p);
    }
    assert_eq!(factor(&h), 32.0);
}

#[test]
fn zoom_out_acts_at_once_and_stops_at_the_whole_page() {
    let mut h = start(vec![data_file("zoom-out")]);
    let p = at_page(&h, 0.5, 0.5);
    zoom_in_at(&mut h, p);
    let p = at_page(&h, 0.5, 0.5);
    zoom_in_at(&mut h, p);
    assert_eq!(factor(&h), 4.0);
    menu_path(&mut h, &["View", "Zoom Out"]);
    assert_eq!(factor(&h), 2.0);
    menu_path(&mut h, &["View", "Zoom Out"]);
    assert_eq!(factor(&h), 1.0);
    assert_eq!(graph_ui(&h).zoom.scroll, egui::Vec2::ZERO);
    menu_path(&mut h, &["View", "Zoom Out"]);
    assert_eq!(factor(&h), 1.0, "never below the whole page");
}

#[test]
fn whole_page_resets_zoom_and_scroll() {
    let mut h = start(vec![data_file("zoom-whole")]);
    let p = at_page(&h, 0.3, 0.3);
    zoom_in_at(&mut h, p);
    let p = at_page(&h, 0.3, 0.3);
    zoom_in_at(&mut h, p);
    assert!(factor(&h) > 2.0);
    menu_path(&mut h, &["View", "Whole Page"]);
    assert_eq!(factor(&h), 1.0);
    assert_eq!(graph_ui(&h).zoom.scroll, egui::Vec2::ZERO);
}

#[test]
fn escape_leaves_the_zoom_in_mode_without_zooming() {
    let mut h = start(vec![data_file("zoom-esc")]);
    menu_path(&mut h, &["View", "Zoom In"]);
    assert_eq!(graph_ui(&h).tool(), GraphTool::ZoomIn);
    key(&mut h, egui::Key::Escape);
    assert_eq!(graph_ui(&h).tool(), GraphTool::Pointer);
    let p = at_page(&h, 0.5, 0.5);
    click_at(&mut h, p);
    assert_eq!(factor(&h), 1.0);
}

#[test]
fn another_tool_leaves_the_zoom_in_mode() {
    let mut h = start(vec![data_file("zoom-other")]);
    menu_path(&mut h, &["View", "Zoom In"]);
    h.get_by_label("Tools toolbar: Data Reader").click();
    settle(&mut h);
    assert_eq!(graph_ui(&h).tool(), GraphTool::Reader);
}

#[test]
fn the_shortcuts_are_ctrl_i_ctrl_m_and_ctrl_w() {
    let mut h = start(vec![data_file("zoom-keys")]);
    key_with(&mut h, egui::Modifiers::COMMAND, egui::Key::I);
    assert_eq!(graph_ui(&h).tool(), GraphTool::ZoomIn);
    let p = at_page(&h, 0.5, 0.5);
    click_at(&mut h, p);
    assert_eq!(factor(&h), 2.0);
    key_with(&mut h, egui::Modifiers::COMMAND, egui::Key::M);
    assert_eq!(factor(&h), 1.0);
    let p = at_page(&h, 0.5, 0.5);
    zoom_in_at(&mut h, p);
    key_with(&mut h, egui::Modifiers::COMMAND, egui::Key::W);
    assert_eq!(factor(&h), 1.0);
    assert_eq!(graph_ui(&h).zoom.scroll, egui::Vec2::ZERO);
}

#[test]
fn clicks_select_the_right_object_when_zoomed() {
    let mut h = start(vec![data_file("zoom-hit")]);
    let legend = on(&h, 0, ObjectKind::Legend);
    zoom_in_at(&mut h, legend);
    assert_eq!(factor(&h), 2.0);
    let legend = on(&h, 0, ObjectKind::Legend);
    click_at(&mut h, legend);
    assert_eq!(graph_ui(&h).selected, Some((0, ObjectKind::Legend)));
    // The axis is on screen when the zoom is about a point on it.
    menu_path(&mut h, &["View", "Whole Page"]);
    let axis = on(&h, 0, ObjectKind::XAxis);
    zoom_in_at(&mut h, axis);
    let axis = on(&h, 0, ObjectKind::XAxis);
    click_at(&mut h, axis);
    assert_eq!(graph_ui(&h).selected, Some((0, ObjectKind::XAxis)));
}

#[test]
fn a_legend_drag_moves_it_by_the_pointer_distance_when_zoomed() {
    let mut h = start(vec![data_file("zoom-drag")]);
    let legend = on(&h, 0, ObjectKind::Legend);
    zoom_in_at(&mut h, legend);
    let from = on(&h, 0, ObjectKind::Legend);
    drag(&mut h, from, from + egui::vec2(-30.0, 20.0));
    settle(&mut h);
    let to = on(&h, 0, ObjectKind::Legend);
    assert!((to - from - egui::vec2(-30.0, 20.0)).length() < 3.0, "{from:?} -> {to:?}");
}

#[test]
fn the_pan_tool_moves_the_view_with_a_drag_and_escape_ends_it() {
    let mut h = start(vec![data_file("zoom-pan")]);
    let p = at_page(&h, 0.5, 0.5);
    zoom_in_at(&mut h, p);
    h.get_by_label("Tools toolbar: Pan").click();
    settle(&mut h);
    assert_eq!(graph_ui(&h).tool(), GraphTool::Pan);
    let before = graph_ui(&h).zoom.scroll;
    let from = at_page(&h, 0.5, 0.5);
    drag(&mut h, from, from + egui::vec2(-60.0, -40.0));
    settle(&mut h);
    let after = graph_ui(&h).zoom.scroll;
    assert!(after.x > before.x && after.y > before.y, "{before:?} -> {after:?}");
    assert!(graph_ui(&h).selected.is_none(), "a pan drag selects and moves nothing");
    key(&mut h, egui::Key::Escape);
    assert_eq!(graph_ui(&h).tool(), GraphTool::Pointer);
}

#[test]
fn the_wheel_scrolls_and_ctrl_wheel_zooms_about_the_pointer() {
    let mut h = start(vec![data_file("zoom-wheel")]);
    let p = at_page(&h, 0.5, 0.5);
    wheel(&mut h, p, egui::vec2(0.0, 2.0), egui::Modifiers::COMMAND);
    assert!(factor(&h) > 1.2, "{}", factor(&h));
    let page = graph(&h).page.clone();
    let under = graph_ui(&h).page_point(page.width_pt * 0.5, page.height_pt * 0.5).unwrap();
    assert!(under.distance(p) < 2.0);
    let zoomed = factor(&h);
    let before = graph_ui(&h).zoom.scroll;
    wheel(&mut h, p, egui::vec2(0.0, -1.0), egui::Modifiers::NONE);
    let after = graph_ui(&h).zoom.scroll;
    assert!(after.y > before.y, "{before:?} -> {after:?}");
    assert_eq!(after.x, before.x);
    wheel(&mut h, p, egui::vec2(0.0, -1.0), egui::Modifiers::SHIFT);
    assert!(graph_ui(&h).zoom.scroll.x > after.x, "Shift+wheel scrolls sideways");
    wheel(&mut h, p, egui::vec2(0.0, -4.0), egui::Modifiers::COMMAND);
    assert!(factor(&h) < zoomed && factor(&h) >= 1.0);
}
