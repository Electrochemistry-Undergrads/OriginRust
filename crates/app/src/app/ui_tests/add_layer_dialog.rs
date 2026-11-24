//! The layer-preset dialog reuses existing addition semantics with a captured parent.
use super::*;
use sheaf_core::graph::{AxisSide, Frame, LayerLink, ScaleLink};
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
fn opening_cancel_and_escape_do_not_edit_the_project() {
    let mut h = setup("add-layer-dialog-dismiss");
    let before = project(&h).clone();
    let revision = h.state().doc.revision();
    open(&mut h);
    assert!(h.get_by_label("Auto Offset Overlapped Axes").accesskit_node().is_disabled());
    kind(&mut h, NewLayerKind::InsetWithData);
    click(&mut h, "User Defined");
    click(&mut h, "Right");
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
fn user_defined_axes_link_to_the_chosen_layer_and_keep_the_source_active() {
    let mut h = setup("add-layer-dialog-user-defined");
    menu_path(&mut h, &["Graph", "New Layer(Axes)", NewLayerKind::Normal.menu_label()]);
    assert_eq!(graph_ui(&h).active_layer, 1);
    let before = project(&h).clone();
    open(&mut h);
    click(&mut h, "User Defined");
    click(&mut h, "Top");
    click(&mut h, "Left");
    click(&mut h, "Right");
    let choose = |h: &mut Harness<'static, SheafApp>, label, value| {
        h.get_by_role_and_label(egui::accesskit::Role::ComboBox, label).click();
        settle(h);
        click(h, value);
    };
    choose(&mut h, "Link To", "Layer 1");
    choose(&mut h, "X Scale Link", "Straight (1 to 1)");
    click(&mut h, "Activate New Layer");
    click(&mut h, "OK");

    let g = graph(&h);
    let layer = &g.layers[2];
    assert_eq!(layer.frame, Frame::WHOLE);
    assert_eq!(layer.link, Some(LayerLink { parent: 0, x: ScaleLink::Straight, y: ScaleLink::None }));
    assert_eq!(layer.x_axis.side, AxisSide::Normal);
    assert!(layer.x_axis.show_opposite && layer.x_axis.opposite_ticks);
    assert_eq!(layer.y_axis.side, AxisSide::Opposite);
    assert_eq!(graph_ui(&h).active_layer, 1);
    assert_eq!(h.state().doc.undo_label(), Some(sheaf_core::layers::ADD_LAYER_UNDO));
    menu(&mut h, "Edit", "Undo");
    assert_eq!(project(&h), &before);
}

#[test]
fn auto_offset_moves_only_the_new_linked_axis_and_undoes_with_the_layer() {
    let mut h = setup("add-layer-dialog-auto-offset");
    let graph_id = graph(&h).id;
    h.state_mut().doc.apply("Set reference frame", |p| {
        let layer = &mut p.graph_mut(graph_id).unwrap().layers[0];
        layer.frame.width = 0.6816;
        layer.y_axis.side = sheaf_core::graph::AxisSide::Normal;
        layer.y_axis.offset_pct = 0.0;
        layer.display.show_y_axes = true;
    });
    settle(&mut h);
    let before = project(&h).clone();
    open(&mut h);
    kind(&mut h, NewLayerKind::LeftY);
    let checkbox = "Auto Offset Overlapped Axes";
    assert!(!h.get_by_label(checkbox).accesskit_node().is_disabled());
    assert_eq!(h.get_by_label(checkbox).accesskit_node().toggled(), Some(egui::accesskit::Toggled::False));
    click(&mut h, checkbox);
    click(&mut h, "OK");

    let g = graph(&h);
    assert_eq!(g.layers[0], before.graph(graph_id).unwrap().layers[0]);
    assert_eq!(g.absolute_frame(1), g.absolute_frame(0));
    assert_eq!(g.layers[1].y_axis.offset_pct, 2.0);
    assert_eq!(g.layers[1].x_axis.offset_pct, 0.0);
    assert_eq!(graph_ui(&h).active_layer, 1);
    assert_eq!(h.state().doc.undo_label(), Some(sheaf_core::layers::ADD_LAYER_UNDO));
    menu(&mut h, "Edit", "Undo");
    assert_eq!(project(&h), &before);
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

#[test]
fn custom_scale_link_validates_and_creates_one_undo_step() {
    let mut h = setup("add-layer-dialog-custom-formula");
    let id = graph(&h).id;
    h.state_mut().doc.apply("Set parent limits", |p| {
        let layer = &mut p.graph_mut(id).unwrap().layers[0];
        layer.x_axis.min = Some(2.0);
        layer.x_axis.max = Some(14.0);
    });
    settle(&mut h);
    let before = project(&h).clone();
    let revision = h.state().doc.revision();
    open(&mut h);
    click(&mut h, "User Defined");
    click(&mut h, "Top");
    let choose = |h: &mut Harness<'static, SheafApp>, label, value| {
        h.get_by_role_and_label(egui::accesskit::Role::ComboBox, label).click();
        settle(h);
        click(h, value);
    };
    choose(&mut h, "Link To", "Layer 1");
    choose(&mut h, "X Scale Link", "Custom Formula");
    let formula = |h: &mut Harness<'static, SheafApp>, label, value: &str| {
        h.get_by_role_and_label(egui::accesskit::Role::TextInput, label).click();
        settle(h);
        ctrl(h, egui::Key::A);
        h.event(egui::Event::Text(value.into()));
        settle(h);
    };
    formula(&mut h, "X From Formula", "1/0");
    click(&mut h, "OK");
    assert!(h.state().add_layer_dialog.is_some());
    assert_eq!(h.state().doc.revision(), revision);
    assert_eq!(project(&h), &before);

    formula(&mut h, "X From Formula", "X1+10");
    formula(&mut h, "X To Formula", "X2+50");
    click(&mut h, "OK");
    assert!(h.state().add_layer_dialog.is_none());
    let layer = graph(&h).valid_link(1).unwrap();
    assert_eq!(layer.x, ScaleLink::Custom { from: "X1+10".into(), to: "X2+50".into() });
    assert_eq!(layer.y, ScaleLink::None);
    let geometry = sheaf_plot::layout_geometry(project(&h), id).unwrap();
    // Origin 2016 layer.x.link.from/to readback (graphing 21.15).
    assert_eq!(geometry[1].x.endpoints(), (12.0, 64.0));
    assert_eq!(h.state().doc.undo_label(), Some(sheaf_core::layers::ADD_LAYER_UNDO));
    menu(&mut h, "Edit", "Undo");
    assert_eq!(project(&h), &before);
    menu(&mut h, "Edit", "Redo");

    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::GraphEdit { graph: id, op: GraphOp::DeleteLayer { layer: 0 } }, &ctx);
    settle(&mut h);
    assert!(graph(&h).layers[0].link.is_none());
    let geometry = sheaf_plot::layout_geometry(project(&h), id).unwrap();
    assert_eq!(geometry[0].x.endpoints(), (12.0, 64.0));
}
