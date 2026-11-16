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
fn the_menu_opens_the_dialog_and_one_graph_is_not_enough() {
    let mut h = with_graphs("merge-open", 1);
    menu_path(&mut h, &["Graph", "Merge Graph Windows..."]);
    assert!(h.state().merge_graphs.is_none());
    assert!(h.state().status.contains("at least two graphs"), "{}", h.state().status);
    let mut h = with_graphs("merge-open2", 2);
    open(&mut h);
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
