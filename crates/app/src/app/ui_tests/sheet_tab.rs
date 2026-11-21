//! The sheet tab's Duplicate Without Data and Sheet Name to Show (T90, workbook-model.md "Sheet tab
//! names").

use super::*;
use sheaf_core::sheet_tab::SheetLabel;

/// T277: the sheet tab's Activate This Sheet in All Books in Folder.
#[test]
fn a_sheet_becomes_active_in_the_other_books_of_the_folder() {
    let mut h = start(vec![data_file("sheet-tab")]);
    show_window(&mut h, "Book1 - sample.csv");
    let book = h.state().books.keys().next().copied().unwrap();
    let mut others = Vec::new();
    h.state_mut().doc.apply("Books", |p| {
        for name in ["Book2", "Book3"] {
            let b = p.add_workbook(name);
            p.workbook_mut(b).unwrap().sheets.push(sheaf_core::Worksheet::new(if name == "Book2" {
                "Fit"
            } else {
                "Other"
            }));
            others.push(b);
        }
        let wb = p.workbook_mut(book).unwrap();
        wb.sheets.push(sheaf_core::Worksheet::new("Fit"));
        wb.active_sheet = 1;
    });
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::Sheet { book, op: SheetOp::ActivateInFolder }, &ctx);
    settle(&mut h);
    let p = project(&h);
    assert_eq!(p.workbook(others[0]).unwrap().active_sheet, 1, "Book2's Fit sheet is active");
    assert_eq!(p.workbook(others[1]).unwrap().active_sheet, 0, "Book3 has no Fit sheet");
    let entries = crate::menus::sheet_tab_menu(&p.workbook(book).unwrap().sheets[1]);
    assert!(entries.iter().any(|e| matches!(e, crate::menus::Entry::Item(i)
        if i.label == "Activate This Sheet in All Books in Folder"
            && i.cmd == Some(crate::menus::Cmd::Sheet(SheetOp::ActivateInFolder)))));
}

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
