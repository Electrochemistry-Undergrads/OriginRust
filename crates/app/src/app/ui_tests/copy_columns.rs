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

#[test]
fn column_copy_dialog_ignores_filtered_rows_by_default_and_can_include_them() {
    let mut h = start(vec![data_file("copy-columns-visible")]);
    show_window(&mut h, "Book1 - sample.csv");
    let book = project(&h).workbooks().next().unwrap().id;
    h.state_mut().doc.edit("Filter rows", |p| p.set_row_filter(book, 0, "col(A) != 2")).unwrap();
    settle(&mut h);
    let source = project(&h).workbook(book).unwrap().clone();
    let open_copy = |h: &mut Harness<'static, SheafApp>| {
        show_window(h, "Book1 - sample.csv");
        right_click_text(h, "B(Y1)");
        menu_entry(h, "Copy Columns to...").click();
        settle(h);
        assert!(h.query_by_label("Ignore Hidden Rows").is_some());
    };
    let copied = |h: &Harness<'static, SheafApp>| {
        project(h).workbooks().last().unwrap().sheets[0].columns[0].numeric().unwrap().to_vec()
    };

    open_copy(&mut h);
    click(&mut h, "OK");
    assert_eq!(copied(&h), [2.5, 6.5, 8.5]);
    assert_eq!(h.state().doc.undo_label(), Some("Copy columns"));
    assert_eq!(project(&h).workbook(book), Some(&source));
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::Undo, &ctx);
    settle(&mut h);
    assert_eq!(project(&h).workbooks().count(), 1);

    open_copy(&mut h);
    click(&mut h, "Ignore Hidden Rows");
    click(&mut h, "OK");
    assert_eq!(copied(&h), [2.5, 4.5, 6.5, 8.5]);
    assert_eq!(project(&h).workbook(book), Some(&source));
}
