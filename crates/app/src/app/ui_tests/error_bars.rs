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
    click(&mut h, "Existing Column");
    assert!(h.get_by_label("OK").accesskit_node().is_disabled(), "no arbitrary column is preselected");
    pick(&mut h, "Add Error Bars to", "2: [Book1]sample!C (down)");
    click(&mut h, "Existing Column");
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
    click(&mut h, "Existing Column");
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
        click(&mut h, "Existing Column");
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
    click(&mut h, "Existing Column");
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
    click(&mut h, "Existing Column");
    pick(&mut h, "Y Error Column", "[Book1]sample!C (down)");
    h.state_mut().doc.apply("Reorder plots", |p| p.graph_mut(graph).unwrap().layers[0].plots.swap(0, 1));
    settle(&mut h);
    assert!(h.get_by_label("OK").accesskit_node().is_disabled());
    assert!(h.query_by_label("The plot changed. Reopen Add Error Bars.").is_some());
    click(&mut h, "Cancel");
    menu_path(&mut h, &["Graph", "Add Error Bars..."]);
    click(&mut h, "Existing Column");
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

#[test]
fn error_bars_are_generated_into_a_new_y_error_column_as_origin_does() {
    // T241: Percent of Data, Standard Deviation of Data and Square Root of Data.
    use crate::dialogs::error_bars::Source;
    let mut h = start(vec![data_file("generate-error")]);
    let (book, graph) = window_ids(&h);
    let y = project(&h).graph(graph).unwrap().layers[0].plots[0].y;
    let values: Vec<f64> = project(&h).column(y).unwrap().numeric().unwrap().to_vec();
    let columns = project(&h).workbook(book).unwrap().sheets[0].columns.len();
    menu_path(&mut h, &["Graph", "Add Error Bars..."]);
    assert!(!h.get_by_label("OK").accesskit_node().is_disabled(), "Percent of Data (5) is ready");
    h.get_by_label("Standard Deviation of Data");
    h.get_by_label("Square Root of Data");
    click(&mut h, "OK");
    settle(&mut h);
    let p = project(&h);
    let sheet = &p.workbook(book).unwrap().sheets[0];
    assert_eq!(sheet.columns.len(), columns + 1);
    let at = sheet.column_index(y).unwrap();
    let err = &sheet.columns[at + 1];
    assert_eq!(err.designation, Designation::YError, "right of the Y column, as Origin wants");
    assert!(err.comments.contains("Percent of Data 5%"), "{}", err.comments);
    assert!(err.formula.as_deref().is_some_and(|f| f.contains("5/100")), "{:?}", err.formula);
    let got = err.numeric().unwrap();
    for (e, v) in got.iter().zip(&values) {
        assert!((e - v.abs() * 0.05).abs() < 1e-12, "{e} vs {v}");
    }
    assert_eq!(p.graph(graph).unwrap().layers[0].plots[0].y_error, Some(err.id));
    assert_eq!(h.state().doc.undo_label(), Some("Add error bars"));
    let first = err.id;

    // Standard deviation times 2 on the second plot: one value on every row.
    menu_path(&mut h, &["Graph", "Add Error Bars..."]);
    pick(&mut h, "Add Error Bars to", "2: [Book1]sample!C (down)");
    h.state_mut().error_bars.as_mut().unwrap().set_source(Source::StdDev, 5.0, 2.0);
    settle(&mut h);
    click(&mut h, "OK");
    settle(&mut h);
    let p = project(&h);
    let plot = &p.graph(graph).unwrap().layers[0].plots[1];
    let ys: Vec<f64> = p.column(plot.y).unwrap().numeric().unwrap().iter().copied().filter(|v| v.is_finite()).collect();
    let n = ys.len() as f64;
    let mean = ys.iter().sum::<f64>() / n;
    let sd = (ys.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (n - 1.0)).sqrt();
    let err = p.column(plot.y_error.unwrap()).unwrap();
    assert!(err.numeric().unwrap().iter().all(|e| (e - 2.0 * sd).abs() < 1e-9), "{:?} vs {sd}", err.numeric());
    assert_eq!(p.graph(graph).unwrap().layers[0].plots[0].y_error, Some(first), "the first plot keeps its errors");

    // Undo takes the column and the link back in one step.
    h.state_mut().doc.undo();
    settle(&mut h);
    assert_eq!(project(&h).workbook(book).unwrap().sheets[0].columns.len(), columns + 1);
    assert_eq!(project(&h).graph(graph).unwrap().layers[0].plots[1].y_error, None);
}
