use super::*;
use sheaf_core::graph::LegendPosition;

#[test]
fn native_top_left_legend_coordinates_survive_the_editor_history_and_saves() {
    let mut h = start(vec![data_file("nativelegendanchor")]);
    let anchor = LegendPosition::ManualTopLeft { x: 1.002754820936639, y: -0.3123018389346861 };
    h.state_mut().doc.apply("Native template anchor", |p| {
        let id = p.graphs().next().unwrap().id;
        p.graph_mut(id).unwrap().layers[0].legend.position = anchor;
    });
    open_props(&mut h, 0, PropsTab::Legend, None);
    let _ = h.get(By::new().label("Top left (fraction of plot area)"));
    click(&mut h, "Apply");
    assert_eq!(
        graph(&h).layers[0].legend.position,
        anchor,
        "Opening and applying must not clamp outside-frame coordinates"
    );
    h.get(By::new().role(egui::accesskit::Role::ComboBox).value("Custom")).click();
    settle(&mut h);
    click(&mut h, "Top right");
    click(&mut h, "Cancel");
    assert_eq!(graph(&h).layers[0].legend.position, anchor);
    open_props(&mut h, 0, PropsTab::Legend, None);
    h.get(By::new().role(egui::accesskit::Role::ComboBox).value("Custom")).click();
    settle(&mut h);
    click(&mut h, "Top right");
    click(&mut h, "OK");
    assert_eq!(graph(&h).layers[0].legend.position, LegendPosition::TopRight);
    menu(&mut h, "Edit", "Undo");
    assert_eq!(graph(&h).layers[0].legend.position, anchor);
    let file = save_dir("nativelegendanchor").join("anchor.sheaf");
    h.state_mut().save_to(file.clone());
    let reopened = start(vec![file]);
    assert_eq!(graph(&reopened).layers[0].legend.position, anchor);
}
