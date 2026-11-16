//! RGB fields remain separate from the selected text until an explicit colour choice.
use super::*;
use sheaf_core::Color;

fn open_rgb(h: &mut Harness<'static, SheafApp>) {
    menu_path(h, &["Text color", "Custom RGB..."]);
}

fn channel(h: &mut Harness<'static, SheafApp>, name: &str, value: &str) {
    h.get_by_role_and_label(egui::accesskit::Role::TextInput, name).click();
    settle(h);
    ctrl(h, egui::Key::A);
    h.event(egui::Event::Text(value.to_string()));
    settle(h);
}

fn rgb(h: &mut Harness<'static, SheafApp>, values: [&str; 3]) {
    open_rgb(h);
    for (name, value) in ["Red", "Green", "Blue"].into_iter().zip(values) {
        channel(h, name, value);
    }
    click(h, "Apply color");
    assert!(h.query_by_label("Apply color").is_none(), "the colour menus close after a choice");
}

fn texts(h: &Harness<'static, SheafApp>, kind: ObjectKind) -> Vec<(String, Color)> {
    let scene = sheaf_plot::graph_scene(project(h), graph(h).id).unwrap();
    let layer = scene.layers.iter().find(|l| l.objects.iter().any(|o| o.layer == 0 && o.kind == kind)).unwrap();
    let object = layer.objects.iter().find(|o| o.layer == 0 && o.kind == kind).unwrap();
    layer.overlay[object.items.clone()]
        .iter()
        .filter_map(
            |item| {
                if let sheaf_plot::scene::Item::Text(t) = item { Some((t.text.clone(), t.color)) } else { None }
            },
        )
        .collect()
}

#[test]
fn text_editor_rgb_formats_one_paired_script_branch_without_changing_its_text() {
    let mut h = start(vec![data_file("editor-rgb-pair")]);
    let before = project(&h).clone();
    menu(&mut h, "Graph", "Add Text...");
    type_into_text_window(&mut h, r"P\=(i,2)Q", false);
    for _ in 0..2 {
        h.key_press(egui::Key::ArrowLeft);
        settle(&mut h);
    }
    h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::ArrowLeft);
    settle(&mut h);
    rgb(&mut h, ["200", "40", "100"]);
    assert_eq!(project(&h), &before);
    click(&mut h, "OK");
    assert_eq!(sheaf_plot::markup::plain(&labels(&h)[0].text), "Pi2Q");
    assert!(labels(&h)[0].text.contains(r"\=(i,"));
    let rendered = texts(&h, ObjectKind::Label(0));
    assert_eq!(
        rendered.iter().filter(|(_, c)| *c == Color::rgb(200, 40, 100)).map(|(s, _)| s.as_str()).collect::<String>(),
        "2"
    );
    assert!(rendered.iter().any(|(s, c)| s == "i" && *c == Color::BLACK));
    ctrl(&mut h, egui::Key::Z);
    assert_eq!(project(&h), &before);
}

#[test]
fn text_editor_rgb_keeps_unicode_script_selection_and_typing_focus_in_a_draft() {
    let mut h = start(vec![data_file("editor-rgb-selection")]);
    let before = project(&h).clone();
    menu(&mut h, "Graph", "Add Text...");
    type_into_text_window(&mut h, "μA cm-2", false);
    for _ in 0..2 {
        h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::ArrowLeft);
        settle(&mut h);
    }
    click(&mut h, "Superscript");
    rgb(&mut h, ["32", "128", "224"]);
    assert_eq!(project(&h), &before);
    type_into_text_window(&mut h, "-1", false);
    assert_eq!(project(&h), &before);
    assert!(h.query_by_label("Formatted text preview").is_some());
    click(&mut h, "OK");
    assert_eq!(sheaf_plot::markup::plain(&labels(&h)[0].text), "μA cm-1");
    let runs = sheaf_plot::markup::parse(&labels(&h)[0].text);
    assert!(runs.iter().any(|r| r.text == "-1" && r.scale < 1.0 && r.color == Some(Color::rgb(32, 128, 224))));
    assert_eq!(labels(&h)[0].font.color, Color::BLACK);
    let path = save_dir("editor-rgb-selection").join("colored.sheaf");
    sheaf_io::save::save_project(project(&h), &path).unwrap();
    println!("RGB label screenshot fixture: {}", path.display());
    ctrl(&mut h, egui::Key::Z);
    assert_eq!(project(&h), &before);
}

#[test]
fn text_editor_rgb_invalid_values_disable_a_choice_and_dismissal_keeps_automatic_text() {
    let mut h = start(vec![data_file("editor-rgb-invalid")]);
    let before = project(&h).clone();
    menu_path(&mut h, &["Format", "Axis Titles", "Y Axis Title..."]);
    open_rgb(&mut h);
    channel(&mut h, "Red", "999");
    channel(&mut h, "Green", "-1");
    channel(&mut h, "Blue", "");
    assert!(h.get_by_label("Apply color").accesskit_node().is_disabled());
    assert!(h.query_by_label("Use whole numbers from 0 to 255.").is_some());
    assert_eq!(project(&h), &before);
    for (name, value) in [("Red", "64"), ("Green", "128"), ("Blue", "192")] {
        channel(&mut h, name, value);
    }
    assert!(!h.get_by_label("Apply color").accesskit_node().is_disabled());
    assert_eq!(project(&h), &before, "valid picker values still need Apply color");
    for _ in 0..2 {
        if not_toolbar(&h, "Custom RGB...").is_empty() {
            break;
        }
        h.key_press(egui::Key::Escape);
        settle(&mut h);
    }
    click(&mut h, "OK");
    assert_eq!(project(&h), &before);
}

#[test]
fn text_editor_rgb_black_white_caret_and_reapply_preserve_the_undo_steps() {
    let mut h = start(vec![data_file("editor-rgb-boundaries")]);
    let before = project(&h).clone();
    menu(&mut h, "Graph", "Add Text...");
    rgb(&mut h, ["255", "255", "255"]);
    type_into_text_window(&mut h, "X", false);
    click(&mut h, "OK");
    assert_eq!(sheaf_plot::markup::parse(&labels(&h)[0].text)[0].color, Some(Color::WHITE));
    let white = project(&h).clone();
    let at = on(&h, 0, ObjectKind::Label(0));
    double_click_at(&mut h, at);
    ctrl(&mut h, egui::Key::A);
    rgb(&mut h, ["0", "0", "0"]);
    click(&mut h, "Apply");
    assert_eq!(sheaf_plot::markup::parse(&labels(&h)[0].text)[0].color, Some(Color::BLACK));
    let black = project(&h).clone();
    rgb(&mut h, ["0", "0", "0"]);
    click(&mut h, "Apply");
    assert_eq!(project(&h), &black);
    click(&mut h, "Cancel");
    ctrl(&mut h, egui::Key::Z);
    assert_eq!(project(&h), &white);
    ctrl(&mut h, egui::Key::Z);
    assert_eq!(project(&h), &before);
}

#[test]
fn text_editor_rgb_titles_and_legend_preserve_base_fonts_links_and_project_roundtrips() {
    let mut h = start(vec![data_file("editor-rgb-titles")]);
    let before = project(&h).clone();
    let original = graph(&h).layers[0].clone();
    for (vertical, values) in [(false, ["31", "127", "255"]), (true, ["255", "127", "31"])] {
        menu_path(&mut h, &["Format", "Axis Titles", if vertical { "Y Axis Title..." } else { "X Axis Title..." }]);
        ctrl(&mut h, egui::Key::A);
        rgb(&mut h, values);
        click(&mut h, "OK");
    }
    menu_path(&mut h, &["Graph", "Legend", "Edit Legend Text..."]);
    ctrl(&mut h, egui::Key::A);
    rgb(&mut h, ["64", "160", "96"]);
    click(&mut h, "OK");
    let expected = graph(&h).layers[0].clone();
    assert_eq!(expected.x_axis.title_font, original.x_axis.title_font);
    assert_eq!(expected.y_axis.title_font, original.y_axis.title_font);
    assert_eq!(expected.legend.font, original.legend.font);
    let raw = expected.legend.text.as_deref().unwrap();
    assert!(raw.contains(r"\l(1)") && raw.contains("%(1)") && raw.contains(r"\l(2)") && raw.contains("%(2)"));
    for (kind, color) in [
        (ObjectKind::XTitle, Color::rgb(31, 127, 255)),
        (ObjectKind::YTitle, Color::rgb(255, 127, 31)),
        (ObjectKind::Legend, Color::rgb(64, 160, 96)),
    ] {
        let rendered = texts(&h, kind);
        assert!(!rendered.is_empty() && rendered.iter().all(|(_, c)| *c == color), "{kind:?}: {rendered:?}");
    }
    assert_eq!(texts(&h, ObjectKind::Legend).iter().map(|(s, _)| s.as_str()).collect::<Vec<_>>(), ["up", "down"]);
    let directory = save_dir("editor-rgb-roundtrip");
    for extension in ["sheaf", "opj"] {
        let path = directory.join(format!("colored.{extension}"));
        sheaf_io::save::save_project(project(&h), &path).unwrap();
        let reopened = sheaf_io::open_project(&path).unwrap();
        let layer = &reopened.project.graphs().next().unwrap().layers[0];
        assert_eq!(layer.x_axis.title, expected.x_axis.title);
        assert_eq!(layer.y_axis.title, expected.y_axis.title);
        assert_eq!(layer.legend, expected.legend);
        if extension == "sheaf" {
            println!("RGB object screenshot fixture: {}", path.display());
        }
    }
    for _ in 0..3 {
        ctrl(&mut h, egui::Key::Z);
    }
    assert_eq!(project(&h), &before);
}
