//! Real picker controls, preference isolation and the existing captured graph requests.
use super::*;
use sheaf_core::Color;
use sheaf_core::graph::{Font, TextLabel};
use sheaf_core::shape::{Shape, ShapeKind};
use sheaf_core::straight_line::StraightLine;

const RGB: Color = Color::rgb(17, 131, 213);

fn start_with(data: &Path, file: Option<PathBuf>) -> Harness<'static, SheafApp> {
    let startup = Startup { paths: vec![data.to_path_buf()], custom_colors_path: file, ..Default::default() };
    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .with_step_dt(0.02)
        .build_eframe(|cc| SheafApp::new(cc, startup));
    h.state_mut().panels.set_pinned(PanelId::Explorer, true);
    settle(&mut h);
    show_window(&mut h, "Graph1 - sample");
    select(&mut h, ObjectKind::Legend);
    h
}

fn select(h: &mut Harness<'static, SheafApp>, kind: ObjectKind) {
    let id = graph(h).id;
    h.state_mut().graphs.get_mut(&id).unwrap().selected = Some((0, kind));
    settle(h);
}

fn open(h: &mut Harness<'static, SheafApp>, prefix: &str, submenu: &str) {
    click(h, &format!("Format toolbar: {prefix}"));
    click(h, &format!("{prefix}: {submenu}"));
}

fn channel(h: &mut Harness<'static, SheafApp>, name: &str, value: &str) {
    h.get_by_role_and_label(egui::accesskit::Role::TextInput, &format!("Graph font color: {name}")).click();
    settle(h);
    ctrl(h, egui::Key::A);
    h.event(egui::Event::Text(value.to_string()));
    settle(h);
}

fn add(h: &mut Harness<'static, SheafApp>) {
    open(h, "Font Color", "Custom RGB...");
    for (name, value) in [("Red", "17"), ("Green", "131"), ("Blue", "213")] {
        channel(h, name, value);
    }
    click(h, "Graph font color: Add to Custom");
    assert!(h.get_by_label("Graph font color: Add to Custom").accesskit_node().is_disabled());
    click(h, "Cancel font color");
}

fn choose(h: &mut Harness<'static, SheafApp>, prefix: &str) {
    open(h, prefix, "Custom colors");
    click(h, &format!("{prefix}: Custom 17,131,213"));
}

#[test]
fn add_cancel_and_restart_keep_preferences_outside_document_history() {
    let data = data_file("custom-wells-restart");
    let file = data.parent().unwrap().join("custom-colors.json");
    let mut h = start_with(&data, Some(file.clone()));
    let before = project(&h).clone();
    let revision = h.state().doc.revision();
    let undo = h.state().doc.undo_label().map(str::to_owned);
    add(&mut h);
    assert_eq!(project(&h), &before);
    assert_eq!(h.state().doc.revision(), revision);
    assert_eq!(h.state().doc.undo_label(), undo.as_deref());
    let saved = std::fs::read(&file).unwrap();
    drop(h);
    crate::custom_colors::initialize(None);
    let mut fresh = start_with(&data, Some(file.clone()));
    let before = project(&fresh).clone();
    let id = graph(&fresh).id;
    choose(&mut fresh, "Font Color");
    let mut expected = before.clone();
    expected.graph_mut(id).unwrap().layers[0].legend.font.color = RGB;
    assert_eq!(project(&fresh), &expected);
    let revision = fresh.state().doc.revision();
    choose(&mut fresh, "Font Color");
    assert_eq!(fresh.state().doc.revision(), revision, "same RGB creates no history");
    menu(&mut fresh, "Edit", "Undo");
    assert_eq!(project(&fresh), &before);
    fresh.key_press_modifiers(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::Z);
    settle(&mut fresh);
    assert_eq!(project(&fresh), &expected);
    assert_eq!(std::fs::read(&file).unwrap(), saved, "choose/Undo/Redo do not rewrite preferences");
    assert!(sheaf_plot::render_svg(project(&fresh), id).unwrap().contains("#1183D5"));
    for ext in ["sheaf", "opj"] {
        let path = data.parent().unwrap().join(format!("applied.{ext}"));
        fresh.state_mut().save_to(path.clone());
        let back = start(vec![path]);
        assert_eq!(graph(&back).layers[0].legend, expected.graph(id).unwrap().layers[0].legend);
    }
}

#[test]
fn one_pool_serves_every_supported_font_fill_and_line_target() {
    let data = data_file("custom-wells-targets");
    let mut h = start_with(&data, None);
    add(&mut h);
    let id = graph(&h).id;
    h.state_mut().doc.apply("fixture", |p| {
        let l = &mut p.graph_mut(id).unwrap().layers[0];
        l.labels.push(TextLabel::new("label", 0.2, 0.2, Font::new("Arial", 18.0)));
        l.shapes = vec![
            Shape::new(ShapeKind::Rectangle, (0.2, 0.2), (0.4, 0.4)),
            Shape::new(ShapeKind::Ellipse, (0.5, 0.3), (0.7, 0.5)),
            Shape::new(ShapeKind::Line, (0.5, 0.6), (0.8, 0.8)),
            Shape::new(ShapeKind::Arrow, (0.2, 0.6), (0.4, 0.8)),
        ];
        l.straight_lines.push(StraightLine::new(true, 2.0));
    });
    for (prefix, kind) in [
        ("Font Color", ObjectKind::Legend),
        ("Font Color", ObjectKind::XTitle),
        ("Font Color", ObjectKind::YTitle),
        ("Font Color", ObjectKind::Label(0)),
        ("Fill Color", ObjectKind::Legend),
        ("Fill Color", ObjectKind::Shape(0)),
        ("Fill Color", ObjectKind::Shape(1)),
        ("Line Color", ObjectKind::Shape(0)),
        ("Line Color", ObjectKind::Shape(1)),
        ("Line Color", ObjectKind::Shape(2)),
        ("Line Color", ObjectKind::Shape(3)),
        ("Line Color", ObjectKind::StraightLine(0)),
    ] {
        select(&mut h, kind);
        let before = project(&h).clone();
        choose(&mut h, prefix);
        let mut expected = before.clone();
        let l = &mut expected.graph_mut(id).unwrap().layers[0];
        match (prefix, kind) {
            ("Font Color", ObjectKind::Legend) => l.legend.font.color = RGB,
            ("Font Color", ObjectKind::XTitle) => l.x_axis.title_font.color = RGB,
            ("Font Color", ObjectKind::YTitle) => l.y_axis.title_font.color = RGB,
            ("Font Color", ObjectKind::Label(i)) => l.labels[i].font.color = RGB,
            ("Fill Color", ObjectKind::Legend) => l.legend.background = Some(RGB),
            ("Fill Color", ObjectKind::Shape(i)) => l.shapes[i].fill = Some(RGB),
            ("Line Color", ObjectKind::Shape(i)) => l.shapes[i].color = RGB,
            ("Line Color", ObjectKind::StraightLine(i)) => l.straight_lines[i].color = RGB,
            _ => unreachable!(),
        }
        assert_eq!(project(&h), &expected, "{prefix}/{kind:?}");
        assert!(sheaf_plot::render_svg(project(&h), id).unwrap().contains("#1183D5"));
        menu(&mut h, "Edit", "Undo");
        assert_eq!(project(&h), &before);
    }
}

#[test]
fn invalid_drafts_cannot_add_and_removal_changes_only_preferences() {
    let data = data_file("custom-wells-remove");
    let file = data.parent().unwrap().join("custom-colors.json");
    let mut h = start_with(&data, Some(file.clone()));
    let before = project(&h).clone();
    let revision = h.state().doc.revision();
    open(&mut h, "Font Color", "Custom RGB...");
    channel(&mut h, "Red", "256");
    assert!(h.get_by_label("Graph font color: Add to Custom").accesskit_node().is_disabled());
    assert!(!file.exists());
    click(&mut h, "Cancel font color");
    add(&mut h);
    open(&mut h, "Font Color", "Custom colors");
    right_click(&mut h, "Font Color: Custom 17,131,213");
    click(&mut h, "Remove custom color");
    assert_eq!(project(&h), &before);
    assert_eq!(h.state().doc.revision(), revision);
    drop(h);
    let mut fresh = start_with(&data, Some(file));
    click(&mut fresh, "Format toolbar: Font Color");
    assert!(fresh.get_by_label("Font Color: Custom colors").accesskit_node().is_disabled());
}

#[test]
fn saved_color_menu_keeps_existing_stale_request_guards() {
    for prefix in ["Font Color", "Fill Color", "Line Color"] {
        for scenario in 0..3 {
            let data = data_file(&format!("custom-wells-stale-{prefix}-{scenario}"));
            let mut h = start_with(&data, None);
            add(&mut h);
            let id = graph(&h).id;
            h.state_mut().doc.apply("fixture", |p| {
                p.graph_mut(id).unwrap().layers[0].shapes.push(Shape::new(ShapeKind::Rectangle, (0.2, 0.2), (0.4, 0.4)))
            });
            select(&mut h, if prefix == "Font Color" { ObjectKind::Legend } else { ObjectKind::Shape(0) });
            open(&mut h, prefix, "Custom colors");
            match scenario {
                0 => h.state_mut().graphs.get_mut(&id).unwrap().selected = Some((0, ObjectKind::XTitle)),
                1 => {
                    h.state_mut().doc.apply("Other", |p| p.graph_mut(id).unwrap().name.push('x'));
                }
                _ => {
                    let copy = project(&h).clone();
                    h.state_mut().doc.replace(copy);
                }
            }
            settle(&mut h);
            let before = project(&h).clone();
            let revision = h.state().doc.revision();
            if let Some(entry) = h.query_by_label(&format!("{prefix}: Custom 17,131,213")) {
                entry.click();
                settle(&mut h);
            }
            assert_eq!(project(&h), &before, "{prefix}/{scenario}");
            assert_eq!(h.state().doc.revision(), revision);
        }
    }
}

#[test]
fn object_rgb_add_buttons_share_the_font_pool_without_applying_their_draft() {
    let data = data_file("custom-wells-object-add");
    let mut h = start_with(&data, None);
    let id = graph(&h).id;
    h.state_mut().doc.apply("fixture", |p| {
        let mut shape = Shape::new(ShapeKind::Rectangle, (0.2, 0.2), (0.4, 0.4));
        shape.fill = Some(RGB);
        shape.color = Color::rgb(20, 19, 18);
        p.graph_mut(id).unwrap().layers[0].shapes.push(shape);
    });
    select(&mut h, ObjectKind::Shape(0));
    let before = project(&h).clone();
    let revision = h.state().doc.revision();
    for (prefix, object) in [("Fill Color", "Object fill color"), ("Line Color", "Object line color")] {
        open(&mut h, prefix, "Custom RGB...");
        click(&mut h, &format!("{object}: Add to Custom"));
        click(&mut h, &format!("Cancel {object}"));
        assert_eq!(project(&h), &before);
        assert_eq!(h.state().doc.revision(), revision);
    }
    select(&mut h, ObjectKind::Legend);
    choose(&mut h, "Font Color");
    assert_eq!(graph(&h).layers[0].legend.font.color, RGB);
    open(&mut h, "Font Color", "Custom colors");
    click(&mut h, "Font Color: Custom 20,19,18");
    assert_eq!(graph(&h).layers[0].legend.font.color, Color::rgb(20, 19, 18));
}

#[test]
fn failed_profile_write_is_visible_and_the_session_well_remains_usable() {
    let data = data_file("custom-wells-write-failure");
    let file = data.parent().unwrap().join("custom-colors.json");
    std::fs::create_dir(&file).unwrap();
    let mut h = start_with(&data, Some(file.clone()));
    let before = project(&h).clone();
    open(&mut h, "Font Color", "Custom RGB...");
    for (name, value) in [("Red", "17"), ("Green", "131"), ("Blue", "213")] {
        channel(&mut h, name, value);
    }
    click(&mut h, "Graph font color: Add to Custom");
    assert!(h.query_by_value("Custom colors are available for this session, but could not be saved.").is_some());
    assert_eq!(project(&h), &before);
    click(&mut h, "Cancel font color");
    choose(&mut h, "Font Color");
    assert_eq!(graph(&h).layers[0].legend.font.color, RGB);
    assert!(file.is_dir());
}
