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
