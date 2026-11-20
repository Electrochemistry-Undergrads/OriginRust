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

#[test]
fn worksheet_header_switches_collapse_the_frozen_bands_and_reopen_from_the_sheet_tab() {
    let mut h = start(vec![data_file("corner-headers")]);
    show_window(&mut h, "Book1 - sample.csv");
    let a = column_id(&h, "A");
    let book = project(&h).locate(a).unwrap().0.id;
    h.state_mut().doc.apply("Freeze", |p| {
        p.workbook_mut(book).unwrap().sheets[0].frozen = sheaf_core::Frozen { rows: 1, columns: 1 };
        assert!(p.column_mut(a).unwrap().set_cell_text(0, "1.25"));
    });
    settle(&mut h);
    let before = sheet(&h).clone();
    let a_before = text_pos(&h, "1.25");
    let b_before = text_pos(&h, "2.5");
    let corner = egui::pos2(text_pos(&h, "Long Name").x, text_pos(&h, "A(X1)").y);
    right_click_at(&mut h, corner);
    menu_entry(&h, "\u{2713} Column Header").click();
    settle(&mut h);
    assert!(h.query_by_label("A(X1)").is_none());
    assert!(h.query_by_label("Long Name").is_some());
    assert!((a_before.y - text_pos(&h, "1.25").y - grid::ROW_HEIGHT).abs() < 0.6);

    let tab = sheet(&h).name.clone();
    let open_tab_view = |h: &mut Harness<'static, SheafApp>| {
        // The active sheet is also named in the Explorer. The tab is below the grid.
        let at =
            h.query_all_by_label(&tab).map(|node| node.rect().center()).max_by(|a, b| a.y.total_cmp(&b.y)).unwrap();
        right_click_at(h, at);
        menu_entry(h, "View").click();
        settle(h);
    };
    open_tab_view(&mut h);
    menu_entry(&h, "\u{2713} Row Header").click();
    settle(&mut h);
    assert!(h.query_by_label("Long Name").is_none());
    for (position, value) in [(a_before, "1.25"), (b_before, "2.5")] {
        let at = text_pos(&h, value);
        assert!((position.x - at.x - 84.0).abs() < 0.6);
        assert!((position.y - at.y - grid::ROW_HEIGHT).abs() < 0.6);
    }
    assert_eq!(sheet(&h).columns, before.columns);
    assert_eq!(sheet(&h).frozen, before.frozen);
    assert!(sheet(&h).view.row_labels && sheet(&h).view.long_name);
    assert_eq!(h.state().doc.undo_label(), Some("Worksheet view"));

    let path = data_file("corner-headers-roundtrip").with_extension("sheaf");
    assert!(h.state_mut().save_to(path.clone()));
    let mut reopened = start(vec![path]);
    show_window(&mut reopened, "Book1 - sample.csv");
    assert!(!sheet(&reopened).view.column_header && !sheet(&reopened).view.row_header);
    assert!(reopened.query_by_label("A(X1)").is_none());
    assert!(reopened.query_by_label("Long Name").is_none());
    assert!(reopened.query_by_label("1.25").is_some());

    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::Undo, &ctx);
    settle(&mut h);
    assert!(!sheet(&h).view.column_header && sheet(&h).view.row_header);
    h.state_mut().apply(Action::Redo, &ctx);
    settle(&mut h);
    assert!(!sheet(&h).view.row_header);
    for name in ["Column Header", "Row Header"] {
        open_tab_view(&mut h);
        menu_entry(&h, name).click();
        settle(&mut h);
    }
    assert_eq!(text_pos(&h, "1.25"), a_before);
    assert_eq!(text_pos(&h, "2.5"), b_before);
    assert_eq!(sheet(&h), &before);
}

#[test]
fn worksheet_cells_remain_selectable_when_every_header_band_is_hidden() {
    let mut h = start(vec![data_file("corner-all-headers-hidden")]);
    show_window(&mut h, "Book1 - sample.csv");
    let book = project(&h).workbooks().next().unwrap().id;
    h.state_mut().doc.apply("Hide headers", |p| {
        let sheet = &mut p.workbook_mut(book).unwrap().sheets[0];
        for switch in ViewSwitch::LABEL_ROWS.into_iter().chain([ViewSwitch::ColumnHeader, ViewSwitch::RowHeader]) {
            sheet.toggle_view_switch(switch);
        }
    });
    settle(&mut h);
    assert!(h.query_by_label("A(X1)").is_none());
    assert!(h.query_by_label("Long Name").is_none());
    click_text(&mut h, "2.5");
    assert_eq!(cells(&h), Sel::Cells { anchor: (1, 0), end: (1, 0) });
}

#[test]
fn page_break_preview_lines_follow_the_print_pages_and_the_view_menu_ticks() {
    let mut h = start(vec![data_file("corner-page-breaks")]);
    show_window(&mut h, "Book1 - sample.csv");
    let book = h.state().books.keys().next().copied().unwrap();
    let b = column_id(&h, "B");
    h.state_mut().doc.apply("Long", |p| {
        p.column_mut(b).unwrap().data = sheaf_core::ColumnData::numeric((0..400).map(f64::from).collect());
    });
    settle(&mut h);
    assert!(h.state().books[&book].page_break_rows().is_none(), "off on a new sheet");
    let labels: Vec<String> = crate::menus::corner_menu(&project(&h).workbook(book).unwrap().sheets[0])
        .iter()
        .filter_map(|e| match e {
            crate::menus::Entry::Item(i) => Some(i.label.clone()),
            _ => None,
        })
        .collect();
    assert!(labels.contains(&"Page Break Preview Lines".to_string()), "{labels:?}");
    menu_path(&mut h, &["View", "Page Break Preview Lines"]);
    settle(&mut h);
    assert!(project(&h).workbook(book).unwrap().sheets[0].view.page_breaks);
    assert_eq!(h.state().doc.undo_label(), Some("Worksheet view"));
    let rows = h.state().books[&book].page_break_rows().expect("the breaks are worked out");
    assert!(!rows.is_empty() && rows.iter().all(|&r| r < 399), "400 rows print on several pages: {rows:?}");
    assert!(h.state().menu_context().page_breaks, "the View menu ticks it");
}

#[test]
fn the_categories_row_lists_a_categorical_columns_categories() {
    let mut h = start(vec![data_file("corner-categories")]);
    show_window(&mut h, "Book1 - sample.csv");
    let book = h.state().books.keys().next().copied().unwrap();
    let b = column_id(&h, "B");
    h.state_mut().doc.apply("Groups", |p| {
        let c = p.column_mut(b).unwrap();
        c.data = sheaf_core::ColumnData::text(vec!["ctrl".into(), "dose".into(), "ctrl".into(), "high".into()]);
        c.categorical = true;
    });
    settle(&mut h);
    assert!(h.query_by_label("Categories").is_none(), "off on a new sheet");
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::Sheet { book, op: SheetOp::ToggleView(ViewSwitch::Categories) }, &ctx);
    settle(&mut h);
    assert!(h.query_by_label("Categories").is_some(), "the row title shows");
    assert!(h.query_by_label("ctrl, dose, high").is_some() || grid_has_text(&h, "ctrl, dose, high"));
    assert_eq!(h.state().doc.undo_label(), Some("Worksheet view"));
}

fn grid_has_text(h: &Harness<'static, SheafApp>, text: &str) -> bool {
    h.query_all_by_value(text).next().is_some()
}
