use super::*;
use sheaf_core::graph::LegendPosition;

fn choose_position(h: &mut Harness<'static, SheafApp>, from: &str, to: &str) {
    h.get(By::new().role(egui::accesskit::Role::ComboBox).value(from)).click();
    settle(h);
    click(h, to);
}

#[test]
fn automatic_legend_position_can_be_restored_cancelled_undone_and_saved() {
    let mut h = start(vec![data_file("legendauto")]);
    h.state_mut().doc.apply("Automatic legend", |p| {
        let id = p.graphs().next().unwrap().id;
        p.graph_mut(id).unwrap().layers[0].legend.position = LegendPosition::Automatic;
    });
    open_props(&mut h, 0, PropsTab::Legend, None);
    choose_position(&mut h, "Automatic", "Top right");
    click(&mut h, "OK");
    assert_eq!(graph(&h).layers[0].legend.position, LegendPosition::TopRight);
    menu(&mut h, "Edit", "Undo");
    assert_eq!(graph(&h).layers[0].legend.position, LegendPosition::Automatic);
    menu(&mut h, "Edit", "Redo");
    assert_eq!(graph(&h).layers[0].legend.position, LegendPosition::TopRight);
    open_props(&mut h, 0, PropsTab::Legend, None);
    choose_position(&mut h, "Top right", "Automatic");
    click(&mut h, "Cancel");
    assert_eq!(graph(&h).layers[0].legend.position, LegendPosition::TopRight);
    open_props(&mut h, 0, PropsTab::Legend, None);
    choose_position(&mut h, "Top right", "Automatic");
    click(&mut h, "OK");
    assert_eq!(graph(&h).layers[0].legend.position, LegendPosition::Automatic);
    let file = save_dir("legendauto").join("legend.sheaf");
    h.state_mut().save_to(file.clone());
    assert!(!h.state().doc.dirty);
    let reopened = start(vec![file]);
    assert_eq!(graph(&reopened).layers[0].legend.position, LegendPosition::Automatic);
}
