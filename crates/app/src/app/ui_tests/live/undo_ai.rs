use super::*;

fn call(a: &mut Assistant, h: &mut Harness<'static, SheafApp>, name: &str, args: Value) {
    let reply = a.call(h, name, args);
    assert_eq!(reply["isError"], false, "{name}: {reply}");
}

fn dispatch(h: &mut Harness<'static, SheafApp>) {
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::UndoAiChanges, &ctx);
}

fn availability(h: &Harness<'static, SheafApp>) -> bool {
    menus::resolve(&menus::Cmd::UndoAiChanges, &h.state().menu_context()).is_ok()
}

#[test]
fn undo_ai_changes_undoes_a_whole_batch_restores_saved_state_and_keeps_exports() {
    let dirs = Dirs::new("undoaibatch");
    let mut h = start_link(&dirs, false);
    tick_link(&mut h);
    assert!(!availability(&h));
    let mut a = Assistant::start(&dirs);
    call(&mut a, &mut h, "import_files", json!({"files": [dirs.csv()]}));
    let book = project(&h).workbooks().next().unwrap().id;
    let y = project(&h).workbook(book).unwrap().sheet().columns[1].id;
    call(&mut a, &mut h, "plot", json!({"y": ["B"]}));
    call(&mut a, &mut h, "set_column_values", json!({"column": "B", "formula": "99"}));
    let exported = dirs.root().join("ai-export.csv");
    call(&mut a, &mut h, "export_data", json!({"path": exported}));
    assert!(h.state().doc.dirty && availability(&h));
    assert_eq!(project(&h).column(y).unwrap().numeric().unwrap(), [99.0; 4]);
    menu(&mut h, "Edit", "Undo AI Changes");
    assert!(project(&h).windows.is_empty());
    assert!(!h.state().doc.dirty, "the saved initial project was restored");
    assert!(h.state().doc.undo_label().is_none() && !availability(&h));
    assert!(h.state().status.contains("Undone 3 AI changes"));
    assert!(h.state().live.active(), "undo does not turn the endpoint off");
    assert!(std::fs::read_to_string(&exported).unwrap().contains("99"));

    // The short no-window bar keeps both ordinary Redo and the disabled AI command reachable.
    click(&mut h, "File");
    assert!(is_disabled(&h, "Undo AI Changes"));
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    let redo = h.state().doc.redo_label().unwrap().to_string();
    menu(&mut h, "File", &format!("Redo {redo}"));
    assert_eq!(project(&h).workbooks().count(), 1);
    assert_eq!(graph_count(&h), 0);
    let redo = h.state().doc.redo_label().unwrap().to_string();
    menu(&mut h, "Edit", &format!("Redo {redo}"));
    assert_eq!(graph_count(&h), 1);
    let redo = h.state().doc.redo_label().unwrap().to_string();
    menu(&mut h, "Edit", &format!("Redo {redo}"));
    assert_eq!(project(&h).column(y).unwrap().numeric().unwrap(), [99.0; 4]);
    assert!(availability(&h));
    a.end();
}

#[test]
fn undo_ai_changes_stops_at_a_manual_step_and_rechecks_a_stale_action() {
    let dirs = Dirs::new("undoaimanual");
    let mut h = start_link(&dirs, false);
    tick_link(&mut h);
    let mut a = Assistant::start(&dirs);
    call(&mut a, &mut h, "import_files", json!({"files": [dirs.csv()]}));
    let book = project(&h).workbooks().next().unwrap().id;
    let y = project(&h).workbook(book).unwrap().sheet().columns[1].id;
    h.state_mut().doc.apply("Owner edit", |p| {
        p.column_mut(y).unwrap().data = ColumnData::numeric(vec![42.0, 43.0, 44.0, 45.0]);
    });
    call(&mut a, &mut h, "set_column_values", json!({"column": "B", "formula": "9"}));
    call(&mut a, &mut h, "plot", json!({"y": ["B"]}));
    menu(&mut h, "Edit", "Undo AI Changes");
    assert_eq!(project(&h).column(y).unwrap().numeric().unwrap(), [42.0, 43.0, 44.0, 45.0]);
    assert_eq!(graph_count(&h), 0);
    assert_eq!(h.state().doc.undo_label(), Some("Owner edit"));
    assert!(h.state().doc.dirty && !availability(&h));
    let before = project(&h).clone();
    let revision = h.state().doc.revision();
    dispatch(&mut h);
    assert_eq!(project(&h), &before);
    assert_eq!(h.state().doc.revision(), revision);
    assert_eq!(h.state().doc.undo_label(), Some("Owner edit"));
    a.end();
}

#[test]
fn undo_ai_changes_waits_for_cell_label_editors_and_dialogs() {
    let dirs = Dirs::new("undoaiguards");
    let mut h = start_link(&dirs, false);
    tick_link(&mut h);
    let mut a = Assistant::start(&dirs);
    call(&mut a, &mut h, "import_files", json!({"files": [dirs.csv()]}));
    settle(&mut h);
    let book = project(&h).workbooks().next().unwrap().id;
    let column = project(&h).workbook(book).unwrap().sheet().columns[0].id;
    let before = project(&h).clone();
    let revision = h.state().doc.revision();
    h.state_mut().books.entry(book).or_default().editing = Some((column, 0, "pending cell".into()));
    assert!(!availability(&h));
    dispatch(&mut h);
    assert_eq!(project(&h), &before);
    h.state_mut().books.get_mut(&book).unwrap().editing = None;
    h.state_mut().books.get_mut(&book).unwrap().editing_label =
        Some((column, LabelRow::LongName, "pending label".into()));
    assert!(!availability(&h));
    dispatch(&mut h);
    assert_eq!(project(&h), &before);
    h.state_mut().books.get_mut(&book).unwrap().editing_label = None;
    h.state_mut().about = true;
    assert!(!availability(&h));
    dispatch(&mut h);
    assert_eq!(project(&h), &before);
    assert_eq!(h.state().doc.revision(), revision);
    h.state_mut().about = false;
    assert!(availability(&h));
    menu(&mut h, "Edit", "Undo AI Changes");
    assert!(project(&h).windows.is_empty());
    a.end();
}

#[test]
fn undo_ai_changes_uses_only_steps_retained_by_the_ordinary_undo_limit() {
    let dirs = Dirs::new("undoailimit");
    let mut h = start_link(&dirs, false);
    h.state_mut().doc.set_limit(2);
    tick_link(&mut h);
    let mut a = Assistant::start(&dirs);
    call(&mut a, &mut h, "import_files", json!({"files": [dirs.csv()]}));
    let book = project(&h).workbooks().next().unwrap().id;
    let y = project(&h).workbook(book).unwrap().sheet().columns[1].id;
    call(&mut a, &mut h, "plot", json!({"y": ["B"]}));
    call(&mut a, &mut h, "set_column_values", json!({"column": "B", "formula": "99"}));
    menu(&mut h, "Edit", "Undo AI Changes");
    assert_eq!(project(&h).workbooks().count(), 1, "the oldest AI import is beyond the undo limit");
    assert_eq!(graph_count(&h), 0);
    assert_eq!(project(&h).column(y).unwrap().numeric().unwrap(), [2.5, 4.5, 6.5, 8.5]);
    assert!(h.state().doc.dirty, "the saved initial state was not reachable");
    assert!(h.state().doc.undo_label().is_none() && !availability(&h));
    assert!(h.state().status.contains("Undone 2 AI changes"));
    a.end();
}
