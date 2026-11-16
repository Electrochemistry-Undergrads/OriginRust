//! Colour changes are text drafts; the existing renderer owns their appearance.
use super::*;
use sheaf_core::Color;

fn color(h: &mut Harness<'static, SheafApp>, name: &str) {
    click(h, "Text color");
    click(h, name);
}

fn object_texts(h: &Harness<'static, SheafApp>, kind: ObjectKind) -> Vec<(String, Color)> {
    let scene = sheaf_plot::graph_scene(project(h), graph(h).id).unwrap();
    let layer = scene.layers.iter().find(|l| l.objects.iter().any(|o| o.layer == 0 && o.kind == kind)).unwrap();
    let object = layer.objects.iter().find(|o| o.layer == 0 && o.kind == kind).unwrap();
    layer.overlay[object.items.clone()]
        .iter()
        .filter_map(|item| {
            if let sheaf_plot::scene::Item::Text(text) = item { Some((text.text.clone(), text.color)) } else { None }
        })
        .collect()
}

#[test]
fn text_editor_color_preserves_unicode_selection_script_and_typing_focus() {
    let mut h = start(vec![data_file("editor-color-selection")]);
    let before = project(&h).clone();
    menu(&mut h, "Graph", "Add Text...");
    type_into_text_window(&mut h, "μA cm-2", false);
    for _ in 0..2 {
        h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::ArrowLeft);
        settle(&mut h);
    }
    click(&mut h, "Superscript");
    color(&mut h, "Blue");
    type_into_text_window(&mut h, "-1", false);
    assert_eq!(project(&h), &before);
    assert!(h.query_by_label("Formatted text preview").is_some());
    click(&mut h, "OK");
    assert_eq!(labels(&h)[0].text, r"μA cm\+(\c4(-1))");
    let text = object_texts(&h, ObjectKind::Label(0));
    assert_eq!(text.iter().filter(|(_, c)| *c == Color::BLUE).map(|(s, _)| s.as_str()).collect::<String>(), "-1");
    assert!(text.iter().any(|(s, c)| s.contains("cm") && *c == Color::BLACK));
    ctrl(&mut h, egui::Key::Z);
    assert_eq!(project(&h), &before);
}

#[test]
fn text_editor_color_reapply_is_a_noop_and_cancel_keeps_only_the_applied_draft() {
    let mut h = start(vec![data_file("editor-color-apply")]);
    let before = project(&h).clone();
    menu(&mut h, "Graph", "Add Text...");
    type_into_text_window(&mut h, "selected", false);
    ctrl(&mut h, egui::Key::A);
    color(&mut h, "Red");
    click(&mut h, "Apply");
    let applied = project(&h).clone();
    color(&mut h, "Red");
    click(&mut h, "Apply");
    assert_eq!(project(&h), &applied);
    assert_eq!(h.state().doc.undo_label(), Some("Add text"));
    type_into_text_window(&mut h, "discard this", true);
    click(&mut h, "Cancel");
    assert_eq!(project(&h), &applied);
    ctrl(&mut h, egui::Key::Z);
    assert_eq!(project(&h), &before);
}

#[test]
fn text_editor_color_caret_white_and_normal_use_existing_history_and_base_font() {
    let mut h = start(vec![data_file("editor-color-caret")]);
    let before = project(&h).clone();
    menu(&mut h, "Graph", "Add Text...");
    color(&mut h, "White");
    type_into_text_window(&mut h, "X", false);
    click(&mut h, "Apply");
    assert_eq!(labels(&h)[0].text, r"\c18(X)");
    assert_eq!(labels(&h)[0].font.color, Color::BLACK);
    let white = project(&h).clone();
    click(&mut h, "OK");
    let at = on(&h, 0, ObjectKind::Label(0));
    double_click_at(&mut h, at);
    ctrl(&mut h, egui::Key::A);
    click(&mut h, "Normal");
    click(&mut h, "OK");
    assert_eq!(labels(&h)[0].text, "X");
    ctrl(&mut h, egui::Key::Z);
    assert_eq!(project(&h), &white);
    ctrl(&mut h, egui::Key::Z);
    assert_eq!(project(&h), &before);
}

#[test]
fn text_editor_color_opening_a_palette_retains_automatic_text_without_a_choice() {
    let mut h = start(vec![data_file("editor-color-dismiss")]);
    let before = project(&h).clone();
    menu_path(&mut h, &["Format", "Axis Titles", "X Axis Title..."]);
    click(&mut h, "Text color");
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    click(&mut h, "OK");
    assert_eq!(project(&h), &before);
}

#[test]
fn text_editor_color_titles_and_legend_keep_references_base_fonts_and_project_roundtrips() {
    let mut h = start(vec![data_file("editor-color-titles")]);
    let before = project(&h).clone();
    let original = graph(&h).layers[0].clone();
    for (vertical, name) in [(false, "Blue"), (true, "Green")] {
        menu_path(&mut h, &["Format", "Axis Titles", if vertical { "Y Axis Title..." } else { "X Axis Title..." }]);
        ctrl(&mut h, egui::Key::A);
        color(&mut h, name);
        click(&mut h, "OK");
    }
    menu_path(&mut h, &["Graph", "Legend", "Edit Legend Text..."]);
    ctrl(&mut h, egui::Key::A);
    color(&mut h, "Red");
    click(&mut h, "OK");
    let expected = graph(&h).layers[0].clone();
    assert_eq!(expected.x_axis.title_font, original.x_axis.title_font);
    assert_eq!(expected.y_axis.title_font, original.y_axis.title_font);
    assert_eq!(expected.legend.font, original.legend.font);
    let raw = expected.legend.text.as_deref().unwrap();
    assert!(raw.contains(r"\l(1)") && raw.contains("%(1)") && raw.contains(r"\l(2)") && raw.contains("%(2)"));
    for (kind, expected_color) in [
        (ObjectKind::XTitle, Color::BLUE),
        (ObjectKind::YTitle, Color::rgb(0, 255, 0)),
        (ObjectKind::Legend, Color::RED),
    ] {
        let texts = object_texts(&h, kind);
        assert!(!texts.is_empty());
        assert!(texts.iter().all(|(_, c)| *c == expected_color), "{kind:?}: {texts:?}");
    }
    assert_eq!(
        object_texts(&h, ObjectKind::Legend).iter().map(|(s, _)| s.as_str()).collect::<Vec<_>>(),
        ["up", "down"]
    );
    let directory = save_dir("editor-color-roundtrip");
    for extension in ["sheaf", "opj"] {
        let path = directory.join(format!("colored.{extension}"));
        sheaf_io::save::save_project(project(&h), &path).unwrap();
        let reopened = sheaf_io::open_project(&path).unwrap();
        let layer = &reopened.project.graphs().next().unwrap().layers[0];
        assert_eq!(layer.x_axis.title, expected.x_axis.title);
        assert_eq!(layer.y_axis.title, expected.y_axis.title);
        assert_eq!(layer.legend, expected.legend);
        if extension == "sheaf" {
            println!("Editor color screenshot fixture: {}", path.display());
        }
    }
    for _ in 0..3 {
        ctrl(&mut h, egui::Key::Z);
    }
    assert_eq!(project(&h), &before);
}
