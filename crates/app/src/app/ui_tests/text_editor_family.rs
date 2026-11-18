use super::*;

fn installed() -> Vec<String> {
    let mut names: Vec<_> = sheaf_plot::SystemFonts::shared()
        .database()
        .faces()
        .flat_map(|face| face.families.iter().map(|(name, _)| name.clone()))
        .filter(|name| {
            !name.eq_ignore_ascii_case("Symbol")
                && name.chars().all(|c| c.is_ascii_alphanumeric() || c == ' ' || c == '-')
        })
        .collect();
    names.sort();
    names.dedup();
    let mut preferred: Vec<_> = ["Courier New", "Times New Roman", "Arial"]
        .into_iter()
        .filter(|name| names.iter().any(|n| n == name))
        .map(str::to_string)
        .collect();
    for name in names {
        if preferred.len() >= 3 {
            break;
        }
        if !preferred.contains(&name) {
            preferred.push(name);
        }
    }
    assert!(!preferred.is_empty(), "the UI fixture needs an installed font");
    preferred
}

fn search(h: &mut Harness<'static, SheafApp>, value: &str) {
    h.get_by_role_and_label(egui::accesskit::Role::TextInput, "Font search").click();
    settle(h);
    ctrl(h, egui::Key::A);
    h.event(egui::Event::Text(value.to_string()));
    settle(h);
}

fn family(h: &mut Harness<'static, SheafApp>, name: &str) {
    click(h, "Text font");
    search(h, name);
    click(h, &format!("Text font: {name}"));
    assert!(h.query_by_label("Font search").is_none());
}

#[test]
fn inline_family_preserves_unicode_script_selection_and_typing_focus() {
    let mut h = start(vec![data_file("editor-family-selection")]);
    let before = project(&h).clone();
    menu(&mut h, "Graph", "Add Text...");
    type_into_text_window(&mut h, "μA cm-2", false);
    for _ in 0..2 {
        h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::ArrowLeft);
        settle(&mut h);
    }
    click(&mut h, "Superscript");
    let name = installed().remove(0);
    family(&mut h, &name);
    assert_eq!(project(&h), &before);
    type_into_text_window(&mut h, "-1", false);
    click(&mut h, "OK");
    assert_eq!(labels(&h)[0].text, format!("μA cm\\+(\\f:{name}(-1))"));
    let runs = sheaf_plot::markup::parse(&labels(&h)[0].text);
    assert!(runs.iter().any(|r| r.text == "-1" && r.scale < 1.0 && r.family.as_deref() == Some(name.as_str())));
    ctrl(&mut h, egui::Key::Z);
    assert_eq!(project(&h), &before);
}

#[test]
fn caret_family_is_an_idempotent_draft_and_cancel_preserves_only_apply() {
    let mut h = start(vec![data_file("editor-family-caret")]);
    let before = project(&h).clone();
    let name = installed().remove(0);
    menu(&mut h, "Graph", "Add Text...");
    family(&mut h, &name);
    type_into_text_window(&mut h, "sample", false);
    assert_eq!(project(&h), &before);
    click(&mut h, "Apply");
    assert_eq!(labels(&h)[0].text, format!("\\f:{name}(sample)"));
    let applied = project(&h).clone();
    let revision = h.state().doc.revision();
    click(&mut h, "Cancel");
    let at = on(&h, 0, ObjectKind::Label(0));
    double_click_at(&mut h, at);
    ctrl(&mut h, egui::Key::A);
    family(&mut h, &name);
    click(&mut h, "Apply");
    assert_eq!(project(&h), &applied);
    assert_eq!(h.state().doc.revision(), revision);
    type_into_text_window(&mut h, "discard", true);
    family(&mut h, &name);
    click(&mut h, "Cancel");
    assert_eq!(project(&h), &applied);
    ctrl(&mut h, egui::Key::Z);
    assert_eq!(project(&h), &before);
}

fn placed_families(h: &Harness<'static, SheafApp>, kind: ObjectKind) -> Vec<String> {
    let scene = sheaf_plot::graph_scene(project(h), graph(h).id).unwrap();
    let layer = scene.layers.iter().find(|l| l.objects.iter().any(|o| o.layer == 0 && o.kind == kind)).unwrap();
    let object = layer.objects.iter().find(|o| o.layer == 0 && o.kind == kind).unwrap();
    layer.overlay[object.items.clone()]
        .iter()
        .filter_map(|item| if let sheaf_plot::scene::Item::Text(t) = item { Some(t.family.clone()) } else { None })
        .collect()
}

#[test]
fn installed_families_reach_styled_runs_and_rendered_glyphs_without_changing_base_font() {
    let mut h = start(vec![data_file("editor-family-render")]);
    for name in installed() {
        let before = project(&h).clone();
        menu(&mut h, "Graph", "Add Text...");
        type_into_text_window(&mut h, r"\b(\c14(A \+(2)))", false);
        ctrl(&mut h, egui::Key::A);
        family(&mut h, &name);
        click(&mut h, "OK");
        let label = &labels(&h)[0];
        assert_eq!(label.text, format!("\\f:{name}(\\b(\\c14(A \\+(2))))"));
        assert_eq!(label.font.family, "Arial");
        let runs = sheaf_plot::markup::parse(&label.text);
        assert!(runs.iter().all(|r| r.family.as_deref() == Some(name.as_str())
            && r.bold
            && r.color == Some(sheaf_core::Color::rgb(0, 0, 160))));
        assert!(runs.iter().any(|r| r.text == "2" && r.scale < 1.0));
        let rendered = placed_families(&h, ObjectKind::Label(0));
        assert!(!rendered.is_empty() && rendered.iter().all(|n| n == &name));
        assert!(sheaf_plot::render_svg(project(&h), graph(&h).id).unwrap().contains(&name));
        ctrl(&mut h, egui::Key::Z);
        assert_eq!(project(&h), &before);
    }
}

#[test]
fn title_and_legend_families_preserve_references_base_fonts_and_project_roundtrips() {
    let mut h = start(vec![data_file("editor-family-save")]);
    let original = graph(&h).layers[0].clone();
    let name = installed().remove(0);
    for vertical in [false, true] {
        menu_path(&mut h, &["Format", "Axis Titles", if vertical { "Y Axis Title..." } else { "X Axis Title..." }]);
        ctrl(&mut h, egui::Key::A);
        family(&mut h, &name);
        click(&mut h, "OK");
    }
    menu_path(&mut h, &["Graph", "Legend", "Edit Legend Text..."]);
    ctrl(&mut h, egui::Key::A);
    family(&mut h, &name);
    click(&mut h, "OK");
    let expected = graph(&h).layers[0].clone();
    assert_eq!(expected.x_axis.title_font, original.x_axis.title_font);
    assert_eq!(expected.y_axis.title_font, original.y_axis.title_font);
    assert_eq!(expected.legend.font, original.legend.font);
    let raw = expected.legend.text.as_deref().unwrap();
    assert!(raw.starts_with(r"\l(1) ") && raw.contains("%(1)") && raw.contains(r"\l(2)") && raw.contains("%(2)"));
    for kind in [ObjectKind::XTitle, ObjectKind::YTitle, ObjectKind::Legend] {
        let rendered = placed_families(&h, kind);
        assert!(!rendered.is_empty() && rendered.iter().all(|n| n == &name), "{kind:?}: {rendered:?}");
    }
    let dir = save_dir("editor-family-save");
    std::fs::write(dir.join("family.svg"), sheaf_plot::render_svg(project(&h), graph(&h).id).unwrap()).unwrap();
    for extension in ["sheaf", "opj"] {
        let file = dir.join(format!("family.{extension}"));
        h.state_mut().save_to(file.clone());
        let back = start(vec![file]);
        assert_eq!(graph(&back).layers[0], expected, "{extension}");
    }
}

#[test]
fn unmatched_search_and_dismissal_leave_automatic_text_automatic() {
    let mut h = start(vec![data_file("editor-family-dismiss")]);
    let before = project(&h).clone();
    menu_path(&mut h, &["Format", "Axis Titles", "X Axis Title..."]);
    click(&mut h, "Text font");
    search(&mut h, "!!! no such font !!!");
    assert!(h.query_by_label("No matching installed fonts").is_some());
    assert_eq!(project(&h), &before);
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    click(&mut h, "OK");
    assert!(h.query_by_label("Formatted text preview").is_none());
    assert_eq!(project(&h), &before);
}

#[test]
fn replacing_the_project_closes_the_font_picker_without_editing_the_new_document() {
    let mut h = start(vec![data_file("editor-family-generation")]);
    menu(&mut h, "Graph", "Add Text...");
    type_into_text_window(&mut h, "old draft", false);
    click(&mut h, "Text font");
    let before = project(&h).clone();
    h.state_mut().doc.replace(before.clone());
    let revision = h.state().doc.revision();
    settle(&mut h);
    assert!(h.query_by_label("Font search").is_none());
    assert!(h.query_by_label("Formatted text preview").is_none());
    assert_eq!(project(&h), &before);
    assert_eq!(h.state().doc.revision(), revision);
}
