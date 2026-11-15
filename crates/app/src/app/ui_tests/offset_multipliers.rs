use super::*;

#[test]
fn offset_multipliers_are_individual_and_support_apply_cancel_and_history() {
    let mut h = start(vec![data_file("offset-multipliers")]);
    let open = |h: &mut Harness<'static, SheafApp>| {
        open_props(h, 0, PropsTab::Plots, Some(0));
        click(h, "Offset");
    };
    open(&mut h);
    assert!(has_label(&h, "Multiplier"));
    assert!(h.query_all_by_role(egui::accesskit::Role::SpinButton).all(|n| n.accesskit_node().is_disabled()));
    click(&mut h, "Cancel");
    assert!(graph(&h).layers[0].plot_offsets.is_empty(), "viewing does not add an entry");
    edit_graph(&mut h, |g| g.layers[0].stack = Stack::Individual);
    let y = graph(&h).layers[0].plots[0].y;
    open(&mut h);
    type_into_number(&mut h, 0, "2");
    type_into_number(&mut h, 2, "3");
    let o = graph(&h).layers[0].plot_offset(y).unwrap();
    assert_eq!((o.x_multiplier, o.y_multiplier), (2.0, 3.0));
    assert!(graph(&h).layers[0].plot_offset(graph(&h).layers[0].plots[1].y).is_none(), "other member unchanged");
    click(&mut h, "Cancel");
    assert!(graph(&h).layers[0].plot_offsets.is_empty());
    open(&mut h);
    type_into_number(&mut h, 0, "2");
    type_into_number(&mut h, 2, "3");
    click(&mut h, "Apply");
    type_into_number(&mut h, 0, "4");
    click(&mut h, "Cancel");
    assert_eq!(graph(&h).layers[0].plot_offset(y).unwrap().x_multiplier, 2.0);
    menu(&mut h, "Edit", "Undo Change graph settings");
    assert!(graph(&h).layers[0].plot_offsets.is_empty());
    menu(&mut h, "Edit", "Redo Change graph settings");
    assert_eq!(graph(&h).layers[0].plot_offset(y).unwrap().y_multiplier, 3.0);
}

#[test]
fn scaled_integration_gadget_starts_in_source_units_and_dragging_keeps_the_area_unscaled() {
    let mut h = start(vec![data_file("scaled-gadget")]);
    edit_graph(&mut h, |g| {
        let layer = &mut g.layers[0];
        layer.stack = Stack::Individual;
        layer.plots[1].visible = false;
        layer.x_axis.min = Some(70.0);
        layer.x_axis.max = Some(130.0);
        let o = layer.plot_offset_mut(layer.plots[0].y);
        (o.x, o.y, o.x_multiplier, o.y_multiplier) = (50.0, 70.0, 20.0, 30.0);
    });
    let (_, graph_id) = window_ids(&h);
    let geom = sheaf_plot::layout_geometry(project(&h), graph_id).unwrap().remove(0);
    menu(&mut h, "Gadgets", "Integrate...");
    let g = graph_ui(&h).gadget.unwrap();
    assert!((g.x0 - (geom.x.from_unit(0.25) - 50.0) / 20.0).abs() < 1e-10);
    assert!((g.x1 - (geom.x.from_unit(0.75) - 50.0) / 20.0).abs() < 1e-10);
    let (x, y) = crate::gadget::plot_xy(project(&h), graph_id, 0, 0).unwrap();
    let source_area = g.result(&x, &y).expect("two original rows inside the displayed range");
    assert_eq!(source_area.area, 5.5, "source area, not scaled display area");
    let range = graph_ui(&h).gadget_rect.expect("scaled range is drawn");
    drag(&mut h, range.right_center(), range.right_center() + egui::vec2(30.0, 0.0));
    assert!(graph_ui(&h).gadget.unwrap().x1 > g.x1);
}
