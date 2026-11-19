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
