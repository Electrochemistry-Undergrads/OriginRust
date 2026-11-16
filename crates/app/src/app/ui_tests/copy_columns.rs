//! The column header's Copy Columns to... (T89, workbook-model.md "Copy Columns to").

use super::*;

#[test]
fn copy_columns_to_a_new_book_in_one_undo_step() {
    let mut h = start(vec![data_file("copy-columns")]);
    show_window(&mut h, "Book1 - sample.csv");
    let (b, c) = (column_id(&h, "B"), column_id(&h, "C"));
    let books = project(&h).workbooks().count();
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::CopyColumnsTo(vec![b, c]), &ctx);
    settle(&mut h);
    assert!(h.state().copy_columns.is_some(), "the window opened");
    click(&mut h, "OK");
    assert!(h.state().copy_columns.is_none(), "OK copies and closes");
    assert_eq!(project(&h).workbooks().count(), books + 1);
    let new = project(&h).workbooks().last().unwrap();
    let cols: Vec<(String, Vec<f64>)> =
        new.sheets[0].columns.iter().map(|c| (c.short_name.clone(), c.numeric().unwrap().to_vec())).collect();
    assert_eq!(cols, [("A".to_string(), vec![2.5, 4.5, 6.5, 8.5]), ("B".to_string(), vec![30.5, 20.5, 10.5, 0.5])]);
    assert_eq!(h.state().doc.undo_label(), Some("Copy columns"));
    h.state_mut().apply(Action::Undo, &ctx);
    settle(&mut h);
    assert_eq!(project(&h).workbooks().count(), books);
}
