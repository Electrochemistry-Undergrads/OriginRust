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
    let mut sheet = sheaf_core::Worksheet::new("Sheet1");
    sheet.toggle_view_switch(ViewSwitch::RowGrid);
    let labels: Vec<String> = crate::menus::corner_menu(&sheet)
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

#[test]
fn the_sampling_interval_row_appears_automatically_and_the_corner_switch_is_undoable() {
    let mut h = start(vec![data_file("corner-sampling")]);
    show_window(&mut h, "Book1 - sample.csv");
    assert!(h.query_by_label("Sampling Interval").is_none());
    let b = column_id(&h, "B");
    h.state_mut().doc.apply("Set", |p| {
        p.set_sampling_interval(&[b], 0.0, 0.5, "Time", "s").unwrap();
    });
    settle(&mut h);
    assert!(text_pos(&h, "Sampling Interval").y > text_pos(&h, "F(x)=").y);
    assert!(h.query_by_label("0.5 (s)").is_some());
    let before = project(&h).column(b).unwrap().clone();

    let corner = egui::pos2(text_pos(&h, "Long Name").x, text_pos(&h, "A(X1)").y);
    right_click_at(&mut h, corner);
    assert!(entry_enabled(&h, "\u{2713} Sampling Interval"));
    menu_entry(&h, "\u{2713} Sampling Interval").click();
    settle(&mut h);
    assert!(h.query_by_label("0.5 (s)").is_none());
    assert_eq!(h.state().doc.undo_label(), Some("Worksheet view"));
    assert_eq!(project(&h).column(b), Some(&before));

    let path = data_file("corner-sampling-roundtrip").with_extension("sheaf");
    assert!(h.state_mut().save_to(path.clone()));
    let mut reopened = start(vec![path]);
    show_window(&mut reopened, "Book1 - sample.csv");
    assert!(reopened.query_by_label("0.5 (s)").is_none());
    let sheet = project(&reopened).locate(b).unwrap().1;
    assert_eq!(sheet.view.sampling_interval, Some(false));
    assert_eq!(project(&reopened).column(b), Some(&before));

    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::Undo, &ctx);
    settle(&mut h);
    assert!(h.query_by_label("0.5 (s)").is_some());
    h.state_mut().apply(Action::Redo, &ctx);
    settle(&mut h);
    assert!(h.query_by_label("0.5 (s)").is_none());
}

#[test]
fn the_sampling_interval_label_can_be_copied_but_not_edited() {
    let mut h = start(vec![data_file("sampling-label-read-only")]);
    show_window(&mut h, "Book1 - sample.csv");
    let b = column_id(&h, "B");
    h.state_mut().doc.apply("Set", |p| {
        p.set_sampling_interval(&[b], 2.0, 0.5, "Time", "s").unwrap();
    });
    settle(&mut h);
    let book = project(&h).locate(b).unwrap().0.id;
    let row = LabelRow::SamplingInterval;
    let (a, _, _) = project(&h).locate(b).unwrap();
    assert_eq!(grid::label_text_full(&a.sheets[0].columns[0], &row), "");
    let label = "x0 = 2, dx = 0.5 (s)";
    assert_eq!(grid::label_text_full(project(&h).column(b).unwrap(), &row), label);
    assert!(h.query_by_label("B Sampling Interval").is_some());
    h.query_by_label("B Sampling Interval").unwrap().click();
    settle(&mut h);
    assert_eq!(cells(&h), Sel::Label { column: b, row });
    h.event(egui::Event::Text("replacement".into()));
    settle(&mut h);
    assert!(h.state().books[&book].editing_label.is_none());

    let original = project(&h).column(b).unwrap().clone();
    let ctx = h.ctx.clone();
    for op in [CellOp::Cut, CellOp::Clear, CellOp::PasteText("99".into())] {
        h.state_mut().apply(Action::Cells { book, op }, &ctx);
    }
    assert_eq!(h.state().doc.undo_label(), Some("Set"));
    assert_eq!(project(&h).column(b), Some(&original));
    h.state_mut().apply(Action::Cells { book, op: CellOp::Copy(CopyKind::Plain) }, &ctx);
    assert_eq!(h.state().copied, label);
}
