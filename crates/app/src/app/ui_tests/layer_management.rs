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
    for tab in ["Add", "Axes", "Display"] {
        click(&mut h, tab);
        assert!(
            h.query_by_label("Not available in Sheaf yet").is_some()
                || h.query_by_label_contains("Not available in Sheaf yet").is_some(),
            "{tab}"
        );
    }
}
