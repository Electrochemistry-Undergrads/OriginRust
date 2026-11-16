use super::*;

fn field(h: &mut Harness<'static, SheafApp>, label: &str, value: &str) {
    h.get_by_role_and_label(egui::accesskit::Role::TextInput, label).click();
    settle(h);
    ctrl(h, egui::Key::A);
    h.event(egui::Event::Text(value.to_string()));
    settle(h);
}

#[test]
fn paired_scripts_keep_unicode_selection_drafts_preview_and_one_undo() {
    let mut h = start(vec![data_file("editor-pair-selection")]);
    let before = project(&h).clone();
    menu(&mut h, "Graph", "Add Text...");
    type_into_text_window(&mut h, "μA2", false);
    h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::ArrowLeft);
    settle(&mut h);
    click(&mut h, "Paired scripts");
    field(&mut h, "Superscript text", "3+");
    click(&mut h, "Insert scripts");
    assert_eq!(project(&h), &before);
    assert!(h.query_by_label("Formatted text preview").is_some());
    type_into_text_window(&mut h, "Z", false);
    click(&mut h, "OK");
    assert_eq!(labels(&h)[0].text, r"μA\=(2,3+)Z");
    let runs = sheaf_plot::markup::parse(&labels(&h)[0].text);
    assert!(runs.iter().any(|r| r.text == "2" && r.scale < 1.0 && r.rise < 0.0));
    assert!(runs.iter().any(|r| r.text == "3+" && r.scale < 1.0 && r.rise > 0.0));
    assert!(runs.iter().any(|r| r.text == "Z" && r.scale == 1.0 && r.rise == 0.0));
    let scene = sheaf_plot::graph_scene(project(&h), graph(&h).id).unwrap();
    let layer = &scene.layers[0];
    let object = layer.objects.iter().find(|o| o.kind == ObjectKind::Label(0)).unwrap();
    let placed: Vec<_> = layer.overlay[object.items.clone()]
        .iter()
        .filter_map(|item| if let sheaf_plot::scene::Item::Text(t) = item { Some(t) } else { None })
        .collect();
    let lower = placed.iter().find(|t| t.text == "2").unwrap();
    let upper = placed.iter().find(|t| t.text == "3+").unwrap();
    assert!((lower.x - upper.x).abs() < 1e-6 && lower.y > upper.y);
    let file = save_dir("editor-pair-label").join("pair-label.sheaf");
    sheaf_io::save::save_project(project(&h), &file).unwrap();
    ctrl(&mut h, egui::Key::Z);
    assert_eq!(project(&h), &before);
}

#[test]
fn caret_pairs_preserve_literal_commas_and_continue_typing_after_the_column() {
    let mut h = start(vec![data_file("editor-pair-caret")]);
    menu(&mut h, "Graph", "Add Text...");
    click(&mut h, "Paired scripts");
    assert!(h.get_by_label("Insert scripts").accesskit_node().is_disabled());
    field(&mut h, "Subscript text", "a,b");
    field(&mut h, "Superscript text", "c,d");
    click(&mut h, "Insert scripts");
    type_into_text_window(&mut h, "P", false);
    click(&mut h, "OK");
    assert_eq!(labels(&h)[0].text, r"\=(a\v(,)b,c\v(,)d)P");
    assert_eq!(sheaf_plot::markup::plain(&labels(&h)[0].text), "a,bc,dP");
}

#[test]
fn paired_insertion_keeps_selected_styles_color_and_font_markup() {
    let mut h = start(vec![data_file("editor-pair-styles")]);
    menu(&mut h, "Graph", "Add Text...");
    type_into_text_window(&mut h, r"\b(\c14(\f:Arial(2)))", false);
    ctrl(&mut h, egui::Key::A);
    click(&mut h, "Paired scripts");
    field(&mut h, "Superscript text", r"\i(3)");
    click(&mut h, "Insert scripts");
    click(&mut h, "OK");
    assert_eq!(labels(&h)[0].text, r"\=(\b(\c14(\f:Arial(2))),\i(3))");
    let runs = sheaf_plot::markup::parse(&labels(&h)[0].text);
    assert!(runs.iter().any(|r| r.text == "2"
        && r.bold
        && r.family.as_deref() == Some("Arial")
        && r.color == Some(sheaf_core::Color::rgb(0, 0, 160))
        && r.rise < 0.0));
    assert!(runs.iter().any(|r| r.text == "3" && r.italic && r.rise > 0.0));
}

#[test]
fn dismissing_scripts_keeps_automatic_text_and_cancel_keeps_only_applied_pairs() {
    let mut h = start(vec![data_file("editor-pair-cancel")]);
    let before = project(&h).clone();
    menu_path(&mut h, &["Format", "Axis Titles", "X Axis Title..."]);
    click(&mut h, "Paired scripts");
    field(&mut h, "Superscript text", "discard");
    click(&mut h, "Cancel scripts");
    click(&mut h, "OK");
    assert!(h.query_by_label("Formatted text preview").is_none());
    assert_eq!(project(&h), &before);
    menu(&mut h, "Graph", "Add Text...");
    click(&mut h, "Paired scripts");
    field(&mut h, "Subscript text", "2");
    field(&mut h, "Superscript text", "3");
    click(&mut h, "Insert scripts");
    click(&mut h, "Apply");
    let applied = project(&h).clone();
    click(&mut h, "Paired scripts");
    field(&mut h, "Subscript text", "4");
    field(&mut h, "Superscript text", "5");
    click(&mut h, "Insert scripts");
    click(&mut h, "Cancel");
    assert_eq!(project(&h), &applied);
    ctrl(&mut h, egui::Key::Z);
    assert_eq!(project(&h), &before);
}

#[test]
fn paired_titles_and_legend_keep_base_fonts_references_and_project_roundtrips() {
    let mut h = start(vec![data_file("editor-pair-save")]);
    let original = graph(&h).layers[0].clone();
    for vertical in [false, true] {
        menu_path(&mut h, &["Format", "Axis Titles", if vertical { "Y Axis Title..." } else { "X Axis Title..." }]);
        ctrl(&mut h, egui::Key::A);
        click(&mut h, "Paired scripts");
        field(&mut h, "Superscript text", "2");
        click(&mut h, "Insert scripts");
        click(&mut h, "OK");
    }
    menu_path(&mut h, &["Graph", "Legend", "Edit Legend Text..."]);
    type_into_text_window(&mut h, " cm2", false);
    h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::ArrowLeft);
    settle(&mut h);
    click(&mut h, "Paired scripts");
    field(&mut h, "Superscript text", "3");
    click(&mut h, "Insert scripts");
    click(&mut h, "OK");
    let expected = graph(&h).layers[0].clone();
    assert_eq!(expected.x_axis.title_font, original.x_axis.title_font);
    assert_eq!(expected.y_axis.title_font, original.y_axis.title_font);
    assert_eq!(expected.legend.font, original.legend.font);
    let raw = expected.legend.text.as_deref().unwrap();
    assert!(raw.contains(r"\l(1)") && raw.contains("%(1)") && raw.contains(r"\l(2)") && raw.contains("%(2)"));
    assert!(raw.ends_with(r" cm\=(2,3)"));
    let dir = save_dir("editor-pair-save");
    std::fs::write(dir.join("pair.svg"), sheaf_plot::render_svg(project(&h), graph(&h).id).unwrap()).unwrap();
    for extension in ["sheaf", "opj"] {
        let file = dir.join(format!("pair.{extension}"));
        h.state_mut().save_to(file.clone());
        let back = start(vec![file]);
        assert_eq!(graph(&back).layers[0], expected, "{extension}");
    }
}

#[test]
fn unsafe_legend_selection_and_replaced_projects_cannot_insert_scripts() {
    let mut h = start(vec![data_file("editor-pair-stale")]);
    let before = project(&h).clone();
    menu_path(&mut h, &["Graph", "Legend", "Edit Legend Text..."]);
    ctrl(&mut h, egui::Key::A);
    click(&mut h, "Paired scripts");
    assert!(h.query_by_label("Select text on one line").is_some());
    assert!(h.get_by_label("Insert scripts").accesskit_node().is_disabled());
    click(&mut h, "Cancel scripts");
    click(&mut h, "Cancel");
    assert_eq!(project(&h), &before);
    menu(&mut h, "Graph", "Add Text...");
    click(&mut h, "Paired scripts");
    field(&mut h, "Subscript text", "2");
    field(&mut h, "Superscript text", "3");
    h.state_mut().doc.replace(before.clone());
    let revision = h.state().doc.revision();
    settle(&mut h);
    assert!(h.query_by_label("Insert scripts").is_none());
    assert_eq!(project(&h), &before);
    assert_eq!(h.state().doc.revision(), revision);
}
