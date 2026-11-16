//! Formatting the Text window's selection stays a draft until the user applies it.
use super::*;

#[test]
fn text_editor_superscript_wraps_only_selected_characters_and_applies_one_undo_step() {
    let mut h = start(vec![data_file("text-editor-script")]);
    let before = project(&h).clone();
    menu(&mut h, "Graph", "Add Text...");
    type_into_text_window(&mut h, "cm-2", false);
    for _ in 0..2 {
        h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::ArrowLeft);
        settle(&mut h);
    }
    click(&mut h, "Superscript");
    assert_eq!(project(&h), &before, "formatting and preview do not edit the project");
    assert!(h.query_by_label("Formatted text preview").is_some());
    click(&mut h, "OK");
    assert_eq!(labels(&h)[0].text, r"cm\+(-2)");
    assert_eq!(h.state().doc.undo_label(), Some("Add text"));
    ctrl(&mut h, egui::Key::Z);
    assert_eq!(project(&h), &before);
}

#[test]
fn text_editor_caret_greek_normal_and_empty_groups_are_drafts() {
    let mut h = start(vec![data_file("text-editor-caret")]);
    let before = project(&h).clone();
    menu(&mut h, "Graph", "Add Text...");
    click(&mut h, "Greek");
    type_into_text_window(&mut h, "m", false);
    assert!(
        h.query_by_label("Formatted text preview").is_some(),
        "typing enters the Greek group after clicking its button"
    );
    click(&mut h, "Apply");
    assert_eq!(labels(&h)[0].text, r"\g(m)");
    let applied = project(&h).clone();
    type_into_text_window(&mut h, "draft", true);
    click(&mut h, "Cancel");
    assert_eq!(project(&h), &applied);
    ctrl(&mut h, egui::Key::Z);
    assert_eq!(project(&h), &before);

    menu(&mut h, "Graph", "Add Text...");
    click(&mut h, "Superscript");
    click(&mut h, "Apply");
    assert_eq!(project(&h), &before, "an empty markup group adds no invisible label");
    click(&mut h, "OK");
    assert!(h.state().text_dialog.is_none());
    assert_eq!(project(&h), &before);
}

#[test]
fn text_editor_unicode_selection_nested_styles_toggle_and_normal() {
    let mut h = start(vec![data_file("text-editor-nested")]);
    menu(&mut h, "Graph", "Add Text...");
    type_into_text_window(&mut h, "μA", false);
    h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::ArrowLeft);
    settle(&mut h);
    click(&mut h, "Bold selection");
    click(&mut h, "Italic selection");
    click(&mut h, "Underline selection");
    click(&mut h, "Underline selection");
    click(&mut h, "Normal");
    click(&mut h, "OK");
    assert_eq!(labels(&h)[0].text, "μA");
}

#[test]
fn text_editor_markup_survives_native_and_origin_companion_reopen() {
    let mut h = start(vec![data_file("text-editor-save")]);
    menu(&mut h, "Graph", "Add Text...");
    type_into_text_window(&mut h, r"\c4(\u(j / \g(m)A cm\+(-2)))", false);
    click(&mut h, "OK");
    let expected = labels(&h)[0].clone();
    let directory = save_dir("text-editor-roundtrip");
    for extension in ["sheaf", "opj"] {
        let path = directory.join(format!("formatted.{extension}"));
        sheaf_io::save::save_project(project(&h), &path).unwrap();
        let reopened = sheaf_io::open_project(&path).unwrap();
        assert_eq!(reopened.project.graphs().next().unwrap().layers[0].labels[0], expected);
        if extension == "sheaf" {
            // A fresh synthetic fixture also permits an actual desktop screenshot outside the test runner.
            println!("Text editor screenshot fixture: {}", path.display());
        }
    }
}

#[test]
fn text_editor_keyboard_formats_the_selection_and_returns_focus_for_typing() {
    let mut h = start(vec![data_file("text-editor-shortcuts")]);
    menu(&mut h, "Graph", "Add Text...");
    type_into_text_window(&mut h, "2", false);
    h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::ArrowLeft);
    settle(&mut h);
    let control = egui::Modifiers { ctrl: true, command: true, ..Default::default() };
    for key in [egui::Key::B, egui::Key::B, egui::Key::U] {
        h.key_press_modifiers(control, key);
        settle(&mut h);
    }
    h.key_press_modifiers(egui::Modifiers { shift: true, ..control }, egui::Key::Equals);
    settle(&mut h);
    click(&mut h, "OK");
    assert_eq!(labels(&h)[0].text, r"\u(\+(2))");
}
