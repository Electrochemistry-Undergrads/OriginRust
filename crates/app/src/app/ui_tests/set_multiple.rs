//! Set Multiple Columns Values (T53): Ctrl+Shift+Q with several columns selected.

use super::*;

/// A sheet of five columns, A to E: the X column 1 to 4 and four Y columns.
fn five_columns(tag: &str) -> PathBuf {
    csv_file(tag, "multi", "x,b,c,d,e\n1,10,20,30,40\n2,11,21,31,41\n3,12,22,32,42\n4,13,23,33,43\n")
}

/// Selects columns by clicking their heads: the first plainly, the others with Ctrl.
fn select(h: &mut Harness<'static, SheafApp>, heads: &[&str]) {
    click_text(h, heads[0]);
    for head in &heads[1..] {
        click_text_with(h, head, egui::Modifiers::COMMAND);
    }
}

fn ctrl_shift_q(h: &mut Harness<'static, SheafApp>) {
    h.key_press_modifiers(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::Q);
    settle(h);
}

fn open_book(tag: &str) -> Harness<'static, SheafApp> {
    let mut h = start(vec![five_columns(tag)]);
    show_window(&mut h, "Book1 - multi");
    h
}

fn type_formula(h: &mut Harness<'static, SheafApp>, text: &str) {
    h.event(egui::Event::Text(text.to_string()));
    settle(h);
}

#[test]
fn ctrl_shift_q_with_b_to_d_selected_sets_all_three_columns_in_one_undo_step() {
    let mut h = open_book("multi-basic");
    let before: Vec<Vec<f64>> = ["B", "C", "D"].iter().map(|c| column(&h, c)).collect();
    select(&mut h, &["B(Y1)", "C(Y1)", "D(Y1)"]);
    ctrl_shift_q(&mut h);
    assert!(h.state().set_values.is_some(), "the window opened");
    // Origin's window (expression-language 7.1): "wcol(j) =" and the columns as positions 2 to 4.
    assert!(has_label(&h, "wcol(j) ="), "the label is Origin's");
    assert!(has_label(&h, "Col (j):  From"), "the range of j is shown");
    type_formula(&mut h, "col(A)*j");
    click(&mut h, "OK");
    assert!(h.state().set_values.is_none(), "OK closes the window");

    assert_eq!(column(&h, "B"), [2.0, 4.0, 6.0, 8.0], "B is the second column of the sheet");
    assert_eq!(column(&h, "C"), [3.0, 6.0, 9.0, 12.0]);
    assert_eq!(column(&h, "D"), [4.0, 8.0, 12.0, 16.0]);
    assert_eq!(column(&h, "E"), [40.0, 41.0, 42.0, 43.0], "E is not selected");
    for short in ["B", "C", "D"] {
        let c = sheet(&h).column_by_name(short).unwrap();
        assert_eq!(c.formula.as_deref(), Some("col(A)*j"), "the formula is stored on {short}");
    }
    assert!(sheet(&h).column_by_name("E").unwrap().formula.is_none());

    // One undo step restores all three; redo sets them again.
    assert_eq!(h.state().doc.undo_label(), Some("Set multiple columns values"));
    menu(&mut h, "Edit", "Undo Set multiple columns values");
    for (short, old) in ["B", "C", "D"].iter().zip(&before) {
        assert_eq!(&column(&h, short), old, "undo restores {short}");
        assert!(sheet(&h).column_by_name(short).unwrap().formula.is_none());
    }
    menu(&mut h, "Edit", "Redo Set multiple columns values");
    assert_eq!(column(&h, "D"), [4.0, 8.0, 12.0, 16.0]);
}

#[test]
fn both_menus_list_set_multiple_columns_values_enabled_for_two_or_more_columns() {
    let mut h = open_book("multi-menu");
    let item = "Set Multiple Columns Values...";
    // With no column, or one, it is greyed in the Column menu and in the context menu.
    click_text(&mut h, "B(Y1)");
    menu_entry(&h, "Column").click();
    settle(&mut h);
    assert!(menu_entry(&h, item).accesskit_node().is_disabled(), "one column: greyed");
    assert!(!menu_entry(&h, "Set Column Values...").accesskit_node().is_disabled());
    h.key_press(egui::Key::Escape);
    settle(&mut h);

    select(&mut h, &["B(Y1)", "D(Y1)"]);
    menu_entry(&h, "Column").click();
    settle(&mut h);
    assert!(!menu_entry(&h, item).accesskit_node().is_disabled(), "two columns: enabled in the Column menu");
    assert!(menu_entry(&h, "Set Column Values...").accesskit_node().is_disabled(), "the single window needs one");
    h.key_press(egui::Key::Escape);
    settle(&mut h);

    right_click_text(&mut h, "B(Y1)");
    assert!(!menu_entry(&h, item).accesskit_node().is_disabled(), "two columns: enabled in the context menu");
    click_containing(&mut h, item);
    assert!(h.state().set_values.is_some());
    assert!(has_label(&h, "wcol(j) ="), "columns that are not side by side open the same window");
}

#[test]
fn ctrl_shift_q_with_one_column_selected_opens_nothing() {
    let mut h = open_book("multi-one");
    click_text(&mut h, "B(Y1)");
    ctrl_shift_q(&mut h);
    assert!(h.state().set_values.is_none());
}

#[test]
fn the_single_column_window_is_unchanged_by_the_multiple_mode() {
    let mut h = open_book("multi-single");
    click_text(&mut h, "C(Y1)");
    ctrl(&mut h, egui::Key::Q);
    assert!(h.state().set_values.is_some());
    assert!(has_label(&h, "Col(C)="));
    assert!(has_label(&h, "|<") && has_label(&h, ">|"), "the column navigation is there");
    assert!(!has_label_containing(&h, "Set Multiple"), "the window is not the multiple one");
    type_formula(&mut h, "col(B)*2");
    click(&mut h, "OK");
    assert_eq!(column(&h, "C"), [20.0, 22.0, 24.0, 26.0]);
    assert_eq!(sheet(&h).column_by_name("D").unwrap().formula, None);
    assert_eq!(h.state().doc.undo_label(), Some("Set column values"));
}

#[test]
fn the_multiple_window_has_no_column_navigation_and_offers_j() {
    let mut h = open_book("multi-window");
    select(&mut h, &["B(Y1)", "C(Y1)"]);
    ctrl_shift_q(&mut h);
    // As in Origin, the column buttons are there but greyed.
    for b in ["|<", "<", ">", ">|"] {
        assert!(h.get_by_label(b).accesskit_node().is_disabled(), "{b} is greyed");
    }
    // The Variables menu lists j, with the sheet position as its hint.
    h.get_by_label("Variables").click();
    settle(&mut h);
    assert!(has_label(&h, "j"));
}

#[test]
fn a_selection_that_is_not_adjacent_is_set_in_sheet_order_with_j_as_the_sheet_position() {
    let mut h = open_book("multi-gap");
    // D and B, clicked in this order: C is skipped.
    select(&mut h, &["D(Y1)", "B(Y1)"]);
    ctrl_shift_q(&mut h);
    type_formula(&mut h, "wcol(j-1)*2");
    click(&mut h, "OK");
    // B reads A (column 1) and D reads C (column 3); C and E stay as they were.
    assert_eq!(column(&h, "B"), [2.0, 4.0, 6.0, 8.0]);
    assert_eq!(column(&h, "D"), [40.0, 42.0, 44.0, 46.0]);
    assert_eq!(column(&h, "C"), [20.0, 21.0, 22.0, 23.0]);
    assert_eq!(column(&h, "E"), [40.0, 41.0, 42.0, 43.0]);
}

#[test]
fn auto_recalculation_follows_a_change_in_a_for_every_selected_column() {
    let mut h = open_book("multi-auto");
    select(&mut h, &["B(Y1)", "C(Y1)", "D(Y1)"]);
    ctrl_shift_q(&mut h);
    type_formula(&mut h, "col(A)*j");
    choose_recalc(&mut h, "Manual", "Auto");
    click(&mut h, "OK");
    assert_eq!(column(&h, "C"), [3.0, 6.0, 9.0, 12.0]);
    for short in ["B", "C", "D"] {
        assert_eq!(sheet(&h).column_by_name(short).unwrap().recalc, sheaf_core::RecalcMode::Auto);
    }

    // Changing a cell of A recalculates all three, in the same undo step as the edit.
    edit_cell(&mut h, "A", 1, "10");
    assert_eq!(column(&h, "B"), [2.0, 20.0, 6.0, 8.0]);
    assert_eq!(column(&h, "C"), [3.0, 30.0, 9.0, 12.0]);
    assert_eq!(column(&h, "D"), [4.0, 40.0, 12.0, 16.0]);
    menu(&mut h, "Edit", "Undo Edit cell");
    assert_eq!(column(&h, "D"), [4.0, 8.0, 12.0, 16.0]);
}

#[test]
fn a_formula_with_a_problem_keeps_the_multiple_window_open_and_changes_nothing() {
    let mut h = open_book("multi-error");
    select(&mut h, &["B(Y1)", "C(Y1)"]);
    ctrl_shift_q(&mut h);
    type_formula(&mut h, "col(Nope)*j");
    click(&mut h, "OK");
    assert!(h.state().set_values.is_some());
    assert_eq!(column(&h, "B"), [10.0, 11.0, 12.0, 13.0]);
    assert_eq!(column(&h, "C"), [20.0, 21.0, 22.0, 23.0]);
    assert_ne!(h.state().doc.undo_label(), Some("Set multiple columns values"));
}

#[test]
fn reopening_on_columns_that_share_a_formula_shows_it_and_a_mixed_selection_starts_empty() {
    let mut h = open_book("multi-reopen");
    select(&mut h, &["B(Y1)", "C(Y1)"]);
    ctrl_shift_q(&mut h);
    type_formula(&mut h, "col(A)+j");
    click(&mut h, "OK");

    // B and C share the formula: the window opens with it, Apply grey.
    select(&mut h, &["B(Y1)", "C(Y1)"]);
    ctrl_shift_q(&mut h);
    assert!(h.get_by_label("Apply").accesskit_node().is_disabled());
    assert_eq!(h.state().set_values.as_ref().unwrap().formula(), "col(A)+j");
    click(&mut h, "Cancel");

    // C and D differ (D has none): the formula box starts empty.
    select(&mut h, &["C(Y1)", "D(Y1)"]);
    ctrl_shift_q(&mut h);
    assert_eq!(h.state().set_values.as_ref().unwrap().formula(), "");
}

/// Origin's "Col (j): From .. To" row: widening the range sets the columns at those sheet positions.
#[test]
fn the_range_of_j_can_be_widened_to_columns_that_were_not_selected() {
    let mut h = open_book("multi-range");
    select(&mut h, &["B(Y1)", "C(Y1)"]);
    ctrl_shift_q(&mut h);
    type_formula(&mut h, "j");
    // The text fields: Row From, Row To, Col (j) From, Col (j) To.
    h.query_all_by_role(egui::accesskit::Role::TextInput).nth(3).expect("the To field of j").focus();
    settle(&mut h);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.event(egui::Event::Text("4".into()));
    settle(&mut h);
    click(&mut h, "OK");
    assert!(h.state().set_values.is_none(), "OK closes the window");
    for (name, j) in [("B", 2.0), ("C", 3.0), ("D", 4.0)] {
        assert_eq!(column(&h, name), [j; 4], "column {name} holds its position");
    }
    assert_eq!(column(&h, "E"), [40.0, 41.0, 42.0, 43.0], "E is outside the range");
}
