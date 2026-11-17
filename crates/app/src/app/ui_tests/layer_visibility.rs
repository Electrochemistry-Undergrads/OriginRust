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
    let at = graph_ui(h).page_point(0., 0.).unwrap() + egui::vec2(9., 9.);
    right_click_at(h, at);
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
