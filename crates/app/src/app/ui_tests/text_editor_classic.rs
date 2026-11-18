//! Classic choices remain inline text drafts through the shared text editor.
use super::*;
use sheaf_core::Color;

fn extra(h: &mut Harness<'static, SheafApp>, name: &str) {
    menu_path(h, &["Text color", "More Classic colors", name]);
    assert!(h.query_by_label("Black").is_none(), "both colour menus close after a choice");
}

#[test]
fn classic_royal_keeps_unicode_script_selection_and_restores_typing_focus() {
    let mut h = start(vec![data_file("editor-classic-selection")]);
    let before = project(&h).clone();
    menu(&mut h, "Graph", "Add Text...");
    type_into_text_window(&mut h, "μA cm-2", false);
    for _ in 0..2 {
        h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::ArrowLeft);
        settle(&mut h);
    }
    click(&mut h, "Superscript");
    extra(&mut h, "Royal");
    assert_eq!(project(&h), &before);
    type_into_text_window(&mut h, "-1", false);
    assert_eq!(project(&h), &before);
    assert!(h.query_by_label("Formatted text preview").is_some());
    click(&mut h, "OK");
    assert_eq!(labels(&h)[0].text, r"μA cm\+(\c14(-1))");
    let runs = sheaf_plot::markup::parse(&labels(&h)[0].text);
    assert!(runs.iter().any(|run| run.text == "-1" && run.scale < 1.0 && run.color == Some(Color::rgb(0, 0, 160))));
    assert_eq!(labels(&h)[0].font.color, Color::BLACK);
    ctrl(&mut h, egui::Key::Z);
    assert_eq!(project(&h), &before);
}

// Origin 2016 EMF indices (graphing7.2), also checked against both native worksheet palettes.
const CLASSIC: [(&str, u8, [u8; 3]); 24] = [
    ("Black", 1, [0, 0, 0]),
    ("Red", 2, [255, 0, 0]),
    ("Green", 3, [0, 255, 0]),
    ("Blue", 4, [0, 0, 255]),
    ("Cyan", 5, [0, 255, 255]),
    ("Magenta", 6, [255, 0, 255]),
    ("Yellow", 7, [255, 255, 0]),
    ("Dark Yellow", 8, [128, 128, 0]),
    ("Navy", 9, [0, 0, 128]),
    ("Purple", 10, [128, 0, 128]),
    ("Wine", 11, [128, 0, 0]),
    ("Olive", 12, [0, 128, 0]),
    ("Dark Cyan", 13, [0, 128, 128]),
    ("Royal", 14, [0, 0, 160]),
    ("Orange", 15, [255, 128, 0]),
    ("Violet", 16, [128, 0, 255]),
    ("Pink", 17, [255, 0, 128]),
    ("White", 18, [255, 255, 255]),
    ("LT Gray", 19, [192, 192, 192]),
    ("Gray", 20, [128, 128, 128]),
    ("LT Yellow", 21, [255, 255, 128]),
    ("LT Cyan", 22, [128, 255, 255]),
    ("LT Magenta", 23, [255, 128, 255]),
    ("Dark Gray", 24, [64, 64, 64]),
];

fn choice(h: &mut Harness<'static, SheafApp>, name: &str) {
    if ["Black", "Red", "Green", "Blue", "Cyan", "Magenta", "Yellow", "White"].contains(&name) {
        menu_path(h, &["Text color", name]);
    } else {
        extra(h, name);
    }
}

fn texts(h: &Harness<'static, SheafApp>, kind: ObjectKind) -> Vec<(String, Color)> {
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
fn every_named_classic_choice_uses_its_native_index_and_rendered_rgb() {
    let mut h = start(vec![data_file("editor-classic-all")]);
    for (name, index, [r, g, b]) in CLASSIC {
        let before = project(&h).clone();
        menu(&mut h, "Graph", "Add Text...");
        type_into_text_window(&mut h, "sample", false);
        ctrl(&mut h, egui::Key::A);
        choice(&mut h, name);
        assert_eq!(project(&h), &before, "{name} stays a draft");
        click(&mut h, "OK");
        assert_eq!(labels(&h)[0].text, format!(r"\c{index}(sample)"), "{name}");
        assert_eq!(labels(&h)[0].font.color, Color::BLACK);
        let rendered = texts(&h, ObjectKind::Label(0));
        assert!(
            !rendered.is_empty() && rendered.iter().all(|(_, c)| *c == Color::rgb(r, g, b)),
            "{name}: {rendered:?}"
        );
        ctrl(&mut h, egui::Key::Z);
        assert_eq!(project(&h), &before, "{name} has one Undo");
    }
}

#[test]
fn extra_classic_choices_are_idempotent_drafts_and_cancel_keeps_only_apply() {
    let mut h = start(vec![data_file("editor-classic-idempotence")]);
    let before = project(&h).clone();
    menu(&mut h, "Graph", "Add Text...");
    type_into_text_window(&mut h, "selected", false);
    ctrl(&mut h, egui::Key::A);
    extra(&mut h, "Olive");
    click(&mut h, "Apply");
    let applied = project(&h).clone();
    let revision = h.state().doc.revision();
    extra(&mut h, "Olive");
    click(&mut h, "Apply");
    assert_eq!(project(&h), &applied);
    assert_eq!(h.state().doc.revision(), revision);
    type_into_text_window(&mut h, "discard me", true);
    extra(&mut h, "Wine");
    click(&mut h, "Cancel");
    assert_eq!(project(&h), &applied);
    ctrl(&mut h, egui::Key::Z);
    assert_eq!(project(&h), &before);
}

#[test]
fn extra_classic_titles_and_legend_preserve_references_fonts_and_roundtrip() {
    let mut h = start(vec![data_file("editor-classic-titles")]);
    let before = project(&h).clone();
    let original = graph(&h).layers[0].clone();
    for (vertical, name) in [(false, "Dark Yellow"), (true, "Olive")] {
        menu_path(&mut h, &["Format", "Axis Titles", if vertical { "Y Axis Title..." } else { "X Axis Title..." }]);
        ctrl(&mut h, egui::Key::A);
        extra(&mut h, name);
        click(&mut h, "OK");
    }
    menu_path(&mut h, &["Graph", "Legend", "Edit Legend Text..."]);
    ctrl(&mut h, egui::Key::A);
    extra(&mut h, "LT Magenta");
    click(&mut h, "OK");
    let expected = graph(&h).layers[0].clone();
    assert_eq!(expected.x_axis.title_font, original.x_axis.title_font);
    assert_eq!(expected.y_axis.title_font, original.y_axis.title_font);
    assert_eq!(expected.legend.font, original.legend.font);
    let raw = expected.legend.text.as_deref().unwrap();
    assert!(raw.contains(r"\l(1)") && raw.contains("%(1)") && raw.contains(r"\l(2)") && raw.contains("%(2)"));
    for (kind, expected_color) in [
        (ObjectKind::XTitle, Color::rgb(128, 128, 0)),
        (ObjectKind::YTitle, Color::rgb(0, 128, 0)),
        (ObjectKind::Legend, Color::rgb(255, 128, 255)),
    ] {
        let rendered = texts(&h, kind);
        assert!(!rendered.is_empty() && rendered.iter().all(|(_, c)| *c == expected_color), "{kind:?}: {rendered:?}");
    }
    assert_eq!(texts(&h, ObjectKind::Legend).iter().map(|(s, _)| s.as_str()).collect::<Vec<_>>(), ["up", "down"]);
    let directory = save_dir("editor-classic-roundtrip");
    for extension in ["sheaf", "opj"] {
        let path = directory.join(format!("classic-editor.{extension}"));
        sheaf_io::save::save_project(project(&h), &path).unwrap();
        let reopened = sheaf_io::open_project(&path).unwrap();
        let layer = &reopened.project.graphs().next().unwrap().layers[0];
        assert_eq!(layer.x_axis.title, expected.x_axis.title);
        assert_eq!(layer.y_axis.title, expected.y_axis.title);
        assert_eq!(layer.legend, expected.legend);
    }
    for _ in 0..3 {
        ctrl(&mut h, egui::Key::Z);
    }
    assert_eq!(project(&h), &before);
}

#[test]
fn extra_classic_choice_seeds_custom_rgb_without_losing_the_selection() {
    let mut h = start(vec![data_file("editor-classic-rgb-seed")]);
    menu(&mut h, "Graph", "Add Text...");
    type_into_text_window(&mut h, "royal", false);
    ctrl(&mut h, egui::Key::A);
    extra(&mut h, "Royal");
    menu_path(&mut h, &["Text color", "Custom RGB..."]);
    click(&mut h, "Apply color");
    click(&mut h, "OK");
    assert_eq!(sheaf_plot::markup::plain(&labels(&h)[0].text), "royal");
    assert!(labels(&h)[0].text.contains(r"\c27262977("));
    let rendered = texts(&h, ObjectKind::Label(0));
    assert!(!rendered.is_empty() && rendered.iter().all(|(_, c)| *c == Color::rgb(0, 0, 160)));
    let path = save_dir("editor-classic-rgb-seed").join("classic-label.sheaf");
    sheaf_io::save::save_project(project(&h), &path).unwrap();
}

#[test]
fn dismissing_the_extra_palette_keeps_automatic_title_text_automatic() {
    let mut h = start(vec![data_file("editor-classic-dismiss")]);
    let before = project(&h).clone();
    menu_path(&mut h, &["Format", "Axis Titles", "X Axis Title..."]);
    menu_path(&mut h, &["Text color", "More Classic colors"]);
    assert_eq!(project(&h), &before);
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    click(&mut h, "OK");
    assert!(h.query_by_label("Formatted text preview").is_none(), "OK closes the editor after dismissal");
    assert_eq!(project(&h), &before);
}
