use super::*;
use crate::text_format::{Change, Snapshot};
use sheaf_core::graph::{Font, TextLabel};

fn start_graph(tag: &str) -> Harness<'static, SheafApp> {
    let mut h = start(vec![data_file(tag)]);
    let id = graph(&h).id;
    let title = project(&h).windows.iter().find(|w| w.id() == id).unwrap().title();
    show_window(&mut h, &title);
    h
}

fn select(h: &mut Harness<'static, SheafApp>, object: ObjectKind) {
    let at = on(h, 0, object);
    click_at(h, at);
    assert_eq!(graph_ui(h).selected, Some((0, object)));
}

fn enter_size(h: &mut Harness<'static, SheafApp>, text: &str) {
    click(h, "Format toolbar: Font Size");
    ctrl(h, egui::Key::A);
    h.event(egui::Event::Text(text.to_string()));
    h.run();
    h.key_press(egui::Key::Enter);
    settle(h);
}

#[test]
fn format_toolbar_controls_follow_graph_text_selection() {
    let mut h = start(vec![data_file("format-toolbar-selection")]);
    let id = graph(&h).id;
    let title = project(&h).windows.iter().find(|w| w.id() == id).unwrap().title();
    show_window(&mut h, &title);
    assert!(h.get_by_label("Format toolbar: Bold").accesskit_node().is_disabled());
    assert_eq!(h.get_by_label("Format toolbar: Font Size").accesskit_node().role(), egui::accesskit::Role::ComboBox);

    let at = on(&h, 0, ObjectKind::Legend);
    click_at(&mut h, at);
    assert_eq!(graph_ui(&h).selected, Some((0, ObjectKind::Legend)));
    assert!(!h.get_by_label("Format toolbar: Bold").accesskit_node().is_disabled());

    let at = on(&h, 0, ObjectKind::Plot(0));
    click_at(&mut h, at);
    assert!(h.get_by_label("Format toolbar: Bold").accesskit_node().is_disabled());
}

#[test]
fn whole_legend_styles_toggle_and_undo_without_rewriting_text_or_data() {
    let mut h = start_graph("format-toolbar-toggle");
    select(&mut h, ObjectKind::Legend);
    let before = project(&h).clone();
    let old_history = h.state().doc.undo_label().map(str::to_owned);
    click(&mut h, "Format toolbar: Bold");
    assert!(graph(&h).layers[0].legend.font.bold);
    assert_eq!(h.state().doc.undo_label(), Some("Format graph text"));
    menu(&mut h, "Edit", "Undo");
    assert_eq!(project(&h), &before);
    assert_eq!(h.state().doc.undo_label(), old_history.as_deref());
    menu(&mut h, "Edit", "Redo");
    assert!(graph(&h).layers[0].legend.font.bold);
    click(&mut h, "Format toolbar: Bold");
    assert!(!graph(&h).layers[0].legend.font.bold, "pressed Bold can toggle off");
    click(&mut h, "Format toolbar: Italic");
    assert!(graph(&h).layers[0].legend.font.italic);
    assert_eq!(graph(&h).layers[0].legend.text, before.graph(graph(&h).id).unwrap().layers[0].legend.text);
    assert_eq!(project(&h).workbooks().collect::<Vec<_>>(), before.workbooks().collect::<Vec<_>>());
}

#[test]
fn font_size_family_and_color_apply_through_the_shared_controls() {
    let mut h = start_graph("format-toolbar-font");
    select(&mut h, ObjectKind::Legend);
    enter_size(&mut h, "18");
    assert_eq!(graph(&h).layers[0].legend.font.size_pt, 18.0);
    click(&mut h, "Format toolbar: Increase Font Size");
    assert_eq!(graph(&h).layers[0].legend.font.size_pt, 22.0);
    click(&mut h, "Format toolbar: Decrease Font Size");
    assert_eq!(graph(&h).layers[0].legend.font.size_pt, 18.0);
    click(&mut h, "Format toolbar: Font Color");
    click(&mut h, "Font Color: Red");
    assert_eq!(graph(&h).layers[0].legend.font.color, sheaf_core::Color::rgb(255, 0, 0));
    click(&mut h, "Format toolbar: Font Color");
    click(&mut h, "Font Color: Blue");
    assert_eq!(graph(&h).layers[0].legend.font.color, sheaf_core::Color::rgb(0, 0, 255));
    let old_family = graph(&h).layers[0].legend.font.family.clone();
    let first_family = sheaf_plot::SystemFonts::shared()
        .database()
        .faces()
        .flat_map(|f| f.families.iter().map(|(n, _)| n.clone()))
        .filter(|name| name != &old_family)
        .min()
        .unwrap();
    click(&mut h, "Format toolbar: Font");
    click(&mut h, &first_family);
    assert_eq!(graph(&h).layers[0].legend.font.family, first_family);
    let before = project(&h).clone();
    let revision = h.state().doc.revision();
    for text in ["0", "-2", "NaN", "inf", "1001", "bad", "18"] {
        enter_size(&mut h, text);
        assert_eq!(project(&h), &before, "invalid or unchanged input: {text}");
        assert_eq!(h.state().doc.revision(), revision);
    }
}

#[test]
fn labels_and_both_titles_use_only_the_selected_objects_font() {
    let mut h = start_graph("format-toolbar-targets");
    let id = graph(&h).id;
    h.state_mut().doc.apply("Seed text", |p| {
        let l = &mut p.graph_mut(id).unwrap().layers[0];
        l.labels.push(TextLabel::new("panel \\i(a)", 0.2, 0.2, Font::new("Arial", 18.0)));
    });
    settle(&mut h);
    for object in [ObjectKind::XTitle, ObjectKind::YTitle, ObjectKind::Label(0)] {
        select(&mut h, object);
        let before = project(&h).clone();
        click(&mut h, "Format toolbar: Italic");
        let actual = Snapshot::graph(&h.state().doc, id, 0, object).unwrap();
        let prior = Snapshot::graph(&Document::new(before.clone()), id, 0, object).unwrap();
        assert_eq!(actual.font.italic, !prior.font.italic);
        assert_eq!(graph(&h).layers[0].legend, before.graph(id).unwrap().layers[0].legend);
        menu(&mut h, "Edit", "Undo");
        assert_eq!(project(&h), &before);
    }
}

#[test]
fn stale_selection_revision_and_dialog_requests_are_ignored() {
    let mut h = start_graph("format-toolbar-stale");
    select(&mut h, ObjectKind::Legend);
    let request = h.state().text_format().unwrap().request(Change::Bold(true));
    let before = project(&h).clone();
    select(&mut h, ObjectKind::XTitle);
    h.state_mut().apply(Action::FormatText(request.clone()), &egui::Context::default());
    assert_eq!(project(&h), &before);
    select(&mut h, ObjectKind::Legend);
    h.state_mut().doc.apply("Other change", |p| p.graph_mut(graph_id(&before)).unwrap().name.push('x'));
    let changed = project(&h).clone();
    h.state_mut().apply(Action::FormatText(request), &egui::Context::default());
    assert_eq!(project(&h), &changed);
    let request = h.state().text_format().unwrap().request(Change::Bold(true));
    let id = graph(&h).id;
    h.state_mut().text_dialog = TextDialog::new_label(&h.state().doc, id, 0, (0.5, 0.5));
    h.state_mut().apply(Action::FormatText(request), &egui::Context::default());
    assert_eq!(project(&h), &changed);
    settle(&mut h);
    assert!(h.get_by_label("Format toolbar: Bold").accesskit_node().is_disabled());
}

fn graph_id(project: &Project) -> WindowId {
    project.graphs().next().unwrap().id
}

#[test]
fn an_open_color_palette_cannot_follow_a_changed_selection_or_project() {
    let mut h = start_graph("format-toolbar-popup");
    select(&mut h, ObjectKind::Legend);
    click(&mut h, "Format toolbar: Font Color");
    let before = project(&h).clone();
    let id = graph(&h).id;
    h.state_mut().graphs.get_mut(&id).unwrap().selected = Some((0, ObjectKind::XTitle));
    settle(&mut h);
    click(&mut h, "Font Color: Red");
    assert_eq!(project(&h), &before, "palette keeps the target captured when opened");

    select(&mut h, ObjectKind::Legend);
    let request = h.state().text_format().unwrap().request(Change::Bold(true));
    h.state_mut().doc.replace(before.clone());
    h.state_mut().apply(Action::FormatText(request), &egui::Context::default());
    assert_eq!(project(&h), &before, "a replaced project cannot receive an old toolbar action");
    assert!(h.state().doc.undo_label().is_none());
}

#[test]
fn toolbar_fonts_survive_native_and_origin_companion_round_trips_and_export() {
    let mut h = start_graph("format-toolbar-save");
    select(&mut h, ObjectKind::Legend);
    click(&mut h, "Format toolbar: Bold");
    click(&mut h, "Format toolbar: Italic");
    enter_size(&mut h, "30");
    assert_eq!(graph(&h).layers[0].legend.font.size_pt, 30.0);
    click(&mut h, "Format toolbar: Font Color");
    click(&mut h, "Font Color: Blue");
    let expected = graph(&h).layers[0].legend.font.clone();
    let id = graph(&h).id;
    let svg = sheaf_plot::render_svg(project(&h), id).unwrap();
    assert!(svg.contains("font-weight=\"bold\"") && svg.contains("font-style=\"italic\""));
    assert!(svg.contains("#0000FF"));
    let dir = save_dir("format-toolbar-save");
    std::fs::write(dir.join("formatted.svg"), svg).unwrap();
    for extension in ["sheaf", "opj"] {
        let file = dir.join(format!("formatted.{extension}"));
        h.state_mut().save_to(file.clone());
        let back = start(vec![file]);
        assert_eq!(graph(&back).layers[0].legend.font, expected, "{extension}");
    }
}
