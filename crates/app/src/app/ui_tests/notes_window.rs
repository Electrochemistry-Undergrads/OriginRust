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
    let menu = menus::cell_menu_for(false, (b, 0), false, false, false, false, (false, false));
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
    let menu = menus::cell_menu_for(false, (b, 0), true, false, false, false, (false, false));
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

#[test]
fn insert_sparklines_draws_a_columns_line_in_the_cell() {
    let mut h = start(vec![data_file("cell-sparkline")]);
    show_window(&mut h, "Book1 - sample.csv");
    let book = h.state().books.keys().next().copied().unwrap();
    let (b, c) = {
        let s = &project(&h).workbook(book).unwrap().sheets[0];
        (s.columns[1].id, s.columns[2].id)
    };
    let menu = menus::cell_menu_for(false, (b, 5), false, false, false, false, (false, false));
    assert!(menu.iter().any(|e| matches!(e, menus::Entry::Item(i)
        if i.label == "Insert Sparklines..." && i.cmd == Some(menus::Cmd::InsertCellSparkline { column: b, row: 5 }))));
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::PickCellSparkline { column: b, row: 5 }, &ctx);
    settle(&mut h);
    click(&mut h, "C - down");
    h.get_by_label("Insert Sparklines").get_by_label("OK").click();
    settle(&mut h);
    assert_eq!(project(&h).workbook(book).unwrap().sheets[0].cell_sparkline(b, 5), Some(c));
    assert_eq!(h.state().doc.undo_label(), Some("Insert Sparklines"));
    h.state_mut().apply(Action::SetCellSparkline { column: b, row: 5, source: None }, &ctx);
    settle(&mut h);
    assert_eq!(project(&h).workbook(book).unwrap().sheets[0].cell_sparkline(b, 5), None);
    assert_eq!(h.state().doc.undo_label(), Some("Remove Sparkline"));
}

#[test]
fn a_png_picture_in_a_cell_is_kept_in_the_project_and_removed_again() {
    let mut h = start(vec![data_file("cell-image")]);
    show_window(&mut h, "Book1 - sample.csv");
    let book = h.state().books.keys().next().copied().unwrap();
    let b = project(&h).workbook(book).unwrap().sheets[0].columns[1].id;
    let menu = menus::cell_menu_for(false, (b, 1), false, false, false, false, (false, false));
    assert!(menu.iter().any(|e| matches!(e, menus::Entry::Item(i)
        if i.label == "Insert Images from Files..." && i.cmd == Some(menus::Cmd::InsertCellImage { column: b, row: 1 }))));
    // A 2 x 1 PNG made here (the file picker is the system's).
    let mut png = Vec::new();
    image::RgbaImage::from_raw(2, 1, vec![255, 0, 0, 255, 0, 0, 255, 255])
        .unwrap()
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .unwrap();
    let ctx = h.ctx.clone();
    let png = std::sync::Arc::new(png);
    h.state_mut().apply(Action::SetCellImage { column: b, row: 1, png: Some(png.clone()) }, &ctx);
    settle(&mut h);
    let held = project(&h).workbook(book).unwrap().sheets[0].cell_image(b, 1).map(|i| i.png.clone());
    assert_eq!(held, Some(png));
    assert_eq!(h.state().doc.undo_label(), Some("Insert Image"));
    let menu = menus::cell_menu_for(false, (b, 1), false, false, true, false, (false, false));
    let remove = menu.iter().find_map(|e| match e {
        menus::Entry::Item(i) if i.label == "Remove Image" => i.cmd.clone(),
        _ => None,
    });
    let action = menus::resolve(&remove.expect("Remove Image"), &h.state().menu_context()).unwrap();
    h.state_mut().apply(action, &ctx);
    settle(&mut h);
    assert!(project(&h).workbook(book).unwrap().sheets[0].cell_image(b, 1).is_none());
}

#[test]
fn insert_arrow_draws_across_the_cell_and_the_row_number_inserts_a_graph() {
    let mut h = start(vec![data_file("cell-arrow")]);
    show_window(&mut h, "Book1 - sample.csv");
    let book = h.state().books.keys().next().copied().unwrap();
    let (a, b) = {
        let s = &project(&h).workbook(book).unwrap().sheets[0];
        (s.columns[0].id, s.columns[1].id)
    };
    let menu = menus::cell_menu_for(false, (b, 2), false, false, false, false, (false, false));
    let double = menu.iter().find_map(|e| match e {
        menus::Entry::Submenu(s) if s.label == "Insert Arrow" => s.entries.iter().find_map(|e| match e {
            menus::Entry::Item(i) if i.label == "Double Arrow" => i.cmd.clone(),
            _ => None,
        }),
        _ => None,
    });
    let ctx = h.ctx.clone();
    let action = menus::resolve(&double.expect("Double Arrow"), &h.state().menu_context()).unwrap();
    h.state_mut().apply(action, &ctx);
    settle(&mut h);
    let arrow = project(&h).workbook(book).unwrap().sheets[0].cell_arrow(b, 2);
    assert!(arrow.is_some_and(|x| x.double));
    assert_eq!(h.state().doc.undo_label(), Some("Insert Arrow"));
    // The row number's Insert Graph... goes to the row's first visible cell.
    let row = menus::row_menu(Some((a, 3)));
    assert!(row.iter().any(|e| matches!(e, menus::Entry::Item(i)
        if i.label == "Insert Graph..." && i.cmd == Some(menus::Cmd::InsertCellGraph { column: a, row: 3 }))));
    assert!(menus::row_menu(None).iter().any(|e| matches!(e, menus::Entry::Item(i)
        if i.label == "Insert Graph..." && i.cmd.is_none())));
}

#[test]
fn set_data_style_rich_text_turns_on_for_the_selected_cells() {
    let mut h = start(vec![data_file("cell-rich-text")]);
    show_window(&mut h, "Book1 - sample.csv");
    let book = h.state().books.keys().next().copied().unwrap();
    let b = project(&h).workbook(book).unwrap().sheets[0].columns[1].id;
    h.state_mut().doc.apply("markup", |p| {
        p.column_mut(b).unwrap().data =
            sheaf_core::ColumnData::text(vec!["x\\+(2) \\b(bold)".into(), "a".into(), "b".into(), "c".into()]);
    });
    settle(&mut h);
    right_click_text(&mut h, "x\\+(2) \\b(bold)");
    let menu = menus::cell_menu_for(false, (b, 0), false, false, false, false, (false, false));
    let rich = menu.iter().find_map(|e| match e {
        menus::Entry::Submenu(s) if s.label == "Set Data Style" => s.entries.iter().find_map(|e| match e {
            menus::Entry::Item(i) if i.label == "Rich Text" => i.cmd.clone(),
            _ => None,
        }),
        _ => None,
    });
    assert_eq!(rich, Some(menus::Cmd::SetRichText(true)));
    let ctx = h.ctx.clone();
    let action = menus::resolve(&menus::Cmd::SetRichText(true), &h.state().menu_context()).unwrap();
    h.state_mut().apply(action, &ctx);
    settle(&mut h);
    assert_eq!(h.state().doc.undo_label(), Some("Set Data Style"));
    let sheet = &project(&h).workbook(book).unwrap().sheets[0];
    assert!(sheet.format_index().resolve(sheet, b, 0).rich_text, "the right-clicked cell is rich text");
    assert!(h.query_by_label("x\\+(2) \\b(bold)").is_none(), "the markup is no longer shown as typed");
}

#[test]
fn set_data_style_float_turns_on_for_the_selected_cells() {
    let mut h = start(vec![data_file("cell-float")]);
    show_window(&mut h, "Book1 - sample.csv");
    let book = h.state().books.keys().next().copied().unwrap();
    let b = project(&h).workbook(book).unwrap().sheets[0].columns[1].id;
    let menu = menus::cell_menu_for(false, (b, 0), false, false, false, false, (false, false));
    let float = menu.iter().find_map(|e| match e {
        menus::Entry::Submenu(s) if s.label == "Set Data Style" => s.entries.iter().find_map(|e| match e {
            menus::Entry::Item(i) if i.label == "Float" => i.cmd.clone(),
            _ => None,
        }),
        _ => None,
    });
    assert_eq!(float, Some(menus::Cmd::SetFloat(true)));
    click_text(&mut h, "2.5");
    let ctx = h.ctx.clone();
    let action = menus::resolve(&menus::Cmd::SetFloat(true), &h.state().menu_context()).unwrap();
    h.state_mut().apply(action, &ctx);
    settle(&mut h);
    assert_eq!(h.state().doc.undo_label(), Some("Set Data Style"));
    let sheet = &project(&h).workbook(book).unwrap().sheets[0];
    assert!(sheet.format_index().resolve(sheet, b, 0).float);
}

#[test]
fn insert_images_from_files_puts_a_picture_object_on_the_graph() {
    let entries = menus::graph_page_menu(0, (100.0, 80.0));
    assert!(entries.iter().any(|e| matches!(e, menus::Entry::Item(i)
        if i.label == "Insert Images From Files..."
            && i.cmd == Some(menus::Cmd::InsertGraphImage { layer: 0, page: (100.0, 80.0) }))));
    let mut h = start(vec![data_file("graph-picture")]);
    let graph = project(&h).graphs().next().unwrap().id;
    let mut png = Vec::new();
    image::RgbaImage::from_raw(40, 20, vec![0u8; 40 * 20 * 4])
        .unwrap()
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .unwrap();
    let ctx = h.ctx.clone();
    let shapes = project(&h).graph(graph).unwrap().layers[0].shapes.len();
    h.state_mut()
        .apply(Action::InsertGraphImage { graph, layer: 0, page: (100.0, 80.0), png: std::sync::Arc::new(png) }, &ctx);
    settle(&mut h);
    let g = project(&h).graph(graph).unwrap();
    assert_eq!(g.layers[0].shapes.len(), shapes + 1);
    let s = g.layers[0].shapes.last().unwrap();
    assert_eq!(s.kind, sheaf_core::shape::ShapeKind::Image);
    assert_eq!(s.picture.as_ref().map(|p| (p.width_px, p.height_px)), Some((40, 20)));
    assert_eq!(h.state().doc.undo_label(), Some("Insert Image"));
    // A file that is not a PNG changes nothing.
    h.state_mut().apply(
        Action::InsertGraphImage { graph, layer: 0, page: (0.0, 0.0), png: std::sync::Arc::new(vec![1, 2, 3]) },
        &ctx,
    );
    assert_eq!(project(&h).graph(graph).unwrap().layers[0].shapes.len(), shapes + 1);
}

#[test]
fn window_properties_list_a_books_dependents_and_a_graphs_books() {
    let mut h = start(vec![data_file("window-dependents")]);
    let book = project(&h).workbooks().next().unwrap().id;
    let graph = project(&h).graphs().next().unwrap().id;
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::RenameWindow(book), &ctx);
    settle(&mut h);
    let graph_name = project(&h).graph(graph).unwrap().name.clone();
    assert!(h.query_by_label(&format!("Dependents: {graph_name}")).is_some(), "the graph plots the book");
    h.state_mut().rename = None;
    h.state_mut().apply(Action::RenameWindow(graph), &ctx);
    settle(&mut h);
    let book_name = project(&h).workbook(book).unwrap().name.clone();
    assert!(h.query_by_label(&format!("Plots data of: {book_name}")).is_some());
}
