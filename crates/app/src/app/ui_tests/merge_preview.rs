use super::*;
use sheaf_core::project::Folder;

fn fixture(tag: &str) -> Harness<'static, SheafApp> {
    let mut h = start(vec![data_file(tag)]);
    let y = project(&h).workbooks().next().unwrap().sheet().columns[1].id;
    for _ in 0..2 {
        h.state_mut().doc.apply("Plot", |p| p.plot_columns(Template::Line, &[y]).unwrap());
    }
    settle(&mut h);
    show_window(&mut h, "Graph1 - sample");
    h
}

fn open(h: &mut Harness<'static, SheafApp>) {
    menu_path(h, &["Graph", "Merge Graph Windows..."]);
    assert!(h.state().merge_graphs.is_some());
}

fn has_image(h: &Harness<'static, SheafApp>) -> bool {
    h.query_by_label("Merged graph preview").is_some()
}

fn number(h: &mut Harness<'static, SheafApp>, label: &str, value: &str) {
    let at = h.get_by_label(label).rect().center();
    double_click_at(h, at);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.event(egui::Event::Text(value.into()));
    h.key_press(egui::Key::Enter);
    settle(h);
}

#[test]
fn manual_preview_and_cancel_preserve_project_history_redo_and_dirty_state() {
    let mut h = fixture("merge-preview-cancel");
    h.state_mut().doc.apply("Temporary graph", |p| {
        p.add_graph(Template::Line);
    });
    h.state_mut().doc.undo();
    h.state_mut().doc.mark_saved();
    settle(&mut h);
    let before = project(&h).clone();
    let revision = h.state().doc.revision();
    let undo = h.state().doc.undo_label().map(str::to_owned);
    let redo = h.state().doc.redo_label().map(str::to_owned);
    open(&mut h);
    assert!(!has_image(&h));
    assert_eq!(h.get_by_label("Auto Preview").accesskit_node().toggled(), Some(egui::accesskit::Toggled::False));
    click(&mut h, "Preview");
    assert!(
        has_image(&h),
        "summary={}, prompt={}, revision={}",
        h.query_by_label("3 source graphs, 3 layers").is_some(),
        h.query_by_label("Click Preview to see the merged graph.").is_some(),
        h.state().doc.revision()
    );
    assert!(h.query_by_label("3 source graphs, 3 layers").is_some());
    click(&mut h, "Keep Source Graphs");
    assert!(!has_image(&h));
    click(&mut h, "Preview");
    assert!(has_image(&h));
    click(&mut h, "Cancel");
    assert_eq!(project(&h), &before);
    assert_eq!(h.state().doc.revision(), revision);
    assert_eq!(h.state().doc.undo_label(), undo.as_deref());
    assert_eq!(h.state().doc.redo_label(), redo.as_deref());
    assert!(!h.state().doc.dirty);
}

#[test]
fn auto_preview_tracks_layout_and_source_ticks_but_ok_is_the_only_undo_step() {
    let mut h = fixture("merge-preview-auto");
    let before = project(&h).clone();
    let revision = h.state().doc.revision();
    open(&mut h);
    click(&mut h, "Auto Preview");
    assert!(has_image(&h));
    number(&mut h, "Number of Rows", "3");
    assert!(has_image(&h));
    h.get(By::new().role(egui::accesskit::Role::ComboBox).value("All in Project")).click();
    settle(&mut h);
    click(&mut h, "Specified");
    click(&mut h, "Graph1");
    assert!(has_image(&h));
    assert!(h.query_by_label("2 source graphs, 2 layers").is_some());
    assert_eq!(project(&h), &before);
    assert_eq!(h.state().doc.revision(), revision);
    click(&mut h, "OK");
    assert_eq!(project(&h).graphs().count(), 4);
    assert_eq!(project(&h).graphs().last().unwrap().layers.len(), 2);
    assert_eq!(h.state().doc.undo_label(), Some("Merge Graph Windows"));
    menu(&mut h, "Edit", "Undo Merge Graph Windows");
    assert_eq!(project(&h), &before);
    menu(&mut h, "Edit", "Redo Merge Graph Windows");
    assert_eq!(project(&h).graphs().last().unwrap().layers.len(), 2);
}

#[test]
fn validation_errors_replace_the_old_image_and_recover_without_history() {
    let mut h = fixture("merge-preview-error");
    let before = project(&h).clone();
    let revision = h.state().doc.revision();
    open(&mut h);
    click(&mut h, "Auto Preview");
    assert!(has_image(&h));
    number(&mut h, "Number of Rows", "1");
    number(&mut h, "Number of Columns", "1");
    assert!(!has_image(&h));
    assert!(h.query_by_label_contains("cannot hold 3 graphs").is_some());
    number(&mut h, "Number of Columns", "3");
    assert!(has_image(&h));
    assert!(h.query_by_label_contains("cannot hold 3 graphs").is_none());
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    assert!(h.state().merge_graphs.is_none());
    assert_eq!(project(&h), &before);
    assert_eq!(h.state().doc.revision(), revision);
}

#[test]
fn live_source_changes_and_project_replacement_invalidate_a_manual_preview() {
    let mut h = fixture("merge-preview-live");
    open(&mut h);
    click(&mut h, "Preview");
    assert!(has_image(&h));
    let id = project(&h).graphs().next().unwrap().id;
    h.state_mut()
        .doc
        .apply("Source label", |p| p.graph_mut(id).unwrap().layers[0].legend.text = Some("Changed".into()));
    settle(&mut h);
    assert!(!has_image(&h));
    click(&mut h, "Preview");
    assert!(has_image(&h));
    let mut replacement = project(&h).clone();
    replacement.graph_mut(id).unwrap().layers[0].legend.text = Some("Replacement".into());
    h.state_mut().doc.replace(replacement);
    settle(&mut h);
    assert!(!has_image(&h));
    click(&mut h, "Auto Preview");
    assert!(has_image(&h));
    assert!(!h.state().doc.dirty);
    assert_eq!(h.state().doc.undo_label(), None);
    click(&mut h, "Cancel");
}

#[test]
fn folder_tree_changes_remove_the_old_image_and_refuse_a_new_preview() {
    let mut h = fixture("merge-preview-folder");
    let ids: Vec<_> = project(&h).graphs().map(|g| g.id).collect();
    h.state_mut().doc.apply("Folder fixture", |p| {
        p.root.folders.push(Folder { name: "Scope".into(), windows: ids[..2].to_vec(), ..Default::default() });
    });
    h.state_mut().panels.explorer.folder = vec![0];
    settle(&mut h);
    open(&mut h);
    h.get(By::new().role(egui::accesskit::Role::ComboBox).value("All in Project")).click();
    settle(&mut h);
    click(&mut h, "All in Active Folder");
    click(&mut h, "Auto Preview");
    assert!(has_image(&h));
    assert!(h.query_by_label("2 source graphs, 2 layers").is_some());
    h.state_mut().doc.apply("Rename folder", |p| p.root.folders[0].name = "Renamed".into());
    settle(&mut h);
    let before = project(&h).clone();
    let revision = h.state().doc.revision();
    assert!(!has_image(&h));
    assert!(h.query_by_label_contains("Source folder tree changed").is_some());
    click(&mut h, "Preview");
    assert!(!has_image(&h));
    click(&mut h, "Cancel");
    assert_eq!(project(&h), &before);
    assert_eq!(h.state().doc.revision(), revision);
}
