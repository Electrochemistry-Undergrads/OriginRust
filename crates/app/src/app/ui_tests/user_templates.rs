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
