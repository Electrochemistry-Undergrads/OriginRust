use super::*;
use sheaf_core::project::ViewMode;

fn mode(h: &Harness<'static, SheafApp>) -> ViewMode {
    h.state().doc.project().view_mode(*h.state().graphs.keys().next().expect("a graph"))
}

fn page_and_content(h: &Harness<'static, SheafApp>) -> (egui::Rect, egui::Rect) {
    graph_ui(h).page_and_content().expect("the graph was drawn")
}

#[test]
fn a_new_graph_is_in_page_view_with_the_page_inside_the_window() {
    let h = start(vec![data_file("vm-new")]);
    assert_eq!(mode(&h), ViewMode::Page);
    let (page, content) = page_and_content(&h);
    assert!(page.width() < content.width() - 1.0 || page.height() < content.height() - 1.0);
    // The page keeps its own aspect.
    let own = graph(&h).page.width_pt / graph(&h).page.height_pt;
    assert!((f64::from(page.width() / page.height()) - own).abs() < 0.01);
}

#[test]
fn the_view_menu_sets_the_mode_and_window_and_draft_view_fill_the_window() {
    let mut h = start(vec![data_file("vm-fill")]);
    menu_path(&mut h, &["View", "Window View"]);
    assert_eq!(mode(&h), ViewMode::Window);
    let (page, content) = page_and_content(&h);
    assert!(page.min.distance(content.min) < 1.0 && page.max.distance(content.max) < 1.0, "{page:?} in {content:?}");
    // The graph itself is unchanged: the page size is the window's, only the picture stretches.
    menu_path(&mut h, &["View", "Draft View"]);
    assert_eq!(mode(&h), ViewMode::Draft);
    let (page, content) = page_and_content(&h);
    assert!(page.min.distance(content.min) < 1.0 && page.max.distance(content.max) < 1.0);
    menu_path(&mut h, &["View", "Print View"]);
    assert_eq!(mode(&h), ViewMode::Print);
    let (page, content) = page_and_content(&h);
    assert!(page.width() < content.width() - 1.0 || page.height() < content.height() - 1.0);
    menu_path(&mut h, &["View", "Page View"]);
    assert_eq!(mode(&h), ViewMode::Page);
    assert!(h.state().doc.project().view_modes.is_empty(), "Page View is the default and is not stored");
}

#[test]
fn ctrl_1_2_and_0_choose_page_print_and_draft_view() {
    let mut h = start(vec![data_file("vm-keys")]);
    key_with(&mut h, egui::Modifiers::COMMAND, egui::Key::Num2);
    assert_eq!(mode(&h), ViewMode::Print);
    key_with(&mut h, egui::Modifiers::COMMAND, egui::Key::Num0);
    assert_eq!(mode(&h), ViewMode::Draft);
    key_with(&mut h, egui::Modifiers::COMMAND, egui::Key::Num1);
    assert_eq!(mode(&h), ViewMode::Page);
}

#[test]
fn a_view_mode_is_not_an_undo_step_and_is_saved_with_the_project() {
    let mut h = start(vec![data_file("vm-save")]);
    let undo_before = h.state().doc.undo_label().map(str::to_string);
    menu_path(&mut h, &["View", "Window View"]);
    assert_eq!(h.state().doc.undo_label().map(str::to_string), undo_before);
    let json = serde_json::to_string(h.state().doc.project()).unwrap();
    let back: sheaf_core::Project = serde_json::from_str(&json).unwrap();
    let id = *h.state().graphs.keys().next().unwrap();
    assert_eq!(back.view_mode(id), ViewMode::Window);
}

#[test]
fn layer_icons_in_the_show_menu_hides_and_shows_the_icons() {
    let mut h = start(vec![data_file("vm-icons")]);
    assert!(graph(&h).page.style.layer_icons);
    let icon = graph_ui(&h).page_point(0.0, 0.0).unwrap() + egui::vec2(9.0, 9.0);
    menu_path(&mut h, &["View", "Show", "Layer Icons"]);
    assert!(!graph(&h).page.style.layer_icons);
    right_click_at(&mut h, icon);
    assert!(h.query_by_label_contains("Hide Layer").is_none(), "no layer icon at the corner");
    close_menus(&mut h);
    menu_path(&mut h, &["View", "Show", "Layer Icons"]);
    assert!(graph(&h).page.style.layer_icons);
}

#[test]
fn the_show_items_that_are_not_measured_stay_grey() {
    let mut h = start(vec![data_file("vm-grey")]);
    menu_path(&mut h, &["View", "Show"]);
    for label in ["Active Layer Indicator", "Object Grid", "Frame", "Master Items on Screen"] {
        assert!(h.get_by_label(label).accesskit_node().is_disabled(), "{label} is not built yet");
    }
    // T177: Labels (and Data, which shares its name with the Data menu) are live (graphing 22.8).
    assert!(!h.get_by_label("Labels").accesskit_node().is_disabled());
}

#[test]
fn full_screen_shows_the_reminder_first_and_escape_ends_it() {
    let mut h = start(vec![data_file("vm-full")]);
    menu_path(&mut h, &["View", "Full Screen"]);
    assert!(h.state().full_screen.is_some());
    assert!(has_label(&h, "Hit ESC to exit"), "the reminder is shown first");
    // The window's menu, toolbars and status bar are gone.
    assert!(h.query_by_label("View").is_none());
    click(&mut h, "OK");
    assert!(h.state().full_screen.as_ref().is_some_and(|f| !f.reminder));
    key(&mut h, egui::Key::Escape);
    assert!(h.state().full_screen.is_none());
    assert!(h.query_by_label("View").is_some(), "the window is back");
}

#[test]
fn full_screen_keeps_the_page_after_the_do_not_show_tick() {
    let mut h = start(vec![data_file("vm-full2")]);
    key_with(&mut h, egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::J);
    assert!(h.state().full_screen.is_some(), "Ctrl+Shift+J starts Full Screen");
    click(&mut h, "Do not show this message in the future");
    click(&mut h, "OK");
    key(&mut h, egui::Key::Escape);
    assert!(h.state().full_screen.is_none());
    key_with(&mut h, egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::J);
    assert!(h.state().full_screen.as_ref().is_some_and(|f| !f.reminder), "the tick switched the reminder off");
    key(&mut h, egui::Key::Escape);
}

#[test]
fn the_page_context_menu_has_view_full_screen() {
    let mut h = start(vec![data_file("vm-ctx")]);
    let page = graph(&h).page.clone();
    let at = graph_ui(&h).page_point(page.width_pt - 6.0, page.height_pt - 6.0).unwrap();
    right_click_at(&mut h, at);
    h.get_by_label_contains("View Full Screen").click();
    settle(&mut h);
    assert!(h.state().full_screen.is_some());
}
