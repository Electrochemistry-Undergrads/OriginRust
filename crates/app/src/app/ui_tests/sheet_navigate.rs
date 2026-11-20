//! The sheet tab's Navigate... (T93, workbook-model.md 13h).

use super::*;

#[test]
fn navigate_lists_the_sheets_and_goes_to_the_picked_one() {
    let mut h = start(vec![data_file("sheet-navigate")]);
    show_window(&mut h, "Book1 - sample.csv");
    let book = h.state().books.keys().next().copied().unwrap();
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::Sheet { book, op: SheetOp::Add }, &ctx);
    settle(&mut h);
    assert_eq!(project(&h).workbook(book).unwrap().active_sheet, 1);
    h.state_mut().apply(Action::Sheet { book, op: SheetOp::Navigate }, &ctx);
    settle(&mut h);
    assert!(h.state().sheet_navigate.is_some(), "the window opened");
    assert_eq!(h.state().sheet_navigate.as_ref().unwrap().selected, 1, "the active sheet is selected");
    // Pick the first sheet and press OK.
    h.state_mut().sheet_navigate.as_mut().unwrap().selected = 0;
    click(&mut h, "OK");
    assert!(h.state().sheet_navigate.is_none());
    assert_eq!(project(&h).workbook(book).unwrap().active_sheet, 0);
}

#[test]
fn navigate_comments_follow_their_sheets_and_show_full_text_on_hover() {
    let mut h = start(vec![data_file("sheet-navigate-comments")]);
    show_window(&mut h, "Book1 - sample.csv");
    let book = project(&h).workbooks().next().unwrap().id;
    let comments = "First sweep\nCheck the baseline.";
    h.state_mut().doc.apply("Comment on sheets", |p| {
        let sheets = &mut p.workbook_mut(book).unwrap().sheets;
        sheets[0].comments = comments.into();
        let mut other = sheaf_core::Worksheet::new("Second data sheet");
        other.comments = "Control run".into();
        sheets.push(other);
    });
    settle(&mut h);
    let before = project(&h).workbook(book).unwrap().clone();
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::Sheet { book, op: SheetOp::Navigate }, &ctx);
    settle(&mut h);
    let first = grid_text(&h, "First sweep").rect().center();
    let second = grid_text(&h, "Control run").rect().center();
    assert!(first.y < second.y);
    assert!(has_label(&h, "Comments"));

    ctx.style_mut(|style| style.interaction.tooltip_delay = 0.0);
    h.event(egui::Event::PointerMoved(first));
    h.run_steps(30);
    assert!(h.query_all(By::new().role(egui::accesskit::Role::Label).value(comments)).next().is_some());
    click(&mut h, "Close");
    assert_eq!(project(&h).workbook(book), Some(&before));
}

#[test]
fn navigate_applies_the_sorted_sheet_order_as_one_undo_step() {
    let path = data_file("sheet-navigate-reorder");
    let saved = path.with_file_name("reordered.sheaf");
    let mut h = start(vec![path]);
    show_window(&mut h, "Book1 - sample.csv");
    let book = project(&h).workbooks().next().unwrap().id;
    h.state_mut().doc.apply("Set sheet names", |p| {
        let b = p.workbook_mut(book).unwrap();
        b.sheets[0].name = "Zeta".into();
        b.sheets[0].comments = "First worksheet".into();
        let mut a = sheaf_core::Worksheet::new("Alpha");
        a.comments = "Second worksheet".into();
        b.sheets.push(a);
        b.sheets.push(sheaf_core::Worksheet::new("Beta"));
    });
    settle(&mut h);
    let original = project(&h).workbook(book).unwrap().clone();
    let revision = h.state().doc.revision();
    let ctx = h.ctx.clone();
    h.get_by_label("Zeta").click_secondary();
    settle(&mut h);
    click(&mut h, "Navigate...");
    assert!(is_disabled(&h, "Apply Reorder"));

    click(&mut h, "Name");
    assert!(grid_text(&h, "Second worksheet").rect().top() < grid_text(&h, "First worksheet").rect().top());
    assert_eq!(project(&h).workbook(book), Some(&original));
    assert_eq!(h.state().doc.revision(), revision);
    click(&mut h, "Name ↑");
    assert!(grid_text(&h, "First worksheet").rect().top() < grid_text(&h, "Second worksheet").rect().top());
    click(&mut h, "Name ↓");
    assert!(!is_disabled(&h, "Apply Reorder"));

    h.state_mut().apply(Action::FindInProject, &ctx);
    settle(&mut h);
    assert!(is_disabled(&h, "Apply Reorder"));
    click(&mut h, "Cancel");
    click(&mut h, "Apply Reorder");
    assert!(is_disabled(&h, "Apply Reorder"));
    assert_eq!(h.state().sheet_navigate.as_ref().unwrap().selected, 2);
    assert_eq!(h.state().doc.undo_label(), Some("Reorder worksheets"));
    let reordered = project(&h).workbook(book).unwrap();
    assert_eq!(reordered.active_sheet, 2);
    assert_eq!(reordered.sheets, [original.sheets[1].clone(), original.sheets[2].clone(), original.sheets[0].clone()]);
    sheaf_io::save::save_project(project(&h), &saved).unwrap();
    let reopened = sheaf_io::open_project(&saved).unwrap().project;
    assert_eq!(reopened.workbook(book), Some(reordered));

    click(&mut h, "Close");
    menu(&mut h, "Edit", "Undo Reorder worksheets");
    assert_eq!(project(&h).workbook(book), Some(&original));
}

/// A workbook with sheets Sheet1..Sheet4 (Sheet1 the sample) and Navigate open on it.
fn four_sheets(tag: &str) -> (Harness<'static, SheafApp>, WindowId) {
    let mut h = start(vec![data_file(tag)]);
    show_window(&mut h, "Book1 - sample.csv");
    let book = h.state().books.keys().next().copied().unwrap();
    let ctx = h.ctx.clone();
    for _ in 0..3 {
        h.state_mut().apply(Action::Sheet { book, op: SheetOp::Add }, &ctx);
    }
    h.state_mut().apply(Action::SelectSheet { book, sheet: 0 }, &ctx);
    h.state_mut().apply(Action::Sheet { book, op: SheetOp::Navigate }, &ctx);
    settle(&mut h);
    (h, book)
}

fn names(h: &Harness<'static, SheafApp>, book: WindowId) -> Vec<String> {
    project(h).workbook(book).unwrap().sheets.iter().map(|s| s.name.clone()).collect()
}

fn pick(h: &mut Harness<'static, SheafApp>, sheets: &[usize]) {
    let d = h.state_mut().sheet_navigate.as_mut().unwrap();
    d.picked = sheets.iter().copied().collect();
    d.selected = sheets[0];
}

fn act(h: &mut Harness<'static, SheafApp>, item: &str) {
    click(h, "Actions");
    click(h, item);
    settle(h);
}

#[test]
fn navigate_actions_move_duplicate_clear_and_delete_the_selected_sheets() {
    // T180: Origin's Actions menu (docs "Navigate Worksheets"), each one undo step.
    let (mut h, book) = four_sheets("sheet-navigate-actions");
    let first = names(&h, book);
    assert_eq!(first.len(), 4);
    pick(&mut h, &[0, 2]);
    act(&mut h, "Move to End");
    assert_eq!(names(&h, book), [first[1].clone(), first[3].clone(), first[0].clone(), first[2].clone()]);
    assert_eq!(h.state().doc.undo_label(), Some("Move worksheets"));
    assert_eq!(h.state().sheet_navigate.as_ref().unwrap().targets(), [2, 3], "the moved sheets stay selected");
    act(&mut h, "Move Before");
    assert_eq!(names(&h, book), [first[1].clone(), first[0].clone(), first[2].clone(), first[3].clone()]);
    menu(&mut h, "Edit", "Undo Move worksheets");
    menu(&mut h, "Edit", "Undo Move worksheets");
    assert_eq!(names(&h, book), first);

    // Duplicate two sheets: each copy right after its sheet, in one step.
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::Sheet { book, op: SheetOp::Navigate }, &ctx);
    settle(&mut h);
    pick(&mut h, &[0, 1]);
    act(&mut h, "Duplicate");
    let dup = names(&h, book);
    assert_eq!(dup.len(), 6);
    assert_eq!((&dup[0], &dup[2]), (&first[0], &first[1]));
    assert!(dup[1].starts_with(&format!("{}_", first[0])) && dup[3].starts_with(&format!("{}_", first[1])));
    let book_ref = project(&h).workbook(book).unwrap();
    assert_eq!(book_ref.sheets[1].row_count(), book_ref.sheets[0].row_count(), "with its data");
    assert_eq!(h.state().doc.undo_label(), Some("Duplicate sheets"));

    // Clear all Data empties the sample sheet and keeps its columns.
    pick(&mut h, &[0]);
    act(&mut h, "Clear all Data");
    let s = &project(&h).workbook(book).unwrap().sheets[0];
    assert!(!s.columns.is_empty() && s.row_count() == 0);

    // Delete the two copies at once.
    pick(&mut h, &[1, 3]);
    act(&mut h, "Delete");
    assert_eq!(names(&h, book).len(), 4);
    assert_eq!(h.state().doc.undo_label(), Some("Delete sheets"));
    // Every sheet cannot go.
    pick(&mut h, &[0, 1, 2, 3]);
    click(&mut h, "Actions");
    assert!(h.get_by_label("Delete").accesskit_node().is_disabled());
}

/// A click with `modifiers` on the Navigate row named `name` (the window is drawn after the tabs).
fn click_row(h: &mut Harness<'static, SheafApp>, name: &str, modifiers: egui::Modifiers) {
    h.run_steps(25);
    let at = h.query_all_by_label(name).last().expect("the row").rect().center();
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

#[test]
fn navigate_ctrl_and_shift_clicks_select_several_sheets() {
    let (mut h, book) = four_sheets("sheet-navigate-multi");
    let names = names(&h, book);
    let targets = |h: &Harness<'static, SheafApp>| h.state().sheet_navigate.as_ref().unwrap().targets();
    assert_eq!(targets(&h), [0], "the active sheet");
    // Ctrl+click adds a sheet, and again removes it.
    click_row(&mut h, &names[2], egui::Modifiers::COMMAND);
    assert_eq!(targets(&h), [0, 2]);
    click_row(&mut h, &names[2], egui::Modifiers::COMMAND);
    assert_eq!(targets(&h), [0]);
    // Shift+click takes the run from the last click.
    click_row(&mut h, &names[3], egui::Modifiers::SHIFT);
    assert_eq!(targets(&h), [0, 1, 2, 3]);
    // A plain click picks one sheet again.
    click_row(&mut h, &names[1], egui::Modifiers::NONE);
    assert_eq!(targets(&h), [1]);
}

#[test]
fn navigate_moves_or_copies_sheets_to_a_new_book_and_excludes_them_from_plotting() {
    // T184: Move to > New Book, Duplicate to New Book, Exclude from Plotting dialogs.
    let (mut h, book) = four_sheets("sheet-navigate-books");
    let first = names(&h, book);
    let books = |h: &Harness<'static, SheafApp>| project(h).workbooks().count();
    let before = books(&h);
    pick(&mut h, &[0]);
    act_in(&mut h, "Move to", "New Book");
    assert_eq!(books(&h), before + 1);
    assert_eq!(names(&h, book), &first[1..]);
    let new = project(&h).workbooks().last().unwrap();
    assert_eq!(new.sheets.len(), 1);
    assert_eq!(new.sheets[0].name, first[0]);
    assert!(new.sheets[0].row_count() > 0, "with its data");
    assert_eq!(h.state().doc.undo_label(), Some("Move sheets to new book"));
    menu(&mut h, "Edit", "Undo Move sheets to new book");
    assert_eq!((books(&h), names(&h, book)), (before, first.clone()));

    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::Sheet { book, op: SheetOp::Navigate }, &ctx);
    settle(&mut h);
    pick(&mut h, &[0, 1]);
    act(&mut h, "Duplicate to New Book");
    assert_eq!(books(&h), before + 1);
    assert_eq!(names(&h, book), first, "the sheets stay");
    let copy = project(&h).workbooks().last().unwrap();
    assert_eq!(copy.sheets.len(), 2);
    let source = &project(&h).workbook(book).unwrap().sheets[0];
    assert_ne!(copy.sheets[0].columns[0].id, source.columns[0].id, "new columns");

    pick(&mut h, &[1]);
    act(&mut h, "Exclude from Plotting dialogs");
    assert!(project(&h).workbook(book).unwrap().sheets[1].exclude_from_plotting);
    assert_eq!(h.state().doc.undo_label(), Some("Exclude from plotting dialogs"));
    click(&mut h, "Actions");
    assert!(h.query_by_label("\u{2713} Exclude from Plotting dialogs").is_some(), "ticked");
    click(&mut h, "\u{2713} Exclude from Plotting dialogs");
    settle(&mut h);
    assert!(!project(&h).workbook(book).unwrap().sheets[1].exclude_from_plotting);
}

fn act_in(h: &mut Harness<'static, SheafApp>, submenu: &str, item: &str) {
    click(h, "Actions");
    menu_path(h, &[submenu, item]);
}
