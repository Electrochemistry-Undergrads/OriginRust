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
