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

#[test]
fn a_report_previews_its_links_and_exports_html_with_the_graph() {
    // T439: a Notes window holding a cell link and a figure line.
    let mut h = start(Vec::new());
    let ctx = h.ctx.clone();
    let mut notes = sheaf_core::WindowId(0);
    h.state_mut().doc.apply("data", |p| {
        let b = p.add_workbook("");
        p.add_column(b, 0, sheaf_core::Designation::X, sheaf_core::ColumnData::numeric(vec![1.0, 2.0])).unwrap();
        let y =
            p.add_column(b, 0, sheaf_core::Designation::Y, sheaf_core::ColumnData::numeric(vec![5.0, 7.5])).unwrap();
        let g = p.plot_columns(sheaf_core::template::Template::Line, &[y]).unwrap();
        let (book, graph) = (p.workbook(b).unwrap().name.clone(), p.graph(g).unwrap().name.clone());
        notes = p.add_notes();
        p.notes_mut(notes).unwrap().text = format!("# Result\nPeak: %([{book}]Sheet1,B,2) mA\n![The run]({graph})");
    });
    settle(&mut h);
    h.state_mut().apply(Action::Focus(notes), &ctx);
    settle(&mut h);
    h.get_by_label("Preview").click();
    settle(&mut h);
    h.get_by_label("Peak: 7.5 mA");
    h.get_by_label("The run");
    // The figure is the graph drawn (T459), named for screen readers.
    let graph_name = project(&h).graphs().next().unwrap().name.clone();
    assert!(h.query_by_label(&format!("Figure: {graph_name}")).is_some(), "the figure's picture");
    let dir = std::env::temp_dir().join(format!("sheaf-app-ui-report-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("run3.html");
    h.state_mut().apply(Action::ExportReportTo { notes, path: path.clone() }, &ctx);
    let html = std::fs::read_to_string(&path).unwrap();
    assert!(html.contains("<h1>Result</h1>") && html.contains("Peak: 7.5 mA"));
    let pngs: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().is_some_and(|x| x == "png"))
        .collect();
    assert_eq!(pngs.len(), 1, "the figure's graph beside the page");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn auto_refresh_re_imports_a_changed_file_and_pauses_when_it_is_gone() {
    // T442: an imported CSV, File > Import > Auto Refresh... OK (60 s, only when changed).
    let path = data_file("auto-refresh");
    let mut h = start(vec![path.clone()]);
    let ctx = h.ctx.clone();
    settle(&mut h);
    let book = project(&h).workbooks().next().expect("the imported book").id;
    h.state_mut().apply(Action::AutoRefresh { book }, &ctx);
    settle(&mut h);
    h.get_by_label("OK").click();
    settle(&mut h);
    assert!(project(&h).workbook(book).unwrap().refresh.is_some());
    let rows = |h: &Harness<'static, SheafApp>| project(h).workbook(book).unwrap().sheets[0].row_count();
    assert_eq!(rows(&h), 4);
    // The clock started with the frames shown so far; nothing is due for about a minute.
    let wait = h.state_mut().run_auto_refresh(0.0).expect("a clock");
    assert!((59.0..=61.0).contains(&wait), "{wait}");
    std::fs::write(&path, "t,up,down\n1,2.5,30.5\n2,4.5,20.5\n3,6.5,10.5\n4,8.5,0.5\n5,9.5,0.1\n").unwrap();
    h.state_mut().run_auto_refresh(30.0);
    assert_eq!(rows(&h), 4, "not due before 60 s");
    h.state_mut().run_auto_refresh(61.0);
    assert_eq!(rows(&h), 5, "re-imported once due");
    // Unchanged: nothing is read again (the undo step stays the last re-import).
    h.state_mut().doc.apply("mark", |p| p.workbook_mut(book).unwrap().long_name = "kept".into());
    h.state_mut().run_auto_refresh(122.0);
    assert_eq!(h.state().doc.undo_label(), Some("mark"));
    // The file is gone: the refresh pauses.
    std::fs::remove_file(&path).unwrap();
    h.state_mut().run_auto_refresh(183.0);
    assert!(project(&h).workbook(book).unwrap().refresh.unwrap().paused);
    assert_eq!(h.state_mut().run_auto_refresh(244.0), None, "a paused book has no clock");
}

#[test]
fn the_operation_tree_lists_a_books_linked_analysis_and_switches_its_mode() {
    // T443: a Smooth run with Recalculate Manual, the Object Manager on the workbook.
    use sheaf_core::analysis::{OutputTarget, Request, RowRange, Tool};
    use sheaf_core::column::RecalcMode;
    let mut h = start(Vec::new());
    let ctx = h.ctx.clone();
    let mut book = sheaf_core::WindowId(0);
    h.state_mut().doc.apply("data", |p| {
        book = p.add_workbook("");
        let xs = (0..10).map(f64::from).collect();
        p.add_column(book, 0, sheaf_core::Designation::X, sheaf_core::ColumnData::numeric(xs)).unwrap();
        let ys = (0..10).map(|i| f64::from(i * i)).collect();
        let y = p.add_column(book, 0, sheaf_core::Designation::Y, sheaf_core::ColumnData::numeric(ys)).unwrap();
        let o = sheaf_calc::SmoothOptions {
            method: sheaf_calc::SmoothMethod::AdjacentAveraging { weighted: false },
            npts: 3,
            boundary: sheaf_calc::Boundary::None,
        };
        let req = Request { columns: vec![y], rows: RowRange::All, tool: Tool::Smooth(o) };
        p.analyze_linked(&req, &OutputTarget::NextToInput, RecalcMode::Manual).unwrap();
    });
    h.state_mut().apply(Action::Focus(book), &ctx);
    h.state_mut().panels.set_pinned(PanelId::Objects, true);
    settle(&mut h);
    let at = h.get_by_label("Smooth (Manual)").rect().center();
    right_click_at(&mut h, at);
    settle(&mut h);
    h.get_by_label("Auto").click();
    settle(&mut h);
    assert_eq!(project(&h).analysis_links[0].mode, RecalcMode::Auto);
    assert_eq!(h.state().doc.undo_label(), Some("Recalculate Mode"));
    h.get_by_label("Smooth (Auto)");
    let book_name = project(&h).workbook(book).unwrap().name.clone();
    h.get_by_label(&format!("Input: [{book_name}]Sheet1!B"));
}

#[test]
fn a_cell_formula_is_typed_shown_in_the_formula_bar_and_follows_its_input() {
    // T448: =A1*10 typed into B1; the Formula Bar shows the formula when B1 is the current cell.
    let mut h = start(Vec::new());
    let ctx = h.ctx.clone();
    let (mut book, mut a, mut b) = (sheaf_core::WindowId(0), sheaf_core::ColumnId(0), sheaf_core::ColumnId(0));
    h.state_mut().doc.apply("data", |p| {
        book = p.add_workbook("");
        a = p.add_column(book, 0, sheaf_core::Designation::X, sheaf_core::ColumnData::numeric(vec![2.0, 3.0])).unwrap();
        b = p.add_column(book, 0, sheaf_core::Designation::Y, sheaf_core::ColumnData::numeric(vec![f64::NAN])).unwrap();
    });
    h.state_mut().apply(Action::Focus(book), &ctx);
    settle(&mut h);
    h.state_mut().apply(Action::SetCell { column: b, row: 0, text: "=A1*10".into() }, &ctx);
    settle(&mut h);
    let cell = |h: &Harness<'static, SheafApp>| {
        sheaf_core::indexed::level_text(project(h).column(b).unwrap(), 0).unwrap_or_default()
    };
    assert_eq!(cell(&h), "20");
    assert_eq!(h.state().doc.undo_label(), Some("Cell Formula"));
    // An edit of A1 is followed in the same step.
    h.state_mut().apply(Action::SetCell { column: a, row: 0, text: "5".into() }, &ctx);
    assert_eq!(cell(&h), "50");
    // View > Formula Bar: it names the cell and shows the formula.
    h.state_mut().apply(Action::ToggleFormulaBar, &ctx);
    h.state_mut().books.get_mut(&book).unwrap().sel = crate::grid::Sel::Cells { anchor: (1, 0), end: (1, 0) };
    settle(&mut h);
    h.get_by_label("B1");
    assert!(h.query_all_by_value("=A1*10").next().is_some(), "the formula in the bar");
    // Fill Down (T455): B1's formula goes to B2 with its row shifted.
    h.get_by_label("Fill Down").click();
    settle(&mut h);
    assert_eq!(project(&h).cell_formula(b, 1), Some("A2*10"));
    assert_eq!(sheaf_core::indexed::level_text(project(&h).column(b).unwrap(), 1).as_deref(), Some("30"), "A2 is 3");
    assert_eq!(h.state().doc.undo_label(), Some("Fill Down"));
    // A bad formula is refused and the cell keeps its formula.
    h.state_mut().apply(Action::SetCell { column: b, row: 0, text: "=A1 +* 2".into() }, &ctx);
    assert_eq!(project(&h).cell_formula(b, 0), Some("A1*10"));
    // Typing a value over it drops the formula.
    h.state_mut().apply(Action::SetCell { column: b, row: 0, text: "7".into() }, &ctx);
    assert!(project(&h).cell_formula(b, 0).is_none());
    assert_eq!(cell(&h), "7");
}
