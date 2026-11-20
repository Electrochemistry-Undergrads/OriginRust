//! Edit > Paste Link, Paste Link Transpose and Remove Links (T211, ADR 0014).

use super::*;
use crate::actions::{CellOp, CopyKind};

#[test]
fn a_pasted_link_follows_its_source_until_typed_over_or_removed() {
    let mut h = start(vec![data_file("cell-links")]);
    show_window(&mut h, "Book1 - sample.csv");
    let book = h.state().books.keys().next().copied().unwrap();
    let (b, c) = {
        let s = &project(&h).workbook(book).unwrap().sheets[0];
        (s.columns[1].id, s.columns[2].id)
    };
    let ctx = h.ctx.clone();
    click_text(&mut h, "2.5");
    h.state_mut().apply(Action::Cells { book, op: CellOp::Copy(CopyKind::Plain) }, &ctx);
    click_text(&mut h, "10.5");
    h.state_mut().apply(Action::Cells { book, op: CellOp::PasteLink(false) }, &ctx);
    settle(&mut h);
    assert_eq!(h.state().doc.undo_label(), Some("Paste Link"));
    assert_eq!(project(&h).column(c).unwrap().numeric().unwrap()[2], 2.5, "the linked cell shows its source");
    let sheet = &project(&h).workbook(book).unwrap().sheets[0];
    assert_eq!(sheet.cell_link(c, 2).map(|l| (l.source, l.source_row)), Some((b, 0)));

    // A change of the source reaches the linked cell in the same undo step.
    h.state_mut().apply(Action::SetCell { column: b, row: 0, text: "99".into() }, &ctx);
    settle(&mut h);
    assert_eq!(project(&h).column(c).unwrap().numeric().unwrap()[2], 99.0);
    h.state_mut().apply(Action::Undo, &ctx);
    settle(&mut h);
    assert_eq!(project(&h).column(c).unwrap().numeric().unwrap()[2], 2.5, "undo restores both");

    // Remove Links keeps the value; typing into a linked cell ends its link.
    h.state_mut().apply(Action::Cells { book, op: CellOp::RemoveLinks }, &ctx);
    settle(&mut h);
    assert!(project(&h).workbook(book).unwrap().sheets[0].cell_link(c, 2).is_none());
    assert_eq!(project(&h).column(c).unwrap().numeric().unwrap()[2], 2.5);
    h.state_mut().apply(Action::Cells { book, op: CellOp::PasteLink(false) }, &ctx);
    h.state_mut().apply(Action::SetCell { column: c, row: 2, text: "7".into() }, &ctx);
    h.state_mut().apply(Action::SetCell { column: b, row: 0, text: "5".into() }, &ctx);
    settle(&mut h);
    assert_eq!(project(&h).column(c).unwrap().numeric().unwrap()[2], 7.0, "typed over: no longer linked");
}

#[test]
fn the_link_items_are_live_in_the_edit_cell_and_column_menus() {
    let mut h = start(vec![data_file("cell-links-menus")]);
    show_window(&mut h, "Book1 - sample.csv");
    menu(&mut h, "Edit", "Paste Link");
    settle(&mut h);
    close_menus(&mut h);
    let b = project(&h).workbooks().next().unwrap().sheets[0].columns[1].id;
    let cell = menus::cell_menu_for(false, (b, 0), false, false, false, false, (false, false));
    for label in ["Paste Link", "Paste Link Transpose", "Remove Links"] {
        assert!(
            cell.iter().any(|e| matches!(e, menus::Entry::Item(i) if i.label == label && i.cmd.is_some())),
            "{label}"
        );
    }
}
