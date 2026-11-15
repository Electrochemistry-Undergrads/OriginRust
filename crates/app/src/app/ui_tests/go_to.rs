use super::*;

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

fn destination(h: &mut Harness<'static, SheafApp>, row: &str, column: &str, extend: bool) {
    ctrl(h, egui::Key::G);
    assert!(h.state().go_to.is_some());
    let row_bottom = h.get_by_label("Go to Row").rect().bottom();
    assert!(h.get_by_label("OK").rect().top() - row_bottom < 90.0, "the button row stays beside the form");
    field(h, "Go to Row", row);
    field(h, "Go to Column", column);
    if extend {
        click(h, "Extend Selection");
    }
    click(h, "OK");
    assert!(h.state().go_to.is_none());
}

#[test]
fn go_to_an_omitted_coordinate_keeps_that_axis_of_the_current_cell() {
    let mut h = start(vec![data_file("goto-optional-cell")]);
    show_window(&mut h, "Book1 - sample");
    click_text(&mut h, "10.5");
    let before = project(&h).clone();
    let revision = h.state().doc.revision();
    destination(&mut h, "10", "", false);
    assert_eq!(book_ui(&h).sel, Sel::Cells { anchor: (2, 9), end: (2, 9) });
    destination(&mut h, "", "b", false);
    assert_eq!(book_ui(&h).sel, Sel::Cells { anchor: (1, 9), end: (1, 9) });
    assert_eq!(project(&h), &before);
    assert_eq!(h.state().doc.revision(), revision);
}

#[test]
fn go_to_extend_matches_origins_forward_backward_inside_and_single_axis_cases() {
    let cases = [
        ((2, 3), "10", "4", vec![1, 2, 3], 1..10),
        ((2, 3), "1", "1", vec![0, 1, 2], 0..4),
        ((3, 9), "4", "3", vec![1, 2], 1..4),
        ((2, 3), "10", "", vec![1, 2], 1..10),
        ((2, 3), "", "4", vec![1, 2, 3], 1..4),
    ];
    for (i, (end, row, column, columns, rows)) in cases.into_iter().enumerate() {
        let mut h = start(vec![data_file(&format!("goto-extend-{i}"))]);
        let book = project(&h).workbooks().next().unwrap().id;
        h.state_mut().doc.apply("Fourth column", |p| {
            p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![1.0; 4])).unwrap();
        });
        settle(&mut h);
        show_window(&mut h, "Book1 - sample");
        let source_sheet = sheet(&h).clone();
        h.state_mut().books.get_mut(&book).unwrap().select_cells(&source_sheet, (1, 1), end);
        settle(&mut h);
        let before = project(&h).clone();
        let revision = h.state().doc.revision();
        destination(&mut h, row, column, true);
        let target = book_ui(&h).target(sheet(&h)).unwrap();
        assert_eq!(target.columns, columns);
        assert_eq!(target.rows, rows);
        assert_eq!(project(&h), &before);
        assert_eq!(h.state().doc.revision(), revision);
    }
}

#[test]
fn go_to_omitted_axis_retains_the_selected_range_on_that_axis() {
    for (i, (row, column, columns, rows)) in
        [("10", "", vec![1, 2], 9..10), ("", "1", vec![0], 1..4)].into_iter().enumerate()
    {
        let mut h = start(vec![data_file(&format!("goto-optional-range-{i}"))]);
        show_window(&mut h, "Book1 - sample");
        select_block(&mut h, "4.5", "0.5");
        destination(&mut h, row, column, false);
        let target = book_ui(&h).target(sheet(&h)).unwrap();
        assert_eq!(target.columns, columns);
        assert_eq!(target.rows, rows);
    }
}

#[test]
fn go_to_clamps_to_row_capacity_and_reveals_empty_rows_without_growing_data() {
    let mut h = start(vec![data_file("goto-distant-row")]);
    show_window(&mut h, "Book1 - sample");
    let before = project(&h).clone();
    let revision = h.state().doc.revision();
    destination(&mut h, "1000", "2", false);
    assert_eq!(book_ui(&h).sel, Sel::Cells { anchor: (1, 33), end: (1, 33) });
    assert!(h.query_by_value("34").is_some(), "the clamped target row is actually visible");
    h.key_press(egui::Key::ArrowUp);
    settle(&mut h);
    assert_eq!(book_ui(&h).sel, Sel::Cells { anchor: (1, 32), end: (1, 32) });
    assert!(h.query_by_value("33").is_some());
    destination(&mut h, "0", "0", false);
    assert_eq!(book_ui(&h).sel, Sel::Cells { anchor: (2, 3), end: (2, 3) });
    assert_eq!(project(&h), &before);
    assert_eq!(h.state().doc.revision(), revision);
}

#[test]
fn go_to_reveals_a_column_more_than_four_viewports_away_and_returns_to_the_first() {
    let mut h = start(vec![data_file("goto-far-column")]);
    let book = project(&h).workbooks().next().unwrap().id;
    h.state_mut().doc.apply("Many columns", |p| {
        for _ in 0..80 {
            p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![1.0])).unwrap();
        }
    });
    settle(&mut h);
    show_window(&mut h, "Book1 - sample");
    let name = grid::head_name(sheet(&h), 82);
    let before = project(&h).clone();
    let revision = h.state().doc.revision();
    destination(&mut h, "0", "83", false);
    assert_eq!(book_ui(&h).sel, Sel::Cells { anchor: (82, 0), end: (82, 0) });
    let header = h.get_by_value(&name).rect();
    assert!((0.0..1280.0).contains(&header.center().x), "far column is visible: {header:?}");
    destination(&mut h, "1", "1", false);
    let header = h.get_by_value("A(X1)").rect();
    assert!((0.0..1280.0).contains(&header.center().x));
    assert_eq!(project(&h), &before);
    assert_eq!(h.state().doc.revision(), revision);
}

#[test]
fn go_to_invalid_entries_empty_ok_cancel_and_escape_preserve_selection_and_history() {
    let mut h = start(vec![data_file("goto-cancel")]);
    show_window(&mut h, "Book1 - sample");
    click_text(&mut h, "4.5");
    let selection = book_ui(&h).sel.clone();
    let before = project(&h).clone();
    let revision = h.state().doc.revision();
    menu_path(&mut h, &["Edit", "Go to..."]);
    for (row, column) in [("-1", "1"), ("2", "999"), ("2147483648", "1"), ("2", "no-such-column")] {
        field(&mut h, "Go to Row", row);
        field(&mut h, "Go to Column", column);
        click(&mut h, "OK");
        assert!(h.state().go_to.is_some());
        assert_eq!(book_ui(&h).sel, selection);
    }
    field(&mut h, "Go to Row", "");
    field(&mut h, "Go to Column", "");
    click(&mut h, "OK");
    assert!(h.state().go_to.is_none());
    assert_eq!(book_ui(&h).sel, selection);
    for escape in [false, true] {
        ctrl(&mut h, egui::Key::G);
        field(&mut h, "Go to Row", "20");
        field(&mut h, "Go to Column", "3");
        if escape {
            h.key_press(egui::Key::Escape);
            settle(&mut h);
        } else {
            click(&mut h, "Cancel");
        }
        assert!(h.state().go_to.is_none());
        assert_eq!(book_ui(&h).sel, selection);
    }
    assert_eq!(project(&h), &before);
    assert_eq!(h.state().doc.revision(), revision);
}
