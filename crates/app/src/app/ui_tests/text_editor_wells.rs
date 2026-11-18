//! Saved choices reuse inline selection, draft, focus and persistence semantics.
use super::*;
use sheaf_core::Color;

const RGB: Color = Color::rgb(17, 131, 213);

fn profile(tag: &str) -> (PathBuf, PathBuf) {
    let data = data_file(tag);
    let file = data.parent().unwrap().join("custom-colors.json");
    (data, file)
}

fn start_with(data: &Path, file: &Path) -> Harness<'static, SheafApp> {
    let startup =
        Startup { paths: vec![data.to_path_buf()], custom_colors_path: Some(file.to_path_buf()), ..Default::default() };
    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .with_step_dt(0.02)
        .build_eframe(|cc| SheafApp::new(cc, startup));
    h.state_mut().panels.set_pinned(PanelId::Explorer, true);
    settle(&mut h);
    show_window(&mut h, "Graph1 - sample");
    h
}

fn seed(file: &Path) {
    std::fs::write(file, serde_json::to_vec(&serde_json::json!({"version":1,"colors":[RGB]})).unwrap()).unwrap();
}

fn channel(h: &mut Harness<'static, SheafApp>, name: &str, value: &str) {
    h.get_by_role_and_label(egui::accesskit::Role::TextInput, name).click();
    settle(h);
    ctrl(h, egui::Key::A);
    h.event(egui::Event::Text(value.to_string()));
    settle(h);
}

fn add(h: &mut Harness<'static, SheafApp>) {
    menu_path(h, &["Text color", "Custom RGB..."]);
    for (name, value) in [("Red", "17"), ("Green", "131"), ("Blue", "213")] {
        channel(h, name, value);
    }
    click(h, "Text color: Add to Custom");
    assert!(h.get_by_label("Text color: Add to Custom").accesskit_node().is_disabled());
}

fn dismiss(h: &mut Harness<'static, SheafApp>) {
    for _ in 0..2 {
        h.key_press(egui::Key::Escape);
        settle(h);
    }
}

fn choose(h: &mut Harness<'static, SheafApp>) {
    menu_path(h, &["Text color", "Text color: Custom colors", "Text color: Custom 17,131,213"]);
}

fn texts(h: &Harness<'static, SheafApp>, kind: ObjectKind) -> Vec<(String, Color)> {
    let scene = sheaf_plot::graph_scene(project(h), graph(h).id).unwrap();
    let layer = &scene.layers[0];
    let object = layer.objects.iter().find(|o| o.kind == kind).unwrap();
    layer.overlay[object.items.clone()]
        .iter()
        .filter_map(|item| match item {
            sheaf_plot::scene::Item::Text(t) => Some((t.text.clone(), t.color)),
            _ => None,
        })
        .collect()
}

#[test]
fn editor_add_and_cancel_survive_restart_and_share_the_toolbar_pool() {
    let (data, file) = profile("editor-wells-restart");
    let mut h = start_with(&data, &file);
    let before = project(&h).clone();
    let revision = h.state().doc.revision();
    menu(&mut h, "Graph", "Add Text...");
    type_into_text_window(&mut h, "discard this draft", false);
    add(&mut h);
    dismiss(&mut h);
    click(&mut h, "Cancel");
    assert_eq!(project(&h), &before);
    assert_eq!(h.state().doc.revision(), revision);
    let saved = std::fs::read(&file).unwrap();
    drop(h);
    crate::custom_colors::initialize(None);
    let mut fresh = start_with(&data, &file);
    let before = project(&fresh).clone();
    let id = graph(&fresh).id;
    menu_path(&mut fresh, &["Graph", "Legend", "Edit Legend Text..."]);
    ctrl(&mut fresh, egui::Key::A);
    choose(&mut fresh);
    assert_eq!(project(&fresh), &before);
    click(&mut fresh, "Cancel");
    assert_eq!(project(&fresh), &before);
    fresh.state_mut().graphs.get_mut(&id).unwrap().selected = Some((0, ObjectKind::Legend));
    settle(&mut fresh);
    for label in ["Format toolbar: Font Color", "Font Color: Custom colors", "Font Color: Custom 17,131,213"] {
        click(&mut fresh, label);
    }
    let mut expected = before.clone();
    expected.graph_mut(id).unwrap().layers[0].legend.font.color = RGB;
    assert_eq!(project(&fresh), &expected);
    menu(&mut fresh, "Edit", "Undo");
    assert_eq!(project(&fresh), &before);
    assert_eq!(std::fs::read(&file).unwrap(), saved, "choose/Cancel/Undo do not rewrite wells");
}

#[test]
fn add_does_not_rewrite_the_text_draft_and_saved_color_keeps_unicode_script_selection_and_focus() {
    let (data, file) = profile("editor-wells-selection");
    let mut h = start_with(&data, &file);
    menu(&mut h, "Graph", "Add Text...");
    type_into_text_window(&mut h, "unchanged", false);
    add(&mut h);
    dismiss(&mut h);
    click(&mut h, "OK");
    assert_eq!(labels(&h)[0].text, "unchanged", "Add does not colour or rewrite a draft");
    menu(&mut h, "Edit", "Undo");
    let before = project(&h).clone();
    menu(&mut h, "Graph", "Add Text...");
    type_into_text_window(&mut h, "μA cm-2", false);
    for _ in 0..2 {
        h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::ArrowLeft);
        settle(&mut h);
    }
    click(&mut h, "Superscript");
    choose(&mut h);
    choose(&mut h);
    type_into_text_window(&mut h, "-1", false);
    assert_eq!(project(&h), &before);
    click(&mut h, "Apply");
    let applied = project(&h).clone();
    let revision = h.state().doc.revision();
    assert_eq!(sheaf_plot::markup::plain(&labels(&h)[0].text), "μA cm-1");
    let runs = sheaf_plot::markup::parse(&labels(&h)[0].text);
    assert!(runs.iter().any(|r| r.text == "-1" && r.scale < 1.0 && r.color == Some(RGB)));
    assert_eq!(labels(&h)[0].text.matches(r"\c").count(), 1, "recolouring the same selection is idempotent");
    assert_eq!(labels(&h)[0].font.color, Color::BLACK);
    click(&mut h, "Apply");
    assert_eq!(project(&h), &applied);
    assert_eq!(h.state().doc.revision(), revision);
    menu_path(&mut h, &["Text color", "Custom RGB..."]);
    for (name, value) in [("Red", "17"), ("Green", "131"), ("Blue", "213")] {
        assert_eq!(
            h.get_by_role_and_label(egui::accesskit::Role::TextInput, name).accesskit_node().value().as_deref(),
            Some(value)
        );
    }
    dismiss(&mut h);
    type_into_text_window(&mut h, "discard", true);
    click(&mut h, "Cancel");
    assert_eq!(project(&h), &applied);
    assert!(texts(&h, ObjectKind::Label(0)).iter().any(|(s, c)| s == "-1" && *c == RGB));
    menu(&mut h, "Edit", "Undo");
    assert_eq!(project(&h), &before);
}

#[test]
fn saved_colors_reach_both_titles_and_legend_preserving_fonts_references_and_both_file_formats() {
    let (data, file) = profile("editor-wells-targets");
    seed(&file);
    let mut h = start_with(&data, &file);
    let before = project(&h).clone();
    let original = graph(&h).layers[0].clone();
    for vertical in [false, true] {
        menu_path(&mut h, &["Format", "Axis Titles", if vertical { "Y Axis Title..." } else { "X Axis Title..." }]);
        ctrl(&mut h, egui::Key::A);
        choose(&mut h);
        click(&mut h, "OK");
    }
    menu_path(&mut h, &["Graph", "Legend", "Edit Legend Text..."]);
    ctrl(&mut h, egui::Key::A);
    choose(&mut h);
    click(&mut h, "OK");
    let expected = graph(&h).clone();
    let layer = &expected.layers[0];
    assert_eq!(layer.legend.font, original.legend.font);
    assert_eq!(layer.x_axis.title_font, original.x_axis.title_font);
    assert_eq!(layer.y_axis.title_font, original.y_axis.title_font);
    let raw = layer.legend.text.as_deref().unwrap();
    for reference in [r"\l(1)", r"\l(2)", "%(1)", "%(2)"] {
        assert!(raw.contains(reference), "{reference}: {raw}");
    }
    for kind in [ObjectKind::XTitle, ObjectKind::YTitle, ObjectKind::Legend] {
        let rendered = texts(&h, kind);
        assert!(!rendered.is_empty() && rendered.iter().all(|(_, c)| *c == RGB));
    }
    assert_eq!(texts(&h, ObjectKind::Legend).iter().map(|(s, _)| s.as_str()).collect::<Vec<_>>(), ["up", "down"]);
    let svg = sheaf_plot::render_svg(project(&h), expected.id).unwrap();
    assert!(svg.contains("#1183D5"));
    for ext in ["sheaf", "opj"] {
        let path = data.parent().unwrap().join(format!("inline-wells.{ext}"));
        sheaf_io::save::save_project(project(&h), &path).unwrap();
        let reopened = sheaf_io::open_project(&path).unwrap();
        assert_eq!(reopened.project.graph(expected.id).unwrap(), &expected, "{ext}");
    }
    for _ in 0..3 {
        menu(&mut h, "Edit", "Undo");
    }
    assert_eq!(project(&h), &before);
}

#[test]
fn invalid_add_and_preference_removal_keep_automatic_text_and_history_unchanged() {
    let (data, file) = profile("editor-wells-automatic");
    let mut h = start_with(&data, &file);
    let before = project(&h).clone();
    let revision = h.state().doc.revision();
    menu_path(&mut h, &["Format", "Axis Titles", "X Axis Title..."]);
    menu_path(&mut h, &["Text color", "Custom RGB..."]);
    channel(&mut h, "Red", "999");
    assert!(h.get_by_label("Text color: Add to Custom").accesskit_node().is_disabled());
    assert!(!file.exists());
    dismiss(&mut h);
    add(&mut h);
    dismiss(&mut h);
    menu_path(&mut h, &["Text color", "Text color: Custom colors"]);
    right_click(&mut h, "Text color: Custom 17,131,213");
    click(&mut h, "Remove custom color");
    click(&mut h, "OK");
    assert_eq!(project(&h), &before);
    assert_eq!(h.state().doc.revision(), revision);
    let record: serde_json::Value = serde_json::from_slice(&std::fs::read(file).unwrap()).unwrap();
    assert_eq!(record["colors"], serde_json::json!([]));
}

#[test]
fn saved_choice_in_a_replaced_project_cannot_apply_a_stale_editor_draft() {
    let (data, file) = profile("editor-wells-generation");
    seed(&file);
    let mut h = start_with(&data, &file);
    menu(&mut h, "Graph", "Add Text...");
    type_into_text_window(&mut h, "stale", false);
    choose(&mut h);
    let replacement = project(&h).clone();
    h.state_mut().doc.replace(replacement.clone());
    settle(&mut h);
    assert!(h.query_by_label("Formatted text preview").is_none());
    assert_eq!(project(&h), &replacement);
    assert!(h.state().doc.undo_label().is_none());
}
