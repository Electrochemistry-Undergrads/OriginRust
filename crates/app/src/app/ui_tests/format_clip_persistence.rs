//! Restart and failure behavior with explicitly isolated format-clipboard paths.
use super::*;
use sheaf_core::Color;

fn start_with(data: &Path, file: &Path) -> Harness<'static, SheafApp> {
    let startup = Startup {
        paths: vec![data.to_path_buf()],
        format_clipboard_path: Some(file.to_path_buf()),
        ..Default::default()
    };
    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .with_step_dt(0.02)
        .build_eframe(|cc| SheafApp::new(cc, startup));
    h.state_mut().panels.set_pinned(PanelId::Explorer, true);
    settle(&mut h);
    show_window(&mut h, "Graph1 - sample");
    h
}

fn edit(h: &mut Harness<'static, SheafApp>, change: impl FnOnce(&mut sheaf_core::Layer)) -> WindowId {
    let id = project(h).graphs().next().unwrap().id;
    h.state_mut().doc.apply("fixture", |p| change(&mut p.graph_mut(id).unwrap().layers[0]));
    settle(h);
    id
}

fn pick(h: &mut Harness<'static, SheafApp>, label: &str) {
    menu_entry(h, label).click();
    settle(h);
}

fn axis_menu(h: &mut Harness<'static, SheafApp>) {
    let at = on(h, 0, ObjectKind::XAxis);
    right_click_at(h, at);
}

#[test]
fn graph_color_clipboard_loads_in_a_fresh_app_and_paste_undo_do_not_rewrite_it() {
    let data = data_file("format-clip-restart-graph");
    let file = data.parent().unwrap().join("format-clipboard.json");
    let mut source = start_with(&data, &file);
    edit(&mut source, |l| l.x_axis.color = Color::RED);
    let before_copy = project(&source).clone();
    let revision = source.state().doc.revision();
    menu_path(&mut source, &["Edit", "Copy Format", "Colors"]);
    assert_eq!(project(&source), &before_copy);
    assert_eq!(source.state().doc.revision(), revision);
    let saved = std::fs::read(&file).unwrap();
    drop(source);
    crate::format_clip_ui::initialize(None);
    assert_eq!(crate::format_clip_ui::kind(), None, "no surviving in-memory copy can hide a load failure");
    let mut target = start_with(&data, &file);
    assert_eq!(crate::format_clip_ui::kind(), Some(crate::format_clip_ui::ClipKind::Graph));
    let id = project(&target).graphs().next().unwrap().id;
    let before = project(&target).graph(id).unwrap().clone();
    let mut expected = before.clone();
    expected.layers[0].x_axis.color = Color::RED;
    menu(&mut target, "Edit", "Paste Format...");
    assert!(target.query_by_label("Fonts").is_none(), "only the captured Colors group is available");
    click(&mut target, "Apply");
    assert_eq!(project(&target).graph(id).unwrap(), &expected);
    assert_eq!(target.state().doc.undo_label(), Some("Apply Theme"));
    menu(&mut target, "Edit", "Undo Apply Theme");
    assert_eq!(project(&target).graph(id).unwrap(), &before);
    assert_eq!(std::fs::read(&file).unwrap(), saved);
}

#[test]
fn legend_style_clipboard_restores_and_paste_keeps_target_content() {
    let data = data_file("format-clip-restart-legend");
    let file = data.parent().unwrap().join("format-clipboard.json");
    let mut source = start_with(&data, &file);
    edit(&mut source, |l| {
        l.legend.font.color = Color::RED;
        l.legend.font.size_pt = 18.0;
        l.legend.frame = true;
        l.legend.background = Some(Color::rgb(230, 245, 250));
        l.legend.text = Some("Source legend".into());
    });
    let at = on(&source, 0, ObjectKind::Legend);
    right_click_at(&mut source, at);
    pick(&mut source, "Copy Format");
    pick(&mut source, "Styles");
    let saved = std::fs::read(&file).unwrap();
    drop(source);
    crate::format_clip_ui::initialize(None);

    let mut target = start_with(&data, &file);
    assert_eq!(crate::format_clip_ui::kind(), Some(crate::format_clip_ui::ClipKind::Legend));
    let id = edit(&mut target, |l| l.legend.text = Some("Target legend".into()));
    let before = project(&target).graph(id).unwrap().clone();
    axis_menu(&mut target);
    assert!(not_toolbar(&target, "Paste Format To").is_empty());
    key(&mut target, egui::Key::Escape);
    let at = on(&target, 0, ObjectKind::Legend);
    right_click_at(&mut target, at);
    pick(&mut target, "Paste Format To");
    pick(&mut target, "Styles");
    let mut expected = before.clone();
    let legend = &mut expected.layers[0].legend;
    legend.font.color = Color::RED;
    legend.font.size_pt = 18.0;
    legend.frame = true;
    legend.background = Some(Color::rgb(230, 245, 250));
    assert_eq!(project(&target).graph(id).unwrap(), &expected);
    assert_eq!(std::fs::read(&file).unwrap(), saved);
}

#[test]
fn partial_axis_clipboard_survives_restart_and_pastes_scales_without_source_styles() {
    let data = data_file("format-clip-restart-axis");
    let file = data.parent().unwrap().join("format-clipboard.json");
    let mut source = start_with(&data, &file);
    edit(&mut source, |l| {
        l.x_axis.min = Some(1.0);
        l.x_axis.max = Some(3.0);
        l.x_axis.major_step = Some(0.5);
        l.x_axis.color = Color::RED;
        l.x_axis.tick_font.size_pt = 22.0;
    });
    axis_menu(&mut source);
    pick(&mut source, "Copy Format");
    pick(&mut source, "Scales");
    let saved = std::fs::read(&file).unwrap();
    drop(source);
    crate::format_clip_ui::initialize(None);
    assert_eq!(crate::format_clip_ui::kind(), None);
    let mut target = start_with(&data, &file);
    assert_eq!(crate::format_clip_ui::kind(), Some(crate::format_clip_ui::ClipKind::Axis));
    let id = edit(&mut target, |l| {
        l.x_axis.color = Color::BLUE;
        l.x_axis.tick_font.size_pt = 10.0;
    });
    let before = project(&target).graph(id).unwrap().clone();
    let mut expected = before.clone();
    let a = &mut expected.layers[0].x_axis;
    a.min = Some(1.0);
    a.max = Some(3.0);
    a.major_step = Some(0.5);
    axis_menu(&mut target);
    pick(&mut target, "Paste Format To");
    pick(&mut target, "All");
    assert_eq!(project(&target).graph(id).unwrap(), &expected);
    menu(&mut target, "Edit", "Undo Apply Theme");
    assert_eq!(project(&target).graph(id).unwrap(), &before);
    target.key_press_modifiers(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::Z);
    settle(&mut target);
    assert_eq!(project(&target).graph(id).unwrap(), &expected);
    assert_eq!(std::fs::read(&file).unwrap(), saved);
}

#[test]
fn corrupt_clipboard_starts_empty_and_a_new_copy_replaces_it_for_the_next_app() {
    let data = data_file("format-clip-corrupt");
    let file = data.parent().unwrap().join("format-clipboard.json");
    std::fs::write(&file, "broken JSON").unwrap();
    let mut h = start_with(&data, &file);
    assert_eq!(crate::format_clip_ui::kind(), None);
    menu(&mut h, "Edit", "Paste Format...");
    assert!(h.query_by_value("The clipboard holds no format.").is_some());
    assert!(h.get_by_label("Apply").accesskit_node().is_disabled());
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "broken JSON");
    click(&mut h, "Close");
    menu_path(&mut h, &["Edit", "Copy Format", "Colors"]);
    drop(h);
    crate::format_clip_ui::initialize(None);
    let _fresh = start_with(&data, &file);
    assert_eq!(crate::format_clip_ui::kind(), Some(crate::format_clip_ui::ClipKind::Graph));
}

#[test]
fn storage_write_failure_keeps_the_in_memory_copy_and_does_not_touch_the_blocker() {
    let data = data_file("format-clip-write-failure");
    let blocker = data.parent().unwrap().join("not-a-directory");
    std::fs::write(&blocker, "keep me").unwrap();
    let file = blocker.join("format-clipboard.json");
    let mut h = start_with(&data, &file);
    let id = edit(&mut h, |l| l.x_axis.color = Color::RED);
    menu_path(&mut h, &["Edit", "Copy Format", "Colors"]);
    assert_eq!(crate::format_clip_ui::kind(), Some(crate::format_clip_ui::ClipKind::Graph));
    assert_eq!(std::fs::read_to_string(&blocker).unwrap(), "keep me");
    edit(&mut h, |l| l.x_axis.color = Color::BLUE);
    menu(&mut h, "Edit", "Paste Format...");
    click(&mut h, "Apply");
    assert_eq!(project(&h).graph(id).unwrap().layers[0].x_axis.color, Color::RED);
    assert_eq!(h.state().doc.undo_label(), Some("Apply Theme"));
    assert!(!file.exists());
}
