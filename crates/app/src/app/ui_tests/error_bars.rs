use super::*;

fn pick(h: &mut Harness<'static, SheafApp>, field: &str, value: &str) {
    click(h, field);
    click(h, value);
}

#[test]
fn error_bars_can_use_an_unmarked_numeric_column_and_change_the_target() {
    let mut h = start(vec![data_file("choose-error")]);
    let (book, graph) = window_ids(&h);
    let err = h
        .state_mut()
        .doc
        .edit("Prepare errors", |p| {
            let err = p.add_column(book, 0, Designation::Disregard, ColumnData::numeric(vec![0.2; 4]))?;
            p.add_column(book, 0, Designation::Label, ColumnData::text(vec!["text".into(); 4]))?;
            Ok::<_, sheaf_core::project::ProjectError>(err)
        })
        .unwrap();
    settle(&mut h);
    menu_path(&mut h, &["Graph", "Add Error Bars..."]);
    assert!(h.get_by_label("OK").accesskit_node().is_disabled(), "no arbitrary column is preselected");
    pick(&mut h, "Add Error Bars to", "2: [Book1]sample!C (down)");
    click(&mut h, "Y Error Column");
    assert!(h.query_by_label("[Book1]sample!E").is_none(), "text columns are excluded");
    click(&mut h, "[Book1]sample!D");
    click(&mut h, "OK");
    let plots = &project(&h).graph(graph).unwrap().layers[0].plots;
    assert_eq!(plots[0].y_error, None);
    assert_eq!(plots[1].y_error, Some(err));
    assert_eq!(project(&h).column(err).unwrap().designation, Designation::Disregard);
}

#[test]
fn error_bars_choose_from_other_workbooks_and_preserve_other_layers_and_styles() {
    let mut h = start(vec![data_file("crossbook-error")]);
    let (_, graph) = window_ids(&h);
    let err = h
        .state_mut()
        .doc
        .edit("Prepare layers", |p| {
            let errors = p.add_workbook("Uncertainty");
            p.workbook_mut(errors).unwrap().name = "Errors".into();
            let err = p.add_column(errors, 0, Designation::Y, ColumnData::numeric(vec![0.3; 4]))?;
            let g = p.graph_mut(graph).unwrap();
            g.layers.push(g.layers[0].clone());
            g.layers[1].plots[1].rows = Some((1, 3));
            g.layers[1].plots[1].x_error = Some(err);
            g.layers[1].plots[1].error.cap_width_pt = 12.0;
            Ok::<_, sheaf_core::project::ProjectError>(err)
        })
        .unwrap();
    h.state_mut().graphs.get_mut(&graph).unwrap().active_layer = 1;
    h.state_mut().graphs.get_mut(&graph).unwrap().selected = Some((1, ObjectKind::Plot(1)));
    settle(&mut h);
    h.state_mut().focus(graph);
    settle(&mut h);
    let before = project(&h).graph(graph).unwrap().clone();
    menu_path(&mut h, &["Graph", "Add Error Bars..."]);
    pick(&mut h, "Y Error Column", "[Errors]Sheet1!A");
    click(&mut h, "OK");
    let mut expected = before.clone();
    expected.layers[1].plots[1].y_error = Some(err);
    assert_eq!(project(&h).graph(graph).unwrap(), &expected);
    menu(&mut h, "Edit", "Undo Add error bars");
    assert_eq!(project(&h).graph(graph).unwrap(), &before);
}

#[test]
fn error_bars_cancel_and_escape_leave_no_project_changes_or_undo_steps() {
    let mut h = start(vec![data_file("cancel-error")]);
    let before = project(&h).clone();
    let revision = h.state().doc.revision();
    let undo = h.state().doc.undo_label().map(str::to_owned);
    for escape in [false, true] {
        menu_path(&mut h, &["Graph", "Add Error Bars..."]);
        pick(&mut h, "Y Error Column", "[Book1]sample!A (t)");
        if escape {
            h.key_press(egui::Key::Escape);
            settle(&mut h);
        } else {
            click(&mut h, "Cancel");
        }
        assert!(h.state().error_bars.is_none());
        assert_eq!(project(&h), &before);
        assert_eq!(h.state().doc.revision(), revision);
        assert_eq!(h.state().doc.undo_label(), undo.as_deref());
    }
}

#[test]
fn error_bars_can_replace_an_existing_error_column_in_one_undo_step() {
    let mut h = start(vec![data_file("replace-error")]);
    let (book, graph) = window_ids(&h);
    let err = h
        .state_mut()
        .doc
        .edit("Prepare existing bars", |p| {
            let err = p.add_column(book, 0, Designation::YError, ColumnData::numeric(vec![0.5; 4]))?;
            let g = p.graph_mut(graph).unwrap();
            for plot in &mut g.layers[0].plots {
                plot.y_error = Some(err);
            }
            Ok::<_, sheaf_core::project::ProjectError>(err)
        })
        .unwrap();
    settle(&mut h);
    menu_path(&mut h, &["Graph", "Add Error Bars..."]);
    assert!(h.query_by_label("OK replaces the Y error column of this plot.").is_some());
    pick(&mut h, "Y Error Column", "[Book1]sample!C (down)");
    click(&mut h, "OK");
    let c = project(&h).workbook(book).unwrap().sheet().column_by_name("C").unwrap().id;
    let plots = &project(&h).graph(graph).unwrap().layers[0].plots;
    assert_eq!(plots[0].y_error, Some(c));
    assert_eq!(plots[1].y_error, Some(err), "the other plot keeps its errors");
    menu(&mut h, "Edit", "Undo Add error bars");
    assert!(project(&h).graph(graph).unwrap().layers[0].plots.iter().all(|p| p.y_error == Some(err)));
}

#[test]
fn error_bars_refuse_a_changed_plot_and_a_source_that_became_text() {
    let mut h = start(vec![data_file("stale-error")]);
    let (_, graph) = window_ids(&h);
    menu_path(&mut h, &["Graph", "Add Error Bars..."]);
    pick(&mut h, "Y Error Column", "[Book1]sample!C (down)");
    h.state_mut().doc.apply("Reorder plots", |p| p.graph_mut(graph).unwrap().layers[0].plots.swap(0, 1));
    settle(&mut h);
    assert!(h.get_by_label("OK").accesskit_node().is_disabled());
    assert!(h.query_by_label("The plot changed. Reopen Add Error Bars.").is_some());
    click(&mut h, "Cancel");
    menu_path(&mut h, &["Graph", "Add Error Bars..."]);
    pick(&mut h, "Y Error Column", "[Book1]sample!A (t)");
    h.state_mut().doc.apply("Change source type", |p| {
        let x = p.graph(graph).unwrap().layers[0].plots[0].x.unwrap();
        p.column_mut(x).unwrap().data = ColumnData::text(vec!["text".into(); 4]);
    });
    settle(&mut h);
    assert!(h.get_by_label("OK").accesskit_node().is_disabled());
    assert!(project(&h).graph(graph).unwrap().layers[0].plots.iter().all(|p| p.y_error.is_none()));
    click(&mut h, "Cancel");
}
