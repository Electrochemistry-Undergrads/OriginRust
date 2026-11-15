use super::*;

fn fixture(tag: &str) -> (Harness<'static, SheafApp>, WindowId) {
    let mut h = start(vec![data_file(tag)]);
    let book = project(&h).workbooks().next().unwrap().id;
    h.state_mut().doc.apply("Column Browser fixture", |p| {
        p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![4.0, 8.0, 12.0])).unwrap();
        let source = &mut p.workbook_mut(book).unwrap().sheets[0];
        source.name = "Sheet1".into();
        let columns = &mut source.columns;
        for (column, (label, values)) in columns.iter_mut().zip([
            ("Time", vec![1.0, 2.0, 3.0, 4.0]),
            ("Shared current", vec![2.0, 4.0]),
            ("Potential", vec![3.0, 6.0, 9.0, 12.0, 15.0, 18.0]),
            ("Shared current", vec![4.0, 8.0, 12.0]),
        ]) {
            column.long_name = label.into();
            column.data = ColumnData::numeric(values);
        }
        let other = p.add_workbook("Other book");
        for (designation, label, values) in [
            (Designation::X, "Other X", vec![10.0, 20.0]),
            (Designation::Y, "Other Y", vec![30.0, 40.0, 50.0, 60.0, 70.0]),
        ] {
            let id = p.add_column(other, 0, designation, ColumnData::numeric(values)).unwrap();
            p.column_mut(id).unwrap().long_name = label.into();
        }
    });
    settle(&mut h);
    show_window(&mut h, "Book1 - sample");
    let source = sheet(&h).clone();
    h.state_mut().books.get_mut(&book).unwrap().select_cells(&source, (2, 6), (2, 6));
    settle(&mut h);
    (h, book)
}

fn original_cell(h: &Harness<'static, SheafApp>, book: WindowId) -> Option<(usize, usize)> {
    h.state().books.get(&book).unwrap().active_cell()
}

fn field(h: &mut Harness<'static, SheafApp>, label: &str, text: &str) {
    h.get_by_label(label).focus();
    settle(h);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.event(egui::Event::Text(text.into()));
    settle(h);
}

fn browser(h: &mut Harness<'static, SheafApp>) {
    click(h, "▶");
    click(h, "Select Columns...");
    assert!(h.query_by_label("List Columns").is_some());
    assert!(h.query_by_label("Go to Row").is_none());
    assert!(!egui::Popup::is_any_open(&h.ctx), "the parent flyout must release input to the browser");
}

fn add(h: &mut Harness<'static, SheafApp>, name: &str) {
    click(h, name);
    click(h, "Add");
}

#[test]
fn column_browser_returns_a_qualified_duplicate_name_without_navigating_until_parent_ok() {
    let (mut h, book) = fixture("goto-browser-qualified");
    let before = project(&h).clone();
    let revision = h.state().doc.revision();
    ctrl(&mut h, egui::Key::G);
    field(&mut h, "Go to Row", "0");
    browser(&mut h);
    assert!(h.query_by_label("Book2 / Sheet1 / B(Y): Other Y").is_none(), "Current Sheet default");
    add(&mut h, "Book1 / Sheet1 / D(Y): Shared current");
    click(&mut h, "OK");
    assert_eq!(
        h.get_by_label("Go to Column").accesskit_node().value().as_deref(),
        Some("[Book1]Sheet1!D\"Shared current\"")
    );
    assert_eq!(original_cell(&h, book), Some((2, 6)));
    click(&mut h, "OK");
    assert_eq!(original_cell(&h, book), Some((3, 2)));
    assert_eq!(project(&h), &before);
    assert_eq!(h.state().doc.revision(), revision);
}

#[test]
fn project_browser_uses_the_selected_column_position_in_the_original_sheet() {
    let (mut h, book) = fixture("goto-browser-other-book");
    let before = project(&h).clone();
    ctrl(&mut h, egui::Key::G);
    field(&mut h, "Go to Row", "0");
    browser(&mut h);
    click(&mut h, "List Columns");
    click(&mut h, "in Current Project");
    add(&mut h, "Book2 / Sheet1 / B(Y): Other Y");
    click(&mut h, "OK");
    assert_eq!(h.get_by_label("Go to Column").accesskit_node().value().as_deref(), Some("[Book2]Sheet1!B\"Other Y\""));
    click(&mut h, "OK");
    assert_eq!(original_cell(&h, book), Some((1, 1)), "original B ends at row 2; Other B ends at 5");
    assert_eq!(project(&h), &before);
}

#[test]
fn browser_cancel_and_escape_preserve_the_parent_fields_and_selection() {
    for escape in [false, true] {
        let (mut h, book) = fixture(&format!("goto-browser-cancel-{escape}"));
        let before = project(&h).clone();
        ctrl(&mut h, egui::Key::G);
        field(&mut h, "Go to Column", "B");
        field(&mut h, "Go to Row", "10");
        browser(&mut h);
        add(&mut h, "Book1 / Sheet1 / D(Y): Shared current");
        if escape {
            h.key_press(egui::Key::Escape);
            settle(&mut h);
        } else {
            click(&mut h, "Cancel");
        }
        assert!(h.state().go_to.is_some());
        assert_eq!(h.get_by_label("Go to Column").accesskit_node().value().as_deref(), Some("B"));
        assert_eq!(h.get_by_label("Go to Row").accesskit_node().value().as_deref(), Some("10"));
        assert_eq!(
            original_cell(&h, book),
            Some((2, 6)),
            "after browser close, escape={escape}, sel={:?}",
            h.state().books.get(&book).unwrap().sel
        );
        click(&mut h, "OK");
        assert_eq!(original_cell(&h, book), Some((1, 9)));
        assert_eq!(project(&h), &before);
    }
}

#[test]
fn multiple_browser_ranges_follow_origin_and_removing_a_range_restores_single_column_lookup() {
    for remove in [false, true] {
        let (mut h, book) = fixture(&format!("goto-browser-ranges-{remove}"));
        let before = project(&h).clone();
        ctrl(&mut h, egui::Key::G);
        field(&mut h, "Go to Row", if remove { "0" } else { "10" });
        browser(&mut h);
        add(&mut h, "Book1 / Sheet1 / A(X): Time");
        add(&mut h, "Book1 / Sheet1 / C(Y): Potential");
        if remove {
            click(&mut h, "[Book1]Sheet1!A\"Time\"");
            click(&mut h, "Remove");
        }
        click(&mut h, "OK");
        assert!(h.get_by_label("Go to Column").accesskit_node().value().unwrap().contains("C\"Potential\""));
        click(&mut h, "OK");
        assert_eq!(original_cell(&h, book), Some(if remove { (2, 5) } else { (3, 9) }));
        assert_eq!(project(&h), &before);
    }
}

#[test]
fn editing_or_resetting_a_browser_reference_discards_its_saved_ordinal() {
    for reset in [false, true] {
        let (mut h, book) = fixture(&format!("goto-browser-edit-{reset}"));
        ctrl(&mut h, egui::Key::G);
        field(&mut h, "Go to Row", if reset { "10" } else { "0" });
        browser(&mut h);
        add(&mut h, "Book1 / Sheet1 / D(Y): Shared current");
        click(&mut h, "OK");
        if reset {
            click(&mut h, "▶");
            click(&mut h, "Reset");
        } else {
            field(&mut h, "Go to Column", "B");
        }
        click(&mut h, "OK");
        assert_eq!(original_cell(&h, book), Some(if reset { (2, 9) } else { (1, 1) }));
    }
}
