//! Edit > Set As Begin, Set As End and Reset to Full Range (T87, workbook-model.md "Column data range").

use super::*;

#[test]
fn set_as_begin_and_end_blank_the_rows_outside_and_reset_brings_them_back() {
    let mut h = start(vec![data_file("data-range")]);
    show_window(&mut h, "Book1 - sample.csv");
    let b = column_id(&h, "B");
    click_text(&mut h, "4.5");
    menu_path(&mut h, &["Edit", "Set As Begin"]);
    assert_eq!(h.state().doc.undo_label(), Some("Set as begin"));
    click_text(&mut h, "6.5");
    menu_path(&mut h, &["Edit", "Set As End"]);
    let r = project(&h).column(b).unwrap().data_range.unwrap();
    assert_eq!((r.begin, r.end), (1, Some(2)));
    // The cells outside show blank; the values stay.
    assert!(h.query_by_label("2.5").is_none() && h.query_by_label("8.5").is_none());
    assert!(h.query_by_label("4.5").is_some());
    assert_eq!(project(&h).numeric(b).unwrap(), [2.5, 4.5, 6.5, 8.5]);
    menu_path(&mut h, &["Edit", "Reset to Full Range"]);
    assert_eq!(project(&h).column(b).unwrap().data_range, None);
    assert!(h.query_by_label("2.5").is_some());
    assert_eq!(h.state().doc.undo_label(), Some("Reset to full range"));
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::Undo, &ctx);
    settle(&mut h);
    assert!(project(&h).column(b).unwrap().data_range.is_some());
}
