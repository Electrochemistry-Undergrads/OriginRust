use super::*;

fn named_columns(tag: &str) -> Harness<'static, SheafApp> {
    let mut h = start(vec![data_file(tag)]);
    let book = project(&h).workbooks().next().unwrap().id;
    h.state_mut().doc.apply("Column labels", |p| {
        p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![4.0, 8.0, 12.0])).unwrap();
        let sheet = &mut p.workbook_mut(book).unwrap().sheets[0];
        for (column, label) in sheet.columns.iter_mut().zip(["Time", "Shared current", "Potential", "Shared current"]) {
            column.long_name = label.into();
        }
    });
    settle(&mut h);
    show_window(&mut h, "Book1 - sample");
    click_text(&mut h, "4.5");
    h
}

fn field(h: &mut Harness<'static, SheafApp>, label: &str, text: &str) {
    h.get_by_label(label).focus();
    settle(h);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.event(egui::Event::Text(text.into()));
    settle(h);
}

#[test]
fn go_to_picker_distinguishes_duplicate_labels_and_waits_for_ok() {
    let mut h = named_columns("goto-picker-duplicate");
    let before = project(&h).clone();
    let revision = h.state().doc.revision();
    let selection = book_ui(&h).sel.clone();
    ctrl(&mut h, egui::Key::G);
    field(&mut h, "Go to Row", "0");
    click(&mut h, "▶");
    assert!(h.query_by_label("B(Y): Shared current").is_some());
    click(&mut h, "D(Y): Shared current");
    assert_eq!(h.get_by_label("Go to Column").accesskit_node().value().as_deref(), Some("D\"Shared current\""));
    assert!(h.state().go_to.is_some());
    assert_eq!(book_ui(&h).sel, selection);
    click(&mut h, "OK");
    assert!(h.state().go_to.is_none());
    assert_eq!(book_ui(&h).sel, Sel::Cells { anchor: (3, 2), end: (3, 2) });
    assert_eq!(project(&h), &before);
    assert_eq!(h.state().doc.revision(), revision);
}

#[test]
fn go_to_picker_reset_preserves_the_row_and_current_column() {
    let mut h = named_columns("goto-picker-reset");
    let before = project(&h).clone();
    let revision = h.state().doc.revision();
    ctrl(&mut h, egui::Key::G);
    field(&mut h, "Go to Row", "10");
    click(&mut h, "▶");
    click(&mut h, "C(Y): Potential");
    click(&mut h, "▶");
    click(&mut h, "Reset");
    click(&mut h, "OK");
    assert!(h.state().go_to.is_none());
    assert_eq!(book_ui(&h).sel, Sel::Cells { anchor: (1, 9), end: (1, 9) });
    assert_eq!(project(&h), &before);
    assert_eq!(h.state().doc.revision(), revision);
}

#[test]
fn go_to_typed_long_names_ignore_ascii_case_and_choose_the_first_duplicate() {
    let mut h = named_columns("goto-picker-typed-names");
    let before = project(&h).clone();
    let revision = h.state().doc.revision();
    for (name, column) in [("Potential", 2), ("potential", 2), ("Shared current", 1)] {
        ctrl(&mut h, egui::Key::G);
        field(&mut h, "Go to Row", "10");
        field(&mut h, "Go to Column", name);
        click(&mut h, "OK");
        assert!(h.state().go_to.is_none(), "{name}");
        assert_eq!(book_ui(&h).sel, Sel::Cells { anchor: (column, 9), end: (column, 9) }, "{name}");
    }
    assert_eq!(project(&h), &before);
    assert_eq!(h.state().doc.revision(), revision);
}

#[test]
fn escape_closes_the_column_picker_before_the_go_to_dialog() {
    let mut h = named_columns("goto-picker-escape");
    let before = project(&h).clone();
    let revision = h.state().doc.revision();
    let selection = book_ui(&h).sel.clone();
    ctrl(&mut h, egui::Key::G);
    click(&mut h, "▶");
    assert!(h.query_by_label("D(Y): Shared current").is_some());
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    assert!(h.state().go_to.is_some());
    assert!(h.query_by_label("D(Y): Shared current").is_none());
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    assert!(h.state().go_to.is_none());
    assert_eq!(book_ui(&h).sel, selection);
    assert_eq!(project(&h), &before);
    assert_eq!(h.state().doc.revision(), revision);
}
