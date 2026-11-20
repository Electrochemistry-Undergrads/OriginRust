//! File > New > Matrix and the matrix window (T179 slice 2).

use super::*;

#[test]
fn a_matrix_is_made_shown_and_a_typed_cell_is_one_undo_step() {
    // An empty project, so that no other window lies over the new one.
    let mut h = start(Vec::new());
    menu_path(&mut h, &["File", "New", "Matrix..."]);
    let books: Vec<_> = project(&h).matrix_books().map(|m| (m.id, m.name.clone())).collect();
    assert_eq!(books.len(), 1);
    let (id, name) = books[0].clone();
    assert_eq!(name, "MBook1");
    assert_eq!(h.state().doc.undo_label(), Some("New matrix"));
    let sheet = |h: &Harness<'static, SheafApp>| project(h).matrix_sheet(id, 0).unwrap().clone();
    assert_eq!((sheet(&h).rows, sheet(&h).cols), (32, 32), "Origin's default size");
    settle(&mut h);
    // The grid shows the column numbers and the missing values.
    assert!(has_label(&h, "32") && has_label(&h, "--"), "column numbers and empty cells are drawn");
    // A double click on the first cell edits it; Enter writes it.
    let cell = h.get_all_by_label("--").next().expect("a cell").rect().center();
    double_click_at(&mut h, cell);
    assert_eq!(h.state().matrices.get(&id).and_then(|m| m.editing_cell()), Some((0, 0, 0)), "the first cell is edited");
    h.event(egui::Event::Text("2.5".into()));
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    assert_eq!(sheet(&h).value(0, 0), 2.5);
    assert_eq!(h.state().doc.undo_label(), Some("Edit matrix cell"));
    assert!(has_label(&h, "2.5"));
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::Undo, &ctx);
    settle(&mut h);
    assert!(sheet(&h).value(0, 0).is_nan(), "one undo step");
    // Text that is not a number changes nothing.
    h.state_mut().apply(Action::SetMatrixCell { id, sheet: 0, row: 1, col: 1, value: Some(-4.0) }, &ctx);
    settle(&mut h);
    assert_eq!(sheet(&h).value(1, 1), -4.0);
    // The Standard toolbar's New Matrix makes another book.
    let action = menus::resolve(&menus::Cmd::NewMatrix, &h.state().menu_context()).unwrap();
    h.state_mut().apply(action, &ctx);
    settle(&mut h);
    assert_eq!(project(&h).matrix_books().count(), 2);
}
