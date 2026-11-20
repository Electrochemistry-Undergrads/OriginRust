//! Classic object palettes preserve captured targets, unrelated fields and history.
use super::*;
use sheaf_core::Color;
use sheaf_core::graph::{Font, TextLabel};
use sheaf_core::shape::{Shape, ShapeKind};
use sheaf_core::straight_line::StraightLine;

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

fn choice(h: &mut Harness<'static, SheafApp>, fill: bool, name: &str) {
    let prefix = if fill { "Fill Color" } else { "Line Color" };
    click(h, &format!("Format toolbar: {prefix}"));
    if !["Black", "Red", "Green", "Blue", "Cyan", "Magenta", "Yellow", "White", "None"].contains(&name) {
        click(h, &format!("{prefix}: More Classic colors"));
    }
    click(h, &format!("{prefix}: {name}"));
    assert!(h.query_by_label(&format!("{prefix}: Black")).is_none(), "root palette closes");
    assert!(h.query_by_label(&format!("{prefix}: Royal")).is_none(), "submenu closes");
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
            Item::Text(_) | Item::Image { .. } => None,
        })
        .collect()
}

fn all_colors(fill: bool, objects: &[ObjectKind], tag: &str) {
    let mut h = setup(tag);
    let id = graph(&h).id;
    for &object in objects {
        for (name, [r, g, b]) in CLASSIC {
            // Undo refreshes graph interaction state; each independent choice selects its target.
            select(&mut h, object);
            let before = project(&h).clone();
            let revision = h.state().doc.revision();
            let color = Color::rgb(r, g, b);
            choice(&mut h, fill, name);
            let mut expected = before.clone();
            let layer = &mut expected.graph_mut(id).unwrap().layers[0];
            match object {
                ObjectKind::Legend => layer.legend.background = Some(color),
                ObjectKind::Shape(i) => {
                    if fill {
                        layer.shapes[i].fill = Some(color);
                    } else {
                        layer.shapes[i].color = color;
                    }
                }
                ObjectKind::StraightLine(i) => layer.straight_lines[i].color = color,
                _ => unreachable!(),
            }
            assert_eq!(project(&h), &expected, "{object:?}/{name}");
            assert_eq!(graph_ui(&h).selected, Some((0, object)));
            let rendered = rendered_colors(&h, object, fill);
            assert!(!rendered.is_empty() && rendered.iter().all(|c| *c == color), "{object:?}/{name}: {rendered:?}");
            if expected == before {
                assert_eq!(h.state().doc.revision(), revision);
            } else {
                assert_eq!(h.state().doc.undo_label(), Some("Format graph object"));
                menu(&mut h, "Edit", "Undo");
                assert_eq!(project(&h), &before, "one Undo: {object:?}/{name}");
            }
        }
    }
}

#[test]
fn all_24_fills_reach_rectangle_circle_and_legend_rendering_with_one_undo() {
    all_colors(true, &[ObjectKind::Shape(0), ObjectKind::Shape(1), ObjectKind::Legend], "object-classic-fill");
}

#[test]
fn all_24_lines_reach_drawn_and_straight_line_rendering_with_one_undo() {
    all_colors(
        false,
        &[
            ObjectKind::Shape(0),
            ObjectKind::Shape(1),
            ObjectKind::Shape(2),
            ObjectKind::Shape(3),
            ObjectKind::StraightLine(0),
        ],
        "object-classic-line",
    );
}

#[test]
fn repeated_choice_and_dismissal_add_no_history_and_none_remains_available() {
    let mut h = setup("object-classic-dismiss");
    for fill in [true, false] {
        choice(&mut h, fill, "Royal");
        let before = project(&h).clone();
        let revision = h.state().doc.revision();
        choice(&mut h, fill, "Royal");
        let prefix = if fill { "Fill Color" } else { "Line Color" };
        click(&mut h, &format!("Format toolbar: {prefix}"));
        click(&mut h, &format!("{prefix}: More Classic colors"));
        h.key_press(egui::Key::Escape);
        settle(&mut h);
        h.key_press(egui::Key::Escape);
        settle(&mut h);
        assert_eq!(project(&h), &before);
        assert_eq!(h.state().doc.revision(), revision);
    }
    let before = project(&h).clone();
    choice(&mut h, true, "None");
    assert_eq!(graph(&h).layers[0].shapes[0].fill, None);
    menu(&mut h, "Edit", "Undo");
    assert_eq!(project(&h), &before);
}

#[test]
fn object_extras_reject_changed_selection_revision_and_generation() {
    for fill in [true, false] {
        for scenario in 0..3 {
            let mut h = setup(&format!("object-classic-stale-{fill}-{scenario}"));
            let id = graph(&h).id;
            let prefix = if fill { "Fill Color" } else { "Line Color" };
            click(&mut h, &format!("Format toolbar: {prefix}"));
            click(&mut h, &format!("{prefix}: More Classic colors"));
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
            if h.query_by_label(&format!("{prefix}: Royal")).is_some() {
                click(&mut h, &format!("{prefix}: Royal"));
            }
            assert_eq!(project(&h), &before, "{fill}/{scenario}");
            assert_eq!(h.state().doc.revision(), revision);
        }
    }
}

#[test]
fn object_colors_preserve_text_geometry_svg_and_native_companion_saves() {
    let mut h = setup("object-classic-save");
    let before = graph(&h).layers[0].clone();
    choice(&mut h, true, "Orange");
    choice(&mut h, false, "Royal");
    select(&mut h, ObjectKind::StraightLine(0));
    choice(&mut h, false, "Wine");
    select(&mut h, ObjectKind::Legend);
    choice(&mut h, true, "LT Cyan");
    let expected = graph(&h).layers[0].clone();
    assert_eq!(expected.labels, before.labels);
    assert_eq!(expected.legend.font, before.legend.font);
    assert_eq!(expected.legend.text, before.legend.text);
    let svg = sheaf_plot::render_svg(project(&h), graph(&h).id).unwrap();
    for color in ["#FF8000", "#0000A0", "#800000", "#80FFFF"] {
        assert!(svg.contains(color), "{color}");
    }
    let dir = save_dir("object-classic-save");
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

// Independent Origin 2016 EMF pen/brush controls, graphing 7.4.
const CLASSIC: [(&str, [u8; 3]); 24] = [
    ("Black", [0, 0, 0]),
    ("Red", [255, 0, 0]),
    ("Green", [0, 255, 0]),
    ("Blue", [0, 0, 255]),
    ("Cyan", [0, 255, 255]),
    ("Magenta", [255, 0, 255]),
    ("Yellow", [255, 255, 0]),
    ("Dark Yellow", [128, 128, 0]),
    ("Navy", [0, 0, 128]),
    ("Purple", [128, 0, 128]),
    ("Wine", [128, 0, 0]),
    ("Olive", [0, 128, 0]),
    ("Dark Cyan", [0, 128, 128]),
    ("Royal", [0, 0, 160]),
    ("Orange", [255, 128, 0]),
    ("Violet", [128, 0, 255]),
    ("Pink", [255, 0, 128]),
    ("White", [255, 255, 255]),
    ("LT Gray", [192, 192, 192]),
    ("Gray", [128, 128, 128]),
    ("LT Yellow", [255, 255, 128]),
    ("LT Cyan", [128, 255, 255]),
    ("LT Magenta", [255, 128, 255]),
    ("Dark Gray", [64, 64, 64]),
];
