//! User graph templates (T56): File > Save Template As..., Plot > User Templates, Manage...

use super::*;

fn start_with(file: PathBuf, templates_dir: Option<PathBuf>) -> Harness<'static, SheafApp> {
    let startup = Startup { paths: vec![file], templates_dir, ..Default::default() };
    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 900.0))
        .with_step_dt(0.02)
        .build_eframe(|cc| SheafApp::new(cc, startup));
    h.state_mut().panels.set_pinned(PanelId::Explorer, true);
    settle(&mut h);
    h
}

fn first_graph(h: &Harness<'static, SheafApp>) -> WindowId {
    project(h).graphs().next().expect("the start-up graph").id
}

/// Makes the first plot of the first graph look unlike the defaults: 4.5 pt, dashed.
fn style_graph(h: &mut Harness<'static, SheafApp>) {
    let id = first_graph(h);
    h.state_mut().doc.apply("Style", |p| {
        let plot = &mut p.graph_mut(id).unwrap().layers[0].plots[0];
        plot.line.width_pt = 4.5;
        plot.line.dash = sheaf_core::graph::Dash::Dash;
    });
    settle(h);
}

fn type_name(h: &mut Harness<'static, SheafApp>, name: &str) {
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.event(egui::Event::Text(name.into()));
    settle(h);
}

#[test]
fn a_saved_template_plots_other_columns_in_its_look_and_survives_a_restart() {
    let file = data_file("user-template");
    let dir = file.parent().unwrap().join("templates");
    let mut h = start_with(file.clone(), Some(dir.clone()));
    style_graph(&mut h);
    show_window(&mut h, "Graph1 - sample");
    menu_path(&mut h, &["File", "Save Template As..."]);
    assert!(h.state().template_prompt.is_some(), "the name prompt is open");
    type_name(&mut h, "Thick dashed");
    h.get_by_label("Save Template As").get_by_label("OK").click();
    settle(&mut h);
    assert!(h.state().template_prompt.is_none(), "OK saves and closes");
    assert_eq!(h.state().user_templates, ["Thick dashed"]);
    assert!(dir.join("Thick dashed.sheaftemplate").is_file());

    // Plot the second Y column with it: the new graph takes the saved look.
    let (book, down) = {
        let b = project(&h).workbooks().next().unwrap();
        (b.id, b.sheets[0].columns[2].id)
    };
    let graphs = project(&h).graphs().count();
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::PlotUserTemplate { book, columns: vec![down], name: "thick DASHED".into() }, &ctx);
    settle(&mut h);
    assert_eq!(project(&h).graphs().count(), graphs + 1);
    let made = project(&h).graphs().last().unwrap();
    let plot = &made.layers[0].plots[0];
    assert_eq!(plot.y, down);
    assert_eq!((plot.line.width_pt, plot.line.dash), (4.5, sheaf_core::graph::Dash::Dash));

    // A new run lists the template again, read from the folder.
    drop(h);
    let h = start_with(file, Some(dir));
    assert_eq!(h.state().user_templates, ["Thick dashed"]);
}

#[test]
fn manage_renames_and_deletes_templates() {
    let file = data_file("user-template-manage");
    let dir = file.parent().unwrap().join("templates");
    let mut h = start_with(file, Some(dir.clone()));
    show_window(&mut h, "Graph1 - sample");
    menu_path(&mut h, &["File", "Save Template As..."]);
    type_name(&mut h, "First");
    h.get_by_label("Save Template As").get_by_label("OK").click();
    settle(&mut h);
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::ManageTemplates, &ctx);
    settle(&mut h);
    click(&mut h, "First");
    h.query_all_by_role(egui::accesskit::Role::TextInput).last().expect("the new name box").focus();
    settle(&mut h);
    type_name(&mut h, "Second");
    click(&mut h, "Rename");
    assert_eq!(h.state().user_templates, ["Second"]);
    assert!(dir.join("Second.sheaftemplate").is_file() && !dir.join("First.sheaftemplate").exists());
    click(&mut h, "Delete");
    assert!(h.state().user_templates.is_empty());
    assert!(!dir.join("Second.sheaftemplate").exists());
    click(&mut h, "Close");
    assert!(h.state().template_manager.is_none());
}

#[test]
fn without_a_template_folder_save_template_as_is_greyed_and_nothing_is_written() {
    let file = data_file("user-template-none");
    let mut h = start_with(file, None);
    show_window(&mut h, "Graph1 - sample");
    h.get_by_label("File").click();
    settle(&mut h);
    assert!(!entry_enabled(&h, "Save Template As..."), "no folder: the item is greyed");
    assert!(h.state().user_templates.is_empty());
}

#[test]
fn a_workbook_saved_as_a_template_makes_an_empty_workbook_from_file_new_from_template() {
    let file = data_file("book-template");
    let dir = file.parent().unwrap().join("templates");
    let mut h = start_with(file, Some(dir.clone()));
    let book = project(&h).workbooks().next().unwrap().id;
    h.state_mut().doc.apply("Label", |p| {
        let b = p.workbook_mut(book).unwrap();
        let c = &mut b.sheets[0].columns[1];
        c.long_name = "Current".into();
        c.units = "uA".into();
    });
    settle(&mut h);
    show_window(&mut h, "Book1 - sample.csv");
    menu_path(&mut h, &["File", "Save Template As..."]);
    let prompt = h.state().template_prompt.as_ref().expect("the name prompt is open");
    assert_eq!(prompt.kind, sheaf_io::user_templates::TemplateKind::Workbook);
    type_name(&mut h, "Run log");
    h.get_by_label("Save Template As").get_by_label("OK").click();
    settle(&mut h);
    assert!(h.state().template_prompt.is_none(), "OK saves and closes");
    assert_eq!(h.state().user_book_templates, ["Run log"]);
    assert!(h.state().user_templates.is_empty(), "not a graph template");
    assert!(dir.join("Run log.sheafbooktemplate").is_file());

    let books = project(&h).workbooks().count();
    menu_path(&mut h, &["File", "New", "From Template", "Run log"]);
    assert_eq!(project(&h).workbooks().count(), books + 1);
    let made = project(&h).workbooks().last().unwrap();
    assert_ne!(made.id, book);
    let cols = &made.sheets[0].columns;
    assert_eq!(cols.len(), 3);
    assert!(cols.iter().all(|c| c.is_empty()), "no data");
    assert_eq!((cols[1].long_name.as_str(), cols[1].units.as_str()), ("Current", "uA"));
    assert_eq!(h.state().doc.undo_label(), Some("New workbook from template"));

    // Manage lists it under the workbook templates and deletes it.
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::ManageTemplates, &ctx);
    settle(&mut h);
    click(&mut h, "Run log");
    click(&mut h, "Delete");
    assert!(h.state().user_book_templates.is_empty());
    assert!(!dir.join("Run log.sheafbooktemplate").exists());
}

#[test]
fn an_analysis_template_brings_its_recalculating_run_to_a_new_book() {
    use sheaf_core::analysis::{Boundary, OutputTarget, Request, RowRange, SmoothMethod, SmoothOptions, Tool};
    let file = data_file("analysis-template");
    let dir = file.parent().unwrap().join("templates");
    let mut h = start_with(file, Some(dir.clone()));
    let (book, up) = {
        let b = project(&h).workbooks().next().unwrap();
        (b.id, b.sheets[0].columns[1].id)
    };
    h.state_mut().doc.apply("Smooth", |p| {
        let o = SmoothOptions {
            method: SmoothMethod::AdjacentAveraging { weighted: false },
            npts: 3,
            boundary: Boundary::None,
        };
        let request = Request { columns: vec![up], rows: RowRange::All, tool: Tool::Smooth(o) };
        p.analyze_linked(&request, &OutputTarget::NextToInput, sheaf_core::RecalcMode::Auto).unwrap();
    });
    settle(&mut h);
    show_window(&mut h, "Book1 - sample.csv");
    menu_path(&mut h, &["File", "Save Workbook As Analysis Template..."]);
    assert!(h.state().template_prompt.as_ref().is_some_and(|p| p.analysis), "the analysis prompt is open");
    type_name(&mut h, "Assay");
    h.get_by_label("Save As Analysis Template").get_by_label("OK").click();
    settle(&mut h);
    assert!(h.state().template_prompt.is_none());
    assert_eq!(h.state().user_book_templates, ["Assay"]);

    menu_path(&mut h, &["File", "New", "From Template", "Assay"]);
    let made = project(&h).workbooks().last().unwrap();
    assert_ne!(made.id, book);
    let (made, input, result) = (made.id, made.sheets[0].columns[1].id, made.sheets[0].columns[2].id);
    assert_eq!(project(&h).analysis_links.len(), 2);
    assert!(project(&h).column(result).unwrap().is_empty());

    // Data typed into the new book's input runs the smoothing again, in place.
    h.state_mut().doc.apply("Enter data", |p| {
        let b = p.workbook_mut(made).unwrap();
        b.sheets[0].columns[0].data = sheaf_core::ColumnData::numeric(vec![1.0, 2.0, 3.0, 4.0]);
        p.column_mut(input).unwrap().data = sheaf_core::ColumnData::numeric(vec![0.0, 3.0, 0.0, 3.0]);
    });
    settle(&mut h);
    assert_eq!(project(&h).column(result).unwrap().len(), 4, "the result follows the new data");
}

#[test]
fn batch_processing_makes_one_book_per_file_from_an_analysis_template() {
    use sheaf_core::analysis::{Boundary, OutputTarget, Request, RowRange, SmoothMethod, SmoothOptions, Tool};
    let file = data_file("batch-processing");
    let folder = file.parent().unwrap().to_path_buf();
    let dir = folder.join("templates");
    // An analysis template: X, Y and an Auto smoothing of Y.
    let mut p = sheaf_core::Project::new();
    let b = p.add_workbook("Assay");
    p.add_column(b, 0, sheaf_core::Designation::X, sheaf_core::ColumnData::numeric(vec![0.0, 1.0, 2.0])).unwrap();
    let y =
        p.add_column(b, 0, sheaf_core::Designation::Y, sheaf_core::ColumnData::numeric(vec![1.0, 0.0, 1.0])).unwrap();
    let o = SmoothOptions {
        method: SmoothMethod::AdjacentAveraging { weighted: false },
        npts: 3,
        boundary: Boundary::None,
    };
    let request = Request { columns: vec![y], rows: RowRange::All, tool: Tool::Smooth(o) };
    p.analyze_linked(&request, &OutputTarget::NextToInput, sheaf_core::RecalcMode::Auto).unwrap();
    let t = sheaf_core::BookTemplate::analysis_from_workbook("Assay", &p, b).unwrap();
    sheaf_io::user_templates::save_book(&dir, &t).unwrap();
    let runs = [folder.join("run1.csv"), folder.join("run2.csv"), folder.join("missing.csv")];
    std::fs::write(&runs[0], "t,v\n1,0\n2,3\n3,0\n4,3\n5,0\n").unwrap();
    std::fs::write(&runs[1], "t,v\n1,1\n2,1\n3,1\n").unwrap();

    let mut h = start_with(file, Some(dir));
    let books = project(&h).workbooks().count();
    show_window(&mut h, "Book1 - sample.csv");
    menu_path(&mut h, &["File", "Batch Processing..."]);
    assert!(h.state().batch.is_some(), "the window is open");
    h.state_mut().batch.as_mut().unwrap().add_files(runs.iter().cloned());
    settle(&mut h);
    h.get_by_label("Batch Processing").get_by_label("OK").click();
    settle(&mut h);
    assert!(h.state().batch.is_none(), "OK runs and closes");
    let made: Vec<_> = project(&h).workbooks().skip(books).collect();
    assert_eq!(made.len(), 2, "one book per readable file");
    assert_eq!(made[0].long_name, "run1.csv");
    assert_eq!(made[1].long_name, "run2.csv");
    let cols = &made[0].sheets[0].columns;
    assert_eq!(cols[1].numeric().unwrap(), [0.0, 3.0, 0.0, 3.0, 0.0]);
    assert_eq!(cols[2].len(), 5, "the smoothing ran on the file's data");
    assert_eq!(made[1].sheets[0].columns[2].len(), 3);
    assert_eq!(h.state().doc.undo_label(), Some("Batch processing"));
}
