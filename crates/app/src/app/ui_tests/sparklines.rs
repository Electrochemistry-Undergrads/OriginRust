//! Worksheet sparklines (T175, workbook-model.md "Sparklines").

use super::*;
use sheaf_core::sheet_view::ViewSwitch;

#[test]
fn add_sparklines_marks_the_columns_and_shows_the_row_in_one_step() {
    let mut h = start(vec![data_file("sparklines")]);
    show_window(&mut h, "Book1 - sample.csv");
    let book = h.state().books.keys().next().copied().unwrap();
    assert!(h.query_by_label("Sparklines").is_none(), "a new sheet has no Sparklines row");
    click_text(&mut h, "B(Y1)");
    let b = column_id(&h, "B");
    let action = menus::resolve(&menus::Cmd::AddSparklines, &h.state().menu_context()).unwrap();
    let ctx = h.ctx.clone();
    h.state_mut().apply(action, &ctx);
    settle(&mut h);
    assert!(project(&h).column(b).unwrap().sparkline);
    assert!(!project(&h).column(column_id(&h, "C")).unwrap().sparkline, "only the selected column");
    assert!(project(&h).workbook(book).unwrap().sheets[0].view.sparklines);
    assert!(h.query_by_label("Sparklines").is_some(), "the row and its title show");
    assert!(text_pos(&h, "Sparklines").y > text_pos(&h, "F(x)=").y);
    assert_eq!(h.state().doc.undo_label(), Some("Add or Update Sparklines"));
    // The corner switch hides the row again; the columns keep their sparklines.
    h.state_mut().apply(Action::Sheet { book, op: SheetOp::ToggleView(ViewSwitch::Sparklines) }, &ctx);
    settle(&mut h);
    assert!(h.query_by_label("Sparklines").is_none());
    assert!(project(&h).column(b).unwrap().sparkline);
    h.state_mut().apply(Action::Undo, &ctx);
    h.state_mut().apply(Action::Undo, &ctx);
    settle(&mut h);
    assert!(!project(&h).column(b).unwrap().sparkline);
    assert!(h.query_by_label("Sparklines").is_none());
}

#[test]
fn the_column_header_menu_offers_add_or_update_sparklines() {
    let menu = crate::menus::column_header_menu();
    assert!(menu.iter().any(|e| matches!(e, crate::menus::Entry::Item(i)
        if i.label == "Add or Update Sparklines..." && i.cmd == Some(crate::menus::Cmd::AddSparklines))));
}

#[test]
fn a_sparkline_spans_the_cell_from_the_first_to_the_last_value() {
    let cell = egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(66.0, 32.0));
    let points = crate::grid::sparkline_points(&[f64::NAN, 1.0, 3.0, 2.0, f64::NAN], cell).unwrap();
    assert_eq!(points.len(), 3);
    // Inside a 3 px margin: the lowest value at the bottom, the highest at the top.
    assert_eq!((points[0].x, points[0].y), (13.0, 49.0));
    assert_eq!((points[1].x, points[1].y), (43.0, 23.0));
    assert_eq!((points[2].x, points[2].y), (73.0, 36.0));
    assert!(crate::grid::sparkline_points(&[1.0, f64::NAN], cell).is_none());
    // Many rows are thinned to about one point per pixel, keeping the last.
    let many: Vec<f64> = (0..10_000).map(|i| (i as f64).sin()).collect();
    let points = crate::grid::sparkline_points(&many, cell).unwrap();
    assert!(points.len() <= 62 && points.last().unwrap().x == 73.0, "{}", points.len());
}
