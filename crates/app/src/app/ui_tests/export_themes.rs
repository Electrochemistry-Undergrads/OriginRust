use super::*;

fn open(file: PathBuf, themes_dir: PathBuf) -> Harness<'static, SheafApp> {
    let startup = Startup { paths: vec![file], themes_dir: Some(themes_dir), ..Default::default() };
    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 900.0))
        .with_step_dt(0.02)
        .build_eframe(|cc| SheafApp::new(cc, startup));
    h.state_mut().panels.set_pinned(PanelId::Explorer, true);
    settle(&mut h);
    show_window(&mut h, "Graph1 - sample");
    ctrl(&mut h, egui::Key::G);
    assert!(h.state().export.is_some());
    h
}

fn theme_menu(h: &mut Harness<'static, SheafApp>, item: &str) {
    click(h, "▶");
    click(h, item);
}

fn svg(h: &mut Harness<'static, SheafApp>) {
    h.get_by_value("Portable Network Graphics (*.png)").click();
    settle(h);
    click(h, "Scalable Vector Graphics (*.svg)");
}

fn save_as(h: &mut Harness<'static, SheafApp>, name: &str) {
    theme_menu(h, "Save As...");
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.event(egui::Event::Text(name.into()));
    settle(h);
    h.get_by_label("Theme Save as...").get_by_label("OK").click();
    settle(h);
}

#[test]
fn graph_export_theme_row_opens_without_writing_settings_on_cancel() {
    let file = data_file("export-theme-cancel");
    let themes_dir = file.parent().unwrap().join("themes");
    let mut h = open(file, themes_dir.clone());
    assert!(h.query_by_label("Dialog Theme").is_some());
    theme_menu(&mut h, "Save As...");
    assert!(h.get_by_label("Apply").accesskit_node().is_disabled());
    h.event(egui::Event::Text("Unsaved theme".into()));
    settle(&mut h);
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    assert!(h.query_by_label("Theme Save as...").is_none());
    assert!(!h.get_by_label("Apply").accesskit_node().is_disabled());
    click(&mut h, "Cancel");
    assert!(h.state().export.is_none());
    assert!(!themes_dir.exists(), "Cancel wrote export settings");
}

#[test]
fn named_graph_export_theme_survives_reopening_and_apply_records_last_used() {
    let file = data_file("export-theme-persist");
    let dir = file.parent().unwrap().to_path_buf();
    let themes_dir = dir.join("themes");
    let mut h = open(file.clone(), themes_dir.clone());
    let before = project(&h).clone();
    svg(&mut h);
    h.state_mut().export.as_mut().unwrap().set_file_name("paper");
    h.state_mut().export.as_mut().unwrap().set_path(&dir.join("output"));
    save_as(&mut h, "Publication figures");
    theme_menu(&mut h, "Save As...");
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.event(egui::Event::Text("Publication   figures".into()));
    settle(&mut h);
    assert!(
        h.query(By::new().role(egui::accesskit::Role::Label).value("A theme of this name will be replaced.")).is_some()
    );
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    assert!(!dir.join("output/paper.svg").exists(), "saving a theme exported a graph");
    theme_menu(&mut h, "System Default");
    assert!(h.query_by_value("Portable Network Graphics (*.png)").is_some());
    theme_menu(&mut h, "Publication figures");
    assert!(h.query_by_value("Scalable Vector Graphics (*.svg)").is_some());
    h.state_mut().export.as_mut().unwrap().set_file_name("paper-revised");
    settle(&mut h);
    assert!(h.query(By::new().role(egui::accesskit::Role::TextInput).value("Publication figures *")).is_some());
    theme_menu(&mut h, "Save");
    assert!(h.query(By::new().role(egui::accesskit::Role::TextInput).value("Publication figures")).is_some());
    assert_eq!(project(&h), &before);
    click(&mut h, "Apply");
    assert!(dir.join("output/paper-revised.svg").is_file());
    click(&mut h, "Cancel");
    drop(h);
    let mut h = open(file, themes_dir);
    assert!(h.query_by_value("Scalable Vector Graphics (*.svg)").is_some());
    assert_eq!(h.state().export.as_ref().unwrap().name_for(&h.state().doc, graph(&h).id), "paper-revised");
    click(&mut h, "Cancel");
}

#[test]
fn graph_export_theme_defaults_and_delete_confirmation_preserve_the_project() {
    let file = data_file("export-theme-defaults-ui");
    let themes_dir = file.parent().unwrap().join("themes");
    let mut h = open(file, themes_dir);
    let before = project(&h).clone();
    svg(&mut h);
    save_as(&mut h, "Draft");
    theme_menu(&mut h, "Save as <default>");
    theme_menu(&mut h, "System Default");
    click(&mut h, "Cancel");
    ctrl(&mut h, egui::Key::G);
    assert!(h.query_by_value("Scalable Vector Graphics (*.svg)").is_some());
    theme_menu(&mut h, "Draft");
    theme_menu(&mut h, "Delete...");
    assert!(h.query_by_label("Delete Dialog Theme").is_some());
    assert!(h.get_by_label("Apply").accesskit_node().is_disabled());
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    theme_menu(&mut h, "Delete...");
    click(&mut h, "Delete");
    theme_menu(&mut h, "Reset");
    assert!(h.query_by_value("Portable Network Graphics (*.png)").is_some());
    assert_eq!(project(&h), &before);
    click(&mut h, "Cancel");
    ctrl(&mut h, egui::Key::G);
    assert!(h.query_by_value("Portable Network Graphics (*.png)").is_some());
    click(&mut h, "Cancel");
}
