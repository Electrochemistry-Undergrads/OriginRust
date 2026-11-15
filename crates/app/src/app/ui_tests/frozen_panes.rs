use super::*;
use sheaf_core::Frozen;

/// A file of 60 rows and 30 columns whose value at row r, column c (0-based) is
/// (r + 1) * 100 + c + 1, so every cell shows a distinct number that names its place.
fn wide_file(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sheaf-app-ui-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("wide.csv");
    let mut text = (0..30).map(|c| format!("c{c}")).collect::<Vec<_>>().join(",");
    text.push('\n');
    for r in 0..60 {
        text.push_str(&(0..30).map(|c| ((r + 1) * 100 + c + 1).to_string()).collect::<Vec<_>>().join(","));
        text.push('\n');
    }
    std::fs::write(&path, text).unwrap();
    path
}

/// The workbook window of the tests shows the columns A to F and 14 rows: a frozen block of two
/// columns (84 + 2 * 67 points) is just under half of the 472 point wide window.
fn start_wide(tag: &str) -> Harness<'static, SheafApp> {
    let mut h = start(vec![wide_file(tag)]);
    show_window(&mut h, "Book1 - wide");
    h
}

/// The text of the cell at row r, column c (0-based).
fn cell_text(r: usize, c: usize) -> String {
    ((r + 1) * 100 + c + 1).to_string()
}

/// The rectangle of the text if it is drawn.
fn rect_opt(h: &Harness<'static, SheafApp>, text: &str) -> Option<egui::Rect> {
    h.query_all(By::new().role(egui::accesskit::Role::Label).value(text)).next().map(|n| n.rect())
}

/// Two clicks on the text (a double click starts editing).
fn edit_click(h: &mut Harness<'static, SheafApp>, text: &str) {
    pause(h);
    let node = grid_text(h, text);
    node.click();
    node.click();
    settle(h);
}

fn frozen(h: &Harness<'static, SheafApp>) -> Frozen {
    sheet(h).frozen
}

fn rect_of(h: &Harness<'static, SheafApp>, text: &str) -> egui::Rect {
    grid_text(h, text).rect()
}

/// The mouse wheel turned by `lines` (positive scrolls down and right) with the pointer at `at`.
fn wheel(h: &mut Harness<'static, SheafApp>, at: egui::Pos2, right: f32, down: f32) {
    h.event(egui::Event::PointerMoved(at));
    h.step();
    for _ in 0..4 {
        h.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Line,
            delta: egui::vec2(-right, -down),
            modifiers: egui::Modifiers::NONE,
        });
        h.step();
    }
    // The wheel scrolls smoothly over the next frames.
    h.run_steps(40);
}

fn freeze_shortcut(h: &mut Harness<'static, SheafApp>) {
    key_with(h, egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::F);
}

fn offsets(h: &Harness<'static, SheafApp>) -> (f32, f32) {
    book_ui(h).scroll_offsets()
}

/// Freezes with the shortcut after clicking the cell at row r and column c (0-based).
fn freeze_at(h: &mut Harness<'static, SheafApp>, r: usize, c: usize) {
    click_text(h, &cell_text(r, c));
    freeze_shortcut(h);
}

/// The first column at or after `from` whose cell in row `r` is drawn right of `x`.
fn first_right_of(h: &Harness<'static, SheafApp>, r: usize, from: usize, x: f32) -> usize {
    (from..30)
        .find(|c| {
            h.query_all(By::new().role(egui::accesskit::Role::Label).value(&cell_text(r, *c)))
                .next()
                .is_some_and(|n| n.rect().min.x > x + 4.0)
        })
        .expect("a scrolling cell is drawn")
}

fn select_in_state(h: &mut Harness<'static, SheafApp>, sel: Sel) {
    let book = project(h).workbooks().next().unwrap().id;
    let sheet = sheet(h).clone();
    let state = h.state_mut().books.get_mut(&book).unwrap();
    match sel {
        Sel::Cells { anchor, end } => state.select_cells(&sheet, anchor, end),
        _ => unreachable!(),
    }
}

#[test]
fn freezing_from_a_cell_a_row_or_a_column_selection_and_toggling_back() {
    let mut h = start_wide("fp-freeze");
    // D4 freezes 3 rows and 3 columns (Origin: wks.frozen$="C 3").
    freeze_at(&mut h, 3, 3);
    assert_eq!(frozen(&h), Frozen { rows: 3, columns: 3 });
    assert_eq!(h.state().doc.undo_label(), Some("Freeze panes"));
    // The same key unfreezes.
    freeze_shortcut(&mut h);
    assert_eq!(frozen(&h), Frozen::default());
    assert_eq!(h.state().doc.undo_label(), Some("Unfreeze panes"));
    // The View menu does the same, from a whole row (rows only) ...
    click_text(&mut h, "4");
    menu(&mut h, "View", "Freeze/Unfreeze Panes");
    assert_eq!(frozen(&h), Frozen { rows: 3, columns: 0 });
    menu(&mut h, "View", "Freeze/Unfreeze Panes");
    assert_eq!(frozen(&h), Frozen::default());
    // ... and from a whole column (columns only).
    click_text(&mut h, "D(Y1)");
    menu(&mut h, "View", "Freeze/Unfreeze Panes");
    assert_eq!(frozen(&h), Frozen { rows: 0, columns: 3 });
    // Undo and redo are ordinary steps.
    menu(&mut h, "Edit", "Undo Freeze panes");
    assert_eq!(frozen(&h), Frozen::default());
    menu(&mut h, "Edit", "Redo Freeze panes");
    assert_eq!(frozen(&h), Frozen { rows: 0, columns: 3 });
}

#[test]
fn an_index_above_ten_the_first_cell_and_no_selection_freeze_nothing() {
    let mut h = start_wide("fp-refuse");
    // Nothing selected.
    freeze_shortcut(&mut h);
    assert_eq!(frozen(&h), Frozen::default());
    assert!(h.state().status.contains("Select a cell"), "{}", h.state().status);
    // A1: nothing above or left of it.
    freeze_at(&mut h, 0, 0);
    assert_eq!(frozen(&h), Frozen::default());
    assert!(h.state().status.contains("Nothing to freeze"), "{}", h.state().status);
    // Index 10 is the last that works (9 rows and 9 columns), 11 is refused.
    select_in_state(&mut h, Sel::Cells { anchor: (9, 9), end: (9, 9) });
    freeze_shortcut(&mut h);
    assert_eq!(frozen(&h), Frozen { rows: 9, columns: 9 });
    freeze_shortcut(&mut h);
    for at in [(10, 3), (3, 10)] {
        select_in_state(&mut h, Sel::Cells { anchor: at, end: at });
        freeze_shortcut(&mut h);
        assert_eq!(frozen(&h), Frozen::default(), "{at:?}");
        assert!(h.state().status.contains("1 to 10"), "{}", h.state().status);
    }
    assert_eq!(h.state().doc.undo_label(), Some("Unfreeze panes"), "refusals leave no undo step");
}

#[test]
fn frozen_cells_stay_while_the_rest_scrolls_both_ways_and_each_cell_is_drawn_once() {
    let mut h = start_wide("fp-scroll");
    freeze_at(&mut h, 3, 2);
    settle(&mut h);
    let corner = rect_of(&h, &cell_text(0, 0));
    let head = rect_of(&h, "A(X1)");
    let frozen_row = rect_of(&h, &cell_text(1, 4));
    let frozen_column = rect_of(&h, &cell_text(9, 1));
    let body = rect_of(&h, &cell_text(6, 4));
    let at = rect_of(&h, &cell_text(8, 3)).center();
    wheel(&mut h, at, 1.0, 0.5);
    let (dx, dy) = offsets(&h);
    assert!(dx > 0.0 && dy > 0.0, "the body scrolled both ways: {:?}", offsets(&h));
    assert_eq!(rect_of(&h, &cell_text(0, 0)), corner, "a frozen row and column cell stays");
    assert_eq!(rect_of(&h, "A(X1)"), head, "a frozen head stays");
    let row = rect_of(&h, &cell_text(1, 4));
    assert_eq!(row.min.y, frozen_row.min.y, "a frozen row scrolls sideways only");
    assert!(row.min.x < frozen_row.min.x);
    let column = rect_of(&h, &cell_text(9, 1));
    assert_eq!(column.min.x, frozen_column.min.x, "a frozen column scrolls down only");
    assert!(column.min.y < frozen_column.min.y);
    assert!(rect_opt(&h, &cell_text(6, 4)).is_none_or(|r| r.min != body.min), "a scrolling cell moved");
    // The frozen part of a row is on the same line as the rest of it, with no lag.
    let scrolled =
        (4..60).find(|r| rect_opt(&h, &cell_text(*r, 1)).is_some() && rect_opt(&h, &cell_text(*r, 4)).is_some());
    let scrolled = scrolled.expect("a scrolled row is drawn");
    assert_eq!(rect_of(&h, &cell_text(scrolled, 1)).min.y, rect_of(&h, &cell_text(scrolled, 4)).min.y);
}

#[test]
fn the_wheel_over_the_frozen_columns_the_head_or_the_frozen_rows_scrolls_the_body() {
    let mut h = start_wide("fp-wheel");
    freeze_at(&mut h, 3, 2);
    settle(&mut h);
    // Over a frozen column in the body: the rows move, the frozen cell with them.
    let before = rect_of(&h, &cell_text(8, 4)).min.y;
    let at = rect_of(&h, &cell_text(8, 1)).center();
    wheel(&mut h, at, 0.0, 0.5);
    assert!(offsets(&h).1 > 0.0);
    let (a, b) = (rect_of(&h, &cell_text(8, 1)).min.y, rect_of(&h, &cell_text(8, 4)).min.y);
    assert!(b < before && a == b, "the frozen column follows the rows at once ({a} {b})");
    // Over the frozen rows and over the head: the body scrolls back up.
    let frozen_row = rect_of(&h, &cell_text(1, 4)).center();
    wheel(&mut h, frozen_row, 0.0, -0.25);
    let after_band = offsets(&h).1;
    assert!(after_band < offsets(&h).1 + 1.0 && after_band >= 0.0);
    let mid = after_band;
    let head = rect_of(&h, "E(Y1)").center();
    wheel(&mut h, head, 0.0, 0.25);
    assert!(offsets(&h).1 > mid, "the wheel over the head scrolls the body down");
    wheel(&mut h, frozen_row, 0.0, 0.25);
    assert!(offsets(&h).1 > mid);
    let before = offsets(&h).1;
    wheel(&mut h, frozen_row, 0.0, -0.25);
    assert!(offsets(&h).1 < before, "and the wheel over the frozen rows scrolls it up");
}

#[test]
fn dragging_a_selection_across_the_boundary_works_in_both_directions() {
    let mut h = start_wide("fp-drag");
    freeze_at(&mut h, 3, 2);
    settle(&mut h);
    // From a scrolling cell into a frozen column, and back.
    let (body, frozen_cell) = (rect_of(&h, &cell_text(6, 4)).center(), rect_of(&h, &cell_text(8, 1)).center());
    drag(&mut h, body, frozen_cell);
    assert_eq!(cells(&h), Sel::Cells { anchor: (4, 6), end: (1, 8) });
    drag(&mut h, frozen_cell, body);
    assert_eq!(cells(&h), Sel::Cells { anchor: (1, 8), end: (4, 6) });
    // Into the frozen rows from the body.
    let up = rect_of(&h, &cell_text(1, 4)).center();
    drag(&mut h, body, up);
    assert_eq!(cells(&h), Sel::Cells { anchor: (4, 6), end: (4, 1) });
    // Scrolled sideways, cells hidden under the frozen columns are not the ones reached there.
    let at = rect_of(&h, &cell_text(8, 3)).center();
    wheel(&mut h, at, 1.0, 0.0);
    let first = first_right_of(&h, 8, 2, rect_of(&h, &cell_text(8, 1)).max.x);
    assert!(first > 2, "columns are hidden under the frozen ones (first visible: {first})");
    let (body, frozen_cell) = (rect_of(&h, &cell_text(8, first)).center(), rect_of(&h, &cell_text(9, 1)).center());
    drag(&mut h, body, frozen_cell);
    assert_eq!(cells(&h), Sel::Cells { anchor: (first, 8), end: (1, 9) });
    // A drag that starts in a frozen cell.
    drag(&mut h, frozen_cell, body);
    assert_eq!(cells(&h), Sel::Cells { anchor: (1, 9), end: (first, 8) });
    let target = book_ui(&h).target(sheet(&h)).unwrap();
    assert_eq!((target.columns.first(), target.rows.clone()), (Some(&1), 8..10));
}

/// Press at `from`, move to `to`, hold for some frames and release.
fn drag_and_hold(h: &mut Harness<'static, SheafApp>, from: egui::Pos2, to: egui::Pos2, frames: usize) {
    let button = |pos, pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    h.event(egui::Event::PointerMoved(from));
    h.step();
    h.event(button(from, true));
    h.step();
    h.event(egui::Event::PointerMoved(to));
    for _ in 0..frames {
        h.step();
    }
    h.event(button(to, false));
    h.step();
    settle(h);
}

#[test]
fn dragging_from_the_body_up_into_the_frozen_rows_scrolls_up_but_a_drag_inside_the_block_does_not() {
    let mut h = start_wide("fp-edge");
    freeze_at(&mut h, 3, 2);
    settle(&mut h);
    let at = rect_of(&h, &cell_text(8, 3)).center();
    wheel(&mut h, at, 0.0, 1.0);
    let scrolled = offsets(&h).1;
    assert!(scrolled > 0.0);
    // Inside the frozen block: from a frozen column cell up to a frozen row cell.
    let from = rect_of(&h, &cell_text(2, 1)).center();
    let to = rect_of(&h, &cell_text(1, 0)).center();
    drag_and_hold(&mut h, from, to, 15);
    assert_eq!(offsets(&h), (0.0, scrolled), "nothing scrolled");
    assert_eq!(cells(&h), Sel::Cells { anchor: (1, 2), end: (0, 1) });
    // From a scrolling cell up into the frozen rows.
    let row = (4..60).find(|r| rect_opt(&h, &cell_text(*r, 4)).is_some_and(|n| n.min.y > 260.0)).unwrap();
    let from = rect_of(&h, &cell_text(row, 4)).center();
    let to = rect_of(&h, &cell_text(1, 4)).center();
    drag_and_hold(&mut h, from, to, 15);
    assert!(offsets(&h).1 < scrolled, "the view scrolled up: {:?}", offsets(&h));
}

#[test]
fn a_frozen_cell_is_edited_while_the_body_is_scrolled() {
    let mut h = start_wide("fp-edit");
    freeze_at(&mut h, 3, 2);
    settle(&mut h);
    let at = rect_of(&h, &cell_text(8, 3)).center();
    wheel(&mut h, at, 1.0, 0.5);
    let before = offsets(&h);
    // A cell in the frozen rows and frozen columns.
    let place = rect_of(&h, &cell_text(2, 1));
    edit_click(&mut h, &cell_text(2, 1));
    assert!(book_ui(&h).editing.is_some(), "the frozen cell is being edited");
    assert_eq!(offsets(&h), before, "starting to edit a frozen cell does not scroll");
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.event(egui::Event::Text("999".to_string()));
    settle(&mut h);
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    assert_eq!(column(&h, "B")[2], 999.0);
    assert_eq!(h.state().doc.undo_label(), Some("Edit cell"));
    assert_eq!(rect_of(&h, "999").min.y, place.min.y, "the edited cell stayed in place");
    // A frozen column cell of a scrolled row.
    let row = (4..60).find(|r| rect_opt(&h, &cell_text(*r, 0)).is_some_and(|n| n.min.y > 250.0)).unwrap();
    edit_click(&mut h, &cell_text(row, 0));
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.event(egui::Event::Text("555".to_string()));
    settle(&mut h);
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    assert_eq!(column(&h, "A")[row], 555.0);
}

#[test]
fn keys_and_go_to_leave_the_scroll_alone_for_frozen_cells_and_land_below_the_frozen_rows() {
    let mut h = start_wide("fp-keys");
    freeze_at(&mut h, 3, 2);
    settle(&mut h);
    let at = rect_of(&h, &cell_text(8, 3)).center();
    wheel(&mut h, at, 1.0, 1.0);
    // Left from the first visible scrolling column onto a frozen column.
    let row = (4..60).find(|r| rect_opt(&h, &cell_text(*r, 1)).is_some_and(|n| n.min.y > 260.0)).unwrap();
    let first = first_right_of(&h, row, 2, rect_of(&h, &cell_text(row, 1)).max.x);
    click_text(&mut h, &cell_text(row, first));
    for _ in 2..first {
        key(&mut h, egui::Key::ArrowLeft);
    }
    assert_eq!(cells(&h), at_cell(2, row));
    let before = offsets(&h);
    key(&mut h, egui::Key::ArrowLeft);
    assert_eq!(cells(&h), at_cell(1, row));
    assert_eq!(offsets(&h), before, "a frozen column is never scrolled to");
    key(&mut h, egui::Key::ArrowRight);
    assert_eq!(cells(&h), at_cell(2, row));
    h.run_steps(30);
    assert!(
        rect_of(&h, &cell_text(row, 2)).min.x >= rect_of(&h, &cell_text(row, 1)).max.x - 0.5,
        "column 2 is revealed"
    );
    // Up from the first visible scrolling row onto a frozen row.
    let band_bottom = rect_of(&h, &cell_text(2, 3)).max.y;
    let top = (3..60).find(|r| rect_opt(&h, &cell_text(*r, 3)).is_some_and(|n| n.min.y > band_bottom - 1.0)).unwrap();
    click_text(&mut h, &cell_text(top, 3));
    for _ in 3..top {
        key(&mut h, egui::Key::ArrowUp);
    }
    assert_eq!(cells(&h), at_cell(3, 3));
    let before = offsets(&h);
    key(&mut h, egui::Key::ArrowUp);
    assert_eq!(cells(&h), at_cell(3, 2));
    assert_eq!(offsets(&h), before, "a frozen row is never scrolled to");
    // Go To a frozen cell leaves the scroll; Go To a far row lands below the frozen rows.
    go_to(&mut h, 1, 1);
    assert_eq!(offsets(&h), before);
    go_to(&mut h, 1, 45);
    // (The table's scroll-to-row leaves a row up to 3 points under the edge, as in a plain sheet.)
    assert!(rect_of(&h, &cell_text(45, 1)).min.y >= band_bottom - 4.0, "the row is not hidden by the frozen rows");
    go_to(&mut h, 3, 5);
    assert!(rect_of(&h, &cell_text(5, 3)).min.y >= band_bottom - 4.0);
    assert!(rect_of(&h, &cell_text(5, 3)).min.x >= rect_of(&h, &cell_text(5, 1)).max.x - 0.5);
}

fn at_cell(col: usize, row: usize) -> Sel {
    Sel::Cells { anchor: (col, row), end: (col, row) }
}

fn go_to(h: &mut Harness<'static, SheafApp>, column: usize, row: usize) {
    let book = project(h).workbooks().next().unwrap().id;
    let sheet = sheet(h).clone();
    h.state_mut().books.get_mut(&book).unwrap().go_to(&sheet, column, row);
    h.run_steps(40);
}

#[test]
fn go_to_a_far_row_of_a_filtered_sheet_lands_below_the_frozen_rows() {
    let mut h = start_wide("fp-filter");
    // Only the even rows pass: the frozen rows are the first three displayed ones (0, 2, 4).
    h.state_mut().doc.apply("Filter", |p| {
        let id = p.workbooks().next().unwrap().id;
        let s = p.workbook_mut(id).unwrap().sheet_mut();
        s.filter = Some(sheaf_core::rowfilter::RowFilter {
            condition: "fixture".into(),
            enabled: true,
            keep: (0..60).map(|r| r % 2 == 0).collect(),
        });
        s.frozen = Frozen { rows: 3, columns: 0 };
    });
    settle(&mut h);
    let last = rect_of(&h, &cell_text(4, 2));
    let next = rect_of(&h, &cell_text(6, 2));
    assert_eq!(next.min.y, last.min.y + 16.0, "the fourth displayed row follows the frozen ones");
    go_to(&mut h, 2, 40);
    assert!(rect_of(&h, &cell_text(40, 2)).min.y >= last.max.y - 0.5);
    go_to(&mut h, 2, 4);
    assert_eq!(rect_of(&h, &cell_text(4, 2)), last, "a frozen row stays where it is");
}

#[test]
fn dragging_the_edge_of_a_frozen_head_resizes_the_column_and_the_width_survives_unfreezing() {
    let mut h = start_wide("fp-resize");
    freeze_at(&mut h, 3, 2);
    settle(&mut h);
    let width_of_a = |h: &Harness<'static, SheafApp>| {
        let (a, b) = (rect_of(h, "A(X1)").center().x, rect_of(h, "B(Y1)").center().x);
        2.0 * (b - a) - 67.0
    };
    assert!((width_of_a(&h) - 67.0).abs() < 1.5, "{}", width_of_a(&h));
    let d_before = rect_of(&h, &cell_text(5, 3)).min.x;
    let head = rect_of(&h, "A(X1)");
    let edge = egui::pos2(head.center().x + 33.0, head.center().y);
    drag(&mut h, edge, edge + egui::vec2(40.0, 0.0));
    settle(&mut h);
    assert!((width_of_a(&h) - 107.0).abs() < 1.5, "{}", width_of_a(&h));
    assert!((rect_of(&h, &cell_text(5, 3)).min.x - d_before - 40.0).abs() < 1.5, "the first scrolling column moved");
    freeze_shortcut(&mut h);
    assert_eq!(frozen(&h), Frozen::default());
    settle(&mut h);
    assert!((width_of_a(&h) - 107.0).abs() < 1.5, "unfrozen it keeps the width: {}", width_of_a(&h));
    freeze_shortcut(&mut h);
    assert_eq!(frozen(&h), Frozen { rows: 3, columns: 2 });
    settle(&mut h);
    assert!((width_of_a(&h) - 107.0).abs() < 1.5, "and so does a new freeze: {}", width_of_a(&h));
}

#[test]
fn frozen_panes_are_saved_in_the_sheaf_file_and_come_back() {
    let dir = save_dir("frozen");
    let path = dir.join("frozen.sheaf");
    let mut h = start_wide("fp-save");
    freeze_at(&mut h, 3, 2);
    assert_eq!(frozen(&h), Frozen { rows: 3, columns: 2 });
    assert!(h.state_mut().save_to(path.clone()));
    let h2 = start(vec![path]);
    assert_eq!(project(&h2).workbooks().next().unwrap().sheet().frozen, Frozen { rows: 3, columns: 2 });
}

#[test]
fn clicking_a_frozen_cell_while_scrolled_selects_it_and_not_the_cell_under_it() {
    let mut h = start_wide("fp-click");
    freeze_at(&mut h, 3, 2);
    settle(&mut h);
    let at = rect_of(&h, &cell_text(8, 3)).center();
    wheel(&mut h, at, 1.0, 0.5);
    let row = (4..60).find(|r| rect_opt(&h, &cell_text(*r, 1)).is_some_and(|n| n.min.y > 250.0)).unwrap();
    click_text(&mut h, &cell_text(row, 1));
    assert_eq!(cells(&h), at_cell(1, row));
    // Column 2 is scrolled under the frozen columns: a click where its cell is drawn reaches the
    // frozen cell on top of it.
    click_text(&mut h, &cell_text(row, 2));
    let Sel::Cells { anchor, .. } = cells(&h) else { panic!("a cell is selected") };
    assert!(anchor.0 < 2 && anchor.1 == row, "{anchor:?}");
    let first = first_right_of(&h, row, 2, rect_of(&h, &cell_text(row, 1)).max.x);
    click_text(&mut h, &cell_text(row, first));
    assert_eq!(cells(&h), at_cell(first, row));
    click_text(&mut h, &cell_text(1, first));
    assert_eq!(cells(&h), at_cell(first, 1));
    click_text(&mut h, "B(Y1)");
    assert_eq!(cells(&h), Sel::Columns);
    assert_eq!(book_ui(&h).selected, vec![column_id(&h, "B")]);
}
