use super::*;

#[test]
fn skip_points_is_on_drop_lines_and_supports_cancel_apply_and_undo() {
    let mut h = start(vec![data_file("skip-points")]);
    open_props(&mut h, 0, PropsTab::Plots, Some(0));
    click(&mut h, "Drop Lines");
    assert!(has_label(&h, "Data Points Display Control"));
    assert!(is_disabled(&h, "Frequency"));
    click(&mut h, "Skip Points");
    assert_eq!(graph(&h).layers[0].plots[0].symbol.every, 2);
    assert!(!is_disabled(&h, "Frequency"));
    click(&mut h, "Cancel");
    assert_eq!(graph(&h).layers[0].plots[0].symbol.every, 1);

    open_props(&mut h, 0, PropsTab::Plots, Some(0));
    click(&mut h, "Drop Lines");
    click(&mut h, "Skip Points");
    click(&mut h, "Frequency");
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.event(egui::Event::Text("3".into()));
    settle(&mut h);
    key(&mut h, egui::Key::Enter);
    assert_eq!(graph(&h).layers[0].plots[0].symbol.every, 3);
    click(&mut h, "Apply");
    click(&mut h, "Skip Points");
    assert_eq!(graph(&h).layers[0].plots[0].symbol.every, 1);
    click(&mut h, "Cancel");
    assert_eq!(graph(&h).layers[0].plots[0].symbol.every, 3);
    menu(&mut h, "Edit", "Undo Change graph settings");
    assert_eq!(graph(&h).layers[0].plots[0].symbol.every, 1);
    menu(&mut h, "Edit", "Redo Change graph settings");
    assert_eq!(graph(&h).layers[0].plots[0].symbol.every, 3);
}
