//! The sheet tab's Duplicate Without Data and Sheet Name to Show (T90, workbook-model.md "Sheet tab
//! names").

use super::*;
use sheaf_core::sheet_tab::SheetLabel;

#[test]
fn duplicate_without_data_and_the_long_name_on_the_tab() {
    let mut h = start(vec![data_file("sheet-tab")]);
    show_window(&mut h, "Book1 - sample.csv");
    let book = h.state().books.keys().next().copied().unwrap();
    h.state_mut().doc.apply("Long name", |p| p.workbook_mut(book).unwrap().sheets[0].long_name = "Scan".into());
    let ctx = h.ctx.clone();
    // Long Name: the tab shows the long name, one undo step.
    h.state_mut().apply(Action::Sheet { book, op: SheetOp::ShowName(SheetLabel::Long) }, &ctx);
    settle(&mut h);
    assert!(h.query_by_label("Scan").is_some(), "the tab shows the long name");
    assert_eq!(h.state().doc.undo_label(), Some("Sheet name to show"));
    // Duplicate Without Data: a new last sheet with the columns and no values, active.
    h.state_mut().apply(Action::Sheet { book, op: SheetOp::DuplicateWithoutData }, &ctx);
    settle(&mut h);
    let b = project(&h).workbook(book).unwrap();
    assert_eq!(b.sheets.len(), 2);
    assert_eq!(b.active_sheet, 1);
    let copy = &b.sheets[1];
    assert_eq!((copy.name.as_str(), copy.long_name.as_str()), ("Sheet1", "Scan 1"), "the first free SheetN");
    assert_eq!(copy.columns.len(), b.sheets[0].columns.len());
    assert!(copy.columns.iter().all(|c| c.data.is_empty()));
    assert_eq!(h.state().doc.undo_label(), Some("Duplicate sheet without data"));
    h.state_mut().apply(Action::Undo, &ctx);
    settle(&mut h);
    assert_eq!(project(&h).workbook(book).unwrap().sheets.len(), 1);
}
