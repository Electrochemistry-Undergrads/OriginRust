use super::*;
use sheaf_core::graph::{
    AxisSide, Frame, LayerBorder, LayerDrawingOrder, LayerElementScale, OppositeAxisStyle, ScaleKind, ScaleLink,
    TickDirection,
};

/// The sample project with Graph1 made of `layers` independent layers (copies of its first one).
fn with_layers(tag: &str, layers: usize) -> Harness<'static, SheafApp> {
    let mut h = start(vec![data_file(tag)]);
    let id = project(&h).graphs().next().unwrap().id;
    h.state_mut().doc.apply("Add layers", |p| {
        let g = p.graph_mut(id).unwrap();
        let base = g.layers[0].clone();
        while g.layers.len() < layers {
            g.layers.push(base.clone());
        }
    });
    settle(&mut h);
    h
}

fn open(h: &mut Harness<'static, SheafApp>) {
    menu_path(h, &["Graph", "Layer Management..."]);
    assert!(h.state().layer_management.is_some());
}

fn number(h: &mut Harness<'static, SheafApp>, name: &str, value: &str) {
    let at = h.get_by_label(name).rect().center();
    double_click_at(h, at);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.event(egui::Event::Text(value.into()));
    h.key_press(egui::Key::Enter);
    settle(h);
}

fn frames(h: &Harness<'static, SheafApp>) -> Vec<Frame> {
    graph(h).layers.iter().map(|l| l.frame).collect()
}

#[test]
fn the_graph_menu_opens_the_window_with_its_parts() {
    let mut h = with_layers("lm-open", 2);
    open(&mut h);
    assert!(h.query_by_label("Layer1").is_some() && h.query_by_label("Layer2").is_some());
    assert!(h.query_by_label("Arrange Order").is_some());
    assert!(h.query_by_label("Keep Layer Aspect Ratio").is_some());
    assert!(h.query_by_label("Horizontal Gap").is_some());
    assert!(h.query_by_label_contains("Use the Apply buttons on each tab").is_some());
    assert!(h.state().layer_management.is_some());
}

#[test]
fn the_page_menu_opens_the_window() {
    let mut h = with_layers("lm-page", 2);
    let g = graph_ui(&h);
    let corner = g.page_point(4.0, 4.0).expect("the page is drawn");
    right_click_at(&mut h, corner);
    click(&mut h, "Layer Management...");
    assert!(h.state().layer_management.is_some());
}

#[test]
fn the_layer_icon_menu_offers_it_and_selects_that_layer() {
    let entries = menus::layer_icon_menu(1, 2, &[], None);
    let item = entries.iter().find_map(|e| match e {
        menus::Entry::Item(i) if i.label == "Layer Management..." => Some(i),
        _ => None,
    });
    assert!(matches!(item.and_then(|i| i.cmd.as_ref()), Some(menus::Cmd::LayerManagement(Some(1)))));
    let mut h = with_layers("lm-icon", 2);
    let graph = project(&h).graphs().next().unwrap().id;
    h.state_mut().apply(Action::LayerManagement { graph, layer: Some(1) }, &egui::Context::default());
    settle(&mut h);
    assert!(h.state().layer_management.is_some());
}

#[test]
fn add_tab_uses_the_chosen_parent_and_scale_in_both_undo_paths() {
    let mut h = with_layers("lm-add-linked", 2);
    let id = graph(&h).id;
    h.state_mut().doc.apply("Set second layer scale", |p| {
        p.graph_mut(id).unwrap().layers[1].x_axis.scale = ScaleKind::Ln;
    });
    settle(&mut h);
    let before = project(&h).clone();
    open(&mut h);
    click(&mut h, "Add");
    assert!(h.get_by_label("Link To").accesskit_node().is_disabled());
    let _ = h.get(By::new().role(egui::accesskit::Role::ComboBox).value("Bottom-X Left-Y"));
    click(&mut h, "Type");
    click(&mut h, "Right-Y (Linked X Scale and Dimension)");
    click(&mut h, "Link To");
    h.get_all_by_label("2").last().unwrap().click();
    settle(&mut h);
    assert!(h.get_by_label("X Scale").accesskit_node().is_disabled());
    let _ = h.get(By::new().role(egui::accesskit::Role::ComboBox).value("Ln"));
    click(&mut h, "Y Scale");
    click(&mut h, "Log10");
    click(&mut h, "Apply");
    assert!(h.query_by_label("Layer3").is_some());
    assert_eq!(project(&h), &before);
    click(&mut h, "Undo");
    assert!(h.query_by_label("Layer3").is_none());
    click(&mut h, "Apply");
    click(&mut h, "OK");
    let added = &graph(&h).layers[2];
    let link = added.link.as_ref().unwrap();
    assert_eq!((link.parent, &link.x, &link.y), (1, &ScaleLink::Straight, &ScaleLink::None));
    assert_eq!((added.x_axis.scale, added.y_axis.scale), (ScaleKind::Ln, ScaleKind::Log10));
    assert_eq!((added.x_axis.side, added.y_axis.side), (AxisSide::Hidden, AxisSide::Opposite));
    assert!(added.plots.is_empty());
    assert_eq!(graph(&h).page_frame(2), graph(&h).page_frame(1));
    menu(&mut h, "Edit", "Undo Layer Management");
    assert_eq!(project(&h), &before);
}

#[test]
fn add_tab_inset_with_data_copies_all_plots_and_cancel_discards_it() {
    let mut h = with_layers("lm-add-inset", 1);
    let id = graph(&h).id;
    h.state_mut().doc.apply("Add another curve", |p| {
        let parent = &mut p.graph_mut(id).unwrap().layers[0];
        parent.y_axis.min = Some(1.0);
        parent.y_axis.max = Some(22.0);
        parent.y_axis.major_step = Some(5.0);
        let plots = &mut parent.plots;
        let mut second = plots[0].clone();
        second.visible = false;
        plots.push(second);
    });
    settle(&mut h);
    let before = project(&h).clone();
    for button in ["Cancel", "OK"] {
        open(&mut h);
        click(&mut h, "Add");
        click(&mut h, "Type");
        click(&mut h, "Inset With Data (Linked Dimension)");
        click(&mut h, "Apply");
        assert!(h.query_by_label("Layer2").is_some());
        assert_eq!(project(&h), &before);
        click(&mut h, button);
        assert!(h.state().layer_management.is_none());
    }
    let layers = &graph(&h).layers;
    assert_eq!(layers.len(), 2);
    assert_eq!(layers[1].plots, layers[0].plots);
    let y = &layers[1].y_axis;
    assert_eq!((y.min, y.max, y.major_step), (None, None, None));
    assert_eq!(graph(&h).link_parent(1), Some(0));
    assert!(layers[1].frame.width < layers[0].frame.width);
}

#[test]
fn arranging_two_layers_in_two_rows_and_ok_is_one_undo_step() {
    let mut h = with_layers("lm-arrange", 2);
    let before = frames(&h);
    let revision = h.state().doc.revision();
    open(&mut h);
    number(&mut h, "Number of Rows", "2");
    click(&mut h, "Apply");
    // Nothing is written until OK; the preview has the new frames.
    assert_eq!(h.state().doc.revision(), revision);
    assert_eq!(frames(&h), before);
    click(&mut h, "OK");
    assert!(h.state().layer_management.is_none());
    let after = frames(&h);
    assert!((after[0].top - 0.10).abs() < 1e-9 && (after[1].top - 0.50).abs() < 1e-9, "{after:?}");
    assert!((after[0].left - 0.15).abs() < 1e-9 && (after[0].width - 0.75).abs() < 1e-9);
    assert_eq!(h.state().doc.undo_label(), Some("Layer Management"));
    menu(&mut h, "Edit", "Undo Layer Management");
    assert_eq!(frames(&h), before);
    menu(&mut h, "Edit", "Redo Layer Management");
    assert_eq!(frames(&h), after);
}

#[test]
fn common_scale_arranges_only_the_selected_layers_and_applies_in_one_undo_step() {
    let mut h = with_layers("lm-common-scale", 3);
    let id = graph(&h).id;
    h.state_mut().doc.apply("Set axis ranges", |p| {
        let graph = p.graph_mut(id).unwrap();
        for (layer, (x, y)) in graph.layers.iter_mut().zip([(10.0, 30.0), (20.0, 20.0), (40.0, 10.0)]) {
            layer.x_axis.min = Some(0.0);
            layer.y_axis.min = Some(0.0);
            layer.x_axis.max = Some(x);
            layer.y_axis.max = Some(y);
        }
    });
    settle(&mut h);
    let before = project(&h).clone();
    open(&mut h);
    click(&mut h, "Layer1");
    click(&mut h, "Layer3");
    number(&mut h, "Number of Rows", "2");
    click(&mut h, "Set Layer Width by Common Scale");
    click(&mut h, "Set Layer Height by Common Scale");
    click(&mut h, "Apply");
    assert_eq!(project(&h), &before);
    click(&mut h, "Undo");
    assert!(h.get_by_label("Undo").accesskit_node().is_disabled());
    click(&mut h, "Apply");
    click(&mut h, "OK");
    let frames = frames(&h);
    assert_eq!(frames[1], before.graph(id).unwrap().layers[1].frame);
    assert!((frames[0].width * 4.0 - frames[2].width).abs() < 1e-9);
    assert!((frames[0].height - frames[2].height * 3.0).abs() < 1e-9);
    assert!((frames[2].top - frames[0].top - frames[0].height - 0.05).abs() < 1e-9);
    menu(&mut h, "Edit", "Undo Layer Management");
    assert_eq!(project(&h), &before);
}

#[test]
fn extra_grid_layers_follow_the_preview_selection_and_both_undo_paths() {
    let mut h = with_layers("lm-extra-grid", 2);
    let before = project(&h).clone();
    let before_frames = frames(&h);
    open(&mut h);
    click(&mut h, "Layer1");
    number(&mut h, "Number of Rows", "2");
    click(&mut h, "Add Extra Layer(s) for Grid");
    click(&mut h, "Apply");
    assert!(h.query_by_label("Layer3").is_some());
    assert_eq!(project(&h), &before);
    click(&mut h, "Apply");
    assert!(h.query_by_label("Layer4").is_none());
    click(&mut h, "Undo");
    click(&mut h, "Undo");
    assert!(h.query_by_label("Layer3").is_none());
    click(&mut h, "Apply");
    assert!(h.query_by_label("Layer3").is_some());
    click(&mut h, "OK");
    let layers = &graph(&h).layers;
    assert_eq!(layers.len(), 3);
    assert_eq!(layers[1].frame, before_frames[1]);
    assert!(layers[2].plots.is_empty() && layers[2].link.is_none());
    assert!((layers[2].frame.top - 0.50).abs() < 1e-9);
    menu(&mut h, "Edit", "Undo Layer Management");
    assert_eq!(project(&h), &before);
}

#[test]
fn undo_in_the_window_takes_back_the_last_apply_of_the_preview() {
    let mut h = with_layers("lm-undo", 2);
    let before = frames(&h);
    open(&mut h);
    number(&mut h, "Number of Rows", "2");
    click(&mut h, "Apply");
    click(&mut h, "Undo");
    click(&mut h, "OK");
    assert_eq!(frames(&h), before);
    assert_ne!(h.state().doc.undo_label(), Some("Layer Management"));
}

#[test]
fn cancel_and_escape_change_nothing() {
    let mut h = with_layers("lm-cancel", 2);
    let before = frames(&h);
    let revision = h.state().doc.revision();
    open(&mut h);
    number(&mut h, "Number of Rows", "2");
    click(&mut h, "Apply");
    click(&mut h, "Cancel");
    assert!(h.state().layer_management.is_none());
    assert_eq!((frames(&h), h.state().doc.revision()), (before.clone(), revision));
    open(&mut h);
    number(&mut h, "Number of Rows", "2");
    click(&mut h, "Apply");
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    assert!(h.state().layer_management.is_none());
    assert_eq!((frames(&h), h.state().doc.revision()), (before, revision));
}

#[test]
fn a_grid_that_is_too_small_shows_the_error_and_changes_nothing() {
    let mut h = with_layers("lm-small", 3);
    open(&mut h);
    number(&mut h, "Number of Rows", "2");
    click(&mut h, "Apply");
    assert!(h.query_by_label_contains("do not fit").is_some());
    click(&mut h, "OK");
    assert_eq!(h.state().doc.undo_label().filter(|l| *l == "Layer Management"), None);
}

#[test]
fn resize_sets_the_width_of_the_selected_layer_as_a_percent_of_the_reference_layer() {
    let mut h = with_layers("lm-resize", 2);
    let before = frames(&h);
    open(&mut h);
    click(&mut h, "Layer2");
    click(&mut h, "Size/Position");
    number(&mut h, "Width", "50");
    click(&mut h, "Apply");
    click(&mut h, "OK");
    let after = frames(&h);
    assert_eq!(after[0], before[0]);
    assert!((after[1].width - 0.5 * before[0].width).abs() < 1e-9, "{after:?}");
    assert!((after[1].height - before[0].height).abs() < 1e-9);
    assert_eq!((after[1].left, after[1].top), (before[1].left, before[1].top));
}

#[test]
fn resize_without_a_selected_layer_says_so() {
    let mut h = with_layers("lm-nosel", 2);
    open(&mut h);
    click(&mut h, "Size/Position");
    click(&mut h, "Apply");
    assert!(h.query_by_label_contains("select at least one layer").is_some());
}

#[test]
fn linking_layer_2_to_layer_1_keeps_its_place_and_shows_in_the_grid() {
    let mut h = with_layers("lm-link", 2);
    let before = project(&h).graphs().next().unwrap().page_frame(1);
    open(&mut h);
    click(&mut h, "Layer2");
    click(&mut h, "Link");
    click(&mut h, "Link To");
    h.get_all_by_label("1").last().unwrap().click();
    settle(&mut h);
    click(&mut h, "Apply");
    click(&mut h, "OK");
    let g = graph(&h);
    assert_eq!(g.layers[1].link.as_ref().map(|l| l.parent), Some(0));
    assert_eq!(g.link_parent(1), Some(0));
    let page = g.page_frame(1);
    assert!((page.left - before.left).abs() < 1e-9 && (page.width - before.width).abs() < 1e-9);
}

#[test]
fn axes_apply_only_the_changed_settings_to_selected_layers() {
    let mut h = with_layers("lm-axes", 3);
    let id = graph(&h).id;
    h.state_mut().doc.apply("Different axis settings", |p| {
        let layers = &mut p.graph_mut(id).unwrap().layers;
        layers[0].y_axis.scale = ScaleKind::Ln;
        layers[0].y_axis.show_title = true;
        layers[2].y_axis.show_title = false;
        for layer in layers {
            layer.x_axis.show_title = true;
            layer.x_axis.show_opposite = false;
        }
    });
    settle(&mut h);
    let before = project(&h).clone();
    open(&mut h);
    click(&mut h, "Axes");
    assert!(h.get_by_label("Apply").accesskit_node().is_disabled());
    click(&mut h, "Layer1");
    click(&mut h, "Layer3");
    let _ = h.get(By::new().role(egui::accesskit::Role::ComboBox).label("Y Scale").value("Mixed"));
    click(&mut h, "Y Scale");
    click(&mut h, "Log10");
    click(&mut h, "Apply");
    assert_eq!(project(&h), &before);
    click(&mut h, "Undo");
    let _ = h.get(By::new().role(egui::accesskit::Role::ComboBox).label("Y Scale").value("Mixed"));
    assert!(h.get_by_label("Undo").accesskit_node().is_disabled());

    for (control, value) in [("Y Scale", "Log10"), ("X Axis Edge", "Top"), ("X Tick", "Out")] {
        click(&mut h, control);
        click(&mut h, value);
    }
    click(&mut h, "X Title");
    click(&mut h, "X Opposite Line");
    click(&mut h, "X Formula");
    h.event(egui::Event::Text("2*x".into()));
    settle(&mut h);
    click(&mut h, "Apply");
    assert_eq!(project(&h), &before);
    click(&mut h, "OK");
    let layers = &graph(&h).layers;
    let original = &before.graph(id).unwrap().layers;
    assert_eq!(layers[1], original[1]);
    for i in [0, 2] {
        let (x, y) = (&layers[i].x_axis, &layers[i].y_axis);
        assert_eq!(x.side, AxisSide::Opposite);
        assert_eq!((x.major_tick, x.minor_tick), (TickDirection::Out, TickDirection::Out));
        assert!(!x.show_title && x.show_opposite);
        assert_eq!(x.tick_label_formula, "2*x");
        assert_eq!(y.scale, ScaleKind::Log10);
        assert_eq!(y.show_title, original[i].y_axis.show_title);
    }
    assert!(sheaf_plot::render_svg(project(&h), id).unwrap().contains("<svg"));
    menu(&mut h, "Edit", "Undo Layer Management");
    assert_eq!(project(&h), &before);
}

#[test]
fn axes_facing_style_changes_only_the_explicit_settings_and_round_trips() {
    let mut h = with_layers("lm-opposite-axis-style", 3);
    let id = graph(&h).id;
    h.state_mut().doc.apply("Existing facing style", |p| {
        let layers = &mut p.graph_mut(id).unwrap().layers;
        layers[1].x_axis.opposite_style.show_title = Some(true);
        layers[1].x_axis.opposite_style.tick_label_formula = Some("x+100".into());
    });
    settle(&mut h);
    let before = project(&h).clone();
    for action in ["Cancel", "OK"] {
        open(&mut h);
        click(&mut h, "Layer1");
        click(&mut h, "Layer2");
        click(&mut h, "Axes");
        assert!(h.get_by_label("Apply").accesskit_node().is_disabled());
        click(&mut h, "X Opposite Line");
        click(&mut h, "X Opposite Ticks");
        click(&mut h, "In");
        click(&mut h, "X Opposite Tick Label");
        click(&mut h, "X Opposite Formula");
        h.event(egui::Event::Text("x+200".into()));
        settle(&mut h);
        click(&mut h, "Apply");
        assert_eq!(project(&h), &before);
        click(&mut h, action);
    }

    let mut expected = before.clone();
    for layer in &mut expected.graph_mut(id).unwrap().layers[..2] {
        let axis = &mut layer.x_axis;
        axis.show_opposite = true;
        axis.opposite_ticks = true;
        axis.opposite_style = OppositeAxisStyle {
            tick_direction: Some(TickDirection::In),
            show_tick_labels: Some(true),
            tick_label_formula: Some("x+200".into()),
            ..axis.opposite_style.clone()
        };
    }
    assert_eq!(project(&h), &expected);
    let folder = save_dir("lm-opposite-axis-style");
    for extension in ["sheaf", "opj"] {
        let saved = sheaf_io::save::save_project(project(&h), &folder.join(format!("facing.{extension}"))).unwrap();
        if extension == "opj" {
            assert!(saved.warnings.iter().any(|warning| warning.contains("opposite X axis style")));
        }
        let reopened = sheaf_io::open_project(&saved.path).unwrap();
        assert_eq!(reopened.project.graph(id), expected.graph(id));
    }
    menu(&mut h, "Edit", "Undo Layer Management");
    assert_eq!(project(&h), &before);
}

#[test]
fn axes_keep_straight_links_and_cancel_the_preview() {
    let mut h = with_layers("lm-axes-linked", 1);
    let id = graph(&h).id;
    h.state_mut().doc.apply("Add linked layer", |p| {
        let graph = p.graph_mut(id).unwrap();
        graph.layers[0].x_axis.scale = ScaleKind::Log10;
        graph.add_layer(sheaf_core::layers::NewLayerKind::RightY, 0).unwrap();
    });
    settle(&mut h);
    let before = project(&h).clone();
    for button in ["Cancel", "OK"] {
        open(&mut h);
        click(&mut h, "Axes");
        click(&mut h, "Layer2");
        assert!(h.get_by_label("X Scale").accesskit_node().is_disabled());
        click(&mut h, "Y Scale");
        click(&mut h, "Log2");
        click(&mut h, "Apply");
        assert_eq!(project(&h), &before);
        click(&mut h, button);
        assert!(h.state().layer_management.is_none());
    }
    let layers = &graph(&h).layers;
    assert_eq!(layers[1].link, before.graph(id).unwrap().layers[1].link);
    assert_eq!(layers[0].x_axis.scale, ScaleKind::Log10);
    assert_eq!(layers[1].x_axis.scale, ScaleKind::Log10);
    assert_eq!(layers[1].y_axis.scale, ScaleKind::Log2);
}

fn background_choice(h: &mut Harness<'static, SheafApp>, name: &str) {
    click(h, "Background Color");
    h.get_all_by_label(name).into_iter().find(|node| !node.accesskit_node().is_disabled()).unwrap().click();
    settle(h);
}

#[test]
fn display_background_apply_is_scoped_and_ok_is_one_undo_step() {
    let mut h = with_layers("lm-background-scope", 3);
    let before = graph(&h).layers.clone();
    let revision = h.state().doc.revision();
    open(&mut h);
    click(&mut h, "Layer2");
    click(&mut h, "Display");
    background_choice(&mut h, "Red");
    click(&mut h, "Apply");
    assert_eq!(graph(&h).layers, before, "Apply only changes the preview");
    assert_eq!(h.state().doc.revision(), revision);
    click(&mut h, "OK");
    let mut expected = before.clone();
    expected[1].background = Some(sheaf_core::Color::RED);
    assert_eq!(graph(&h).layers, expected);
    assert_eq!(h.state().doc.undo_label(), Some("Layer Management"));
    menu(&mut h, "Edit", "Undo Layer Management");
    assert_eq!(graph(&h).layers, before);
    menu(&mut h, "Edit", "Redo Layer Management");
    assert_eq!(graph(&h).layers, expected);
}

#[test]
fn display_background_mixed_layers_wait_for_a_choice_and_none_clears_only_the_selection() {
    let mut h = with_layers("lm-background-mixed", 3);
    let id = project(&h).graphs().next().unwrap().id;
    h.state_mut().doc.apply("Existing backgrounds", |p| {
        let layers = &mut p.graph_mut(id).unwrap().layers;
        for (layer, c) in
            layers.iter_mut().zip([sheaf_core::Color::RED, sheaf_core::Color::BLUE, sheaf_core::Color::WHITE])
        {
            layer.background = Some(c);
        }
    });
    settle(&mut h);
    let before = graph(&h).layers.clone();
    open(&mut h);
    click(&mut h, "Layer1");
    click(&mut h, "Layer2");
    click(&mut h, "Display");
    let _ = h.get(By::new().role(egui::accesskit::Role::ComboBox).value("Mixed"));
    assert!(h.get_by_label("Apply").accesskit_node().is_disabled());
    click(&mut h, "OK");
    assert_eq!(graph(&h).layers, before);
    open(&mut h);
    click(&mut h, "Layer1");
    click(&mut h, "Layer2");
    click(&mut h, "Display");
    background_choice(&mut h, "None");
    click(&mut h, "Apply");
    click(&mut h, "OK");
    let mut expected = before;
    expected[0].background = None;
    expected[1].background = None;
    assert_eq!(graph(&h).layers, expected);
}

#[test]
fn display_background_cancel_escape_and_preview_undo_preserve_the_source() {
    let mut h = with_layers("lm-background-cancel", 2);
    let before = graph(&h).layers.clone();
    let revision = h.state().doc.revision();
    for end in ["Cancel", "Escape", "Undo"] {
        open(&mut h);
        click(&mut h, "Layer1");
        click(&mut h, "Display");
        background_choice(&mut h, "Blue");
        click(&mut h, "Apply");
        match end {
            "Escape" => {
                h.key_press(egui::Key::Escape);
                settle(&mut h);
            }
            "Undo" => {
                click(&mut h, "Undo");
                click(&mut h, "OK");
            }
            _ => click(&mut h, "Cancel"),
        }
        assert!(h.state().layer_management.is_none());
        assert_eq!(graph(&h).layers, before);
        assert_eq!(h.state().doc.revision(), revision);
    }
}

#[test]
fn display_background_rgb_renders_and_survives_native_and_companion_saves() {
    let mut h = with_layers("lm-background-rgb", 1);
    open(&mut h);
    click(&mut h, "Layer1");
    click(&mut h, "Display");
    background_choice(&mut h, "White");
    for (label, value) in [("Background Red", "17"), ("Background Green", "131"), ("Background Blue", "213")] {
        number(&mut h, label, value);
    }
    click(&mut h, "Apply");
    click(&mut h, "OK");
    let c = sheaf_core::Color::rgb(17, 131, 213);
    assert_eq!(graph(&h).layers[0].background, Some(c));
    let id = project(&h).graphs().next().unwrap().id;
    let scene = sheaf_plot::graph_scene(project(&h), id).unwrap();
    assert!(
        scene.layers[0]
            .grid
            .iter()
            .any(|item| matches!(item, sheaf_plot::scene::Item::Rect { fill: Some(fill), .. } if *fill == c))
    );
    assert!(sheaf_plot::render_svg(project(&h), id).unwrap().contains("#1183D5"));
    let folder = save_dir("lm-background-rgb");
    for ext in ["sheaf", "opj"] {
        let saved = sheaf_io::save::save_project(project(&h), &folder.join(format!("background.{ext}"))).unwrap();
        if ext == "opj" {
            assert!(saved.warnings.iter().any(|w| w.contains("layer background colour")));
        }
        let reopened = sheaf_io::open_project(&saved.path).unwrap();
        assert_eq!(reopened.project.graphs().next().unwrap().layers[0].background, Some(c));
    }
}

#[test]
fn display_border_edit_is_scoped_renders_outside_the_frame_and_round_trips() {
    use sheaf_core::Color;
    use sheaf_plot::scene::Item;

    let mut h = with_layers("lm-border", 2);
    let id = graph(&h).id;
    h.state_mut().doc.apply("Layer background", |p| {
        p.graph_mut(id).unwrap().layers[1].background = Some(Color::WHITE);
    });
    settle(&mut h);
    let before = project(&h).clone();
    open(&mut h);
    click(&mut h, "Layer2");
    click(&mut h, "Display");
    click(&mut h, "Border Fill Color");
    click(&mut h, "Red");
    click(&mut h, "Border Color");
    click(&mut h, "Blue");
    click(&mut h, "Apply");
    click(&mut h, "Border Dimensions");
    for input in ["", "25"] {
        click(&mut h, "Left Border");
        h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
        h.key_press(egui::Key::Backspace);
        if !input.is_empty() {
            h.event(egui::Event::Text(input.into()));
        }
        settle(&mut h);
        assert_eq!(h.get_by_label("Apply").accesskit_node().is_disabled(), input.is_empty());
    }
    click(&mut h, "Apply");
    assert_eq!(project(&h), &before);
    click(&mut h, "OK");
    let expected =
        LayerBorder { fill: Some(Color::RED), outline: Some(Color::BLUE), dimensions_pct: [25.0, 10.0, 12.4, 15.2] };
    let mut expected_layers = before.graph(id).unwrap().layers.clone();
    expected_layers[1].border = expected;
    assert_eq!(graph(&h).layers, expected_layers);

    let scene = sheaf_plot::graph_scene(project(&h), id).unwrap();
    let layer = &scene.layers[1];
    let [Item::Rect { rect, fill: Some(Color::RED), stroke: None }, Item::Rect { fill: Some(Color::WHITE), .. }, ..] =
        layer.grid.as_slice()
    else {
        panic!("border fill must come before the layer background");
    };
    let clip = layer.clip;
    assert!((rect.x - (clip.x - clip.w * 0.25)).abs() < 1e-8);
    assert!((rect.y - (clip.y - clip.h * 0.1)).abs() < 1e-8);
    assert!((rect.w - clip.w * 1.374).abs() < 1e-8);
    assert!((rect.h - clip.h * 1.252).abs() < 1e-8);
    assert!(layer.overlay.iter().any(|item| {
        matches!(item, Item::Rect { rect: outline, fill: None, stroke: Some(stroke) } if outline == rect && stroke.color == Color::BLUE)
    }));
    let svg = sheaf_plot::render_svg(project(&h), id).unwrap();
    assert!(svg.contains("#FF0000") && svg.contains("#0000FF"));
    let folder = save_dir("lm-border");
    for extension in ["sheaf", "opj"] {
        let saved = sheaf_io::save::save_project(project(&h), &folder.join(format!("border.{extension}"))).unwrap();
        if extension == "opj" {
            assert!(saved.warnings.iter().any(|w| w.contains("layer management border colours and dimensions")));
        }
        let reopened = sheaf_io::open_project(&saved.path).unwrap();
        assert_eq!(reopened.project.graphs().next().unwrap().layers, expected_layers);
    }
    menu(&mut h, "Edit", "Undo Layer Management");
    assert_eq!(project(&h), &before);
}

#[test]
fn drawing_order_is_page_wide_and_commits_only_after_apply_and_ok() {
    let mut h = with_layers("lm-drawing-order", 2);
    let before = project(&h).clone();
    let id = graph(&h).id;
    let choose_composite = |h: &mut Harness<'static, SheafApp>| {
        click(h, "Drawing Order");
        click(h, "Draw Composite");
    };
    open(&mut h);
    click(&mut h, "Display");
    assert!(h.get_by_label("Background Color").accesskit_node().is_disabled());
    choose_composite(&mut h);
    click(&mut h, "Apply");
    assert_eq!(project(&h), &before);
    click(&mut h, "Undo");
    assert!(h.get_by_label("Apply").accesskit_node().is_disabled());
    choose_composite(&mut h);
    click(&mut h, "Apply");
    click(&mut h, "Cancel");
    assert_eq!(project(&h), &before);

    open(&mut h);
    click(&mut h, "Layer2");
    click(&mut h, "Display");
    choose_composite(&mut h);
    click(&mut h, "Apply");
    click(&mut h, "OK");
    let mut expected = before.clone();
    expected.graph_mut(id).unwrap().page.style.layer_drawing_order = LayerDrawingOrder::Composite;
    assert_eq!(project(&h), &expected);
    let folder = save_dir("lm-drawing-order");
    for extension in ["sheaf", "opj"] {
        let saved = sheaf_io::save::save_project(project(&h), &folder.join(format!("order.{extension}"))).unwrap();
        if extension == "opj" {
            assert!(saved.warnings.iter().any(|warning| warning.contains("layer drawing order flag")));
        }
        let reopened = sheaf_io::open_project(&saved.path).unwrap();
        assert_eq!(
            reopened.project.graphs().next().unwrap().page.style.layer_drawing_order,
            LayerDrawingOrder::Composite
        );
    }
    menu(&mut h, "Edit", "Undo Layer Management");
    assert_eq!(project(&h), &before);
}

#[test]
fn drawing_order_rejects_a_page_order_changed_behind_the_preview() {
    let mut h = with_layers("lm-drawing-order-stale", 2);
    let id = graph(&h).id;
    open(&mut h);
    click(&mut h, "Display");
    click(&mut h, "Drawing Order");
    click(&mut h, "Draw Composite");
    click(&mut h, "Apply");
    h.state_mut().doc.apply("Independent drawing order", |p| {
        p.graph_mut(id).unwrap().page.style.layer_drawing_order = LayerDrawingOrder::Composite;
    });
    settle(&mut h);
    click(&mut h, "OK");
    assert!(h.state().layer_management.is_some());
    assert!(h.query_by_label_contains("drawing order changed").is_some());
    assert_eq!(h.state().doc.undo_label(), Some("Independent drawing order"));
}

#[test]
fn display_fixed_element_scale_validates_and_round_trips_the_selected_layer() {
    let mut h = with_layers("lm-fixed-element-scale", 2);
    let before = project(&h).clone();
    let id = graph(&h).id;
    open(&mut h);
    click(&mut h, "Display");
    click(&mut h, "Scale Elements");
    assert!(h.get_by_label("Factor").accesskit_node().is_disabled());
    click(&mut h, "Layer2");
    assert!(h.get_by_label("Apply").accesskit_node().is_disabled());
    for (value, invalid) in [("0", true), ("inf", true), ("0.75", false)] {
        click(&mut h, "Factor");
        h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
        h.event(egui::Event::Text(value.into()));
        settle(&mut h);
        assert_eq!(h.get_by_label("Apply").accesskit_node().is_disabled(), invalid);
    }
    click(&mut h, "Apply");
    assert_eq!(project(&h), &before);
    click(&mut h, "Undo");
    assert!(h.get_by_label("Apply").accesskit_node().is_disabled());
    number(&mut h, "Factor", "0.75");
    click(&mut h, "Apply");
    click(&mut h, "OK");
    let mut expected = before.clone();
    expected.graph_mut(id).unwrap().layers[1].element_scale = LayerElementScale::Fixed(0.75);
    assert_eq!(project(&h), &expected);
    let folder = save_dir("lm-fixed-element-scale");
    for extension in ["sheaf", "opj"] {
        let saved = sheaf_io::save::save_project(project(&h), &folder.join(format!("scale.{extension}"))).unwrap();
        if extension == "opj" {
            assert!(saved.warnings.iter().any(|warning| warning.contains("layer management element scaling")));
        }
        let reopened = sheaf_io::open_project(&saved.path).unwrap();
        assert_eq!(reopened.project.graph(id), expected.graph(id));
    }
    menu(&mut h, "Edit", "Undo Layer Management");
    assert_eq!(project(&h), &before);
}

#[test]
fn display_frame_element_scale_preserves_mixed_layers_until_apply() {
    let mut h = with_layers("lm-with-frame-element-scale", 3);
    let id = graph(&h).id;
    h.state_mut().doc.apply("Existing scaling", |p| {
        p.graph_mut(id).unwrap().layers[1].element_scale = LayerElementScale::WithLayerFrame;
    });
    settle(&mut h);
    let before = project(&h).clone();
    for button in ["Cancel", "OK"] {
        open(&mut h);
        click(&mut h, "Layer1");
        click(&mut h, "Layer2");
        click(&mut h, "Display");
        click(&mut h, "Scale Elements");
        assert!(h.get_by_label("Apply").accesskit_node().is_disabled());
        click(&mut h, "Scale with Layer Frame");
        assert!(h.get_by_label("Factor").accesskit_node().is_disabled());
        click(&mut h, "Apply");
        assert_eq!(project(&h), &before);
        click(&mut h, button);
    }
    let mut expected = before;
    expected.graph_mut(id).unwrap().layers[0].element_scale = LayerElementScale::WithLayerFrame;
    assert_eq!(project(&h), &expected);
}

#[test]
fn display_background_does_not_overwrite_layers_edited_while_the_dialog_is_open() {
    let mut h = with_layers("lm-background-stale", 2);
    let id = project(&h).graphs().next().unwrap().id;
    open(&mut h);
    click(&mut h, "Layer1");
    click(&mut h, "Display");
    background_choice(&mut h, "Red");
    click(&mut h, "Apply");
    h.state_mut().doc.apply("Independent layer edit", |p| {
        p.graph_mut(id).unwrap().layers[1].x_axis.title = Some("Keep this title".into());
    });
    settle(&mut h);
    let current = graph(&h).layers.clone();
    click(&mut h, "OK");
    assert!(h.state().layer_management.is_some());
    assert!(h.query_by_label_contains("graph layers changed").is_some());
    assert_eq!(graph(&h).layers, current);
    assert_eq!(h.state().doc.undo_label(), Some("Independent layer edit"));
    click(&mut h, "Cancel");
}

#[test]
fn display_background_requires_selection_and_unchanged_or_reverted_colors_do_not_add_history() {
    let mut h = with_layers("lm-background-noop", 2);
    let revision = h.state().doc.revision();
    open(&mut h);
    click(&mut h, "Display");
    assert!(h.get_by_label("Background Color").accesskit_node().is_disabled());
    assert!(h.get_by_label("Apply").accesskit_node().is_disabled());
    click(&mut h, "Layer1");
    background_choice(&mut h, "None");
    assert!(h.get_by_label("Apply").accesskit_node().is_disabled());
    background_choice(&mut h, "Red");
    click(&mut h, "Apply");
    background_choice(&mut h, "None");
    click(&mut h, "Apply");
    click(&mut h, "OK");
    assert_eq!(h.state().doc.revision(), revision);
    assert_ne!(h.state().doc.undo_label(), Some("Layer Management"));
}

#[test]
fn nonlinear_custom_scale_add_validates_before_changing_the_preview() {
    use sheaf_core::scales::CustomScale;

    let mut h = with_layers("lm-add-custom-formula", 1);
    let id = graph(&h).id;
    let before = project(&h).clone();
    open(&mut h);
    click(&mut h, "Add");
    click(&mut h, "X Scale");
    click(&mut h, "Custom Formula");
    let text = |h: &mut Harness<'static, SheafApp>, label, value: &str| {
        h.get_by_role_and_label(egui::accesskit::Role::TextInput, label).click();
        settle(h);
        ctrl(h, egui::Key::A);
        h.event(egui::Event::Text(value.into()));
        settle(h);
    };
    text(&mut h, "X Direct Formula", "x*x");
    text(&mut h, "X From", "1");
    text(&mut h, "X To", "5");
    click(&mut h, "Apply");
    assert!(h.query_by_label("Layer2").is_none());
    assert!(h.query_by_label_contains("direct and inverse formulas").is_some());
    assert_eq!(project(&h), &before);

    text(&mut h, "X Inverse Formula", "sqrt(x)");
    click(&mut h, "Apply");
    assert!(h.query_by_label("Layer2").is_some());
    assert_eq!(project(&h), &before);
    click(&mut h, "Undo");
    assert!(h.query_by_label("Layer2").is_none());
    click(&mut h, "Apply");
    click(&mut h, "OK");

    let x = &graph(&h).layers[1].x_axis;
    assert_eq!(x.scale, ScaleKind::CustomFormula);
    assert_eq!(x.custom_scale, Some(CustomScale { direct: "x*x".into(), inverse: "sqrt(x)".into() }));
    let geometry = sheaf_plot::layout_geometry(project(&h), id).unwrap();
    assert_eq!(geometry[1].x.endpoints(), (1.0, 5.0));
    assert_eq!(geometry[1].x.to_unit(2.0), 0.125);
    assert_eq!(h.state().doc.undo_label(), Some("Layer Management"));
    menu(&mut h, "Edit", "Undo");
    assert_eq!(project(&h), &before);
}

#[test]
fn discrete_scale_add_uses_the_inset_plots_and_undoes_the_layer() {
    let mut h = with_layers("lm-add-discrete", 1);
    let id = graph(&h).id;
    let before = project(&h).clone();
    open(&mut h);
    click(&mut h, "Add");
    click(&mut h, "Type");
    click(&mut h, "Inset With Data (Linked Dimension)");
    click(&mut h, "X Scale");
    click(&mut h, "Discrete");
    click(&mut h, "Apply");
    assert!(h.query_by_label("Layer2").is_some());
    assert_eq!(project(&h), &before);
    click(&mut h, "OK");

    assert_eq!(graph(&h).layers[1].x_axis.scale, ScaleKind::Discrete);
    let geometry = sheaf_plot::layout_geometry(project(&h), id).unwrap();
    assert!(geometry[1].x.discrete.is_some());
    assert_eq!(h.state().doc.undo_label(), Some("Layer Management"));
    menu(&mut h, "Edit", "Undo");
    assert_eq!(project(&h), &before);
}
