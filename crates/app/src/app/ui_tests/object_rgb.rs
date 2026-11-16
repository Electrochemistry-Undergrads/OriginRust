//! Custom object colours keep captured targets, drafts, presets and history independent.
use super::*;
use sheaf_core::Color;
use sheaf_core::graph::{Font, TextLabel};
use sheaf_core::shape::{Shape, ShapeKind};
use sheaf_core::straight_line::StraightLine;

fn open(h: &mut Harness<'static, SheafApp>, fill: bool) {
    let prefix = if fill { "Fill Color" } else { "Line Color" };
    click(h, &format!("Format toolbar: {prefix}"));
    click(h, &format!("{prefix}: Custom RGB..."));
}

fn prefix(fill: bool) -> &'static str {
    if fill { "Object fill color" } else { "Object line color" }
}

fn channel(h: &mut Harness<'static, SheafApp>, fill: bool, name: &str, value: &str) {
    h.get_by_role_and_label(egui::accesskit::Role::TextInput, &format!("{}: {name}", prefix(fill))).click();
    settle(h);
    ctrl(h, egui::Key::A);
    if value.is_empty() {
        h.key_press(egui::Key::Backspace);
    } else {
        h.event(egui::Event::Text(value.to_string()));
    }
    settle(h);
}

fn seed(h: &Harness<'static, SheafApp>, fill: bool, values: [&str; 3]) {
    for (name, expected) in ["Red", "Green", "Blue"].into_iter().zip(values) {
        assert_eq!(
            h.get_by_role_and_label(egui::accesskit::Role::TextInput, &format!("{}: {name}", prefix(fill)))
                .accesskit_node()
                .value()
                .as_deref(),
            Some(expected)
        );
    }
}

fn closed(h: &Harness<'static, SheafApp>, fill: bool) {
    let root = if fill { "Fill Color" } else { "Line Color" };
    assert!(h.query_by_label(&format!("{}: Red", prefix(fill))).is_none());
    assert!(h.query_by_label(&format!("{root}: Black")).is_none());
}

fn rgb(h: &mut Harness<'static, SheafApp>, fill: bool, values: [&str; 3]) {
    open(h, fill);
    for (name, value) in ["Red", "Green", "Blue"].into_iter().zip(values) {
        channel(h, fill, name, value);
    }
    click(h, &format!("Apply {}", prefix(fill)));
    closed(h, fill);
}

#[test]
fn rgb_boundaries_reach_all_supported_objects_and_only_the_requested_property() {
    let mut h = setup("object-rgb-targets");
    let id = graph(&h).id;
    for (fill, object) in [
        (true, ObjectKind::Shape(0)),
        (true, ObjectKind::Shape(1)),
        (true, ObjectKind::Legend),
        (false, ObjectKind::Shape(0)),
        (false, ObjectKind::Shape(1)),
        (false, ObjectKind::Shape(2)),
        (false, ObjectKind::Shape(3)),
        (false, ObjectKind::StraightLine(0)),
    ] {
        for [r, g, b] in [[17, 131, 213], [0, 0, 0], [255, 255, 255], [0, 255, 128]] {
            select(&mut h, object);
            let before = project(&h).clone();
            let revision = h.state().doc.revision();
            let values = [r.to_string(), g.to_string(), b.to_string()];
            rgb(&mut h, fill, [&values[0], &values[1], &values[2]]);
            let color = Color::rgb(r, g, b);
            let mut expected = before.clone();
            let l = &mut expected.graph_mut(id).unwrap().layers[0];
            match object {
                ObjectKind::Legend => l.legend.background = Some(color),
                ObjectKind::Shape(i) => {
                    if fill {
                        l.shapes[i].fill = Some(color);
                    } else {
                        l.shapes[i].color = color;
                    }
                }
                ObjectKind::StraightLine(i) => l.straight_lines[i].color = color,
                _ => unreachable!(),
            }
            assert_eq!(project(&h), &expected);
            assert_eq!(graph_ui(&h).selected, Some((0, object)));
            let rendered = rendered_colors(&h, object, fill);
            assert!(!rendered.is_empty() && rendered.iter().all(|c| *c == color), "{fill}/{object:?}: {rendered:?}");
            if before == expected {
                assert_eq!(h.state().doc.revision(), revision);
            } else {
                menu(&mut h, "Edit", "Undo");
                assert_eq!(project(&h), &before);
            }
        }
    }
}

#[test]
fn invalid_channels_cancel_and_valid_enter_preserve_the_same_history_rules() {
    for fill in [true, false] {
        let mut h = setup(&format!("object-rgb-invalid-{fill}"));
        let before = project(&h).clone();
        let revision = h.state().doc.revision();
        open(&mut h, fill);
        for value in ["", "256", "-1", "1.5", "NaN"] {
            channel(&mut h, fill, "Red", value);
            assert!(h.get_by_label(&format!("Apply {}", prefix(fill))).accesskit_node().is_disabled());
            h.key_press(egui::Key::Enter);
            settle(&mut h);
            assert!(h.query_by_label(&format!("Apply {}", prefix(fill))).is_some());
            assert_eq!(project(&h), &before);
            assert_eq!(h.state().doc.revision(), revision);
        }
        click(&mut h, &format!("Cancel {}", prefix(fill)));
        closed(&h, fill);
        open(&mut h, fill);
        for (name, value) in [("Red", "17"), ("Green", "131"), ("Blue", "213")] {
            channel(&mut h, fill, name, value);
        }
        h.key_press(egui::Key::Enter);
        settle(&mut h);
        closed(&h, fill);
        let got = &graph(&h).layers[0].shapes[0];
        assert_eq!(if fill { got.fill.unwrap() } else { got.color }, Color::rgb(17, 131, 213));
        menu(&mut h, "Edit", "Undo");
        assert_eq!(project(&h), &before);
    }
}

#[test]
fn reopening_seeds_current_fill_or_line_and_unchanged_apply_adds_no_history() {
    let mut h = setup("object-rgb-reseed");
    rgb(&mut h, true, ["17", "131", "213"]);
    let revision = h.state().doc.revision();
    open(&mut h, true);
    seed(&h, true, ["17", "131", "213"]);
    click(&mut h, "Apply Object fill color");
    closed(&h, true);
    assert_eq!(h.state().doc.revision(), revision);
    open(&mut h, false);
    seed(&h, false, ["0", "0", "0"]);
    click(&mut h, "Cancel Object line color");
    click(&mut h, "Format toolbar: Fill Color");
    click(&mut h, "Fill Color: None");
    let before = project(&h).clone();
    let revision = h.state().doc.revision();
    open(&mut h, true);
    seed(&h, true, ["255", "255", "255"]);
    click(&mut h, "Cancel Object fill color");
    assert_eq!(project(&h), &before);
    assert_eq!(h.state().doc.revision(), revision);
    select(&mut h, ObjectKind::Shape(1));
    open(&mut h, true);
    seed(&h, true, ["255", "255", "255"]);
    click(&mut h, "Cancel Object fill color");
    click(&mut h, "Format toolbar: Line Color");
    click(&mut h, "Line Color: More Classic colors");
    click(&mut h, "Line Color: Royal");
    open(&mut h, false);
    seed(&h, false, ["0", "0", "160"]);
    click(&mut h, "Cancel Object line color");
}

#[test]
fn stale_custom_fill_and_line_requests_never_follow_another_selection_or_project() {
    for fill in [true, false] {
        for scenario in 0..3 {
            let mut h = setup(&format!("object-rgb-stale-{fill}-{scenario}"));
            let id = graph(&h).id;
            open(&mut h, fill);
            channel(&mut h, fill, "Red", "17");
            match scenario {
                0 => h.state_mut().graphs.get_mut(&id).unwrap().selected = Some((0, ObjectKind::Shape(1))),
                1 => {
                    h.state_mut().doc.apply("Other edit", |p| p.graph_mut(id).unwrap().name.push('x'));
                }
                _ => {
                    let replacement = project(&h).clone();
                    h.state_mut().doc.replace(replacement);
                }
            }
            settle(&mut h);
            let before = project(&h).clone();
            let revision = h.state().doc.revision();
            if h.query_by_label(&format!("Apply {}", prefix(fill))).is_some() {
                click(&mut h, &format!("Apply {}", prefix(fill)));
            }
            assert_eq!(project(&h), &before);
            assert_eq!(h.state().doc.revision(), revision);
        }
    }
}

#[test]
fn custom_object_colors_keep_styles_geometry_svg_and_both_project_formats() {
    let mut h = setup("object-rgb-save");
    let before = graph(&h).layers[0].clone();
    rgb(&mut h, true, ["17", "131", "213"]);
    rgb(&mut h, false, ["20", "19", "18"]);
    select(&mut h, ObjectKind::StraightLine(0));
    rgb(&mut h, false, ["1", "2", "3"]);
    select(&mut h, ObjectKind::Legend);
    rgb(&mut h, true, ["0", "255", "128"]);
    let expected = graph(&h).layers[0].clone();
    assert_eq!(expected.labels, before.labels);
    assert_eq!(expected.legend.font, before.legend.font);
    assert_eq!(expected.legend.text, before.legend.text);
    let svg = sheaf_plot::render_svg(project(&h), graph(&h).id).unwrap();
    for color in ["#1183D5", "#141312", "#010203", "#00FF80"] {
        assert!(svg.contains(color), "{color}");
    }
    let dir = save_dir("object-rgb-save");
    for extension in ["sheaf", "opj"] {
        let file = dir.join(format!("objects.{extension}"));
        h.state_mut().save_to(file.clone());
        let back = start(vec![file]);
        let got = &graph(&back).layers[0];
        assert_eq!(got.shapes, expected.shapes, "{extension}");
        assert_eq!(got.straight_lines, expected.straight_lines, "{extension}");
        assert_eq!(got.legend, expected.legend, "{extension}");
        assert_eq!(got.labels, expected.labels, "{extension}");
    }
}

#[test]
fn unsupported_targets_and_escape_leave_the_document_untouched() {
    let mut h = setup("object-rgb-escape");
    for object in [ObjectKind::Shape(2), ObjectKind::Shape(3), ObjectKind::StraightLine(0)] {
        select(&mut h, object);
        assert!(h.get_by_label("Format toolbar: Fill Color").accesskit_node().is_disabled());
    }
    select(&mut h, ObjectKind::Label(0));
    for prefix in ["Fill Color", "Line Color"] {
        assert!(h.get_by_label(&format!("Format toolbar: {prefix}")).accesskit_node().is_disabled());
    }
    select(&mut h, ObjectKind::Legend);
    assert!(h.get_by_label("Format toolbar: Line Color").accesskit_node().is_disabled());
    select(&mut h, ObjectKind::Shape(0));
    let before = project(&h).clone();
    let revision = h.state().doc.revision();
    for fill in [true, false] {
        open(&mut h, fill);
        channel(&mut h, fill, "Red", "17");
        h.key_press(egui::Key::Escape);
        settle(&mut h);
        h.key_press(egui::Key::Escape);
        settle(&mut h);
        assert_eq!(project(&h), &before);
        assert_eq!(h.state().doc.revision(), revision);
    }
}

#[test]
fn classic_presets_replace_unapplied_rgb_drafts_and_seed_the_next_picker() {
    let mut h = setup("object-rgb-classic");
    let before = project(&h).clone();
    open(&mut h, true);
    channel(&mut h, true, "Red", "17");
    click(&mut h, "Fill Color: Red");
    closed(&h, true);
    assert_eq!(graph(&h).layers[0].shapes[0].fill, Some(Color::rgb(255, 0, 0)));
    open(&mut h, true);
    seed(&h, true, ["255", "0", "0"]);
    click(&mut h, "Cancel Object fill color");
    open(&mut h, false);
    channel(&mut h, false, "Red", "17");
    click(&mut h, "Line Color: More Classic colors");
    click(&mut h, "Line Color: Royal");
    closed(&h, false);
    open(&mut h, false);
    seed(&h, false, ["0", "0", "160"]);
    click(&mut h, "Cancel Object line color");
    menu(&mut h, "Edit", "Undo");
    menu(&mut h, "Edit", "Undo");
    assert_eq!(project(&h), &before);
}

fn setup(tag: &str) -> Harness<'static, SheafApp> {
    let mut h = start(vec![data_file(tag)]);
    let id = graph(&h).id;
    let title = project(&h).windows.iter().find(|w| w.id() == id).unwrap().title();
    show_window(&mut h, &title);
    h.state_mut().doc.apply("Seed", |p| {
        let l = &mut p.graph_mut(id).unwrap().layers[0];
        l.labels.push(TextLabel::new(r"raw \i(label)", 0.2, 0.2, Font::new("Arial", 18.0)));
        l.shapes = vec![
            Shape::new(ShapeKind::Rectangle, (0.2, 0.2), (0.4, 0.4)),
            Shape::new(ShapeKind::Ellipse, (0.5, 0.3), (0.7, 0.5)),
            Shape::new(ShapeKind::Line, (0.5, 0.6), (0.8, 0.8)),
            Shape::new(ShapeKind::Arrow, (0.2, 0.6), (0.4, 0.8)),
        ];
        l.straight_lines.push(StraightLine::new(true, 2.0));
        l.legend.visible = true;
    });
    select(&mut h, ObjectKind::Shape(0));
    h
}

fn select(h: &mut Harness<'static, SheafApp>, object: ObjectKind) {
    let id = graph(h).id;
    h.state_mut().graphs.get_mut(&id).unwrap().selected = Some((0, object));
    settle(h);
}

fn rendered_colors(h: &Harness<'static, SheafApp>, object: ObjectKind, fill: bool) -> Vec<Color> {
    use sheaf_plot::scene::Item;
    let scene = sheaf_plot::graph_scene(project(h), graph(h).id).unwrap();
    let layer = &scene.layers[0];
    let target = layer.objects.iter().find(|o| o.layer == 0 && o.kind == object).unwrap();
    layer.overlay[target.items.clone()]
        .iter()
        .filter_map(|item| match item {
            Item::Rect { fill: c, stroke, .. } | Item::Path { fill: c, stroke, .. } => {
                if fill {
                    *c
                } else {
                    stroke.as_ref().map(|s| s.color)
                }
            }
            Item::Text(_) => None,
        })
        .collect()
}
