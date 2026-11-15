use super::*;

const QUERY: &str = "Find what (Enter -- for missing value):";

fn field(h: &mut Harness<'static, SheafApp>, label: &str, text: &str) {
    h.get_by_label(label).focus();
    settle(h);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.event(egui::Event::Text(text.into()));
    if text.is_empty() {
        h.key_press(egui::Key::Backspace);
    }
    settle(h);
}

#[test]
fn find_seeds_the_selected_display_and_numeric_mode_matches_values_not_substrings() {
    let mut h = start(vec![data_file("find-numeric-default")]);
    show_window(&mut h, "Book1 - sample");
    click_text(&mut h, "4.5");
    let before = project(&h).clone();
    let revision = h.state().doc.revision();
    ctrl(&mut h, egui::Key::F);
    assert_eq!(h.get_by_label(QUERY).accesskit_node().value().as_deref(), Some("4.5"));
    assert_eq!(h.get_by_label("Tolerance:").accesskit_node().value().as_deref(), Some("1e-8"));
    assert!(h.query_by_label("Use absolute value").is_some());
    assert!(h.query_by_label("Match case").is_none());
    field(&mut h, QUERY, "0.5");
    click(&mut h, "Find Next");
    assert_eq!(book_ui(&h).active_cell(), Some((2, 3)), "0.5, rather than the displayed substring in 30.5");
    click(&mut h, "Close");
    assert_eq!(project(&h), &before);
    assert_eq!(h.state().doc.revision(), revision);
}

#[test]
fn numeric_replace_absolute_values_keeps_sign_and_is_one_undo_step() {
    let mut h = start(vec![data_file("replace-absolute-sign")]);
    let book = project(&h).workbooks().next().unwrap().id;
    h.state_mut().doc.apply("Signed data", |p| {
        p.workbook_mut(book).unwrap().sheets[0].columns[1].data = ColumnData::numeric(vec![-2.0, 2.0, -3.0, 0.0]);
    });
    settle(&mut h);
    show_window(&mut h, "Book1 - sample");
    let before = project(&h).clone();
    ctrl(&mut h, egui::Key::H);
    field(&mut h, QUERY, "2");
    field(&mut h, "Replace with:", "7");
    click(&mut h, "Use absolute value");
    click(&mut h, "Keep sign if condition true");
    click(&mut h, "Replace All");
    assert_eq!(column(&h, "B"), [-7.0, 7.0, -3.0, 0.0]);
    assert_eq!(h.state().doc.undo_label(), Some("Replace all"));
    click(&mut h, "Close");
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::Undo, &ctx);
    assert_eq!(project(&h), &before);
}

#[test]
fn find_and_replace_tabs_keep_inputs_and_wildcards_replace_whole_matches() {
    let mut h = start(vec![data_file("replace-wildcard-tabs")]);
    let book = project(&h).workbooks().next().unwrap().id;
    h.state_mut().doc.apply("Text data", |p| {
        p.workbook_mut(book).unwrap().sheets[0].columns[1].data =
            ColumnData::text(["abXXc", "xabc", "abc", "z"].map(str::to_string).to_vec());
    });
    settle(&mut h);
    show_window(&mut h, "Book1 - sample");
    ctrl(&mut h, egui::Key::H);
    click(&mut h, "String");
    field(&mut h, QUERY, "a*c");
    field(&mut h, "Replace with:", "Z");
    click(&mut h, "Use wildcards");
    click(&mut h, "Find");
    assert!(h.query_by_label("Replace with:").is_none());
    click(&mut h, "Replace");
    assert_eq!(h.get_by_label(QUERY).accesskit_node().value().as_deref(), Some("a*c"));
    assert_eq!(h.get_by_label("Replace with:").accesskit_node().value().as_deref(), Some("Z"));
    click(&mut h, "Replace All");
    let text = sheet(&h).columns[1].text().unwrap().iter().map(String::as_str).collect::<Vec<_>>();
    assert_eq!(text, ["Z", "xabc", "Z", "z"]);
}

#[test]
fn search_up_finds_the_previous_numeric_match_and_invalid_input_preserves_state() {
    let mut h = start(vec![data_file("find-search-up")]);
    let book = project(&h).workbooks().next().unwrap().id;
    h.state_mut().doc.apply("Repeated data", |p| {
        p.workbook_mut(book).unwrap().sheets[0].columns[1].data = ColumnData::numeric(vec![2.0, 5.0, 2.0, 8.0]);
    });
    settle(&mut h);
    show_window(&mut h, "Book1 - sample");
    let source = sheet(&h).clone();
    h.state_mut().books.get_mut(&book).unwrap().select_cells(&source, (1, 1), (1, 1));
    settle(&mut h);
    let before = project(&h).clone();
    let revision = h.state().doc.revision();
    ctrl(&mut h, egui::Key::F);
    field(&mut h, QUERY, "2");
    click(&mut h, "Search up");
    click(&mut h, "Find Next");
    assert_eq!(book_ui(&h).active_cell(), Some((1, 0)));
    for (query, tolerance) in [("Alpha", "1e-8"), ("2", "NaN"), ("2", "-1")] {
        field(&mut h, QUERY, query);
        field(&mut h, "Tolerance:", tolerance);
        h.key_press(egui::Key::Enter);
        settle(&mut h);
        assert_eq!(book_ui(&h).active_cell(), Some((1, 0)));
        assert!(h.state().find.is_some());
    }
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    assert!(h.state().find.is_none());
    assert_eq!(project(&h), &before);
    assert_eq!(h.state().doc.revision(), revision);
}
