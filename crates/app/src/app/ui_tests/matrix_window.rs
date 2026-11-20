//! File > New > Matrix and the matrix window (T192 slice 2).

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

#[test]
fn the_matrix_menu_transforms_sets_values_converts_and_plots_the_active_sheet() {
    let mut h = start(Vec::new());
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::NewMatrix, &ctx);
    settle(&mut h);
    let id = project(&h).matrix_books().next().unwrap().id;
    let sheet = |h: &Harness<'static, SheafApp>| project(h).matrix_sheet(id, 0).unwrap().clone();
    assert_eq!(h.state().menu_context().kind, menus::WindowKind::Matrix, "the new matrix window is active");
    // Set Matrix Values... with a formula in i and j.
    menu_path(&mut h, &["Matrix", "Set Matrix Values..."]);
    h.query_all_by_role(egui::accesskit::Role::TextInput).last().expect("the formula box").focus();
    settle(&mut h);
    h.event(egui::Event::Text("i*100+j".into()));
    settle(&mut h);
    click(&mut h, "OK");
    assert_eq!(sheet(&h).value(0, 1), 102.0, "row 1, column 2");
    assert_eq!(h.state().doc.undo_label(), Some("Set Matrix Values"));
    // Transpose swaps rows and columns, one undo step.
    menu_path(&mut h, &["Matrix", "Transpose"]);
    assert_eq!(sheet(&h).value(1, 0), 102.0);
    assert_eq!(h.state().doc.undo_label(), Some("Transpose Matrix"));
    h.state_mut().apply(Action::Undo, &ctx);
    settle(&mut h);
    assert_eq!(sheet(&h).value(0, 1), 102.0);
    // Flip Horizontal mirrors the columns.
    menu_path(&mut h, &["Matrix", "Flip", "Horizontal"]);
    assert_eq!(sheet(&h).value(0, 31), 101.0);
    // Convert to Worksheet makes a workbook of the cells.
    let books = project(&h).workbooks().count();
    let action = menus::resolve(&menus::Cmd::Matrix(menus::MatrixCmd::ToWorksheet), &h.state().menu_context()).unwrap();
    h.state_mut().apply(action, &ctx);
    settle(&mut h);
    assert_eq!(project(&h).workbooks().count(), books + 1);
    // Plot > Contour/Heat Map > Heat Map of the active matrix.
    show_window(&mut h, "MBook1");
    let graphs = project(&h).graphs().count();
    let action =
        menus::resolve(&menus::Cmd::PlotField(sheaf_core::template::FieldTemplate::HeatMap), &h.state().menu_context())
            .unwrap();
    h.state_mut().apply(action, &ctx);
    settle(&mut h);
    assert_eq!(project(&h).graphs().count(), graphs + 1);
    assert!(project(&h).graphs().last().unwrap().layers[0].field.is_some(), "a heat map layer");
}

#[test]
fn worksheet_convert_to_matrix_direct_and_xyz_gridding_make_matrix_books() {
    // The Worksheet menu has both items.
    let mut found = Vec::new();
    for m in menus::bar(menus::WindowKind::Workbook) {
        fn walk(entries: &[menus::Entry], found: &mut Vec<menus::Cmd>) {
            for e in entries {
                match e {
                    menus::Entry::Item(i) => found.extend(i.cmd.clone()),
                    menus::Entry::Submenu(s) => walk(&s.entries, found),
                    _ => {}
                }
            }
        }
        walk(&m.entries, &mut found);
    }
    for xyz in [false, true] {
        assert!(found.contains(&menus::Cmd::ConvertToMatrix { xyz }), "Convert to Matrix xyz={xyz}");
    }
    let mut h = start(Vec::new());
    let ctx = h.ctx.clone();
    h.state_mut().doc.apply("Data", |p| {
        let book = p.add_workbook("grid");
        // A 3 x 2 grid of XYZ points, and two plain columns.
        let xs = vec![1.0, 2.0, 3.0, 1.0, 2.0, 3.0];
        let ys = vec![10.0, 10.0, 10.0, 20.0, 20.0, 20.0];
        let zs = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        for (d, v) in [(Designation::X, xs), (Designation::Y, ys), (Designation::Z, zs)] {
            p.add_column(book, 0, d, ColumnData::numeric(v)).unwrap();
        }
    });
    settle(&mut h);
    let book = project(&h).workbooks().next().unwrap().id;
    let cols: Vec<sheaf_core::ColumnId> =
        project(&h).workbook(book).unwrap().sheet().columns.iter().map(|c| c.id).collect();
    h.state_mut().apply(Action::ConvertToMatrix { book, columns: cols.clone(), xyz: true }, &ctx);
    settle(&mut h);
    assert_eq!(h.state().doc.undo_label(), Some("Convert to Matrix"));
    let m = project(&h).matrix_books().next().expect("a matrix book").sheets[0].clone();
    assert_eq!((m.cols, m.rows), (3, 2), "X across the columns, Y down the rows");
    assert_eq!((m.x_from, m.x_to, m.y_from, m.y_to), (1.0, 3.0, 10.0, 20.0));
    assert_eq!(m.value(1, 2), 6.0);
    // Direct: the two first columns become two matrix columns of six rows.
    h.state_mut().apply(Action::ConvertToMatrix { book, columns: cols[..2].to_vec(), xyz: false }, &ctx);
    settle(&mut h);
    let m = project(&h).matrix_books().nth(1).expect("a second matrix book").sheets[0].clone();
    assert_eq!((m.cols, m.rows), (2, 6));
    assert_eq!(m.value(5, 1), 20.0);
}
