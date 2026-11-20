use super::*;

#[test]
fn data_style_more_formats_only_the_selected_range_and_keeps_apply_cancel_and_undo() {
    let mut h = start(vec![data_file("worksheet-style-more")]);
    h.set_size(egui::vec2(1280.0, 1400.0));
    show_window(&mut h, "Book1 - sample.csv");
    let b = sheet(&h).column_by_name("B").unwrap().id;
    let before = project(&h).clone();
    let revision = h.state().doc.revision();
    right_click_text(&mut h, "4.5");
    menu_path(&mut h, &["Set Data Style"]);
    assert!(entry_enabled(&h, "Rich Text"), "Rich Text works (T208)");
    assert!(entry_enabled(&h, "Float"), "Float works (T214)");
    assert!(!entry_enabled(&h, "Wrap Text"), "unsupported text controls remain disabled");
    assert!(entry_enabled(&h, "More..."));
    menu_path(&mut h, &["More..."]);
    assert!(h.state().cell_format.is_some());
    assert_eq!(h.state().doc.revision(), revision, "opening More is inert");
    click(&mut h, "Bold");
    click(&mut h, "Apply");
    assert!(h.state().cell_format.is_some(), "Apply keeps the dialog open");
    assert!(sheet(&h).cell_format_at(b, 1).bold);
    assert!(!sheet(&h).cell_format_at(b, 0).bold);
    assert!(!sheet(&h).cell_format_at(b, 2).bold);
    assert_eq!(sheet(&h).columns, before.workbooks().next().unwrap().sheet().columns);
    assert_eq!(h.state().doc.undo_label(), Some("Format cells"));
    let applied = project(&h).clone();
    let revision = h.state().doc.revision();
    click(&mut h, "Italic");
    click(&mut h, "Cancel");
    assert!(h.state().cell_format.is_none());
    assert_eq!(project(&h), &applied, "Cancel discards the unapplied edits");
    assert_eq!(h.state().doc.revision(), revision);
    menu(&mut h, "Edit", "Undo");
    assert_eq!(project(&h), &before, "one Undo reverses the applied formatting");
}
