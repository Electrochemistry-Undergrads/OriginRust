//! Adding and deleting layers from the menus, the toolbar and the layer icon (T61).
use super::*;
use sheaf_core::layers::NewLayerKind;

fn setup(tag: &str) -> (Harness<'static, SheafApp>, WindowId) {
    let mut h = start(vec![data_file(&format!("layers-{tag}"))]);
    show_window(&mut h, "Graph1 - sample");
    let id = project(&h).graphs().next().unwrap().id;
    (h, id)
}

fn layers(h: &Harness<'static, SheafApp>) -> usize {
    project(h).graphs().next().unwrap().layers.len()
}

fn edit(h: &mut Harness<'static, SheafApp>, graph: WindowId, op: GraphOp) {
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::GraphEdit { graph, op }, &ctx);
    settle(h);
}

#[test]
fn new_layer_menu_adds_the_layer_makes_it_active_and_is_one_undo_step() {
    let (mut h, _) = setup("menu");
    assert_eq!(layers(&h), 1);
    menu_path(&mut h, &["Graph", "New Layer(Axes)", "Inset (Linked Dimension)"]);
    assert_eq!(layers(&h), 2);
    assert_eq!(graph_ui(&h).active_layer, 1, "the new layer is active");
    assert_eq!(h.state().doc.undo_label(), Some("Create"));
    let g = project(&h).graphs().next().unwrap();
    assert_eq!(g.layers[1].link.unwrap().parent, 0);
    menu(&mut h, "Edit", "Undo Create");
    assert_eq!(layers(&h), 1);
    h.key_press_modifiers(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::Z);
    settle(&mut h);
    assert_eq!(layers(&h), 2, "redo brings the layer back");
}

#[test]
fn the_toolbar_buttons_add_their_kind_of_layer() {
    let (mut h, _) = setup("toolbar");
    for (tip, _kind) in [
        ("Add Top-X Layer", NewLayerKind::TopX),
        ("Add Right-Y Layer", NewLayerKind::RightY),
        ("Add Inset Graph With Data", NewLayerKind::InsetWithData),
    ] {
        let n = layers(&h);
        h.get_by_label(&format!("Graph toolbar: {tip}")).click();
        settle(&mut h);
        assert_eq!(layers(&h), n + 1, "{tip}");
        let g = project(&h).graphs().next().unwrap();
        assert!(g.layers[n].link.is_some(), "{tip}");
    }
}

#[test]
fn deleting_a_layer_repairs_the_active_layer_and_the_selection_and_undoes_in_one_step() {
    let (mut h, id) = setup("delete");
    edit(&mut h, id, GraphOp::AddLayer { layer: 0, kind: NewLayerKind::RightY });
    edit(&mut h, id, GraphOp::AddLayer { layer: 0, kind: NewLayerKind::Inset });
    assert_eq!(layers(&h), 3);
    {
        let ui = h.state_mut().graphs.get_mut(&id).unwrap();
        ui.active_layer = 2;
        ui.selected = Some((2, ObjectKind::Legend));
    }
    // Delete layer 2 (index 1): the active layer and the selection move up.
    edit(&mut h, id, GraphOp::DeleteLayer { layer: 1 });
    assert_eq!(layers(&h), 2);
    assert_eq!(h.state().doc.undo_label(), Some("Undelete frame"));
    assert_eq!(graph_ui(&h).active_layer, 1);
    assert_eq!(graph_ui(&h).selected, Some((1, ObjectKind::Legend)));
    // Delete the active layer: the one before it takes over, and its own selection goes.
    edit(&mut h, id, GraphOp::DeleteLayer { layer: 1 });
    assert_eq!(layers(&h), 1);
    assert_eq!(graph_ui(&h).active_layer, 0);
    assert_eq!(graph_ui(&h).selected, None);
    // The only layer stays.
    let before = h.state().doc.undo_label().map(str::to_string);
    edit(&mut h, id, GraphOp::DeleteLayer { layer: 0 });
    assert_eq!(layers(&h), 1);
    assert_eq!(h.state().doc.undo_label().map(str::to_string), before, "a refused delete is no undo step");
    menu(&mut h, "Edit", "Undo Undelete frame");
    assert_eq!(layers(&h), 2);
}

#[test]
fn deleting_the_parent_from_the_icon_menu_unlinks_the_children() {
    let (mut h, id) = setup("parent");
    edit(&mut h, id, GraphOp::AddLayer { layer: 0, kind: NewLayerKind::RightY });
    edit(&mut h, id, GraphOp::DeleteLayer { layer: 0 });
    let g = project(&h).graphs().next().unwrap();
    assert_eq!(g.layers.len(), 1);
    assert!(g.layers[0].link.is_none());
}

#[test]
fn the_layer_icon_menu_deletes_and_opens_layer_properties() {
    let entries = menus::layer_icon_menu(1, 2, &[], None);
    let find = |label: &str| {
        entries.iter().find_map(|e| match e {
            menus::Entry::Item(i) if i.label == label => Some(i.cmd.clone()),
            _ => None,
        })
    };
    assert!(matches!(find("Delete Layer"), Some(Some(menus::Cmd::GraphEdit(GraphOp::DeleteLayer { layer: 1 })))));
    assert!(matches!(
        find("Layer Properties..."),
        Some(Some(menus::Cmd::GraphPropsAt { layer: 1, tab: PropsTab::Layer, plot: None }))
    ));
    let single = menus::layer_icon_menu(0, 1, &[], None);
    assert!(single.iter().any(|e| matches!(e, menus::Entry::Item(i) if i.label == "Delete Layer" && i.cmd.is_none())));
}

#[test]
fn format_layer_properties_opens_plot_details_on_the_active_layer() {
    let (mut h, id) = setup("props");
    edit(&mut h, id, GraphOp::AddLayer { layer: 0, kind: NewLayerKind::TopX });
    menu(&mut h, "Format", "Layer Properties...");
    let d = h.state().graph_props.as_ref().expect("Plot Details is open");
    assert_eq!((d.layer, d.tab()), (1, PropsTab::Layer));
}

#[test]
fn a_plot_moves_to_another_layer_in_one_undo_step() {
    let (mut h, id) = setup("move-plot");
    edit(&mut h, id, GraphOp::AddLayer { layer: 0, kind: NewLayerKind::RightY });
    let count = |h: &Harness<'static, SheafApp>| {
        let g = project(h).graphs().next().unwrap();
        (g.layers[0].plots.len(), g.layers[1].plots.len())
    };
    let (a, b) = count(&h);
    assert!(a >= 1);
    edit(&mut h, id, GraphOp::MovePlotToLayer { layer: 0, plot: 0, to: 1 });
    assert_eq!(count(&h), (a - 1, b + 1));
    assert_eq!(h.state().doc.undo_label(), Some("Move plot to layer"));
    menu(&mut h, "Edit", "Undo Move plot to layer");
    assert_eq!(count(&h), (a, b));
}

#[test]
fn an_axis_offset_survives_save_and_reload_of_the_graph() {
    let (mut h, id) = setup("offset");
    edit(&mut h, id, GraphOp::AddLayer { layer: 0, kind: NewLayerKind::RightY });
    let mut g = project(&h).graph(id).unwrap().clone();
    g.layers[1].y_axis.offset_pct = 12.0;
    let back: sheaf_core::graph::Graph = serde_json::from_str(&serde_json::to_string(&g).unwrap()).unwrap();
    assert_eq!(back.layers[1].y_axis.offset_pct, 12.0);
    assert_eq!(back.layers[0].y_axis.offset_pct, 0.0);
}
