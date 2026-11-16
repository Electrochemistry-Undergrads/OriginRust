//! The worksheet corner menu's View switches (T92, workbook-model.md "Worksheet corner menu").

use super::*;
use sheaf_core::sheet_view::ViewSwitch;

#[test]
fn the_corner_switches_hide_label_rows_and_their_titles() {
    let mut h = start(vec![data_file("corner-menu")]);
    show_window(&mut h, "Book1 - sample.csv");
    let book = h.state().books.keys().next().copied().unwrap();
    assert!(h.query_by_label("Units").is_some(), "a new sheet shows the Units row");
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::Sheet { book, op: SheetOp::ToggleView(ViewSwitch::Units) }, &ctx);
    settle(&mut h);
    assert!(h.query_by_label("Units").is_none(), "the Units row is hidden");
    assert!(h.query_by_label("Long Name").is_some());
    assert_eq!(h.state().doc.undo_label(), Some("Worksheet view"));
    // Row Labels off: the titles go, the rows stay.
    h.state_mut().apply(Action::Sheet { book, op: SheetOp::ToggleView(ViewSwitch::RowLabels) }, &ctx);
    settle(&mut h);
    assert!(h.query_by_label("Long Name").is_none());
    let view = project(&h).workbook(book).unwrap().sheets[0].view;
    assert!(!view.units && !view.row_labels && view.long_name && view.column_grid);
    h.state_mut().apply(Action::Undo, &ctx);
    h.state_mut().apply(Action::Undo, &ctx);
    settle(&mut h);
    assert!(h.query_by_label("Units").is_some() && h.query_by_label("Long Name").is_some());
}

#[test]
fn the_corner_menu_ticks_what_shows() {
    let mut view = sheaf_core::sheet_view::SheetView::default();
    view.toggle(ViewSwitch::RowGrid);
    let labels: Vec<String> = crate::menus::corner_menu(&view)
        .iter()
        .filter_map(|e| match e {
            crate::menus::Entry::Item(i) => Some(i.label.clone()),
            _ => None,
        })
        .collect();
    assert!(labels.contains(&"\u{2713} Long Name".to_string()), "{labels:?}");
    assert!(labels.contains(&"Row Grid".to_string()), "{labels:?}");
    assert!(labels.contains(&"\u{2713} Column Grid".to_string()), "{labels:?}");
}
