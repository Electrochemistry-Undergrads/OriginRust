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
    show_window(&mut h, "Book1 - sample.csv");
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
fn browser_exclude_hides_the_exact_column_selects_the_nearest_position_and_keeps_original_ordinals() {
    for (excluded, retained) in [("B", "D"), ("D", "B")] {
        let (mut h, book) = fixture(&format!("goto-browser-exclude-{excluded}"));
        let before = project(&h).clone();
        let revision = h.state().doc.revision();
        ctrl(&mut h, egui::Key::G);
        field(&mut h, "Go to Row", "0");
        browser(&mut h);
        assert!(h.get_by_label("Exclude").accesskit_node().is_disabled());
        let removed = format!("Book1 / Sheet1 / {excluded}(Y): Shared current");
        click(&mut h, &removed);
        click(&mut h, "Exclude");
        assert!(h.query_by_label(&removed).is_none());
        assert!(
            h.query_by_label(&format!("Book1 / Sheet1 / {retained}(Y): Shared current")).is_some(),
            "duplicate long name stays visible"
        );
        click(&mut h, "Add");
        assert!(h.query_by_label("[Book1]Sheet1!C\"Potential\"").is_some(), "C follows B or precedes the final D");
        click(&mut h, "OK");
        assert_eq!(
            h.get_by_label("Go to Column").accesskit_node().value().as_deref(),
            Some("[Book1]Sheet1!C\"Potential\"")
        );
        click(&mut h, "OK");
        assert_eq!(original_cell(&h, book), Some((2, 5)), "C keeps original ordinal 3 after B is excluded");
        assert_eq!(project(&h), &before);
        assert_eq!(h.state().doc.revision(), revision);
    }
}

#[test]
fn browser_exclude_scope_changes_restore_columns_and_clear_the_upper_selection() {
    let (mut h, book) = fixture("goto-browser-exclude-scope");
    let before = project(&h).clone();
    ctrl(&mut h, egui::Key::G);
    browser(&mut h);
    click(&mut h, "Book1 / Sheet1 / B(Y): Shared current");
    click(&mut h, "Exclude");
    assert!(h.query_by_label("Book1 / Sheet1 / B(Y): Shared current").is_none());
    click(&mut h, "List Columns");
    click(&mut h, "in Current Book");
    assert!(h.query_by_label("Book1 / Sheet1 / B(Y): Shared current").is_some());
    assert!(h.get_by_label("Add").accesskit_node().is_disabled());
    assert!(h.get_by_label("Exclude").accesskit_node().is_disabled());
    click(&mut h, "Book1 / Sheet1 / C(Y): Potential");
    click(&mut h, "Exclude");
    click(&mut h, "List Columns");
    click(&mut h, "in Current Sheet");
    assert!(h.query_by_label("Book1 / Sheet1 / C(Y): Potential").is_some());
    assert!(h.get_by_label("Add").accesskit_node().is_disabled());
    click(&mut h, "Cancel");
    click(&mut h, "Cancel");
    assert_eq!(original_cell(&h, book), Some((2, 6)));
    assert_eq!(project(&h), &before);
}

#[test]
fn browser_exclude_is_reset_when_the_browser_is_cancelled_and_reopened() {
    let (mut h, book) = fixture("goto-browser-exclude-reopen");
    let before = project(&h).clone();
    ctrl(&mut h, egui::Key::G);
    browser(&mut h);
    click(&mut h, "Book1 / Sheet1 / B(Y): Shared current");
    click(&mut h, "Exclude");
    click(&mut h, "Cancel");
    browser(&mut h);
    assert!(h.query_by_label("Book1 / Sheet1 / B(Y): Shared current").is_some());
    assert!(h.get_by_label("Exclude").accesskit_node().is_disabled());
    click(&mut h, "Cancel");
    click(&mut h, "Cancel");
    assert_eq!(original_cell(&h, book), Some((2, 6)));
    assert_eq!(project(&h), &before);
}

#[test]
fn browser_exclude_preserves_an_already_added_range_and_its_original_destination() {
    let (mut h, book) = fixture("goto-browser-exclude-added");
    let before = project(&h).clone();
    ctrl(&mut h, egui::Key::G);
    field(&mut h, "Go to Row", "0");
    browser(&mut h);
    add(&mut h, "Book1 / Sheet1 / B(Y): Shared current");
    click(&mut h, "Exclude");
    assert!(h.query_by_label("Book1 / Sheet1 / B(Y): Shared current").is_none());
    assert!(h.query_by_label("[Book1]Sheet1!B\"Shared current\"").is_some());
    click(&mut h, "OK");
    assert_eq!(
        h.get_by_label("Go to Column").accesskit_node().value().as_deref(),
        Some("[Book1]Sheet1!B\"Shared current\"")
    );
    click(&mut h, "OK");
    assert_eq!(original_cell(&h, book), Some((1, 1)));
    assert_eq!(project(&h), &before);
}

#[test]
fn browser_exclude_keeps_block_add_disabled_across_an_excluded_column_gap() {
    let (mut h, book) = fixture("goto-browser-exclude-block");
    let before = project(&h).clone();
    ctrl(&mut h, egui::Key::G);
    browser(&mut h);
    click(&mut h, "Book1 / Sheet1 / B(Y): Shared current");
    click(&mut h, "Exclude");
    click(&mut h, "Book1 / Sheet1 / A(X): Time");
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    settle(&mut h);
    assert!(h.get_by_label("Add as One Block").accesskit_node().is_disabled());
    click(&mut h, "Add as One Block");
    assert!(h.get_by_label("OK").accesskit_node().is_disabled());
    click(&mut h, "Add");
    for reference in ["[Book1]Sheet1!A\"Time\"", "[Book1]Sheet1!C\"Potential\"", "[Book1]Sheet1!D\"Shared current\""] {
        assert!(h.query_by_label(reference).is_some());
    }
    assert!(h.query_by_label("[Book1]Sheet1!B\"Shared current\"").is_none());
    click(&mut h, "Cancel");
    click(&mut h, "Cancel");
    assert_eq!(original_cell(&h, book), Some((2, 6)));
    assert_eq!(project(&h), &before);
}

#[test]
fn browser_exclude_multiple_columns_selects_the_first_removed_position_in_either_click_order() {
    for reverse in [false, true] {
        let (mut h, book) = fixture(&format!("goto-browser-exclude-multi-{reverse}"));
        let before = project(&h).clone();
        ctrl(&mut h, egui::Key::G);
        field(&mut h, "Go to Row", "0");
        browser(&mut h);
        let a = "Book1 / Sheet1 / A(X): Time";
        let c = "Book1 / Sheet1 / C(Y): Potential";
        click(&mut h, if reverse { c } else { a });
        select_with(&mut h, if reverse { a } else { c }, egui::Modifiers::CTRL);
        click(&mut h, "Exclude");
        assert!(h.query_by_label(a).is_none());
        assert!(h.query_by_label(c).is_none());
        click(&mut h, "Add");
        assert!(
            h.query_by_label("[Book1]Sheet1!B\"Shared current\"").is_some(),
            "first removed row chooses B, reverse={reverse}"
        );
        click(&mut h, "OK");
        click(&mut h, "OK");
        assert_eq!(original_cell(&h, book), Some((1, 1)));
        assert_eq!(project(&h), &before);
    }
}

#[test]
fn browser_exclude_all_columns_leaves_no_selection_and_scope_change_restores_them() {
    let (mut h, book) = fixture("goto-browser-exclude-all");
    let before = project(&h).clone();
    ctrl(&mut h, egui::Key::G);
    browser(&mut h);
    click(&mut h, "Book1 / Sheet1 / A(X): Time");
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    settle(&mut h);
    click(&mut h, "Exclude");
    for row in [
        "Book1 / Sheet1 / A(X): Time",
        "Book1 / Sheet1 / B(Y): Shared current",
        "Book1 / Sheet1 / C(Y): Potential",
        "Book1 / Sheet1 / D(Y): Shared current",
    ] {
        assert!(h.query_by_label(row).is_none());
    }
    for label in ["Add", "Exclude", "OK"] {
        assert!(h.get_by_label(label).accesskit_node().is_disabled());
    }
    click(&mut h, "List Columns");
    click(&mut h, "in Current Book");
    assert!(h.query_by_label("Book1 / Sheet1 / A(X): Time").is_some());
    assert!(h.query_by_label("Book1 / Sheet1 / D(Y): Shared current").is_some());
    click(&mut h, "Cancel");
    click(&mut h, "Cancel");
    assert_eq!(original_cell(&h, book), Some((2, 6)));
    assert_eq!(project(&h), &before);
}

fn select_with(h: &mut Harness<'static, SheafApp>, label: &str, modifiers: egui::Modifiers) {
    pause(h);
    let at = h.get_by_label(label).rect().center();
    h.event(egui::Event::PointerMoved(at));
    h.step();
    for pressed in [true, false] {
        h.input_mut().modifiers = modifiers;
        h.event(egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed, modifiers });
        h.step();
    }
    h.input_mut().modifiers = egui::Modifiers::NONE;
    settle(h);
}

fn row_button(h: &mut Harness<'static, SheafApp>, label: &str) {
    h.get_all_by_label(label).find(|node| !node.accesskit_node().is_disabled()).unwrap().click();
    settle(h);
}

fn edit_rows(h: &mut Harness<'static, SheafApp>, reference: &str) {
    click(h, &format!("Edit rows for {reference}"));
    assert!(h.query_by_label("Entire Column(s)").is_some());
}

#[test]
fn browser_row_subset_transfers_without_changing_the_original_column_end_or_history() {
    let (mut h, book) = fixture("goto-browser-row-subset");
    let before = project(&h).clone();
    let revision = h.state().doc.revision();
    ctrl(&mut h, egui::Key::G);
    field(&mut h, "Go to Row", "0");
    browser(&mut h);
    add(&mut h, "Book1 / Sheet1 / C(Y): Potential");
    edit_rows(&mut h, "[Book1]Sheet1!C\"Potential\"");
    assert!(h.get_by_label("From").accesskit_node().is_disabled());
    assert!(h.get_by_label("To").accesskit_node().is_disabled());
    assert_eq!(h.get_by_label("From").accesskit_node().value().as_deref(), Some("1"));
    assert_eq!(h.get_by_label("To").accesskit_node().value().as_deref(), Some("6"));
    click(&mut h, "Entire Column(s)");
    field(&mut h, "From", "2");
    field(&mut h, "To", "4");
    row_button(&mut h, "OK");
    assert!(h.query_by_label("Entire Column(s)").is_none());
    assert!(h.query_by_label("[Book1]Sheet1!C\"Potential\"[2:4]").is_some());
    assert_eq!(original_cell(&h, book), Some((2, 6)));
    click(&mut h, "OK");
    assert_eq!(
        h.get_by_label("Go to Column").accesskit_node().value().as_deref(),
        Some("[Book1]Sheet1!C\"Potential\"[2:4]")
    );
    assert_eq!(original_cell(&h, book), Some((2, 6)));
    click(&mut h, "OK");
    assert_eq!(original_cell(&h, book), Some((2, 5)), "row subset does not redefine Go To Row 0");
    assert_eq!(project(&h), &before);
    assert_eq!(h.state().doc.revision(), revision);
}

#[test]
fn browser_row_editor_uses_the_sheet_end_even_for_a_shorter_selected_column() {
    let (mut h, book) = fixture("goto-browser-rows-sheet-end");
    let before = project(&h).clone();
    ctrl(&mut h, egui::Key::G);
    field(&mut h, "Go to Row", "0");
    browser(&mut h);
    add(&mut h, "Book1 / Sheet1 / B(Y): Shared current");
    edit_rows(&mut h, "[Book1]Sheet1!B\"Shared current\"");
    assert_eq!(h.get_by_label("To").accesskit_node().value().as_deref(), Some("6"), "B ends at 2, sheet at 6");
    click(&mut h, "Entire Column(s)");
    field(&mut h, "From", "3");
    field(&mut h, "To", "6");
    row_button(&mut h, "OK");
    assert!(h.query_by_label("[Book1]Sheet1!B\"Shared current\"[3:6]").is_some());
    click(&mut h, "OK");
    assert_eq!(
        h.get_by_label("Go to Column").accesskit_node().value().as_deref(),
        Some("[Book1]Sheet1!B\"Shared current\"[3:6]")
    );
    click(&mut h, "OK");
    assert_eq!(original_cell(&h, book), Some((1, 1)));
    assert_eq!(project(&h), &before);
}

#[test]
fn browser_row_editor_preserves_reversed_endpoints_as_origin_does() {
    let (mut h, book) = fixture("goto-browser-row-reversed");
    let before = project(&h).clone();
    ctrl(&mut h, egui::Key::G);
    browser(&mut h);
    add(&mut h, "Book1 / Sheet1 / C(Y): Potential");
    edit_rows(&mut h, "[Book1]Sheet1!C\"Potential\"");
    click(&mut h, "Entire Column(s)");
    field(&mut h, "From", "4");
    field(&mut h, "To", "2");
    row_button(&mut h, "OK");
    assert!(h.query_by_label("[Book1]Sheet1!C\"Potential\"[4:2]").is_some());
    edit_rows(&mut h, "[Book1]Sheet1!C\"Potential\"[4:2]");
    assert_eq!(h.get_by_label("From").accesskit_node().value().as_deref(), Some("4"));
    assert_eq!(h.get_by_label("To").accesskit_node().value().as_deref(), Some("2"));
    row_button(&mut h, "Cancel");
    click(&mut h, "OK");
    assert_eq!(
        h.get_by_label("Go to Column").accesskit_node().value().as_deref(),
        Some("[Book1]Sheet1!C\"Potential\"[4:2]")
    );
    click(&mut h, "Cancel");
    assert_eq!(original_cell(&h, book), Some((2, 6)));
    assert_eq!(project(&h), &before);
}

#[test]
fn browser_row_editor_entire_column_restores_the_reference_and_default_bounds() {
    let (mut h, book) = fixture("goto-browser-row-entire");
    let before = project(&h).clone();
    ctrl(&mut h, egui::Key::G);
    field(&mut h, "Go to Row", "0");
    browser(&mut h);
    add(&mut h, "Book1 / Sheet1 / C(Y): Potential");
    edit_rows(&mut h, "[Book1]Sheet1!C\"Potential\"");
    click(&mut h, "Entire Column(s)");
    field(&mut h, "From", "2");
    field(&mut h, "To", "4");
    row_button(&mut h, "OK");
    edit_rows(&mut h, "[Book1]Sheet1!C\"Potential\"[2:4]");
    click(&mut h, "Entire Column(s)");
    assert!(h.get_by_label("From").accesskit_node().is_disabled());
    assert!(h.get_by_label("To").accesskit_node().is_disabled());
    row_button(&mut h, "OK");
    assert!(h.query_by_label("[Book1]Sheet1!C\"Potential\"[2:4]").is_none());
    edit_rows(&mut h, "[Book1]Sheet1!C\"Potential\"");
    assert_eq!(h.get_by_label("From").accesskit_node().value().as_deref(), Some("1"));
    assert_eq!(h.get_by_label("To").accesskit_node().value().as_deref(), Some("6"));
    assert!(h.get_by_label("From").accesskit_node().is_disabled());
    row_button(&mut h, "Cancel");
    click(&mut h, "OK");
    assert_eq!(
        h.get_by_label("Go to Column").accesskit_node().value().as_deref(),
        Some("[Book1]Sheet1!C\"Potential\"")
    );
    click(&mut h, "OK");
    assert_eq!(original_cell(&h, book), Some((2, 5)));
    assert_eq!(project(&h), &before);
}

#[test]
fn browser_row_editor_enter_keeps_the_entire_column_and_respects_focused_cancel() {
    for cancel in [false, true] {
        let (mut h, book) = fixture(&format!("goto-browser-row-enter-{cancel}"));
        let before = project(&h).clone();
        ctrl(&mut h, egui::Key::G);
        browser(&mut h);
        add(&mut h, "Book1 / Sheet1 / C(Y): Potential");
        edit_rows(&mut h, "[Book1]Sheet1!C\"Potential\"");
        if cancel {
            click(&mut h, "Entire Column(s)");
            field(&mut h, "From", "2");
            field(&mut h, "To", "4");
            h.get_all_by_label("Cancel").find(|node| !node.accesskit_node().is_disabled()).unwrap().focus();
            settle(&mut h);
        }
        h.key_press(egui::Key::Enter);
        settle(&mut h);
        assert!(h.query_by_label("Entire Column(s)").is_none());
        assert!(h.query_by_label("List Columns").is_some());
        assert!(h.query_by_label("[Book1]Sheet1!C\"Potential\"").is_some());
        assert!(h.query_by_label("[Book1]Sheet1!C\"Potential\"[2:4]").is_none());
        assert!(h.query_by_label("[Book1]Sheet1!C\"Potential\"[1:6]").is_none(), "Enter must not toggle Entire");
        click(&mut h, "Cancel");
        click(&mut h, "Cancel");
        assert_eq!(original_cell(&h, book), Some((2, 6)));
        assert_eq!(project(&h), &before);
    }
}

#[test]
fn browser_block_rows_use_endpoint_references_and_survive_removing_a_preceding_range() {
    let (mut h, book) = fixture("goto-browser-block-rows");
    let before = project(&h).clone();
    ctrl(&mut h, egui::Key::G);
    field(&mut h, "Go to Row", "0");
    browser(&mut h);
    add(&mut h, "Book1 / Sheet1 / A(X): Time");
    click(&mut h, "Book1 / Sheet1 / B(Y): Shared current");
    select_with(&mut h, "Book1 / Sheet1 / D(Y): Shared current", egui::Modifiers::SHIFT);
    click(&mut h, "Add as One Block");
    edit_rows(&mut h, "[Book1]Sheet1!B\"Shared current\":D\"Shared current\"");
    click(&mut h, "Entire Column(s)");
    field(&mut h, "From", "2");
    field(&mut h, "To", "4");
    row_button(&mut h, "OK");
    let reference = "[Book1]Sheet1!B\"Shared current\"[2]:D\"Shared current\"[4]";
    assert!(h.query_by_label(reference).is_some());
    click(&mut h, "[Book1]Sheet1!A\"Time\"");
    click(&mut h, "Remove");
    assert!(h.query_by_label(reference).is_some());
    edit_rows(&mut h, reference);
    assert_eq!(h.get_by_label("From").accesskit_node().value().as_deref(), Some("2"));
    assert_eq!(h.get_by_label("To").accesskit_node().value().as_deref(), Some("4"));
    row_button(&mut h, "Cancel");
    click(&mut h, "OK");
    assert_eq!(h.get_by_label("Go to Column").accesskit_node().value().as_deref(), Some(reference));
    click(&mut h, "OK");
    assert_eq!(original_cell(&h, book), Some((3, 2)));
    assert_eq!(project(&h), &before);
}

#[test]
fn browser_row_editor_normalizes_native_integer_prefixes_and_keeps_single_row_intervals() {
    for (index, (from, to, first, last)) in
        [("1.5", "6", 1, 6), ("2.9", "6.9", 2, 6), ("2+1", "6", 2, 6), ("2abc", "2e1", 2, 2)].into_iter().enumerate()
    {
        let (mut h, book) = fixture(&format!("goto-browser-row-prefix-{index}"));
        let before = project(&h).clone();
        ctrl(&mut h, egui::Key::G);
        browser(&mut h);
        add(&mut h, "Book1 / Sheet1 / C(Y): Potential");
        edit_rows(&mut h, "[Book1]Sheet1!C\"Potential\"");
        click(&mut h, "Entire Column(s)");
        field(&mut h, "From", from);
        field(&mut h, "To", to);
        row_button(&mut h, "OK");
        let reference = format!("[Book1]Sheet1!C\"Potential\"[{first}:{last}]");
        edit_rows(&mut h, &reference);
        assert_eq!(h.get_by_label("From").accesskit_node().value().as_deref(), Some(first.to_string().as_str()));
        assert_eq!(h.get_by_label("To").accesskit_node().value().as_deref(), Some(last.to_string().as_str()));
        row_button(&mut h, "Cancel");
        click(&mut h, "OK");
        assert_eq!(h.get_by_label("Go to Column").accesskit_node().value().as_deref(), Some(reference.as_str()));
        click(&mut h, "Cancel");
        assert_eq!(original_cell(&h, book), Some((2, 6)));
        assert_eq!(project(&h), &before);
    }
}

#[test]
fn browser_row_editor_refuses_out_of_bounds_or_nonnumeric_input_without_closing_or_changing_the_range() {
    let (mut h, book) = fixture("goto-browser-row-invalid");
    let before = project(&h).clone();
    ctrl(&mut h, egui::Key::G);
    browser(&mut h);
    add(&mut h, "Book1 / Sheet1 / C(Y): Potential");
    edit_rows(&mut h, "[Book1]Sheet1!C\"Potential\"");
    click(&mut h, "Entire Column(s)");
    for (from, to) in [("0", "6"), ("1", "0"), ("1", "7"), ("abc", "6")] {
        field(&mut h, "From", from);
        field(&mut h, "To", to);
        row_button(&mut h, "OK");
        assert!(h.query_by_label("Entire Column(s)").is_some());
        assert!(h.query_by_label("[Book1]Sheet1!C\"Potential\"").is_some());
        assert_eq!(original_cell(&h, book), Some((2, 6)));
        assert_eq!(project(&h), &before);
    }
    field(&mut h, "From", "4");
    field(&mut h, "To", "2");
    row_button(&mut h, "OK");
    assert!(h.query_by_label("[Book1]Sheet1!C\"Potential\"[4:2]").is_some());
    click(&mut h, "Cancel");
    click(&mut h, "Cancel");
    assert_eq!(project(&h), &before);
}

#[test]
fn browser_row_editor_allows_entire_empty_columns_with_native_zero_end_and_default_enter() {
    let (mut h, book) = fixture("goto-browser-row-empty");
    h.state_mut().doc.apply("Empty browser fixture", |project| {
        for column in &mut project.workbook_mut(book).unwrap().sheets[0].columns {
            column.data = ColumnData::numeric(Vec::new());
        }
    });
    settle(&mut h);
    let before = project(&h).clone();
    ctrl(&mut h, egui::Key::G);
    browser(&mut h);
    add(&mut h, "Book1 / Sheet1 / C(Y): Potential");
    edit_rows(&mut h, "[Book1]Sheet1!C\"Potential\"");
    assert_eq!(h.get_by_label("From").accesskit_node().value().as_deref(), Some("1"));
    assert_eq!(h.get_by_label("To").accesskit_node().value().as_deref(), Some("0"));
    assert!(h.get_by_label("To").accesskit_node().is_disabled());
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    assert!(h.query_by_label("Entire Column(s)").is_none());
    assert!(h.query_by_label("[Book1]Sheet1!C\"Potential\"").is_some());
    click(&mut h, "Cancel");
    click(&mut h, "Cancel");
    assert_eq!(project(&h), &before);
}

#[test]
fn browser_row_editor_cancel_and_escape_keep_the_existing_range_and_both_parents() {
    for escape in [false, true] {
        let (mut h, book) = fixture(&format!("goto-browser-row-cancel-{escape}"));
        let before = project(&h).clone();
        ctrl(&mut h, egui::Key::G);
        browser(&mut h);
        add(&mut h, "Book1 / Sheet1 / C(Y): Potential");
        edit_rows(&mut h, "[Book1]Sheet1!C\"Potential\"");
        click(&mut h, "Entire Column(s)");
        field(&mut h, "From", "2");
        field(&mut h, "To", "4");
        row_button(&mut h, "OK");
        edit_rows(&mut h, "[Book1]Sheet1!C\"Potential\"[2:4]");
        assert_eq!(h.get_by_label("From").accesskit_node().value().as_deref(), Some("2"));
        assert_eq!(h.get_by_label("To").accesskit_node().value().as_deref(), Some("4"));
        field(&mut h, "From", "1");
        field(&mut h, "To", "3");
        if escape {
            h.key_press(egui::Key::Escape);
            settle(&mut h);
        } else {
            row_button(&mut h, "Cancel");
        }
        assert!(h.query_by_label("Entire Column(s)").is_none());
        assert!(h.query_by_label("List Columns").is_some());
        assert!(h.query_by_label("[Book1]Sheet1!C\"Potential\"[2:4]").is_some());
        assert!(h.state().go_to.is_some());
        assert_eq!(original_cell(&h, book), Some((2, 6)));
        click(&mut h, "Cancel");
        click(&mut h, "Cancel");
        assert_eq!(project(&h), &before);
    }
}

#[test]
fn browser_shift_range_adds_individual_columns_or_one_block_without_editing_the_document() {
    for block in [false, true] {
        let (mut h, book) = fixture(&format!("goto-browser-shift-{block}"));
        let before = project(&h).clone();
        let revision = h.state().doc.revision();
        ctrl(&mut h, egui::Key::G);
        field(&mut h, "Go to Row", "0");
        browser(&mut h);
        click(&mut h, "Book1 / Sheet1 / B(Y): Shared current");
        for _ in 0..2 {
            h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::ArrowDown);
            settle(&mut h);
        }
        assert!(!h.get_by_label("Add as One Block").accesskit_node().is_disabled());
        click(&mut h, if block { "Add as One Block" } else { "Add" });
        if block {
            assert!(h.query_by_label("[Book1]Sheet1!B\"Shared current\":D\"Shared current\"").is_some());
            assert!(h.query_by_label("[Book1]Sheet1!C\"Potential\"").is_none());
        } else {
            for reference in [
                "[Book1]Sheet1!B\"Shared current\"",
                "[Book1]Sheet1!C\"Potential\"",
                "[Book1]Sheet1!D\"Shared current\"",
            ] {
                assert!(h.query_by_label(reference).is_some(), "{reference}");
            }
        }
        click(&mut h, "OK");
        let expected = if block {
            "[Book1]Sheet1!B\"Shared current\":D\"Shared current\""
        } else {
            "[Book1]Sheet1!(B\"Shared current\",C\"Potential\",D\"Shared current\")"
        };
        assert_eq!(h.get_by_label("Go to Column").accesskit_node().value().as_deref(), Some(expected));
        assert_eq!(original_cell(&h, book), Some((2, 6)));
        click(&mut h, "OK");
        assert_eq!(original_cell(&h, book), Some((3, 2)));
        assert_eq!(project(&h), &before);
        assert_eq!(h.state().doc.revision(), revision);
    }
}

#[test]
fn browser_ctrl_selection_requires_a_contiguous_same_sheet_block_and_toggles_columns() {
    let (mut h, book) = fixture("goto-browser-ctrl");
    let before = project(&h).clone();
    ctrl(&mut h, egui::Key::G);
    field(&mut h, "Go to Row", "0");
    browser(&mut h);
    click(&mut h, "Book1 / Sheet1 / A(X): Time");
    select_with(&mut h, "Book1 / Sheet1 / C(Y): Potential", egui::Modifiers::CTRL);
    assert!(h.get_by_label("Add as One Block").accesskit_node().is_disabled());
    click(&mut h, "Add as One Block");
    assert!(h.get_by_label("OK").accesskit_node().is_disabled());
    click(&mut h, "Add");
    assert!(h.query_by_label("[Book1]Sheet1!A\"Time\"").is_some());
    assert!(h.query_by_label("[Book1]Sheet1!C\"Potential\"").is_some());
    assert!(h.query_by_label("[Book1]Sheet1!B\"Shared current\"").is_none());
    select_with(&mut h, "Book1 / Sheet1 / A(X): Time", egui::Modifiers::CTRL);
    select_with(&mut h, "Book1 / Sheet1 / B(Y): Shared current", egui::Modifiers::CTRL);
    assert!(!h.get_by_label("Add as One Block").accesskit_node().is_disabled());
    click(&mut h, "Add as One Block");
    assert!(h.query_by_label("[Book1]Sheet1!B\"Shared current\":C\"Potential\"").is_some());
    click(&mut h, "Cancel");
    assert_eq!(original_cell(&h, book), Some((2, 6)));
    assert_eq!(project(&h), &before);
}

#[test]
fn browser_select_all_adds_one_range_and_remove_deletes_the_entire_group() {
    let (mut h, book) = fixture("goto-browser-all-remove");
    let before = project(&h).clone();
    ctrl(&mut h, egui::Key::G);
    browser(&mut h);
    click(&mut h, "Book1 / Sheet1 / A(X): Time");
    h.key_press_modifiers(egui::Modifiers::CTRL, egui::Key::A);
    settle(&mut h);
    click(&mut h, "Add as One Block");
    click(&mut h, "[Book1]Sheet1!A\"Time\":D\"Shared current\"");
    click(&mut h, "Remove");
    assert!(h.get_by_label("OK").accesskit_node().is_disabled());
    assert!(h.query_by_label("[Book1]Sheet1!A\"Time\":D\"Shared current\"").is_none());
    click(&mut h, "Cancel");
    click(&mut h, "Cancel");
    assert_eq!(original_cell(&h, book), Some((2, 6)));
    assert_eq!(project(&h), &before);
}

#[test]
fn browser_block_and_singleton_preserve_grouped_references_and_the_native_last_column_rule() {
    for append in [false, true] {
        let (mut h, book) = fixture(&format!("goto-browser-block-list-{append}"));
        let before = project(&h).clone();
        ctrl(&mut h, egui::Key::G);
        field(&mut h, "Go to Row", "0");
        browser(&mut h);
        click(&mut h, "Book1 / Sheet1 / B(Y): Shared current");
        select_with(&mut h, "Book1 / Sheet1 / C(Y): Potential", egui::Modifiers::SHIFT);
        click(&mut h, "Add as One Block");
        if append {
            add(&mut h, "Book1 / Sheet1 / D(Y): Shared current");
            assert!(h.query_by_label("[Book1]Sheet1!B\"Shared current\":C\"Potential\"").is_some());
            assert!(h.query_by_label("[Book1]Sheet1!D\"Shared current\"").is_some());
        }
        click(&mut h, "OK");
        let expected = if append {
            "[Book1]Sheet1!(B\"Shared current\":C\"Potential\",D\"Shared current\")"
        } else {
            "[Book1]Sheet1!B\"Shared current\":C\"Potential\""
        };
        assert_eq!(h.get_by_label("Go to Column").accesskit_node().value().as_deref(), Some(expected));
        click(&mut h, "OK");
        assert_eq!(original_cell(&h, book), Some((3, 2)), "a B:C range still targets original D3");
        assert_eq!(project(&h), &before);
    }
}

#[test]
fn browser_shift_click_across_books_cannot_be_added_as_a_block_and_scope_resets_selection() {
    let (mut h, book) = fixture("goto-browser-mixed");
    let before = project(&h).clone();
    ctrl(&mut h, egui::Key::G);
    browser(&mut h);
    click(&mut h, "List Columns");
    click(&mut h, "in Current Project");
    click(&mut h, "Book1 / Sheet1 / D(Y): Shared current");
    select_with(&mut h, "Book2 / Sheet1 / A(X): Other X", egui::Modifiers::SHIFT);
    assert!(h.get_by_label("Add as One Block").accesskit_node().is_disabled());
    click(&mut h, "Add");
    assert!(h.query_by_label("[Book1]Sheet1!D\"Shared current\"").is_some());
    assert!(h.query_by_label("[Book2]Sheet1!A\"Other X\"").is_some());
    click(&mut h, "List Columns");
    click(&mut h, "in Current Sheet");
    assert!(h.get_by_label("Add").accesskit_node().is_disabled());
    assert!(h.query_by_label("[Book2]Sheet1!A\"Other X\"").is_some(), "added ranges survive scope changes");
    click(&mut h, "Cancel");
    click(&mut h, "Cancel");
    assert_eq!(original_cell(&h, book), Some((2, 6)));
    assert_eq!(project(&h), &before);
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
