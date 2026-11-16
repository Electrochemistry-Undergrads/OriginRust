//! The column header's Swap Columns... and Edit > Paste Transpose (T86, workbook-model.md "Swap
//! Columns and Paste Transpose").

use super::*;

#[test]
fn paste_transpose_pastes_rows_as_columns_in_one_undo_step() {
    let mut h = start(vec![data_file("paste-transpose")]);
    show_window(&mut h, "Book1 - sample.csv");
    let (b, c) = (column_id(&h, "B"), column_id(&h, "C"));
    click_text(&mut h, "4.5");
    menu_path(&mut h, &["Edit", "Paste Transpose"]);
    paste(&mut h, "7\t8\t9\n70\t80\t90\n");
    assert_eq!(project(&h).numeric(b).unwrap(), [2.5, 7.0, 8.0, 9.0]);
    assert_eq!(project(&h).numeric(c).unwrap(), [30.5, 70.0, 80.0, 90.0]);
    assert_eq!(h.state().doc.undo_label(), Some("Paste transpose"));
    // The next ordinary paste is not transposed.
    click_text(&mut h, "30.5");
    paste(&mut h, "1\t2\n");
    assert_eq!(project(&h).numeric(c).unwrap()[0], 1.0);
    assert_eq!(h.state().doc.undo_label(), Some("Paste"));
}

#[test]
fn swap_columns_exchanges_two_columns_in_one_undo_step() {
    let mut h = start(vec![data_file("swap-columns")]);
    show_window(&mut h, "Book1 - sample.csv");
    let (a, c) = (column_id(&h, "A"), column_id(&h, "C"));
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::SwapColumns(a, c), &ctx);
    settle(&mut h);
    let order: Vec<String> = sheet(&h).columns.iter().map(|c| c.short_name.clone()).collect();
    assert_eq!(order, ["C", "B", "A"]);
    assert_eq!(sheet(&h).columns[2].designation, sheaf_core::Designation::X, "A keeps its designation");
    assert_eq!(h.state().doc.undo_label(), Some("Swap columns"));
    h.state_mut().apply(Action::Undo, &ctx);
    settle(&mut h);
    let order: Vec<String> = sheet(&h).columns.iter().map(|c| c.short_name.clone()).collect();
    assert_eq!(order, ["A", "B", "C"]);
}
