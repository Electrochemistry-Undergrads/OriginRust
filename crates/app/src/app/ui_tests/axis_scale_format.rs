//! Axis scale-only clipboard entries use the existing typed format engine and history.
use super::*;
use sheaf_core::{Color, format_clip::AxisPart, graph::Axis, layers::NewLayerKind};

fn axis(g: &sheaf_core::Graph, layer: usize, y: bool) -> &Axis {
    if y { &g.layers[layer].y_axis } else { &g.layers[layer].x_axis }
}

fn axis_mut(g: &mut sheaf_core::Graph, layer: usize, y: bool) -> &mut Axis {
    if y { &mut g.layers[layer].y_axis } else { &mut g.layers[layer].x_axis }
}

fn fixture(tag: &str, layer: usize, y: bool) -> (Harness<'static, SheafApp>, WindowId) {
    let mut h = start(vec![data_file(&format!("axis-scale-format-{tag}-{layer}-{y}"))]);
    show_window(&mut h, "Graph1 - sample");
    let id = project(&h).graphs().next().unwrap().id;
    h.state_mut().doc.apply("source fixture", |p| {
        let g = p.graph_mut(id).unwrap();
        if layer == 1 {
            g.add_layer(NewLayerKind::InsetWithData, 0).unwrap();
        }
        let a = axis_mut(g, layer, y);
        a.min = Some(1.0);
        a.max = Some(3.0);
        a.major_step = Some(0.5);
        a.color = Color::RED;
        a.tick_font.size_pt = 22.0;
        a.line_width_pt = 3.0;
    });
    settle(&mut h);
    (h, id)
}

fn pick(h: &mut Harness<'static, SheafApp>, label: &str) {
    menu_entry(h, label).click();
    settle(h);
}

fn axis_menu(h: &mut Harness<'static, SheafApp>, layer: usize, y: bool) {
    let at = on(h, layer, if y { ObjectKind::YAxis } else { ObjectKind::XAxis });
    right_click_at(h, at);
}

fn exercise(copy: &str, paste: &str) {
    for (layer, y) in [(0, false), (0, true), (1, false), (1, true)] {
        let (mut h, id) = fixture(copy, layer, y);
        let source = project(&h).graph(id).unwrap().clone();
        let revision = h.state().doc.revision();
        let undo = h.state().doc.undo_label().map(str::to_string);
        axis_menu(&mut h, layer, y);
        pick(&mut h, "Copy Format");
        pick(&mut h, copy);
        assert_eq!(project(&h).graph(id).unwrap(), &source);
        assert_eq!(h.state().doc.revision(), revision);
        assert_eq!(h.state().doc.undo_label().map(str::to_string), undo);
        h.state_mut().doc.apply("target fixture", |p| {
            let a = axis_mut(p.graph_mut(id).unwrap(), layer, y);
            a.min = None;
            a.max = None;
            a.major_step = None;
            a.color = Color::BLUE;
            a.tick_font.size_pt = 10.0;
            a.line_width_pt = 1.0;
        });
        settle(&mut h);
        let before = project(&h).graph(id).unwrap().clone();
        let mut expected = before.clone();
        let a = axis_mut(&mut expected, layer, y);
        a.min = Some(1.0);
        a.max = Some(3.0);
        a.major_step = Some(0.5);
        axis_menu(&mut h, layer, y);
        pick(&mut h, "Paste Format To");
        pick(&mut h, paste);
        assert_eq!(project(&h).graph(id).unwrap(), &expected, "only selected axis scale: {layer}/{y}");
        assert_eq!(axis(&expected, layer, y).color, Color::BLUE);
        assert_eq!(h.state().doc.undo_label(), Some("Apply Theme"));
        let revision = h.state().doc.revision();
        axis_menu(&mut h, layer, y);
        pick(&mut h, "Paste Format To");
        pick(&mut h, paste);
        assert_eq!(h.state().doc.revision(), revision, "repeated paste is inert");
        menu(&mut h, "Edit", "Undo Apply Theme");
        assert_eq!(project(&h).graph(id).unwrap(), &before);
        h.key_press_modifiers(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::Z);
        settle(&mut h);
        assert_eq!(project(&h).graph(id).unwrap(), &expected);
    }
}

#[test]
fn copying_axis_scales_then_pasting_all_keeps_target_styles_other_axes_and_layers() {
    exercise("Scales", "All");
}

#[test]
fn pasting_axis_scales_from_an_all_clip_keeps_target_styles_other_axes_and_layers() {
    exercise("All", "Scales");
}

#[test]
fn scale_paste_from_a_style_only_clip_and_stale_targets_change_no_history() {
    let (mut h, id) = fixture("no-op", 0, false);
    assert!(crate::format_clip_ui::copy_axis(&h.state().doc, id, 0, false, AxisPart::Styles));
    let before = project(&h).graph(id).unwrap().clone();
    let revision = h.state().doc.revision();
    let undo = h.state().doc.undo_label().map(str::to_string);
    axis_menu(&mut h, 0, false);
    pick(&mut h, "Paste Format To");
    pick(&mut h, "Scales");
    assert_eq!(h.state().doc.revision(), revision);
    assert_eq!(h.state().doc.undo_label().map(str::to_string), undo);
    for (graph, layer) in [(id, usize::MAX), (WindowId(u64::MAX), 0)] {
        let ctx = h.ctx.clone();
        h.state_mut().apply(Action::PasteAxisFormat { graph, layer, y: false, part: AxisPart::Scales }, &ctx);
        settle(&mut h);
        assert_eq!(project(&h).graph(id).unwrap(), &before);
        assert_eq!(h.state().doc.revision(), revision);
        assert_eq!(h.state().doc.undo_label().map(str::to_string), undo);
    }
}
