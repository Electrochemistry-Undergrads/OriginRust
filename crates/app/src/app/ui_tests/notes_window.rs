//! File > New > Notes and the Notes window (T154).

use super::*;

#[test]
fn a_notes_window_is_made_typed_in_and_its_edit_is_one_undo_step() {
    let mut h = start(vec![data_file("notes-window")]);
    menu_path(&mut h, &["File", "New", "Notes"]);
    let notes: Vec<_> = project(&h).notes().map(|n| (n.id, n.name.clone())).collect();
    assert_eq!(notes.len(), 1);
    let (id, name) = notes[0].clone();
    assert_eq!(name, "Notes1");
    assert_eq!(h.state().doc.undo_label(), Some("New notes"));
    settle(&mut h);
    h.query_all_by_role(egui::accesskit::Role::MultilineTextInput).last().expect("the text box").focus();
    settle(&mut h);
    h.event(egui::Event::Text("hello notes".into()));
    settle(&mut h);
    assert_eq!(project(&h).notes_window(id).unwrap().text, "", "nothing is written while typing");
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    assert_eq!(project(&h).notes_window(id).unwrap().text, "hello notes", "written when the box lets go");
    assert_eq!(h.state().doc.undo_label(), Some("Edit notes"));
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::Undo, &ctx);
    settle(&mut h);
    assert_eq!(project(&h).notes_window(id).unwrap().text, "", "one undo step");
    // The toolbar button makes another.
    let action = menus::resolve(&menus::Cmd::NewNotes, &h.state().menu_context()).unwrap();
    h.state_mut().apply(action, &ctx);
    settle(&mut h);
    assert_eq!(project(&h).notes().count(), 2);
}

#[test]
fn add_notes_as_sheet_makes_a_text_sheet_edited_in_place_of_the_grid() {
    let entries = menus::sheet_tab_menu(&sheaf_core::Worksheet::new("Sheet1"));
    assert!(entries.iter().any(|e| matches!(e, menus::Entry::Item(i)
        if i.label == "Add Notes as Sheet" && i.cmd == Some(menus::Cmd::Sheet(SheetOp::AddNotes)))));
    let mut h = start(vec![data_file("notes-sheet")]);
    show_window(&mut h, "Book1 - sample.csv");
    let book = h.state().books.keys().next().copied().unwrap();
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::Sheet { book, op: SheetOp::AddNotes }, &ctx);
    settle(&mut h);
    let b = project(&h).workbook(book).unwrap();
    assert_eq!(b.sheets.len(), 2);
    assert_eq!((b.active_sheet, b.sheets[1].name.as_str()), (1, "Notes"));
    assert!(b.sheets[1].is_notes());
    assert_eq!(h.state().doc.undo_label(), Some("Add Notes as Sheet"));
    h.query_all_by_role(egui::accesskit::Role::MultilineTextInput).last().expect("the text box").focus();
    settle(&mut h);
    h.event(egui::Event::Text("run notes".into()));
    settle(&mut h);
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    assert_eq!(project(&h).workbook(book).unwrap().sheets[1].notes_text.as_deref(), Some("run notes"));
    assert_eq!(h.state().doc.undo_label(), Some("Edit notes"));
    h.state_mut().apply(Action::Undo, &ctx);
    settle(&mut h);
    assert_eq!(project(&h).workbook(book).unwrap().sheets[1].notes_text.as_deref(), Some(""));
}

#[test]
fn add_graph_as_sheet_shows_the_picked_graph_in_the_workbook() {
    let mut h = start(vec![data_file("graph-sheet")]);
    show_window(&mut h, "Book1 - sample.csv");
    let book = h.state().books.keys().next().copied().unwrap();
    let graph = project(&h).graphs().next().unwrap().id;
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::Sheet { book, op: SheetOp::AddGraph }, &ctx);
    settle(&mut h);
    assert!(h.state().graph_sheet_picker.is_some(), "the picker is open");
    h.get_by_label("Add Graph as Sheet").get_by_label("OK").click();
    settle(&mut h);
    assert!(h.state().graph_sheet_picker.is_none());
    let b = project(&h).workbook(book).unwrap();
    assert_eq!(b.sheets.len(), 2);
    assert_eq!((b.active_sheet, b.sheets[1].graph_sheet), (1, Some(graph)));
    assert_eq!(b.sheets[1].name, "Graph1");
    assert_eq!(h.state().doc.undo_label(), Some("Add Graph as Sheet"));
    // The graph window stays; deleting it leaves the sheet with a note.
    h.state_mut().doc.apply("drop graph", |p| p.windows.retain(|w| w.id() != graph));
    settle(&mut h);
    assert!(h.query_by_label("The graph this sheet showed was deleted.").is_some());
}

#[test]
fn insert_graph_puts_a_picture_of_the_graph_in_the_cell() {
    let mut h = start(vec![data_file("cell-graph")]);
    show_window(&mut h, "Book1 - sample.csv");
    let book = h.state().books.keys().next().copied().unwrap();
    let graph = project(&h).graphs().next().unwrap().id;
    let b = project(&h).workbook(book).unwrap().sheets[0].columns[1].id;
    let menu = menus::cell_menu_for(false, (b, 0), false);
    assert!(menu.iter().any(|e| matches!(e, menus::Entry::Item(i)
        if i.label == "Insert Graph..." && i.cmd == Some(menus::Cmd::InsertCellGraph { column: b, row: 0 }))));
    assert!(!menu.iter().any(|e| matches!(e, menus::Entry::Item(i) if i.label == "Remove Graph")));
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::PickCellGraph { column: b, row: 0 }, &ctx);
    settle(&mut h);
    h.get_by_label("Insert Graph").get_by_label("OK").click();
    settle(&mut h);
    assert_eq!(project(&h).workbook(book).unwrap().sheets[0].cell_graph(b, 0), Some(graph));
    assert_eq!(h.state().doc.undo_label(), Some("Insert Graph"));
    let menu = menus::cell_menu_for(false, (b, 0), true);
    let remove = menu.iter().find_map(|e| match e {
        menus::Entry::Item(i) if i.label == "Remove Graph" => i.cmd.clone(),
        _ => None,
    });
    let action = menus::resolve(&remove.expect("Remove Graph"), &h.state().menu_context()).unwrap();
    h.state_mut().apply(action, &ctx);
    settle(&mut h);
    assert_eq!(project(&h).workbook(book).unwrap().sheets[0].cell_graph(b, 0), None);
    assert_eq!(h.state().doc.undo_label(), Some("Remove Graph"));
}
