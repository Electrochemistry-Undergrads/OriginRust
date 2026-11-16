use super::*;
use sheaf_core::Color;

fn start_graph(tag: &str) -> Harness<'static, SheafApp> {
    let mut h = start(vec![data_file(tag)]);
    let id = graph(&h).id;
    let title = project(&h).windows.iter().find(|w| w.id() == id).unwrap().title();
    show_window(&mut h, &title);
    let at = on(&h, 0, ObjectKind::Legend);
    click_at(&mut h, at);
    assert_eq!(graph_ui(&h).selected, Some((0, ObjectKind::Legend)));
    h
}

fn extra(h: &mut Harness<'static, SheafApp>, name: &str) {
    click(h, "Format toolbar: Font Color");
    click(h, "Font Color: More Classic colors");
    click(h, &format!("Font Color: {name}"));
    assert!(h.query_by_label("Font Color: Black").is_none(), "the root palette must close too");
}

#[test]
fn graph_royal_changes_only_the_selected_legend_font_and_has_one_undo() {
    let mut h = start_graph("graph-classic-legend");
    let before = project(&h).clone();
    let id = graph(&h).id;
    extra(&mut h, "Royal");
    let mut expected = before.clone();
    expected.graph_mut(id).unwrap().layers[0].legend.font.color = Color::rgb(0, 0, 160);
    assert_eq!(project(&h), &expected);
    assert_eq!(graph(&h).layers[0].legend.text, before.graph(id).unwrap().layers[0].legend.text);
    menu(&mut h, "Edit", "Undo");
    assert_eq!(project(&h), &before);
}

// Expected RGB from the inspected Origin 2016 graph palette and independent EMF mapping.
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

fn select(h: &mut Harness<'static, SheafApp>, object: ObjectKind) {
    let id = graph(h).id;
    h.state_mut().graphs.get_mut(&id).unwrap().selected = Some((0, object));
    settle(h);
    assert!(!h.get_by_label("Format toolbar: Font Color").accesskit_node().is_disabled());
}

fn choice(h: &mut Harness<'static, SheafApp>, name: &str) {
    if ["Black", "Red", "Green", "Blue", "Cyan", "Magenta", "Yellow", "White"].contains(&name) {
        click(h, "Format toolbar: Font Color");
        click(h, &format!("Font Color: {name}"));
    } else {
        extra(h, name);
    }
}

fn rendered_colors(h: &Harness<'static, SheafApp>, object: ObjectKind) -> Vec<Color> {
    let scene = sheaf_plot::graph_scene(project(h), graph(h).id).unwrap();
    let layer = scene.layers.iter().find(|l| l.objects.iter().any(|o| o.layer == 0 && o.kind == object)).unwrap();
    let target = layer.objects.iter().find(|o| o.layer == 0 && o.kind == object).unwrap();
    layer.overlay[target.items.clone()]
        .iter()
        .filter_map(|item| if let sheaf_plot::scene::Item::Text(text) = item { Some(text.color) } else { None })
        .collect()
}

#[test]
fn all_24_colors_target_each_graph_text_font_and_reach_the_rendered_scene() {
    use sheaf_core::graph::{Font, TextLabel};
    let mut h = start_graph("graph-classic-all");
    let id = graph(&h).id;
    h.state_mut().doc.apply("Seed label", |p| {
        p.graph_mut(id).unwrap().layers[0].labels.push(TextLabel::new(
            r"panel \i(a)",
            0.2,
            0.2,
            Font::new("Arial", 18.0),
        ));
    });
    settle(&mut h);
    for object in [ObjectKind::Legend, ObjectKind::XTitle, ObjectKind::YTitle, ObjectKind::Label(0)] {
        for (name, [r, g, b]) in CLASSIC {
            select(&mut h, object);
            let before = project(&h).clone();
            let revision = h.state().doc.revision();
            let color = Color::rgb(r, g, b);
            choice(&mut h, name);
            let mut expected = before.clone();
            let layer = &mut expected.graph_mut(id).unwrap().layers[0];
            let font = match object {
                ObjectKind::Legend => &mut layer.legend.font,
                ObjectKind::XTitle => &mut layer.x_axis.title_font,
                ObjectKind::YTitle => &mut layer.y_axis.title_font,
                ObjectKind::Label(0) => &mut layer.labels[0].font,
                _ => unreachable!(),
            };
            font.color = color;
            assert_eq!(project(&h), &expected, "{object:?} / {name}");
            let rendered = rendered_colors(&h, object);
            assert!(
                !rendered.is_empty() && rendered.iter().all(|actual| *actual == color),
                "{object:?} / {name}: {rendered:?}"
            );
            if expected == before {
                assert_eq!(h.state().doc.revision(), revision, "an unchanged font colour is a no-op");
            } else {
                menu(&mut h, "Edit", "Undo");
                assert_eq!(project(&h), &before, "{object:?} / {name} has one Undo");
            }
        }
    }
}

#[test]
fn extra_graph_choices_reject_stale_selection_revision_and_generation() {
    for scenario in 0..3 {
        let mut h = start_graph(&format!("graph-classic-stale-{scenario}"));
        let id = graph(&h).id;
        click(&mut h, "Format toolbar: Font Color");
        click(&mut h, "Font Color: More Classic colors");
        match scenario {
            0 => h.state_mut().graphs.get_mut(&id).unwrap().selected = Some((0, ObjectKind::XTitle)),
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
        if h.query_by_label("Font Color: Royal").is_some() {
            click(&mut h, "Font Color: Royal");
        }
        assert_eq!(project(&h), &before, "stale scenario {scenario}");
        assert_eq!(h.state().doc.revision(), revision);
    }
}

#[test]
fn graph_font_extras_keep_shape_fill_and_line_palettes_separate() {
    use sheaf_core::shape::{Shape, ShapeKind};
    let mut h = start_graph("graph-classic-domains");
    extra(&mut h, "Royal");
    let revision = h.state().doc.revision();
    extra(&mut h, "Royal");
    assert_eq!(h.state().doc.revision(), revision);
    let id = graph(&h).id;
    h.state_mut().doc.apply("Seed rectangle", |p| {
        p.graph_mut(id).unwrap().layers[0].shapes.push(Shape::new(ShapeKind::Rectangle, (0.2, 0.2), (0.4, 0.4)));
    });
    h.state_mut().graphs.get_mut(&id).unwrap().selected = Some((0, ObjectKind::Shape(0)));
    settle(&mut h);
    assert!(h.get_by_label("Format toolbar: Font Color").accesskit_node().is_disabled());
    let before = project(&h).clone();
    for prefix in ["Fill Color", "Line Color"] {
        click(&mut h, &format!("Format toolbar: {prefix}"));
        assert!(h.query_by_label(&format!("{prefix}: Red")).is_some());
        assert!(h.query_by_label(&format!("{prefix}: More Classic colors")).is_none());
        assert!(h.query_by_label(&format!("{prefix}: Royal")).is_none());
        h.key_press(egui::Key::Escape);
        settle(&mut h);
    }
    assert_eq!(project(&h), &before);
}

#[test]
fn extra_graph_font_color_preserves_styles_svg_and_project_roundtrips() {
    let mut h = start_graph("graph-classic-save");
    click(&mut h, "Format toolbar: Bold");
    click(&mut h, "Format toolbar: Italic");
    let before = graph(&h).layers[0].legend.clone();
    extra(&mut h, "Royal");
    let expected = graph(&h).layers[0].legend.clone();
    assert_eq!(expected.text, before.text);
    assert_eq!(expected.font.family, before.font.family);
    assert_eq!(expected.font.size_pt, before.font.size_pt);
    assert!(expected.font.bold && expected.font.italic);
    let svg = sheaf_plot::render_svg(project(&h), graph(&h).id).unwrap();
    assert!(svg.contains("#0000A0") && svg.contains("font-weight=\"bold\"") && svg.contains("font-style=\"italic\""));
    let dir = save_dir("graph-classic-save");
    std::fs::write(dir.join("classic.svg"), svg).unwrap();
    for extension in ["sheaf", "opj"] {
        let file = dir.join(format!("classic.{extension}"));
        h.state_mut().save_to(file.clone());
        let back = start(vec![file]);
        assert_eq!(graph(&back).layers[0].legend, expected, "{extension}");
    }
}
