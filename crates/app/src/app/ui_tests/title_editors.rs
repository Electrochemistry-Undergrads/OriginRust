//! Existing graph-title and legend text use the shared selection editor.
use super::*;

fn open_title(h: &mut Harness<'static, SheafApp>, vertical: bool) {
    menu_path(h, &["Format", "Axis Titles", if vertical { "Y Axis Title..." } else { "X Axis Title..." }]);
}

fn open_legend(h: &mut Harness<'static, SheafApp>) {
    let at = on(h, 0, ObjectKind::Legend);
    right_click_at(h, at);
    click(h, "Edit Text...");
}

#[test]
fn title_editor_formats_whole_automatic_legend_names_and_preserves_its_symbols() {
    let mut h = start(vec![data_file("title-editor-whole-legend")]);
    let before = project(&h).clone();
    open_legend(&mut h);
    ctrl(&mut h, egui::Key::A);
    click(&mut h, "Bold selection");
    assert_eq!(project(&h), &before);
    click(&mut h, "OK");
    let raw = graph(&h).layers[0].legend.text.as_ref().unwrap();
    assert!(raw.contains(r"\l(1)") && raw.contains(r"\l(2)"));
    let scene = sheaf_plot::graph_scene(project(&h), graph(&h).id).unwrap();
    let layer =
        scene.layers.iter().find(|l| l.objects.iter().any(|o| o.layer == 0 && o.kind == ObjectKind::Legend)).unwrap();
    let object = layer.objects.iter().find(|o| o.layer == 0 && o.kind == ObjectKind::Legend).unwrap();
    let texts: Vec<_> = layer.overlay[object.items.clone()]
        .iter()
        .filter_map(|item| if let sheaf_plot::scene::Item::Text(t) = item { Some(t) } else { None })
        .collect();
    for name in ["up", "down"] {
        assert!(texts.iter().any(|t| t.text == name && t.bold), "{name}: {texts:?}");
    }
    ctrl(&mut h, egui::Key::Z);
    assert_eq!(project(&h), &before);
}

#[test]
fn title_editor_menu_formats_selected_axis_title_text_without_editing_the_draft() {
    let mut h = start(vec![data_file("title-editor-menu")]);
    let before = project(&h).clone();
    menu(&mut h, "Format", "Axis Titles");
    click(&mut h, "X Axis Title...");
    type_into_text_window(&mut h, "cm-2", true);
    for _ in 0..2 {
        h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::ArrowLeft);
        settle(&mut h);
    }
    click(&mut h, "Superscript");
    assert_eq!(project(&h), &before);
    click(&mut h, "OK");
    assert_eq!(graph(&h).layers[0].x_axis.title.as_deref(), Some(r"cm\+(-2)"));
    assert_eq!(graph(&h).layers[0].y_axis.title, before.graphs().next().unwrap().layers[0].y_axis.title);
    ctrl(&mut h, egui::Key::Z);
    assert_eq!(project(&h), &before);
}

#[test]
fn title_editors_keep_automatic_metadata_markup_on_ok_cancel_and_font_only_apply() {
    let mut h = start(vec![data_file("title-editor-auto")]);
    let (a, b) = (column_id(&h, "A"), column_id(&h, "B"));
    let gid = graph(&h).id;
    h.state_mut().doc.apply("Set metadata", |p| {
        p.column_mut(a).unwrap().units = r"s\+(1)".into();
        p.column_mut(b).unwrap().long_name = r"\g(m)A".into();
        let l = &mut p.graph_mut(gid).unwrap().layers[0];
        l.x_axis.title_font.size_pt = 500.0;
        l.y_axis.title_font.size_pt = 500.0;
        l.legend.font.size_pt = 500.0;
        l.legend.visible = false;
    });
    for target in 0..3 {
        let before = project(&h).clone();
        if target == 2 {
            menu_path(&mut h, &["Graph", "Legend", "Edit Legend Text..."]);
        } else {
            open_title(&mut h, target == 1);
        }
        assert!(h.query_by_label("Formatted text preview").is_some());
        click(&mut h, "OK");
        assert_eq!(project(&h), &before, "opening and accepting preserves automatic None and raw metadata");
        if target == 2 {
            menu_path(&mut h, &["Graph", "Legend", "Edit Legend Text..."]);
        } else {
            open_title(&mut h, target == 1);
        }
        h.get_by_role_and_label(egui::accesskit::Role::CheckBox, "Italic").click();
        settle(&mut h);
        assert_eq!(project(&h), &before);
        click(&mut h, "Apply");
        let l = &graph(&h).layers[0];
        let font = match target {
            0 => &l.x_axis.title_font,
            1 => &l.y_axis.title_font,
            _ => &l.legend.font,
        };
        assert!(font.italic);
        assert_eq!(font.size_pt, 500.0, "the editor preserves sizes supported by the Format toolbar");
        assert!(l.x_axis.title.is_none() && l.y_axis.title.is_none() && l.legend.text.is_none());
        let applied = project(&h).clone();
        type_into_text_window(&mut h, "discard this", true);
        click(&mut h, "Cancel");
        assert_eq!(project(&h), &applied);
        ctrl(&mut h, egui::Key::Z);
        assert_eq!(project(&h), &before);
    }
}

#[test]
fn title_editor_uses_the_active_layer_and_preserves_the_other_axis_and_layer() {
    let mut h = start(vec![data_file("title-editor-layer")]);
    let gid = graph(&h).id;
    h.state_mut().doc.apply("Add layer", |p| {
        let g = p.graph_mut(gid).unwrap();
        let mut layer = g.layers[0].clone();
        layer.y_axis.show_title = false;
        g.layers.push(layer);
    });
    h.state_mut().graphs.get_mut(&gid).unwrap().active_layer = 1;
    settle(&mut h);
    let before = project(&h).clone();
    open_title(&mut h, true);
    type_into_text_window(&mut h, r"j / \g(m)A cm\+(-2)", true);
    click(&mut h, "Show on graph");
    assert_eq!(project(&h), &before);
    click(&mut h, "OK");
    assert_eq!(graph(&h).layers[1].y_axis.title.as_deref(), Some(r"j / \g(m)A cm\+(-2)"));
    assert!(graph(&h).layers[1].y_axis.show_title);
    assert_eq!(graph(&h).layers[0], before.graph(gid).unwrap().layers[0]);
    assert_eq!(graph(&h).layers[1].x_axis, before.graph(gid).unwrap().layers[1].x_axis);
    ctrl(&mut h, egui::Key::Z);
    assert_eq!(project(&h), &before);
}

#[test]
fn title_editor_legend_keeps_symbols_substitutions_undo_and_project_roundtrips() {
    let mut h = start(vec![data_file("title-editor-legend")]);
    open_title(&mut h, false);
    type_into_text_window(&mut h, r"\c4(\u(j / \g(m)A cm\+(-2)))", true);
    click(&mut h, "OK");
    let before = project(&h).clone();
    open_legend(&mut h);
    type_into_text_window(&mut h, r"\l(1) %(1) cm-2", true);
    for _ in 0..2 {
        h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::ArrowLeft);
        settle(&mut h);
    }
    click(&mut h, "Superscript");
    type_into_text_window(&mut h, "-1", false);
    assert!(h.query_by_label("Formatted text preview").is_some());
    assert_eq!(project(&h), &before);
    click(&mut h, "OK");
    let expected = graph(&h).layers[0].legend.clone();
    assert_eq!(expected.text.as_deref(), Some(r"\l(1) %(1) cm\+(-1)"));
    assert_eq!(h.state().doc.undo_label(), Some("Edit legend text"));
    let directory = save_dir("title-editor-roundtrip");
    for extension in ["sheaf", "opj"] {
        let path = directory.join(format!("formatted.{extension}"));
        sheaf_io::save::save_project(project(&h), &path).unwrap();
        let reopened = sheaf_io::open_project(&path).unwrap();
        assert_eq!(reopened.project.graphs().next().unwrap().layers[0].legend, expected);
        assert_eq!(reopened.project.graphs().next().unwrap().layers[0].x_axis.title, graph(&h).layers[0].x_axis.title);
        if extension == "sheaf" {
            println!("Title editor screenshot fixture: {}", path.display());
        }
    }
    ctrl(&mut h, egui::Key::Z);
    assert_eq!(project(&h), &before);
}

#[test]
fn title_editor_restores_automatic_text_and_closes_after_project_replacement() {
    let mut h = start(vec![data_file("title-editor-reset")]);
    let before = project(&h).clone();
    open_title(&mut h, false);
    type_into_text_window(&mut h, "custom title", true);
    click(&mut h, "Apply");
    assert_eq!(graph(&h).layers[0].x_axis.title.as_deref(), Some("custom title"));
    click(&mut h, "Automatic text");
    click(&mut h, "Apply");
    assert!(graph(&h).layers[0].x_axis.title.is_none());
    click(&mut h, "Cancel");
    assert_eq!(project(&h), &before);
    open_title(&mut h, false);
    type_into_text_window(&mut h, "stale", true);
    h.state_mut().doc.replace(before.clone());
    settle(&mut h);
    assert!(h.state().text_dialog.is_none());
    assert_eq!(project(&h), &before);
}

#[test]
fn title_editor_axis_api_retains_raw_metadata_and_histogram_default_titles() {
    let mut h = start(vec![data_file("title-editor-default-api")]);
    let (a, b) = (column_id(&h, "A"), column_id(&h, "B"));
    let gid = graph(&h).id;
    h.state_mut().doc.apply("Metadata", |p| {
        p.column_mut(a).unwrap().units = r"s\+(1)".into();
        p.column_mut(b).unwrap().long_name = r"\g(m)A".into();
    });
    assert_eq!(
        sheaf_plot::axis_title(project(&h), gid, 0, false).unwrap(),
        project(&h).column(a).unwrap().axis_title()
    );
    assert_eq!(sheaf_plot::axis_title(project(&h), gid, 0, true).unwrap(), project(&h).column(b).unwrap().axis_title());
    h.state_mut().doc.apply("Histogram", |p| {
        p.graph_mut(gid).unwrap().layers[0].plots[0].kind = sheaf_core::graph::PlotKind::Histogram
    });
    assert_eq!(sheaf_plot::axis_title(project(&h), gid, 0, true).unwrap(), "Count");
    assert_eq!(
        sheaf_plot::axis_title(project(&h), gid, 0, false).unwrap(),
        project(&h).column(b).unwrap().axis_title()
    );
}
