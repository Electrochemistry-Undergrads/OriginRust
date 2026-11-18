//! Column > Set Sampling Interval... and Show X Column... (T84, workbook-model.md "Sampling interval").

use super::*;

#[test]
fn set_sampling_interval_fills_in_the_current_one_and_sets_it_in_one_undo_step() {
    let mut h = start(vec![data_file("sampling-set")]);
    show_window(&mut h, "Book1 - sample.csv");
    let b = column_id(&h, "B");
    h.state_mut().doc.apply("Set", |p| {
        p.set_sampling_interval(&[b], 0.0, 0.5, "", "").unwrap();
    });
    click_text(&mut h, "B(Y1)");
    menu_path(&mut h, &["Column", "Set Sampling Interval..."]);
    assert!(h.state().sampling.is_some(), "the window opened");
    // Units, then Long Name.
    let boxes = h.query_all_by_role(egui::accesskit::Role::TextInput).count();
    assert!(boxes >= 2, "{boxes}");
    h.query_all_by_role(egui::accesskit::Role::TextInput).nth(boxes - 2).unwrap().focus();
    settle(&mut h);
    h.event(egui::Event::Text("s".into()));
    settle(&mut h);
    h.query_all_by_role(egui::accesskit::Role::TextInput).nth(boxes - 1).unwrap().focus();
    settle(&mut h);
    h.event(egui::Event::Text("Time".into()));
    settle(&mut h);
    click(&mut h, "OK");
    assert!(h.state().sampling.is_none(), "OK sets and closes");
    let s = project(&h).column(b).unwrap().sampling.clone().unwrap();
    assert_eq!((s.x0, s.inc, s.units.as_str(), s.long_name.as_str()), (0.0, 0.5, "s", "Time"));
    assert_eq!(h.state().doc.undo_label(), Some("Set sampling interval"));
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::Undo, &ctx);
    settle(&mut h);
    assert_eq!(project(&h).column(b).unwrap().sampling.as_ref().map(|s| s.units.as_str()), Some(""));
}

#[test]
fn show_x_column_with_clear_inserts_an_x_column_left_of_the_column() {
    let mut h = start(vec![data_file("sampling-show")]);
    show_window(&mut h, "Book1 - sample.csv");
    let b = column_id(&h, "B");
    let before = project(&h).locate(b).unwrap().1.columns.len();
    h.state_mut().doc.apply("Set", |p| {
        p.set_sampling_interval(&[b], 10.0, 2.0, "Time", "s").unwrap();
    });
    click_text(&mut h, "B(Y1)");
    menu_path(&mut h, &["Column", "Show X Column..."]);
    assert!(h.state().show_x.is_some(), "the window opened");
    click(&mut h, "Clear Sampling Interval");
    click(&mut h, "OK");
    assert!(h.state().show_x.is_none(), "OK shows and closes");
    let (_, sheet, _) = project(&h).locate(b).unwrap();
    let at = sheet.column_index(b).unwrap();
    let x = &sheet.columns[at - 1];
    assert_eq!(x.designation, sheaf_core::Designation::X);
    assert_eq!((x.long_name.as_str(), x.units.as_str()), ("Time", "s"));
    let rows = sheet.columns[at].data.len();
    assert_eq!(x.numeric().unwrap(), (0..rows).map(|i| 10.0 + 2.0 * i as f64).collect::<Vec<_>>());
    assert_eq!(sheet.columns[at].sampling, None, "cleared");
    assert_eq!(h.state().doc.undo_label(), Some("Show X column"));
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::Undo, &ctx);
    settle(&mut h);
    assert_eq!(project(&h).locate(b).unwrap().1.columns.len(), before);
    assert!(project(&h).column(b).unwrap().sampling.is_some());
}

#[test]
fn show_x_column_of_a_column_without_an_interval_says_so_and_changes_nothing() {
    let mut h = start(vec![data_file("sampling-none")]);
    show_window(&mut h, "Book1 - sample.csv");
    let b = column_id(&h, "B");
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::ShowXColumn { columns: vec![b] }, &ctx);
    settle(&mut h);
    click(&mut h, "OK");
    assert!(h.state().show_x.is_some(), "the window stays open with the reason");
    assert!(h.query_by_label_contains("has no sampling interval").is_some());
    assert_eq!(h.state().doc.undo_label(), Some("Import data"), "no new undo step");
}
