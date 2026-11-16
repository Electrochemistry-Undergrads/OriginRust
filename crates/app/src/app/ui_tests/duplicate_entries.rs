//! Duplicate entry points must keep the active-window and clicked-window targets distinct.
use super::*;

const BUTTON: &str = "Standard toolbar: Duplicate";

fn assert_copy(h: &Harness<'static, SheafApp>, before: &Project, source: WindowId, workbook: bool) {
    let copied = h.state().active.expect("the copy becomes active");
    assert_ne!(copied, source);
    assert!(h.state().workspace.is_shown(copied));
    assert_eq!(project(h).workbooks().count(), before.workbooks().count() + usize::from(workbook));
    assert_eq!(project(h).graphs().count(), before.graphs().count() + usize::from(!workbook));
    if workbook {
        let original = before.workbook(source).unwrap();
        let copy = project(h).workbook(copied).expect("copied the workbook, not the other graph");
        assert_eq!(project(h).workbook(source), Some(original));
        assert_ne!(copy.name, original.name);
        assert_eq!(copy.sheets.len(), original.sheets.len());
        for (sheet, original) in copy.sheets.iter().zip(&original.sheets) {
            assert_eq!(sheet.columns.len(), original.columns.len());
            for (column, original) in sheet.columns.iter().zip(&original.columns) {
                assert_ne!(column.id, original.id);
                assert_eq!(column.data, original.data);
            }
        }
    } else {
        let original = before.graph(source).unwrap();
        let copy = project(h).graph(copied).expect("copied the graph, not the other workbook");
        assert_eq!(project(h).graph(source), Some(original));
        assert_ne!(copy.name, original.name);
        assert_eq!(copy.layers, original.layers);
    }
    assert_eq!(h.state().doc.undo_label(), Some("Duplicate window"));
}

fn undo_redo_copy(h: &mut Harness<'static, SheafApp>, before: &Project) {
    let copied = project(h).clone();
    ctrl(h, egui::Key::Z);
    assert_eq!(project(h), before, "one Undo removes only the new copy");
    h.key_press_modifiers(egui::Modifiers { shift: true, ..egui::Modifiers::COMMAND }, egui::Key::Z);
    settle(h);
    assert_eq!(project(h), &copied, "Redo restores the same copy and column identities");
}

#[test]
fn duplicate_entry_standard_is_state_disabled_without_an_active_window_and_supports_an_empty_book() {
    let mut h = start(Vec::new());
    let empty = project(&h).clone();
    let revision = h.state().doc.revision();
    assert!(h.get_by_label(BUTTON).accesskit_node().is_disabled());
    click(&mut h, BUTTON);
    assert_eq!(project(&h), &empty);
    assert_eq!(h.state().doc.revision(), revision);

    click(&mut h, "Standard toolbar: New Workbook");
    let source = h.state().active.unwrap();
    let before = project(&h).clone();
    assert!(!h.get_by_label(BUTTON).accesskit_node().is_disabled());
    click(&mut h, BUTTON);
    assert_copy(&h, &before, source, true);
    undo_redo_copy(&mut h, &before);
}

#[test]
fn duplicate_entry_standard_copies_the_active_workbook_or_graph_instead_of_the_explorer_selection() {
    for workbook in [true, false] {
        let mut h = start(vec![data_file(&format!("duplicate-standard-{workbook}"))]);
        let (book, graph) = window_ids(&h);
        let (source, selected) = if workbook { (book, graph) } else { (graph, book) };
        let active_title = project(&h).window(source).unwrap().title();
        let selected_title = project(&h).window(selected).unwrap().title();
        show_window(&mut h, &active_title);
        click(&mut h, &selected_title);
        assert_eq!(h.state().active, Some(source));
        let before = project(&h).clone();
        assert!(!h.get_by_label(BUTTON).accesskit_node().is_disabled());
        click(&mut h, BUTTON);
        assert_copy(&h, &before, source, workbook);
        undo_redo_copy(&mut h, &before);
    }
}

#[test]
fn duplicate_entry_explorer_copies_its_clicked_workbook_or_graph_even_when_that_window_is_hidden() {
    for workbook in [true, false] {
        for hidden in [false, true] {
            let mut h = start(vec![data_file(&format!("duplicate-explorer-{workbook}-{hidden}"))]);
            let (book, graph) = window_ids(&h);
            let (source, other) = if workbook { (book, graph) } else { (graph, book) };
            let title = project(&h).window(source).unwrap().title();
            let other_title = project(&h).window(other).unwrap().title();
            show_window(&mut h, &other_title);
            if hidden {
                right_click(&mut h, &title);
                click(&mut h, "Hide");
                assert!(!h.state().workspace.is_shown(source));
            }
            right_click(&mut h, &title);
            assert_eq!(h.state().active, Some(other), "right click selects without activating the source");
            let before = project(&h).clone();
            assert!(!h.get_by_label("Duplicate").accesskit_node().is_disabled());
            click(&mut h, "Duplicate");
            assert_copy(&h, &before, source, workbook);
            undo_redo_copy(&mut h, &before);
        }
    }
}

#[test]
fn duplicate_entry_explorer_cancel_keeps_the_project_and_history_unchanged() {
    let mut h = start(vec![data_file("duplicate-explorer-cancel")]);
    let before = project(&h).clone();
    let revision = h.state().doc.revision();
    let title = project(&h).workbooks().next().unwrap().id;
    let title = project(&h).window(title).unwrap().title();
    right_click(&mut h, &title);
    assert!(!h.get_by_label("Duplicate").accesskit_node().is_disabled());
    close_menus(&mut h);
    assert_eq!(project(&h), &before);
    assert_eq!(h.state().doc.revision(), revision);
}
