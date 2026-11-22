use super::*;
use sheaf_core::graph::Frame;

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
    assert_eq!(g.layers[1].link.map(|l| l.parent), Some(0));
    assert_eq!(g.link_parent(1), Some(0));
    let page = g.page_frame(1);
    assert!((page.left - before.left).abs() < 1e-9 && (page.width - before.width).abs() < 1e-9);
}

#[test]
fn the_tabs_that_are_not_built_are_disabled() {
    let mut h = with_layers("lm-tabs", 2);
    open(&mut h);
    for tab in ["Add", "Axes"] {
        click(&mut h, tab);
        assert!(
            h.query_by_label("Not available in Sheaf yet").is_some()
                || h.query_by_label_contains("Not available in Sheaf yet").is_some(),
            "{tab}"
        );
    }
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
