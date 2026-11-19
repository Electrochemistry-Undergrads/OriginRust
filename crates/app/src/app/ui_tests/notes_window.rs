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
