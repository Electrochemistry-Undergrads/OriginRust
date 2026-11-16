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

fn open(h: &mut Harness<'static, SheafApp>) {
    click(h, "Format toolbar: Font Color");
    click(h, "Font Color: Custom RGB...");
}

fn channel(h: &mut Harness<'static, SheafApp>, name: &str, value: &str) {
    h.get_by_role_and_label(egui::accesskit::Role::TextInput, &format!("Graph font color: {name}")).click();
    settle(h);
    ctrl(h, egui::Key::A);
    if value.is_empty() {
        h.key_press(egui::Key::Backspace);
    } else {
        h.event(egui::Event::Text(value.to_string()));
    }
    settle(h);
}

fn seed(h: &Harness<'static, SheafApp>, values: [&str; 3]) {
    for (name, expected) in ["Red", "Green", "Blue"].into_iter().zip(values) {
        assert_eq!(
            h.get_by_role_and_label(egui::accesskit::Role::TextInput, &format!("Graph font color: {name}"))
                .accesskit_node()
                .value()
                .as_deref(),
            Some(expected)
        );
    }
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

fn rgb(h: &mut Harness<'static, SheafApp>, values: [&str; 3]) {
    open(h);
    for (name, value) in ["Red", "Green", "Blue"].into_iter().zip(values) {
        channel(h, name, value);
    }
    click(h, "Apply font color");
    assert!(h.query_by_label("Graph font color: Red").is_none());
    assert!(h.query_by_label("Font Color: Black").is_none());
}

#[test]
fn custom_rgb_targets_only_the_selected_legend_font_and_has_one_undo() {
    let mut h = start_graph("graph-font-rgb-legend");
    let before = project(&h).clone();
    let id = graph(&h).id;
    rgb(&mut h, ["17", "131", "213"]);
    let mut expected = before.clone();
    expected.graph_mut(id).unwrap().layers[0].legend.font.color = Color::rgb(17, 131, 213);
    assert_eq!(project(&h), &expected);
    menu(&mut h, "Edit", "Undo");
    assert_eq!(project(&h), &before);
}

#[test]
fn rgb_boundaries_target_all_four_fonts_and_reach_actual_rendered_text() {
    use sheaf_core::graph::{Font, TextLabel};
    let mut h = start_graph("graph-font-rgb-all");
    let id = graph(&h).id;
    h.state_mut().doc.apply("Seed label", |p| {
        p.graph_mut(id).unwrap().layers[0].labels.push(TextLabel::new(
            r"label \i(a)",
            0.2,
            0.2,
            Font::new("Arial", 18.0),
        ))
    });
    for kind in [ObjectKind::Legend, ObjectKind::XTitle, ObjectKind::YTitle, ObjectKind::Label(0)] {
        for [r, g, b] in [[0, 0, 0], [255, 255, 255], [1, 2, 3], [17, 131, 213]] {
            h.state_mut().graphs.get_mut(&id).unwrap().selected = Some((0, kind));
            settle(&mut h);
            let before = project(&h).clone();
            let revision = h.state().doc.revision();
            let channels = [r.to_string(), g.to_string(), b.to_string()];
            rgb(&mut h, [&channels[0], &channels[1], &channels[2]]);
            let color = Color::rgb(r, g, b);
            let mut expected = before.clone();
            let layer = &mut expected.graph_mut(id).unwrap().layers[0];
            match kind {
                ObjectKind::Legend => layer.legend.font.color = color,
                ObjectKind::XTitle => layer.x_axis.title_font.color = color,
                ObjectKind::YTitle => layer.y_axis.title_font.color = color,
                ObjectKind::Label(0) => layer.labels[0].font.color = color,
                _ => unreachable!(),
            }
            assert_eq!(project(&h), &expected);
            let rendered = texts(&h, kind);
            assert!(!rendered.is_empty() && rendered.iter().all(|(_, c)| *c == color), "{kind:?}: {rendered:?}");
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
fn invalid_channels_and_cancel_are_inert_and_valid_enter_applies() {
    let mut h = start_graph("graph-font-rgb-invalid");
    let before = project(&h).clone();
    let id = graph(&h).id;
    open(&mut h);
    seed(&h, ["0", "0", "0"]);
    for value in ["", "256", "-1", "1.5", "NaN"] {
        channel(&mut h, "Red", value);
        assert!(h.get_by_label("Apply font color").accesskit_node().is_disabled());
        h.key_press(egui::Key::Enter);
        settle(&mut h);
        assert_eq!(project(&h), &before);
        assert!(h.query_by_label("Apply font color").is_some());
    }
    channel(&mut h, "Red", "17");
    channel(&mut h, "Green", "131");
    channel(&mut h, "Blue", "213");
    click(&mut h, "Cancel font color");
    assert_eq!(project(&h), &before);
    assert!(h.query_by_label("Font Color: Black").is_none());
    open(&mut h);
    seed(&h, ["0", "0", "0"]);
    channel(&mut h, "Red", "17");
    channel(&mut h, "Green", "131");
    channel(&mut h, "Blue", "213");
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    let mut expected = before.clone();
    expected.graph_mut(id).unwrap().layers[0].legend.font.color = Color::rgb(17, 131, 213);
    assert_eq!(project(&h), &expected);
    assert!(h.query_by_label("Apply font color").is_none());
}

#[test]
fn reopening_seeds_the_current_font_and_unchanged_accept_has_no_history() {
    let mut h = start_graph("graph-font-rgb-reseed");
    rgb(&mut h, ["17", "131", "213"]);
    let before = project(&h).clone();
    let revision = h.state().doc.revision();
    open(&mut h);
    seed(&h, ["17", "131", "213"]);
    click(&mut h, "Apply font color");
    assert_eq!(project(&h), &before);
    assert_eq!(h.state().doc.revision(), revision);
    click(&mut h, "Format toolbar: Font Color");
    click(&mut h, "Font Color: Red");
    open(&mut h);
    seed(&h, ["255", "0", "0"]);
    click(&mut h, "Cancel font color");
}

#[test]
fn rgb_requests_reject_stale_selection_revision_and_replaced_projects() {
    for scenario in 0..3 {
        let mut h = start_graph(&format!("graph-font-rgb-stale-{scenario}"));
        let id = graph(&h).id;
        open(&mut h);
        channel(&mut h, "Red", "17");
        channel(&mut h, "Green", "131");
        channel(&mut h, "Blue", "213");
        match scenario {
            0 => h.state_mut().graphs.get_mut(&id).unwrap().selected = Some((0, ObjectKind::XTitle)),
            1 => {
                h.state_mut().doc.apply("Other edit", |p| p.graph_mut(id).unwrap().name.push('x'));
            }
            _ => {
                let copy = project(&h).clone();
                h.state_mut().doc.replace(copy);
            }
        }
        settle(&mut h);
        let before = project(&h).clone();
        let revision = h.state().doc.revision();
        if h.query_by_label("Apply font color").is_some() {
            click(&mut h, "Apply font color");
        }
        assert_eq!(project(&h), &before, "scenario{scenario}");
        assert_eq!(h.state().doc.revision(), revision);
    }
}

#[test]
fn custom_rgb_keeps_styles_raw_text_svg_and_project_companion_roundtrips() {
    let mut h = start_graph("graph-font-rgb-save");
    click(&mut h, "Format toolbar: Bold");
    click(&mut h, "Format toolbar: Italic");
    let before = graph(&h).layers[0].legend.clone();
    rgb(&mut h, ["17", "131", "213"]);
    let expected = graph(&h).layers[0].legend.clone();
    assert_eq!(expected.text, before.text);
    assert_eq!(expected.font.family, before.font.family);
    assert_eq!(expected.font.size_pt, before.font.size_pt);
    assert!(expected.font.bold && expected.font.italic);
    let svg = sheaf_plot::render_svg(project(&h), graph(&h).id).unwrap();
    assert!(svg.contains("#1183D5") && svg.contains("font-weight=\"bold\"") && svg.contains("font-style=\"italic\""));
    let dir = save_dir("graph-font-rgb-save");
    std::fs::write(dir.join("rgb.svg"), svg).unwrap();
    for extension in ["sheaf", "opj"] {
        let file = dir.join(format!("rgb.{extension}"));
        h.state_mut().save_to(file.clone());
        let back = start(vec![file]);
        assert_eq!(graph(&back).layers[0].legend, expected, "{extension}");
    }
}

#[test]
fn whole_font_rgb_preserves_explicit_inline_color_precedence() {
    use sheaf_core::graph::{Font, TextLabel};
    let mut h = start_graph("graph-font-rgb-inline");
    let id = graph(&h).id;
    h.state_mut().doc.apply("Seed label", |p| {
        p.graph_mut(id).unwrap().layers[0].labels.push(TextLabel::new(r"a \c2(b)", 0.2, 0.2, Font::new("Arial", 18.0)))
    });
    h.state_mut().graphs.get_mut(&id).unwrap().selected = Some((0, ObjectKind::Label(0)));
    settle(&mut h);
    let before = project(&h).clone();
    rgb(&mut h, ["17", "131", "213"]);
    let mut expected = before.clone();
    expected.graph_mut(id).unwrap().layers[0].labels[0].font.color = Color::rgb(17, 131, 213);
    assert_eq!(project(&h), &expected);
    let rendered = texts(&h, ObjectKind::Label(0));
    assert!(rendered.iter().any(|(s, c)| s == "a " && *c == Color::rgb(17, 131, 213)));
    assert!(rendered.iter().any(|(s, c)| s == "b" && *c == Color::rgb(255, 0, 0)));
}
