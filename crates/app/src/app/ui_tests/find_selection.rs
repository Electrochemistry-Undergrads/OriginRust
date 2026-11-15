use super::*;

const QUERY: &str = "Find what (Enter -- for missing value):";
const RANGE: &str = "Find/Replace in selected range";

fn field(h: &mut Harness<'static, SheafApp>, label: &str, text: &str) {
    h.get_by_label(label).focus();
    settle(h);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.event(egui::Event::Text(text.into()));
    settle(h);
}

fn selected_book(name: &str) -> (Harness<'static, SheafApp>, WindowId) {
    let mut h = start(vec![data_file(name)]);
    let book = project(&h).workbooks().next().unwrap().id;
    h.state_mut().doc.apply("Selection fixture", |p| {
        let s = &mut p.workbook_mut(book).unwrap().sheets[0];
        for (column, data) in s.columns.iter_mut().zip([[1.0, 2.0, 3.0], [4.0, 5.0, 6.0], [5.0, 5.0, 5.0]]) {
            column.data = ColumnData::numeric(data.to_vec());
        }
    });
    settle(&mut h);
    show_window(&mut h, "Book1 - sample");
    let source = sheet(&h).clone();
    h.state_mut().books.get_mut(&book).unwrap().select_cells(&source, (1, 1), (1, 2));
    settle(&mut h);
    (h, book)
}

#[test]
fn find_selection_survives_hits_and_an_unrestricted_hit_removes_the_range_option() {
    let (mut h, _) = selected_book("find-selected-range");
    let before = project(&h).clone();
    let revision = h.state().doc.revision();
    ctrl(&mut h, egui::Key::F);
    assert_eq!(h.get_by_label(RANGE).accesskit_node().toggled(), Some(egui::accesskit::Toggled::True));
    // Toggling before searching preserves the original rectangle.
    click(&mut h, RANGE);
    click(&mut h, RANGE);
    for (query, expected) in [("6", (1, 2)), ("5", (1, 1)), ("6", (1, 2)), ("5", (1, 1))] {
        field(&mut h, QUERY, query);
        click(&mut h, "Find Next");
        assert_eq!(book_ui(&h).active_cell(), Some(expected));
        assert!(h.query_by_label(RANGE).is_some());
    }
    click(&mut h, RANGE);
    click(&mut h, "Find Next");
    assert_eq!(book_ui(&h).active_cell(), Some((2, 0)), "unchecked search reaches C1");
    assert!(h.query_by_label(RANGE).is_none());
    click(&mut h, "Close");
    assert_eq!(project(&h), &before);
    assert_eq!(h.state().doc.revision(), revision);
}

#[test]
fn replace_all_changes_only_the_opening_rectangle_in_one_undo_step() {
    let (mut h, book) = selected_book("replace-selected-range");
    h.state_mut().doc.apply("Repeated values", |p| {
        p.workbook_mut(book).unwrap().sheets[0].columns[1].data = ColumnData::numeric(vec![5.0; 3]);
    });
    settle(&mut h);
    let before = project(&h).clone();
    ctrl(&mut h, egui::Key::H);
    field(&mut h, QUERY, "5");
    field(&mut h, "Replace with:", "9");
    click(&mut h, "Replace All");
    assert_eq!(column(&h, "B"), [5.0, 9.0, 9.0]);
    assert_eq!(column(&h, "C"), [5.0; 3]);
    assert_eq!(h.state().doc.undo_label(), Some("Replace all"));
    click(&mut h, "Close");
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::Undo, &ctx);
    assert_eq!(project(&h), &before);
}

#[test]
fn string_find_and_single_replace_keep_the_opening_range_across_tabs() {
    let (mut h, book) = selected_book("find-string-selected-range");
    h.state_mut().doc.apply("Text fixture", |p| {
        for c in &mut p.workbook_mut(book).unwrap().sheets[0].columns[1..] {
            c.data = ColumnData::text(vec!["abc".into(); 3]);
        }
    });
    settle(&mut h);
    ctrl(&mut h, egui::Key::F);
    click(&mut h, "String");
    field(&mut h, QUERY, "b");
    click(&mut h, "Find Next");
    assert_eq!(book_ui(&h).active_cell(), Some((1, 2)));
    click(&mut h, "Find Next");
    assert_eq!(book_ui(&h).active_cell(), Some((1, 1)), "substring path wraps inside B2:B3");
    click(&mut h, "Replace");
    field(&mut h, "Replace with:", "X");
    h.get_by_role_and_label(egui::accesskit::Role::Button, "Replace").click();
    settle(&mut h);
    assert_eq!(book_ui(&h).active_cell(), Some((1, 2)));
    click(&mut h, "Replace All");
    let texts = |c: usize| sheet(&h).columns[c].text().unwrap().iter().map(String::as_str).collect::<Vec<_>>();
    assert_eq!(texts(1), ["abc", "aXc", "aXc"]);
    assert_eq!(texts(2), ["abc"; 3]);
}

#[test]
fn column_and_row_selections_offer_a_range_but_a_single_cell_does_not() {
    for (name, selection, expected) in [
        ("column", Sel::Columns, true),
        ("row", Sel::Rows { anchor: 1, end: 1 }, true),
        ("all", Sel::All, true),
        ("cell", Sel::Cells { anchor: (1, 1), end: (1, 1) }, false),
        ("none", Sel::None, false),
    ] {
        let (mut h, book) = selected_book(&format!("find-range-{name}"));
        h.state_mut().books.get_mut(&book).unwrap().sel = selection;
        settle(&mut h);
        ctrl(&mut h, egui::Key::F);
        assert_eq!(h.query_by_label(RANGE).is_some(), expected, "{name}");
        click(&mut h, "Close");
    }
}
