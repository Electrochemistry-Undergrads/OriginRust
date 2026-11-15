use super::*;

fn prepared(name: &str) -> (Harness<'static, SheafApp>, WindowId, Vec<ColumnId>) {
    let mut h = start(vec![data_file(name)]);
    let (book, _) = window_ids(&h);
    h.state_mut().doc.apply("Grouping metadata", |p| {
        for n in 0..3 {
            p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![n as f64; 4])).unwrap();
        }
        for (i, column) in p.workbook_mut(book).unwrap().sheet_mut().columns.iter_mut().skip(1).enumerate() {
            column.long_name = ["Beta", "Alpha", " beta ", "", "alpha"][i].into();
            column.units = if i < 2 { "mA" } else { "uA" }.into();
            column.comments = if i < 3 { "run\nfirst" } else { "run\nsecond" }.into();
            column.params.insert("Batch".into(), if i < 3 { "Batch A" } else { "Batch B" }.into());
        }
    });
    settle(&mut h);
    show_window(&mut h, "Book1 - sample");
    let corner = egui::pos2(text_pos(&h, "Long Name").x, text_pos(&h, "A(X1)").y);
    click_at(&mut h, corner);
    assert_eq!(book_ui(&h).selected.len(), 6);
    let ys = sheet(&h).columns.iter().skip(1).map(|c| c.id).collect();
    (h, book, ys)
}

fn number(h: &mut Harness<'static, SheafApp>, name: &str, value: &str) {
    let at = h.get_by_label(name).rect().center();
    double_click_at(h, at);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.event(egui::Event::Text(value.into()));
    h.key_press(egui::Key::Enter);
    settle(h);
}

#[test]
fn label_panels_follow_measured_origin_groups_and_frames_with_one_undo_redo() {
    let (mut h, book, ys) = prepared("label-panel-defaults");
    let before = project(&h).workbook(book).unwrap().clone();
    let revision = h.state().doc.revision();
    menu_path(&mut h, &["Plot", "Multi-Panel", "Multiple Panels by Label..."]);
    assert!(h.state().label_panels.is_some());
    assert_eq!(h.state().doc.revision(), revision);
    click(&mut h, "OK");
    assert!(h.state().label_panels.is_none());
    let graph = project(&h).graphs().last().unwrap();
    assert_eq!(h.state().active, Some(graph.id));
    assert_eq!(
        graph.layers.iter().map(|l| l.plots.iter().map(|p| p.y).collect::<Vec<_>>()).collect::<Vec<_>>(),
        [vec![ys[0], ys[2]], vec![ys[1], ys[4]], vec![ys[3]]]
    );
    assert_eq!(
        graph.layers.iter().map(|l| l.y_axis.title.as_deref().unwrap()).collect::<Vec<_>>(),
        ["Beta", "Alpha", ""]
    );
    assert!(graph.layers.iter().all(|l| l.link.is_none()));
    for (i, (left, top)) in [(0.12, 0.10), (0.535, 0.10), (0.12, 0.515)].into_iter().enumerate() {
        let f = graph.page_frame(i);
        assert!((f.left - left).abs() < 1e-9 && (f.top - top).abs() < 1e-9);
        assert!((f.width - 0.365).abs() < 1e-9 && (f.height - 0.365).abs() < 1e-9);
    }
    assert_eq!(project(&h).workbook(book).unwrap(), &before);
    menu(&mut h, "Edit", "Undo Panels by label");
    assert_eq!(project(&h).graphs().count(), 1);
    menu(&mut h, "Edit", "Redo Panels by label");
    assert_eq!(project(&h).graphs().count(), 2);
}

#[test]
fn label_panels_units_column_plot_manual_rows_and_selected_data_range() {
    let mut h = start(vec![data_file("label-panel-rows")]);
    let (book, _) = window_ids(&h);
    h.state_mut().doc.apply("Units", |p| {
        let columns = &mut p.workbook_mut(book).unwrap().sheet_mut().columns;
        columns[1].units = "mA".into();
        columns[2].units = "uA".into();
    });
    settle(&mut h);
    show_window(&mut h, "Book1 - sample");
    select_block(&mut h, "4.5", "10.5");
    menu_path(&mut h, &["Plot", "Multi-Panel", "Multiple Panels by Label..."]);
    click(&mut h, "Group Identifier");
    h.get_by_role_and_label(egui::accesskit::Role::Button, "Units").click();
    settle(&mut h);
    click(&mut h, "Plot Type");
    h.query_all_by_label("Column").last().unwrap().click();
    settle(&mut h);
    click(&mut h, "Auto Rows");
    number(&mut h, "Number of Rows", "1");
    number(&mut h, "Horizontal Gap", "8");
    number(&mut h, "Left Margin", "20");
    click(&mut h, "OK");
    let g = project(&h).graphs().last().unwrap();
    assert_eq!(g.layers.len(), 2);
    assert_eq!(g.layers.iter().map(|l| l.y_axis.title.as_deref().unwrap()).collect::<Vec<_>>(), ["mA", "uA"]);
    assert!(g.layers.iter().flat_map(|l| &l.plots).all(|p| p.kind == PlotKind::Column && p.rows == Some((1, 3))));
    assert!((g.page_frame(0).left - 0.20).abs() < 1e-9);
    assert!((g.page_frame(0).width - 0.31).abs() < 1e-9);
    assert!((g.page_frame(1).left - 0.59).abs() < 1e-9);
    assert!((g.page_frame(0).height - 0.78).abs() < 1e-9);
}

#[test]
fn label_panels_read_existing_parameters_and_full_comments_without_editing_them() {
    for (identifier, expected) in
        [("Parameter: Batch", ["Batch A", "Batch B"]), ("Comments", ["run\nfirst", "run\nsecond"])]
    {
        let (mut h, book, _) = prepared("label-panel-metadata");
        let before = project(&h).workbook(book).unwrap().clone();
        menu_path(&mut h, &["Plot", "Multi-Panel", "Multiple Panels by Label..."]);
        click(&mut h, "Group Identifier");
        h.get_by_role_and_label(egui::accesskit::Role::Button, identifier).click();
        settle(&mut h);
        click(&mut h, "Plot Type");
        click(&mut h, "Scatter");
        click(&mut h, "OK");
        let g = project(&h).graphs().last().unwrap();
        assert_eq!(g.layers.iter().map(|l| l.plots.len()).collect::<Vec<_>>(), [3, 2]);
        assert_eq!(g.layers.iter().map(|l| l.y_axis.title.as_deref().unwrap()).collect::<Vec<_>>(), expected);
        assert!(g.layers.iter().flat_map(|l| &l.plots).all(|p| p.kind == PlotKind::Scatter));
        assert_eq!(project(&h).workbook(book).unwrap(), &before);
    }
}

#[test]
fn label_panels_invalid_grid_or_margins_cancel_and_escape_do_not_edit() {
    let (mut h, _, _) = prepared("label-panel-cancel");
    let before = project(&h).clone();
    let revision = h.state().doc.revision();
    menu_path(&mut h, &["Plot", "Multi-Panel", "Multiple Panels by Label..."]);
    click(&mut h, "Auto Rows");
    click(&mut h, "Auto Columns");
    number(&mut h, "Number of Rows", "1");
    number(&mut h, "Number of Columns", "1");
    assert!(h.get_by_label("OK").accesskit_node().is_disabled());
    number(&mut h, "Number of Rows", "3");
    number(&mut h, "Left Margin", "90");
    number(&mut h, "Right Margin", "90");
    assert!(h.get_by_label("OK").accesskit_node().is_disabled());
    click(&mut h, "Cancel");
    assert!(h.state().label_panels.is_none());
    assert_eq!(project(&h), &before);
    assert_eq!(h.state().doc.revision(), revision);
    menu_path(&mut h, &["Plot", "Multi-Panel", "Multiple Panels by Label..."]);
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    assert!(h.state().label_panels.is_none());
    assert_eq!(project(&h), &before);
    assert_eq!(h.state().doc.revision(), revision);
}
