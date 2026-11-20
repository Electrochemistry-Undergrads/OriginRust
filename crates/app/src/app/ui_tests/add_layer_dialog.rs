//! The layer-preset dialog reuses existing addition semantics with a captured parent.
use super::*;
use sheaf_core::layers::NewLayerKind;

fn setup(tag: &str) -> Harness<'static, SheafApp> {
    let mut h = start(vec![data_file(tag)]);
    show_window(&mut h, "Graph1 - sample");
    h
}

fn open(h: &mut Harness<'static, SheafApp>) {
    menu_path(h, &["Graph", "New Layer(Axes)", "Open Dialog..."]);
    assert!(h.state().add_layer_dialog.is_some());
}

fn kind(h: &mut Harness<'static, SheafApp>, kind: NewLayerKind) {
    h.get_by_role_and_label(egui::accesskit::Role::ComboBox, "Layer Type").click();
    settle(h);
    click(h, kind.menu_label());
}

#[test]
fn all_nine_dialog_presets_match_the_existing_direct_menu_with_one_undo() {
    let mut h = setup("add-layer-dialog-presets");
    for preset in NewLayerKind::ALL {
        let id = graph(&h).id;
        h.state_mut().graphs.get_mut(&id).unwrap().active_layer = 0;
        settle(&mut h);
        let before = project(&h).clone();
        menu_path(&mut h, &["Graph", "New Layer(Axes)", preset.menu_label()]);
        let expected = project(&h).clone();
        menu(&mut h, "Edit", "Undo");
        assert_eq!(project(&h), &before);
        h.state_mut().graphs.get_mut(&id).unwrap().active_layer = 0;
        settle(&mut h);
        open(&mut h);
        assert_eq!(
            h.get_by_label("Activate New Layer").accesskit_node().toggled(),
            Some(egui::accesskit::Toggled::True)
        );
        kind(&mut h, preset);
        click(&mut h, "OK");
        assert!(h.state().add_layer_dialog.is_none());
        assert_eq!(project(&h), &expected, "{preset:?}");
        assert_eq!(graph_ui(&h).active_layer, 1, "{preset:?}");
        assert_eq!(h.state().doc.undo_label(), Some(sheaf_core::layers::ADD_LAYER_UNDO));
        menu(&mut h, "Edit", "Undo");
        assert_eq!(project(&h), &before, "one undo: {preset:?}");
    }
}

#[test]
fn opening_cancel_escape_and_unavailable_custom_controls_do_not_edit_the_project() {
    let mut h = setup("add-layer-dialog-dismiss");
    let before = project(&h).clone();
    let revision = h.state().doc.revision();
    open(&mut h);
    assert!(h.get_by_label("User Defined").accesskit_node().is_disabled());
    assert!(h.get_by_label("Auto Offset Overlapped Axes").accesskit_node().is_disabled());
    kind(&mut h, NewLayerKind::InsetWithData);
    click(&mut h, "Activate New Layer");
    click(&mut h, "Cancel");
    assert_eq!(project(&h), &before);
    assert_eq!(h.state().doc.revision(), revision);
    open(&mut h);
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    assert!(h.state().add_layer_dialog.is_none());
    assert_eq!(project(&h), &before);
    assert_eq!(h.state().doc.revision(), revision);
}

#[test]
fn unchecked_activation_keeps_the_captured_parent_active_and_preserves_other_layers() {
    let mut h = setup("add-layer-dialog-activation");
    menu_path(&mut h, &["Graph", "New Layer(Axes)", NewLayerKind::Normal.menu_label()]);
    assert_eq!(graph_ui(&h).active_layer, 1);
    let before = project(&h).clone();
    let layers = graph(&h).layers.clone();
    open(&mut h);
    kind(&mut h, NewLayerKind::TopX);
    click(&mut h, "Activate New Layer");
    click(&mut h, "OK");
    let g = graph(&h);
    assert_eq!(&g.layers[..2], layers.as_slice());
    assert_eq!(g.layers[2].link.as_ref().unwrap().parent, 1);
    assert_eq!(graph_ui(&h).active_layer, 1);
    menu(&mut h, "Edit", "Undo");
    assert_eq!(project(&h), &before);
    menu(&mut h, "Edit", "Redo");
    assert_eq!(graph(&h).layers.len(), 3);
    assert_eq!(graph(&h).layers[2].link.as_ref().unwrap().parent, 1);
}

#[test]
fn changing_active_layer_while_open_does_not_retarget_the_captured_parent() {
    let mut h = setup("add-layer-dialog-capture");
    menu_path(&mut h, &["Graph", "New Layer(Axes)", NewLayerKind::Normal.menu_label()]);
    let id = graph(&h).id;
    open(&mut h);
    h.state_mut().graphs.get_mut(&id).unwrap().active_layer = 0;
    settle(&mut h);
    kind(&mut h, NewLayerKind::RightY);
    click(&mut h, "OK");
    assert_eq!(graph(&h).layers[2].link.as_ref().unwrap().parent, 1);
    assert_eq!(graph_ui(&h).active_layer, 2);
}

#[test]
fn revised_replaced_or_removed_source_layers_close_without_an_add_or_history() {
    for scenario in 0..3 {
        let mut h = setup(&format!("add-layer-dialog-stale-{scenario}"));
        menu_path(&mut h, &["Graph", "New Layer(Axes)", NewLayerKind::Normal.menu_label()]);
        let id = graph(&h).id;
        open(&mut h);
        match scenario {
            0 => {
                h.state_mut().doc.apply("Other edit", |p| p.graph_mut(id).unwrap().name.push('x'));
            }
            1 => {
                let replacement = project(&h).clone();
                h.state_mut().doc.replace(replacement);
            }
            _ => {
                h.state_mut().doc.apply("Remove source layer", |p| {
                    p.graph_mut(id).unwrap().delete_layer(1);
                });
            }
        }
        let before = project(&h).clone();
        let revision = h.state().doc.revision();
        settle(&mut h);
        assert!(h.state().add_layer_dialog.is_none(), "{scenario}");
        assert_eq!(project(&h), &before, "{scenario}");
        assert_eq!(h.state().doc.revision(), revision, "{scenario}");
    }
}

#[test]
fn default_choice_adds_an_independent_layer_and_invalid_actions_add_nothing() {
    let mut h = setup("add-layer-dialog-default");
    let book = project(&h).workbooks().next().unwrap().id;
    let before = project(&h).clone();
    h.state_mut().apply(Action::AddLayerDialog { graph: book, layer: 0 }, &egui::Context::default());
    assert!(h.state().add_layer_dialog.is_none());
    assert_eq!(project(&h), &before);
    let id = graph(&h).id;
    h.state_mut().apply(Action::AddLayerDialog { graph: id, layer: 42 }, &egui::Context::default());
    assert!(h.state().add_layer_dialog.is_none());
    open(&mut h);
    click(&mut h, "OK");
    assert_eq!(graph(&h).layers.len(), 2);
    assert!(graph(&h).layers[1].link.is_none());
    assert!(graph(&h).layers[1].plots.is_empty());
}

#[test]
fn an_inset_created_by_the_dialog_renders_and_survives_native_and_companion_saves() {
    let mut h = setup("add-layer-dialog-save");
    let original = graph(&h).layers[0].clone();
    open(&mut h);
    kind(&mut h, NewLayerKind::InsetWithData);
    click(&mut h, "OK");
    let expected = graph(&h).layers.clone();
    assert_eq!(expected[1].plots.len(), 1);
    assert_eq!(expected[1].plots[0], original.plots[0]);
    assert_eq!(expected[1].link.as_ref().unwrap().parent, 0);
    let scene = sheaf_plot::graph_scene(project(&h), graph(&h).id).unwrap();
    assert_eq!(scene.layers.len(), 2);
    assert!(!scene.layers[1].plots.is_empty());
    let svg = sheaf_plot::render_svg(project(&h), graph(&h).id).unwrap();
    assert!(svg.contains("<svg"));
    let dir = save_dir("add-layer-dialog-save");
    for extension in ["sheaf", "opj"] {
        let file = dir.join(format!("inset.{extension}"));
        h.state_mut().save_to(file.clone());
        let back = start(vec![file]);
        assert_eq!(graph(&back).layers, expected, "{extension}");
    }
}
