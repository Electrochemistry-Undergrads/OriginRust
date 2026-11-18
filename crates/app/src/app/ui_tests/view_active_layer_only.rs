//! View's documented alias reuses the existing layer-visibility engine.
use super::*;
use sheaf_core::layers::NewLayerKind;

fn setup(tag: &str, multiple: bool) -> Harness<'static, SheafApp> {
    let mut h = start(vec![data_file(tag)]);
    show_window(&mut h, "Graph1 - sample");
    let id = graph(&h).id;
    if multiple {
        h.state_mut().doc.apply("fixture", |p| {
            let g = p.graph_mut(id).unwrap();
            let right = g.add_layer(NewLayerKind::RightY, 0).unwrap();
            g.layers[right].plots = g.layers[0].plots.clone();
            g.add_layer(NewLayerKind::InsetWithData, 0).unwrap();
            g.layers[0].plots[1].visible = false;
            g.layers[right].display.show_y_axes = false;
            g.layers[2].display.clip_data = false;
        });
        settle(&mut h);
    }
    h
}
fn choose(h: &mut Harness<'static, SheafApp>) {
    menu_path(h, &["View", "Show", "Active Layer Only"]);
}

#[test]
fn view_toggle_targets_the_active_layer_preserves_settings_and_shares_one_step_history() {
    for target in 0..3 {
        let mut h = setup(&format!("view-active-only-{target}"), true);
        let id = graph(&h).id;
        h.state_mut().graphs.get_mut(&id).unwrap().active_layer = target;
        settle(&mut h);
        assert_eq!(h.state().menu_context().active_layer_only, Some(false));
        let before = project(&h).clone();
        let original = graph(&h).clone();
        let geometry =
            sheaf_plot::layout_geometry(project(&h), id).unwrap().iter().map(|l| l.frame).collect::<Vec<_>>();
        choose(&mut h);
        let mut expected = before.clone();
        for (i, l) in expected.graph_mut(id).unwrap().layers.iter_mut().enumerate() {
            l.display.hidden = i != target;
        }
        assert_eq!(project(&h), &expected);
        assert_eq!(h.state().menu_context().active_layer_only, Some(true));
        assert_eq!(h.state().doc.undo_label(), Some("Hide other layers"));
        assert_eq!(
            sheaf_plot::layout_geometry(project(&h), id).unwrap().iter().map(|l| l.frame).collect::<Vec<_>>(),
            geometry
        );
        let scene = sheaf_plot::graph_scene(project(&h), id).unwrap();
        assert!(scene.layers.iter().flat_map(|l| &l.objects).all(|o| o.layer == target));
        menu(&mut h, "Edit", "Undo");
        assert_eq!(project(&h), &before);
        h.key_press_modifiers(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::Z);
        settle(&mut h);
        assert_eq!(project(&h), &expected);
        choose(&mut h);
        assert_eq!(graph(&h), &original);
        assert_eq!(h.state().menu_context().active_layer_only, Some(false));
    }
}

#[test]
fn shortcut_and_icon_menu_share_hidden_target_and_show_all_behavior_and_saves() {
    let mut h = setup("view-active-only-key", true);
    let id = graph(&h).id;
    h.state_mut().doc.apply("fixture hidden", |p| {
        for l in &mut p.graph_mut(id).unwrap().layers {
            l.display.hidden = true;
        }
    });
    h.state_mut().graphs.get_mut(&id).unwrap().active_layer = 1;
    settle(&mut h);
    assert_eq!(h.state().menu_context().active_layer_only, Some(false));
    h.key_press_modifiers(egui::Modifiers::COMMAND | egui::Modifiers::ALT, egui::Key::D);
    settle(&mut h);
    assert_eq!(graph(&h).layers.iter().map(|l| l.display.hidden).collect::<Vec<_>>(), [true, false, true]);
    let expected = graph(&h).clone();
    let at = graph_ui(&h).page_point(0., 0.).unwrap() + egui::vec2(26., 9.);
    right_click_at(&mut h, at);
    assert!(h.query_by_label("\u{2713} Hide Other Layers").is_some());
    click(&mut h, "\u{2713} Hide Other Layers");
    assert!(graph(&h).layers.iter().all(|l| !l.display.hidden));
    assert_eq!(h.state().menu_context().active_layer_only, Some(false));
    choose(&mut h);
    assert_eq!(graph(&h), &expected);
    let dir = save_dir("view-active-only-save");
    for ext in ["sheaf", "opj"] {
        let path = dir.join(format!("isolated.{ext}"));
        let saved = sheaf_io::save::save_project(project(&h), &path).unwrap();
        if ext == "opj" {
            assert!(saved.companion.is_some());
            assert!(saved.warnings.iter().any(|w| w.contains("hidden layer visibility was not written")));
        }
        assert_eq!(sheaf_io::open_project(&path).unwrap().project.graph(id), Some(&expected));
    }
}

#[test]
fn single_layer_stale_layer_and_workbook_shortcuts_do_not_edit_or_add_history() {
    let mut h = setup("view-active-only-noop", false);
    let id = graph(&h).id;
    let before = project(&h).clone();
    let revision = h.state().doc.revision();
    menu_path(&mut h, &["View", "Show"]);
    assert!(h.get_by_label_contains("Active Layer Only").accesskit_node().is_disabled());
    close_menus(&mut h);
    h.key_press_modifiers(egui::Modifiers::COMMAND | egui::Modifiers::ALT, egui::Key::D);
    settle(&mut h);
    assert_eq!(project(&h), &before);
    assert_eq!(h.state().doc.revision(), revision);
    h.state_mut().graphs.get_mut(&id).unwrap().active_layer = usize::MAX;
    settle(&mut h);
    assert_eq!(h.state().menu_context().active_layer_only, None);
    h.key_press_modifiers(egui::Modifiers::COMMAND | egui::Modifiers::ALT, egui::Key::D);
    settle(&mut h);
    assert_eq!(project(&h), &before);
    assert_eq!(h.state().doc.revision(), revision);
    show_window(&mut h, "Book1 - sample.csv");
    let before = project(&h).clone();
    let revision = h.state().doc.revision();
    h.key_press_modifiers(egui::Modifiers::COMMAND | egui::Modifiers::ALT, egui::Key::D);
    settle(&mut h);
    assert_eq!(project(&h), &before);
    assert_eq!(h.state().doc.revision(), revision);
}
