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
    for label in ["Layer Grid", "Master Items on Screen"] {
        assert!(h.get_by_label(label).accesskit_node().is_disabled(), "{label} is not built yet");
    }
    // T177: Labels (and Data, which shares its name with the Data menu) are live (graphing 22.8).
    assert!(!h.get_by_label("Labels").accesskit_node().is_disabled());
}

#[test]
fn plot_details_frame_is_independent_of_opposite_axis_lines() {
    let mut h = start(vec![data_file("vm-display-frame")]);
    let id = graph(&h).id;
    h.state_mut().doc.apply("Fixture", |p| {
        let layer = &mut p.graph_mut(id).unwrap().layers[0];
        layer.x_axis.show_opposite = true;
        layer.y_axis.show_opposite = true;
    });
    assert!(!graph(&h).layers[0].display.show_frame);
    open_props(&mut h, 0, PropsTab::Plots, None);
    click(&mut h, "Layer 1");
    click(&mut h, "Display");
    click(&mut h, "Frame");
    click(&mut h, "OK");
    let layer = &graph(&h).layers[0];
    assert!(layer.display.show_frame);
    assert!(layer.x_axis.show_opposite && layer.y_axis.show_opposite);
    menu(&mut h, "Edit", "Undo Change graph settings");
    assert!(!graph(&h).layers[0].display.show_frame);
}

#[test]
fn the_frame_check_follows_the_active_layer_and_toggles_only_its_outline() {
    let mut h = start(vec![data_file("vm-show-frame")]);
    let id = graph(&h).id;
    h.state_mut().doc.apply("Two panels", |p| {
        let g = p.graph_mut(id).unwrap();
        let mut second = g.layers[0].clone();
        g.layers[0].frame.width *= 0.5;
        second.frame.left += g.layers[0].frame.width + 0.05;
        second.frame.width = g.layers[0].frame.width;
        second.x_axis.offset_pct = -25.0;
        second.y_axis.show_opposite = true;
        second.display.show_x_axes = false;
        second.display.show_y_axes = false;
        g.layers.push(second);
    });
    h.state_mut().graphs.get_mut(&id).unwrap().active_layer = 1;
    settle(&mut h);
    let before = project(&h).clone();
    let picture = sheaf_plot::graph_scene(&before, id).unwrap();
    assert_eq!(h.state().menu_context().show_frame, Some(false));

    menu_path(&mut h, &["View", "Show", "Frame"]);
    let mut expected = before.clone();
    expected.graph_mut(id).unwrap().layers[1].display.show_frame = true;
    assert_eq!(project(&h), &expected);
    assert_ne!(sheaf_plot::graph_scene(project(&h), id).unwrap(), picture);
    assert_eq!(h.state().doc.undo_label(), Some("Show Frame"));
    assert_eq!(h.state().menu_context().show_frame, Some(true));

    h.state_mut().graphs.get_mut(&id).unwrap().active_layer = 0;
    settle(&mut h);
    assert_eq!(h.state().menu_context().show_frame, Some(false));
    h.state_mut().graphs.get_mut(&id).unwrap().active_layer = 1;
    settle(&mut h);
    menu_path(&mut h, &["View", "Show", "Frame"]);
    assert_eq!(project(&h), &before);

    menu(&mut h, "Edit", "Undo Show Frame");
    assert_eq!(project(&h), &expected);
    menu(&mut h, "Edit", "Undo Show Frame");
    assert_eq!(project(&h), &before);
}

#[test]
fn the_frame_command_is_disabled_for_a_polar_layer() {
    let mut h = start(vec![data_file("vm-polar-frame")]);
    let graph = h.state_mut().doc.apply("Polar graph", |p| p.add_graph(Template::Polar));
    h.state_mut().focus(graph);
    settle(&mut h);
    menu_path(&mut h, &["View", "Show"]);
    assert!(h.get_by_label("Frame").accesskit_node().is_disabled());
}

#[test]
fn active_layer_indicator_is_a_saved_page_switch_with_undo_and_identical_graph_output() {
    let mut h = start(vec![data_file("view-active-indicator")]);
    let first = graph(&h).id;
    let other = h.state_mut().doc.apply("Second graph", |p| p.add_graph(sheaf_core::template::Template::Line));
    show_window(&mut h, "Graph1 - sample");
    assert!(!h.state().menu_context().active_layer_indicator);
    let picture = sheaf_plot::graph_scene(project(&h), first).unwrap();
    let before = project(&h).clone();

    menu_path(&mut h, &["View", "Show", "Active Layer Indicator"]);
    let mut expected = before.clone();
    expected.graph_mut(first).unwrap().page.style.active_layer_indicator = true;
    assert_eq!(project(&h), &expected);
    assert!(h.state().menu_context().active_layer_indicator);
    assert!(!project(&h).graph(other).unwrap().page.style.active_layer_indicator);
    assert_eq!(sheaf_plot::graph_scene(project(&h), first).unwrap(), picture);
    assert_eq!(h.state().doc.undo_label(), Some("Show Active Layer Indicator"));

    let path = save_dir("view-active-indicator").join("indicator.sheaf");
    sheaf_io::save::save_project(project(&h), &path).unwrap();
    let reopened = sheaf_io::open_project(&path).unwrap().project;
    assert!(reopened.graph(first).unwrap().page.style.active_layer_indicator);
    assert!(!reopened.graph(other).unwrap().page.style.active_layer_indicator);

    menu(&mut h, "Edit", "Undo");
    assert_eq!(project(&h), &before);
    assert!(!h.state().menu_context().active_layer_indicator);
    menu(&mut h, "Edit", "Redo");
    assert_eq!(project(&h), &expected);
}

#[test]
fn axis_layer_icons_select_the_axis_owner_and_are_saved_per_graph() {
    let mut h = start(vec![data_file("view-axis-layer-icons")]);
    let first = graph(&h).id;
    let other = h.state_mut().doc.apply("Two layers and a second graph", |p| {
        let g = p.graph_mut(first).unwrap();
        let right = g.add_layer(sheaf_core::layers::NewLayerKind::RightY, 0).unwrap();
        g.layers[right].plots = g.layers[0].plots.clone();
        p.add_graph(Template::Line)
    });
    show_window(&mut h, "Graph1 - sample");
    menu_path(&mut h, &["View", "Show", "Layer Icons"]);
    let picture = sheaf_plot::graph_scene(project(&h), first).unwrap();
    let right_badge = "Select layer 2 from Y axis";
    assert!(h.query_by_label(right_badge).is_none());

    menu_path(&mut h, &["View", "Show", "Axis Layer Icons"]);
    assert!(h.state().menu_context().axis_layer_icons);
    assert!(!project(&h).graph(first).unwrap().page.style.layer_icons);
    assert!(!project(&h).graph(other).unwrap().page.style.axis_layer_icons);
    assert_eq!(sheaf_plot::graph_scene(project(&h), first).unwrap(), picture);
    assert_eq!(h.state().doc.undo_label(), Some("Show Axis Layer Icons"));
    assert!(has_label(&h, "Select layer 1 from X axis"));
    assert!(has_label(&h, "Select layer 1 from Y axis"));
    assert_eq!(h.state().graphs[&first].active_layer, 0);
    click(&mut h, right_badge);
    assert_eq!(h.state().graphs[&first].active_layer, 1);

    let path = save_dir("view-axis-layer-icons").join("axis-icons.sheaf");
    sheaf_io::save::save_project(project(&h), &path).unwrap();
    let reopened = sheaf_io::open_project(&path).unwrap().project;
    assert!(reopened.graph(first).unwrap().page.style.axis_layer_icons);
    assert!(!reopened.graph(other).unwrap().page.style.axis_layer_icons);
    menu(&mut h, "Edit", "Undo");
    assert!(!h.state().menu_context().axis_layer_icons);
    assert!(h.query_by_label(right_badge).is_none());
    menu(&mut h, "Edit", "Redo");
    assert!(h.state().menu_context().axis_layer_icons);
    assert!(has_label(&h, right_badge));
}

#[test]
fn object_grid_is_a_saved_page_switch() {
    let mut h = start(vec![data_file("view-object-grid")]);
    let first = graph(&h).id;
    let other = h.state_mut().doc.apply("Second graph", |p| p.add_graph(Template::Line));
    show_window(&mut h, "Graph1 - sample");
    assert!(!h.state().menu_context().object_grid);
    let picture = sheaf_plot::graph_scene(project(&h), first).unwrap();

    menu_path(&mut h, &["View", "Show", "Object Grid"]);
    assert!(h.state().menu_context().object_grid);
    assert!(!project(&h).graph(other).unwrap().page.style.object_grid);
    assert_eq!(h.state().doc.undo_label(), Some("Show Object Grid"));
    assert_eq!(sheaf_plot::graph_scene(project(&h), first).unwrap(), picture);

    let path = save_dir("view-object-grid").join("object-grid.sheaf");
    sheaf_io::save::save_project(project(&h), &path).unwrap();
    let reopened = sheaf_io::open_project(&path).unwrap().project;
    assert!(reopened.graph(first).unwrap().page.style.object_grid);
    assert!(!reopened.graph(other).unwrap().page.style.object_grid);

    menu(&mut h, "Edit", "Undo");
    assert!(!h.state().menu_context().object_grid);
    menu(&mut h, "Edit", "Redo");
    assert!(h.state().menu_context().object_grid);
    menu_path(&mut h, &["View", "Show", "Object Grid"]);
    assert!(!h.state().menu_context().object_grid);
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
