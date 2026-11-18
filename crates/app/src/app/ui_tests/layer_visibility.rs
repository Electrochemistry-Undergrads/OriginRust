//! Hide Layer's icon menu, restoration, readers, history and saved project behavior (T61).
use super::*;
use crate::reader;

fn setup(tag: &str) -> (Harness<'static, SheafApp>, WindowId) {
    let mut h = start(vec![data_file(&format!("layer-visibility-{tag}"))]);
    show_window(&mut h, "Graph1 - sample");
    let id = project(&h).graphs().next().unwrap().id;
    (h, id)
}

fn icon_menu(h: &mut Harness<'static, SheafApp>) {
    icon_menu_at(h, 0);
}

fn icon_menu_at(h: &mut Harness<'static, SheafApp>, layer: usize) {
    let at = graph_ui(h).page_point(0., 0.).unwrap() + egui::vec2(9. + layer as f32 * 17., 9.);
    right_click_at(h, at);
}

fn multi_layer(tag: &str) -> (Harness<'static, SheafApp>, WindowId) {
    let (mut h, id) = setup(tag);
    h.state_mut().doc.apply("fixture", |p| {
        use sheaf_core::{graph::LegendScope, layers::NewLayerKind};
        let g = p.graph_mut(id).unwrap();
        let right = g.add_layer(NewLayerKind::RightY, 0).unwrap();
        g.layers[right].plots = g.layers[0].plots.clone();
        g.add_layer(NewLayerKind::InsetWithData, 0).unwrap();
        g.layers[0].plots[1].visible = false;
        g.layers[right].plots[0].visible = false;
        g.layers[0].display.show_y_axes = false;
        g.layers[right].display.clip_data = false;
        g.layers[2].display.hidden = true;
        for (i, l) in g.layers.iter_mut().enumerate() {
            l.legend.scope = LegendScope::Layer;
            l.legend.visible = true;
            l.legend.text = Some(format!("Owned legend {}", i + 1));
        }
    });
    settle(&mut h);
    (h, id)
}

#[test]
fn hide_other_layers_toggles_every_layer_element_and_restores_all_without_changing_settings() {
    for (target, isolated) in [(0, [false, true, true]), (1, [true, false, true]), (2, [true, true, false])] {
        let (mut h, id) = multi_layer(&format!("others-{target}"));
        let original = project(&h).graph(id).unwrap().clone();
        let geometry = |h: &Harness<'static, SheafApp>| {
            sheaf_plot::layout_geometry(project(h), id)
                .unwrap()
                .iter()
                .map(|g| (g.frame, g.x.min, g.x.max, g.y.min, g.y.max))
                .collect::<Vec<_>>()
        };
        let original_geometry = geometry(&h);
        icon_menu_at(&mut h, target);
        click(&mut h, "Hide Other Layers");
        let mut expected = original.clone();
        for (l, hidden) in expected.layers.iter_mut().zip(isolated) {
            l.display.hidden = hidden;
        }
        assert_eq!(project(&h).graph(id).unwrap(), &expected);
        assert_eq!(h.state().doc.undo_label(), Some("Hide other layers"));
        assert_eq!(geometry(&h), original_geometry);
        let scene = sheaf_plot::graph_scene(project(&h), id).unwrap();
        assert!(scene.layers.iter().flat_map(|l| &l.objects).all(|o| o.layer == target));
        assert!(scene.layers.iter().flat_map(|l| &l.objects).any(|o| o.kind == ObjectKind::Legend));
        menu(&mut h, "Edit", "Undo Hide other layers");
        assert_eq!(project(&h).graph(id).unwrap(), &original);
        h.key_press_modifiers(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::Z);
        settle(&mut h);
        assert_eq!(project(&h).graph(id).unwrap(), &expected);
        icon_menu_at(&mut h, target);
        click(&mut h, "\u{2713} Hide Other Layers");
        for l in &mut expected.layers {
            l.display.hidden = false;
        }
        assert_eq!(project(&h).graph(id).unwrap(), &expected);
        assert_eq!(geometry(&h), original_geometry);
        assert!(sheaf_plot::graph_scene(project(&h), id).unwrap().layers.iter().all(|l| !l.objects.is_empty()));
        menu(&mut h, "Edit", "Undo Hide other layers");
        assert_eq!(
            project(&h).graph(id).unwrap().layers.iter().map(|l| l.display.hidden).collect::<Vec<_>>(),
            isolated
        );
    }
}

#[test]
fn hide_other_layers_is_unchecked_when_every_layer_is_hidden_and_shows_the_requested_layer() {
    let (mut h, id) = multi_layer("all-hidden");
    h.state_mut().doc.apply("fixture", |p| {
        for l in &mut p.graph_mut(id).unwrap().layers {
            l.display.hidden = true;
        }
    });
    settle(&mut h);
    icon_menu_at(&mut h, 1);
    assert!(h.query_by_label("\u{2713} Hide Layer").is_some());
    assert!(h.query_by_label("\u{2713} Hide Other Layers").is_none());
    click(&mut h, "Hide Other Layers");
    assert_eq!(
        project(&h).graph(id).unwrap().layers.iter().map(|l| l.display.hidden).collect::<Vec<_>>(),
        [true, false, true]
    );
    icon_menu_at(&mut h, 1);
    click(&mut h, "\u{2713} Hide Other Layers");
    assert!(project(&h).graph(id).unwrap().layers.iter().all(|l| !l.display.hidden));
}

#[test]
fn hide_other_layers_refuses_single_layer_and_stale_targets_without_history_or_revision_changes() {
    let (mut h, id) = setup("others-noop");
    for hidden in [false, true] {
        h.state_mut().doc.apply("fixture", |p| p.graph_mut(id).unwrap().layers[0].display.hidden = hidden);
        let original = project(&h).graph(id).unwrap().clone();
        for (graph, layer) in [(id, 0), (id, 1), (id, usize::MAX), (WindowId(u64::MAX), 0)] {
            let before = h.state().doc.revision();
            let undo = h.state().doc.undo_label().map(str::to_string);
            let ctx = h.ctx.clone();
            h.state_mut().apply(Action::GraphEdit { graph, op: GraphOp::HideOtherLayers { layer } }, &ctx);
            settle(&mut h);
            assert_eq!(h.state().doc.revision(), before);
            assert_eq!(h.state().doc.undo_label().map(str::to_string), undo);
            assert_eq!(project(&h).graph(id).unwrap(), &original);
        }
    }
}

#[test]
fn hide_layer_is_a_checked_toggle_and_restores_individually_hidden_plots_and_axes() {
    let (mut h, id) = setup("toggle");
    h.state_mut().doc.apply("fixture", |p| {
        let l = &mut p.graph_mut(id).unwrap().layers[0];
        l.plots[1].visible = false;
        l.display.show_y_axes = false;
    });
    settle(&mut h);
    let original = project(&h).graph(id).unwrap().clone();
    let before = sheaf_plot::render_svg(project(&h), id).unwrap();
    icon_menu(&mut h);
    click(&mut h, "Hide Layer");
    assert!(project(&h).graph(id).unwrap().layers[0].display.hidden);
    assert_eq!(h.state().doc.undo_label(), Some("Hide layer"));
    assert!(!project(&h).graph(id).unwrap().layers[0].plots[1].visible);
    assert!(!project(&h).graph(id).unwrap().layers[0].display.show_y_axes);
    assert!(graph_ui(&h).point_on(0, ObjectKind::XAxis).is_none());
    icon_menu(&mut h);
    click(&mut h, "\u{2713} Hide Layer");
    assert_eq!(project(&h).graph(id).unwrap(), &original);
    assert_eq!(sheaf_plot::render_svg(project(&h), id).unwrap(), before);
    menu(&mut h, "Edit", "Undo Hide layer");
    assert!(project(&h).graph(id).unwrap().layers[0].display.hidden);
    h.key_press_modifiers(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::Z);
    settle(&mut h);
    assert_eq!(project(&h).graph(id).unwrap(), &original);
}

#[test]
fn reader_and_data_edit_tools_cannot_pick_or_draw_points_of_a_hidden_layer() {
    let (mut h, id) = setup("reader");
    let point = reader::Point { layer: 0, plot: 0, row: 1 };
    let geometry = sheaf_plot::layout_geometry(project(&h), id).unwrap();
    let screen = |(x, y): (f64, f64)| egui::pos2(x as f32, y as f32);
    let (at, _, _) = reader::place(project(&h), id, &geometry, &screen, point).unwrap();
    assert!(reader::nearest(project(&h), id, &geometry, &screen, at).is_some());
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::GraphEdit { graph: id, op: GraphOp::HideLayer { layer: 0 } }, &ctx);
    settle(&mut h);
    assert!(reader::place(project(&h), id, &geometry, &screen, point).is_none());
    assert!(reader::nearest(project(&h), id, &geometry, &screen, at).is_none());
    assert!(reader::nearest_of(project(&h), id, &geometry, &screen, reader::Pick::Edit, at).is_none());
}

#[test]
fn hidden_layers_survive_sheaf_and_opj_companion_save_reopen_with_an_explicit_native_warning() {
    let (mut h, id) = setup("save");
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::GraphEdit { graph: id, op: GraphOp::HideLayer { layer: 0 } }, &ctx);
    let dir = std::env::temp_dir().join(format!("sheaf-layer-visibility-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for ext in ["sheaf", "opj"] {
        let path = dir.join(format!("hidden.{ext}"));
        let saved = sheaf_io::save::save_project(project(&h), &path).unwrap();
        if ext == "opj" {
            assert!(saved.companion.is_some());
            assert!(saved.warnings.iter().any(|w| w.contains("hidden layer visibility was not written")));
        }
        let back = sheaf_io::open_project(&path).unwrap();
        assert_eq!(back.companion_used, ext == "opj");
        assert_eq!(back.project.graph(id), project(&h).graph(id));
    }
    std::fs::remove_dir_all(dir).unwrap();
}
