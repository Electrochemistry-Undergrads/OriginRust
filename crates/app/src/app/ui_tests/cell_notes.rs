use super::*;

fn start_book(tag: &str) -> Harness<'static, SheafApp> {
    let mut h = start(vec![data_file(tag)]);
    show_window(&mut h, "Book1 - sample");
    h
}

fn col_id(h: &Harness<'static, SheafApp>, short: &str) -> sheaf_core::ColumnId {
    sheet(h).column_by_name(short).unwrap().id
}

/// The notes of the sheet as (column short name, row, text).
fn notes(h: &Harness<'static, SheafApp>) -> Vec<(String, usize, String)> {
    let s = sheet(h);
    s.notes.iter().map(|n| (s.column(n.column).unwrap().short_name.clone(), n.row, n.text.clone())).collect()
}

/// Adds a note to the cell showing `cell_text` through the cell menu and the editor.
fn add_note(h: &mut Harness<'static, SheafApp>, cell_text: &str, note: &str) {
    click_text(h, cell_text);
    right_click_text(h, cell_text);
    menu_path(h, &["Add Note..."]);
    h.event(egui::Event::Text(note.to_string()));
    settle(h);
    click(h, "OK");
}

/// The red note tabs (filled triangles) the last frame painted.
fn tabs(h: &Harness<'static, SheafApp>) -> Vec<egui::Rect> {
    fn walk(shape: &egui::Shape, out: &mut Vec<egui::Rect>) {
        match shape {
            egui::Shape::Path(p) if p.fill == egui::Color32::from_rgb(192, 0, 0) => {
                out.push(egui::Rect::from_points(&p.points));
            }
            egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, out)),
            _ => {}
        }
    }
    let mut out = Vec::new();
    for clipped in &h.output().shapes {
        walk(&clipped.shape, &mut out);
    }
    out
}

#[test]
fn add_from_the_cell_menu_paints_the_corner_tab_and_undo_redo_work() {
    let mut h = start_book("note-add");
    assert!(tabs(&h).is_empty());
    add_note(&mut h, "4.5", "checked twice");
    let b = col_id(&h, "B");
    assert_eq!(sheet(&h).note(b, 1), Some("checked twice"));
    assert_eq!(h.state().doc.undo_label(), Some("Add note"));
    assert!(h.state().cell_note.is_none());
    settle(&mut h);
    let text = grid_text(&h, "4.5").rect();
    let t = tabs(&h);
    assert_eq!(t.len(), 1, "one tab for one note");
    assert!(t[0].width() > 3.0 && t[0].width() < 12.0, "a small tab");
    assert!(t[0].left() > text.center().x && t[0].center().y < text.center().y, "upper-right corner of the cell");
    assert!((t[0].center().y - text.center().y).abs() < 16.0, "in the row of the cell");

    h.state_mut().doc.undo();
    settle(&mut h);
    assert!(notes(&h).is_empty());
    assert!(tabs(&h).is_empty());
    h.state_mut().doc.redo();
    assert_eq!(sheet(&h).note(b, 1), Some("checked twice"));
}

#[test]
fn hovering_the_cell_shows_the_note_text() {
    let mut h = start_book("note-hover");
    add_note(&mut h, "6.5", "hover me");
    pause(&mut h);
    assert!(h.query_by_label("hover me").is_none());
    let at = grid_text(&h, "6.5").rect().center();
    h.event(egui::Event::PointerMoved(at));
    for _ in 0..30 {
        h.step();
    }
    assert!(
        h.query(By::new().role(egui::accesskit::Role::Label).value("hover me")).is_some()
            || h.query_by_label("hover me").is_some(),
        "the popup shows the note"
    );
}

#[test]
fn edit_and_delete_from_the_cell_menu_are_one_undo_step_each() {
    let mut h = start_book("note-edit");
    add_note(&mut h, "4.5", "first");
    let b = col_id(&h, "B");
    right_click_text(&mut h, "4.5");
    menu_path(&mut h, &["Edit Note..."]);
    assert_eq!(h.state().cell_note.as_ref().unwrap().text, "first");
    h.event(egui::Event::Text(" and more".to_string()));
    settle(&mut h);
    click(&mut h, "OK");
    assert_eq!(sheet(&h).note(b, 1), Some("first and more"));
    assert_eq!(h.state().doc.undo_label(), Some("Edit note"));

    right_click_text(&mut h, "4.5");
    menu_path(&mut h, &["Delete Note"]);
    assert!(notes(&h).is_empty());
    assert_eq!(h.state().doc.undo_label(), Some("Delete note"));
    // The menu offers Add again.
    right_click_text(&mut h, "4.5");
    assert!(h.query_by_label("Add Note...").is_some());
    assert!(h.query_by_label("Edit Note...").is_none());
    key(&mut h, egui::Key::Escape);
    h.state_mut().doc.undo();
    assert_eq!(sheet(&h).note(b, 1), Some("first and more"));
}

#[test]
fn cancel_and_an_empty_new_note_change_nothing() {
    let mut h = start_book("note-cancel");
    click_text(&mut h, "4.5");
    right_click_text(&mut h, "4.5");
    menu_path(&mut h, &["Add Note..."]);
    h.event(egui::Event::Text("lost".to_string()));
    settle(&mut h);
    click(&mut h, "Cancel");
    assert!(notes(&h).is_empty());
    right_click_text(&mut h, "4.5");
    menu_path(&mut h, &["Add Note..."]);
    click(&mut h, "OK");
    assert!(notes(&h).is_empty());
    assert_ne!(h.state().doc.undo_label(), Some("Add note"));
}

#[test]
fn sorting_the_sheet_keeps_the_note_on_its_value() {
    let mut h = start_book("note-sort");
    add_note(&mut h, "6.5", "the 6.5");
    let (a, c) = (col_id(&h, "A"), col_id(&h, "C"));
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::SortSheet { column: c, descending: false }, &ctx);
    settle(&mut h);
    // C ascending reverses the rows: 6.5 was row 2 of 4, now row 1.
    assert_eq!(column(&h, "B"), [8.5, 6.5, 4.5, 2.5]);
    assert_eq!(notes(&h), vec![("B".to_string(), 1, "the 6.5".to_string())]);
    let _ = a;
    h.state_mut().doc.undo();
    assert_eq!(sheet(&h).note(col_id(&h, "B"), 2), Some("the 6.5"));
}

#[test]
fn notes_survive_save_and_reload_through_the_companion() {
    let dir = save_dir("cellnote");
    let opj = dir.join("n.opj");
    let mut h = start_book("note-save");
    add_note(&mut h, "4.5", "kept");
    h.state_mut().path = Some(opj.clone());
    ctrl_s(&mut h);
    let (title, text) = h.state().message.clone().expect("a message says what only the companion holds");
    assert!(title.contains("companion") && text.contains("cell note"), "{title}: {text}");
    let h2 = start(vec![opj]);
    assert_eq!(project(&h2).workbooks().next().unwrap().sheet().notes, sheet(&h).notes);
}

#[test]
fn a_note_needs_a_single_selected_cell_and_works_in_frozen_overlay_cells() {
    let mut h = start_book("note-frozen");
    let ctx = h.ctx.clone();
    let book = project(&h).workbooks().next().unwrap().id;
    h.state_mut().apply(Action::DeleteNote(book), &ctx);
    settle(&mut h);
    assert!(h.state().status.contains("Select"), "{}", h.state().status);
    // Freeze at B2: row 1 and column A frozen; the note on a frozen-column cell still has its tab.
    click_text(&mut h, "4.5");
    key_with(&mut h, egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::F);
    assert!(!sheet(&h).frozen.is_none());
    let a = col_id(&h, "A");
    h.state_mut().doc.apply("Add note", |p| {
        p.workbook_mut(book).unwrap().sheet_mut().set_note(a, 2, "in the frozen column");
    });
    settle(&mut h);
    assert_eq!(tabs(&h).len(), 1);
}
