//! File > Import > Re-Import Directly (T98, workbook-model.md "Re-Import").

use super::*;

#[test]
fn reimport_directly_reads_the_changed_file_into_the_same_columns() {
    let path = data_file("reimport");
    let mut h = start(vec![path.clone()]);
    show_window(&mut h, "Book1 - sample.csv");
    let book = h.state().books.keys().next().copied().unwrap();
    let b = column_id(&h, "B");
    let menu = h.state().menu_context();
    assert!(crate::menus::resolve(&crate::menus::Cmd::Reimport(false), &menu).is_ok(), "an imported book");
    // The file changes: new values, one row more and a fourth column.
    std::fs::write(&path, "t,up,down,extra\n1,3.5,30.5,9\n2,5.5,20.5,9\n3,7.5,10.5,9\n4,9.5,0.5,9\n5,11.5,-9.5,9\n")
        .unwrap();
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::Reimport { book, choose: false }, &ctx);
    settle(&mut h);
    assert_eq!(project(&h).column(b).unwrap().numeric().unwrap(), [3.5, 5.5, 7.5, 9.5, 11.5]);
    assert_eq!(project(&h).workbook(book).unwrap().sheet().columns.len(), 3, "the extra column is not added");
    assert_eq!(h.state().doc.undo_label(), Some("Re-import"));
    h.state_mut().apply(Action::Undo, &ctx);
    settle(&mut h);
    assert_eq!(project(&h).column(b).unwrap().numeric().unwrap(), [2.5, 4.5, 6.5, 8.5]);
}

#[test]
fn a_book_made_in_sheaf_has_no_reimport() {
    let mut h = start(vec![]);
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::NewWorkbook, &ctx);
    settle(&mut h);
    let menu = h.state().menu_context();
    assert!(crate::menus::resolve(&crate::menus::Cmd::Reimport(false), &menu).is_err());
}
