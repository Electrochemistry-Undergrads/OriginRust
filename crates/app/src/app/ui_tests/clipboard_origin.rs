use super::*;

#[test]
fn origin_missing_copy_and_empty_formula_label_row_work_on_both_table_paths() {
    for imported in [false, true] {
        let mut h = cell_regression_table(imported, "origin-clipboard");
        let a = column_id(&h, "A");
        let b = column_id(&h, "B");
        h.state_mut().doc.apply("Clipboard fixture", |p| {
            for c in &mut p.workbook_mut(p.locate(a).unwrap().0.id).unwrap().sheets[0].columns {
                c.data = ColumnData::numeric(vec![99.0; 3]);
            }
            let c = p.column_mut(a).unwrap();
            c.data = ColumnData::mixed_numbers(vec![0.30000000000000004, f64::NAN, 3.0]);
            c.long_name = "Potential".into();
            c.units = "V".into();
            c.comments = "First comment".into();
        });
        settle(&mut h);
        click_text(&mut h, "A(X1)");
        ctrl(&mut h, egui::Key::C);
        let text = h.state().copied.clone();
        assert_eq!(text, "0.30000000000000004\n--\n3\n");
        let at = worksheet_cell_pos(&h, 1, 0);
        click_at(&mut h, at);
        paste(&mut h, &text);
        let values = project(&h).column(b).unwrap().numeric().unwrap();
        assert_eq!(values[0].to_bits(), 0.30000000000000004_f64.to_bits());
        assert!(values[1].is_nan());
        assert_eq!(values[2], 3.0);
        ctrl(&mut h, egui::Key::Z);
        assert_eq!(project(&h).column(b).unwrap().numeric().unwrap(), [99.0; 3]);
        click_text(&mut h, "A(X1)");
        menu_path(&mut h, &["Edit", "Copy", "Copy including label rows"]);
        assert!(h.state().copied.starts_with("Potential\nV\nFirst comment\n\n"));
    }
}
