//! Edit > Copy > Copy formula, Copy label rows, Copy formula and label rows, Copy All (T88,
//! workbook-model.md 13.4c): whole columns only, as Origin 2016's.

use super::*;
use sheaf_core::clipboard::CopyPart;

#[test]
fn the_copy_variants_need_whole_columns_and_copy_their_rows() {
    let mut h = start(vec![data_file("copy-variants")]);
    show_window(&mut h, "Book1 - sample.csv");
    let b = column_id(&h, "B");
    h.state_mut().doc.apply("Label", |p| p.column_mut(b).unwrap().long_name = "Bee".into());
    let book = h.state().books.keys().next().copied().unwrap();
    let ctx = h.ctx.clone();
    // Whole column B (a block of cells is refused; see the menu tests).
    click_text(&mut h, "B(Y1)");
    h.state_mut().apply(Action::Cells { book, op: CellOp::Copy(CopyKind::Part(CopyPart::LabelRows)) }, &ctx);
    settle(&mut h);
    assert_eq!(h.state().copied, "Bee\n\n\n\n");
    h.state_mut().apply(Action::Cells { book, op: CellOp::Copy(CopyKind::Part(CopyPart::All)) }, &ctx);
    settle(&mut h);
    assert_eq!(h.state().copied, "\nBee\n\n\n2.5\n4.5\n6.5\n8.5\n");
}
