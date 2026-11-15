use super::*;

#[test]
fn displayed_and_full_precision_clipboard_text_both_paste_the_exact_numbers() {
    for imported in [false, true] {
        let mut h = cell_regression_table(imported, "clipboard-formats");
        let a = column_id(&h, "A");
        let b = column_id(&h, "B");
        h.state_mut().doc.apply("Clipboard formats fixture", |p| {
            let book = p.locate(a).unwrap().0.id;
            for c in &mut p.workbook_mut(book).unwrap().sheets[0].columns {
                c.data = ColumnData::numeric(vec![99.0; 3]);
            }
            let c = p.column_mut(a).unwrap();
            c.data = ColumnData::numeric(vec![0.30000000000000004, f64::NAN, 1.2345678901234567]);
            c.format = sheaf_core::format::NumberFormat::Decimal(2);
            c.long_name = "Potential".into();
            c.units = "V".into();
        });
        settle(&mut h);
        for (full, expected) in [(false, "0.30\r\n--\r\n1.23"), (true, "0.3\r\n--\r\n1.23456789012346")] {
            click_text(&mut h, "A(X1)");
            let before = h.state().doc.revision();
            if full {
                menu_path(&mut h, &["Edit", "Copy", "Copy full precision"]);
            } else {
                ctrl(&mut h, egui::Key::C);
            }
            assert_eq!(h.state().doc.revision(), before);
            assert_eq!(h.state().worksheet_clipboard.external_text(), Some(expected));
            let at = worksheet_cell_pos(&h, 1, 0);
            click_at(&mut h, at);
            paste(&mut h, &expected.replace("\r\n", "\n"));
            let values = project(&h).column(b).unwrap().numeric().unwrap();
            assert_eq!(values[0].to_bits(), 0.30000000000000004_f64.to_bits());
            assert!(values[1].is_nan());
            assert_eq!(values[2].to_bits(), 1.2345678901234567_f64.to_bits());
            ctrl(&mut h, egui::Key::Z);
            assert_eq!(project(&h).column(b).unwrap().numeric().unwrap(), [99.0; 3]);
        }
        click_text(&mut h, "A(X1)");
        menu_path(&mut h, &["Edit", "Copy", "Copy including label rows"]);
        assert_eq!(h.state().worksheet_clipboard.external_text(), Some("Potential\r\nV\r\n\r\n\r\n0.30\r\n--\r\n1.23"));
        click_text(&mut h, "A(X1)");
        ctrl(&mut h, egui::Key::X);
        let external = h.state().worksheet_clipboard.external_text().unwrap().replace("\r\n", "\n");
        assert!(project(&h).column(a).unwrap().is_empty());
        let at = worksheet_cell_pos(&h, 1, 0);
        click_at(&mut h, at);
        paste(&mut h, &external);
        assert_eq!(project(&h).column(b).unwrap().numeric().unwrap()[0].to_bits(), 0.30000000000000004_f64.to_bits());
        ctrl(&mut h, egui::Key::Z);
        ctrl(&mut h, egui::Key::Z);
        assert_eq!(project(&h).column(a).unwrap().numeric().unwrap()[0].to_bits(), 0.30000000000000004_f64.to_bits());
    }
}

#[test]
fn an_empty_text_cell_can_replace_an_existing_text_cell_and_undo() {
    let mut h = cell_regression_table(false, "empty-clipboard-format");
    let a = column_id(&h, "A");
    let b = column_id(&h, "B");
    h.state_mut().doc.apply("Empty text fixture", |p| {
        p.column_mut(a).unwrap().data = ColumnData::text(vec!["".into()]);
        p.column_mut(b).unwrap().data = ColumnData::text(vec!["old".into()]);
    });
    settle(&mut h);
    let at = worksheet_cell_pos(&h, 0, 0);
    click_at(&mut h, at);
    ctrl(&mut h, egui::Key::C);
    assert_eq!(h.state().worksheet_clipboard.external_text(), Some("\n"));
    let at = worksheet_cell_pos(&h, 1, 0);
    click_at(&mut h, at);
    paste(&mut h, "\n");
    assert_eq!(project(&h).column(b).unwrap().cell_text(0), "");
    ctrl(&mut h, egui::Key::Z);
    assert_eq!(project(&h).column(b).unwrap().cell_text(0), "old");
}

#[test]
fn copying_a_label_clears_the_previous_worksheet_payload_even_when_the_text_matches() {
    let mut h = cell_regression_table(false, "plain-label-clipboard");
    let a = column_id(&h, "A");
    let b = column_id(&h, "B");
    h.state_mut().doc.apply("Label clipboard fixture", |p| {
        let c = p.column_mut(a).unwrap();
        c.data = ColumnData::numeric(vec![0.30000000000000004]);
        c.format = sheaf_core::format::NumberFormat::Decimal(2);
        c.units = "0.30".into();
    });
    settle(&mut h);
    let at = worksheet_cell_pos(&h, 0, 0);
    click_at(&mut h, at);
    ctrl(&mut h, egui::Key::C);
    let book = project(&h).locate(a).unwrap().0.id;
    h.state_mut().books.get_mut(&book).unwrap().sel = Sel::Label { column: a, row: LabelRow::Units };
    settle(&mut h);
    ctrl(&mut h, egui::Key::C);
    assert!(h.state().worksheet_clipboard.external_text().is_none());
    let at = worksheet_cell_pos(&h, 1, 0);
    click_at(&mut h, at);
    paste(&mut h, "0.30");
    assert_eq!(project(&h).column(b).unwrap().numeric().unwrap()[0].to_bits(), 0.3_f64.to_bits());
}

#[test]
fn native_copy_events_keep_alt_precision_and_shift_labels() {
    let mut h = cell_regression_table(false, "native-copy-modifiers");
    let a = column_id(&h, "A");
    h.state_mut().doc.apply("Copy modifier fixture", |p| {
        let c = p.column_mut(a).unwrap();
        c.data = ColumnData::numeric(vec![0.30000000000000004]);
        c.format = sheaf_core::format::NumberFormat::Decimal(2);
        c.long_name = "Potential".into();
    });
    settle(&mut h);
    let at = worksheet_cell_pos(&h, 0, 0);
    click_at(&mut h, at);
    h.event_modifiers(egui::Event::Copy, egui::Modifiers::COMMAND | egui::Modifiers::ALT);
    settle(&mut h);
    assert_eq!(h.state().worksheet_clipboard.external_text(), Some("0.3"));
    h.event_modifiers(egui::Event::Copy, egui::Modifiers::COMMAND | egui::Modifiers::SHIFT);
    settle(&mut h);
    assert_eq!(h.state().worksheet_clipboard.external_text(), Some("Potential\r\n\r\n\r\n\r\n0.30"));
    h.event_modifiers(egui::Event::Copy, egui::Modifiers::COMMAND);
    settle(&mut h);
    assert_eq!(h.state().worksheet_clipboard.external_text(), Some("0.30"));
}
