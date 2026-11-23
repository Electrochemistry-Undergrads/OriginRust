use super::*;

/// The sample project with `n` graphs in all (the import makes the first), and Graph1 shown so that
/// the Graph menu is there.
fn with_graphs(tag: &str, n: usize) -> Harness<'static, SheafApp> {
    let mut h = start(vec![data_file(tag)]);
    let ys: Vec<_> = {
        let b = project(&h).workbooks().next().unwrap();
        b.sheet().y_columns().into_iter().map(|i| b.sheet().columns[i].id).collect()
    };
    for k in 1..n {
        h.state_mut().doc.apply("Plot", |p| p.plot_columns(Template::Line, &[ys[k % ys.len()]]).unwrap());
    }
    settle(&mut h);
    show_window(&mut h, "Graph1 - sample");
    h
}

fn open(h: &mut Harness<'static, SheafApp>) {
    menu_path(h, &["Graph", "Merge Graph Windows..."]);
    assert!(h.state().merge_graphs.is_some());
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
fn the_menu_opens_the_dialog_with_all_in_project_for_multiple_graphs() {
    let mut h = with_graphs("merge-open2", 2);
    open(&mut h);
    assert!(h.query_by_value("All in Project").is_some());
    assert!(h.query_by_label("Keep Source Graphs").is_some());
    assert!(h.query_by_label("Rearrange Layout").is_some());
}

#[test]
fn ok_merges_three_graphs_into_one_new_graph_in_one_undo_step() {
    let mut h = with_graphs("merge-ok", 3);
    open(&mut h);
    click(&mut h, "OK");
    assert!(h.state().merge_graphs.is_none());
    assert_eq!(project(&h).graphs().count(), 4);
    let g = project(&h).graphs().last().unwrap();
    assert_eq!(g.layers.len(), 3);
    assert_eq!(h.state().active, Some(g.id));
    assert_eq!(h.state().doc.undo_label(), Some("Merge Graph Windows"));
    menu(&mut h, "Edit", "Undo Merge Graph Windows");
    assert_eq!(project(&h).graphs().count(), 3);
    menu(&mut h, "Edit", "Redo Merge Graph Windows");
    assert_eq!(project(&h).graphs().count(), 4);
}

#[test]
fn roman_panel_labels_follow_the_dialog_draft_and_source_order() {
    let mut h = with_graphs("merge-roman", 4);
    let before = project(&h).clone();
    open(&mut h);
    click(&mut h, "Arrange Settings");
    click(&mut h, "Spacing (in % of Page Dimension)");
    click(&mut h, "Add Label");
    h.get(By::new().role(egui::accesskit::Role::ComboBox).value("None")).click();
    settle(&mut h);
    click(&mut h, "I, II, III");
    click(&mut h, "Preview");
    let preview = "4 source graphs, 4 layers";
    assert!(h.query_by_label(preview).is_some());

    h.get(By::new().role(egui::accesskit::Role::ComboBox).value("I, II, III")).click();
    settle(&mut h);
    click(&mut h, "i, ii, iii");
    assert!(h.query_by_label(preview).is_none());
    assert_eq!(project(&h), &before);
    click(&mut h, "OK");

    let merged = project(&h).graphs().last().unwrap();
    let labels: Vec<_> = merged.layers.iter().map(|layer| layer.labels[0].text.as_str()).collect();
    assert_eq!(labels, ["i", "ii", "iii", "iv"]);
    menu(&mut h, "Edit", "Undo Merge Graph Windows");
    assert_eq!(project(&h), &before);
}

#[test]
fn source_name_labels_copy_the_short_names_once_per_graph() {
    use sheaf_core::layers::NewLayerKind;

    let mut h = with_graphs("merge-source-names", 2);
    let sources: Vec<_> = project(&h).graphs().map(|graph| graph.id).collect();
    h.state_mut().doc.apply("Set source names", |project| {
        for (&id, name) in sources.iter().zip(["SourceZ", "SourceA"]) {
            project.rename_window(id, name).unwrap();
            project.graph_mut(id).unwrap().long_name = format!("Title of {name}");
        }
        project.graph_mut(sources[0]).unwrap().add_layer(NewLayerKind::RightY, 0).unwrap();
    });
    settle(&mut h);
    let before = project(&h).clone();
    open(&mut h);
    click(&mut h, "Arrange Settings");
    click(&mut h, "Spacing (in % of Page Dimension)");
    click(&mut h, "Add Label");
    h.get(By::new().role(egui::accesskit::Role::ComboBox).value("None")).click();
    settle(&mut h);
    click(&mut h, "Source Graph Name");
    click(&mut h, "Preview");
    assert!(h.query_by_label("2 source graphs, 3 layers").is_some());
    assert_eq!(project(&h), &before);
    click(&mut h, "OK");

    let merged_id = project(&h).graphs().last().unwrap().id;
    h.state_mut().doc.apply("Rename a source", |project| project.rename_window(sources[0], "RenamedZ").unwrap());
    let merged = project(&h).graph(merged_id).unwrap();
    let labels: Vec<_> =
        merged.layers.iter().map(|layer| layer.labels.first().map(|label| label.text.as_str())).collect();
    assert_eq!(labels, [Some("SourceZ"), None, Some("SourceA")]);
}

#[test]
fn without_keep_source_graphs_the_sources_go_and_one_undo_brings_them_back() {
    let mut h = with_graphs("merge-nokeep", 3);
    open(&mut h);
    click(&mut h, "Keep Source Graphs");
    click(&mut h, "OK");
    assert_eq!(project(&h).graphs().count(), 1);
    assert_eq!(project(&h).graphs().next().unwrap().layers.len(), 3);
    menu(&mut h, "Edit", "Undo Merge Graph Windows");
    assert_eq!(project(&h).graphs().count(), 3);
}

#[test]
fn cancel_changes_nothing() {
    let mut h = with_graphs("merge-cancel", 3);
    let revision = h.state().doc.revision();
    open(&mut h);
    click(&mut h, "Cancel");
    assert!(h.state().merge_graphs.is_none());
    assert_eq!(h.state().doc.revision(), revision);
    assert_eq!(project(&h).graphs().count(), 3);
}

#[test]
fn a_grid_too_small_for_the_graphs_shows_the_error_and_keeps_the_dialog() {
    let mut h = with_graphs("merge-error", 3);
    let revision = h.state().doc.revision();
    open(&mut h);
    number(&mut h, "Number of Rows", "1");
    number(&mut h, "Number of Columns", "1");
    click(&mut h, "OK");
    assert!(h.state().merge_graphs.is_some());
    assert!(h.query_by_label_contains("cannot hold 3 graphs").is_some());
    assert_eq!(h.state().doc.revision(), revision);
    assert_eq!(project(&h).graphs().count(), 3);
}
