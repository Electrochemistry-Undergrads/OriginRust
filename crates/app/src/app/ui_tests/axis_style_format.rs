//! Axis Styles context entries reuse the published typed style group without scales.
use super::*;
use sheaf_core::{
    Color,
    format_clip::AxisPart,
    graph::{Axis, Font, TickDirection},
    layers::NewLayerKind,
};

fn axis(g: &sheaf_core::Graph, layer: usize, y: bool) -> &Axis {
    if y { &g.layers[layer].y_axis } else { &g.layers[layer].x_axis }
}
fn axis_mut(g: &mut sheaf_core::Graph, layer: usize, y: bool) -> &mut Axis {
    if y { &mut g.layers[layer].y_axis } else { &mut g.layers[layer].x_axis }
}
fn fixture(tag: &str, layer: usize, y: bool) -> (Harness<'static, SheafApp>, WindowId) {
    let mut h = start(vec![data_file(&format!("axis-styles-{tag}-{layer}-{y}"))]);
    show_window(&mut h, "Graph1 - sample");
    let id = graph(&h).id;
    h.state_mut().doc.apply("source", |p| {
        let g = p.graph_mut(id).unwrap();
        if layer == 1 {
            g.add_layer(NewLayerKind::InsetWithData, 0).unwrap();
        }
        let a = axis_mut(g, layer, y);
        a.min = Some(1.);
        a.max = Some(3.);
        a.major_step = Some(0.5);
        a.minor_ticks = 7;
        a.title = Some("Source title must not be pasted".into());
        a.color = Color::RED;
        a.color_follows_plot = false;
        a.title_color_follows_plot = false;
        a.tick_color = Some(Color::rgb(0, 255, 255));
        a.line_width_pt = 3.5;
        a.tick_width_pt = Some(2.5);
        a.tick_font = Font::new("Courier New", 20.);
        a.tick_font.bold = true;
        a.tick_font.color = Color::BLUE;
        a.title_font = Font::new("Times New Roman", 18.);
        a.title_font.italic = true;
        a.title_font.color = Color::rgb(0, 255, 0);
        a.major_tick = TickDirection::In;
        a.minor_tick = TickDirection::Both;
        a.major_tick_len_pt = 7.;
        a.minor_tick_len_pt = 3.;
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
        let source = axis(graph(&h), layer, y).clone();
        let before = project(&h).clone();
        let revision = h.state().doc.revision();
        let undo = h.state().doc.undo_label().map(str::to_owned);
        axis_menu(&mut h, layer, y);
        pick(&mut h, "Copy Format");
        pick(&mut h, copy);
        assert_eq!(project(&h), &before);
        assert_eq!(h.state().doc.revision(), revision);
        assert_eq!(h.state().doc.undo_label(), undo.as_deref());
        if copy == "Styles" {
            menu(&mut h, "Edit", "Paste Format...");
            assert!(h.get_by_label("Scales").accesskit_node().is_disabled());
            assert!(!h.get_by_label("Styles").accesskit_node().is_disabled());
            click(&mut h, "Close");
        }
        h.state_mut().doc.apply("target", |p| {
            let a = axis_mut(p.graph_mut(id).unwrap(), layer, y);
            a.min = Some(0.);
            a.max = Some(10.);
            a.major_step = Some(2.);
            a.minor_ticks = 3;
            a.title = Some("Target title stays".into());
            a.color = Color::rgb(0, 255, 0);
            a.color_follows_plot = true;
            a.title_color_follows_plot = true;
            a.tick_color = None;
            a.line_width_pt = 1.;
            a.tick_width_pt = None;
            a.tick_font = Font::new("Arial", 10.);
            a.title_font = Font::new("Arial", 12.);
            a.major_tick = TickDirection::Out;
            a.minor_tick = TickDirection::Out;
            a.major_tick_len_pt = 9.;
            a.minor_tick_len_pt = 4.5;
        });
        settle(&mut h);
        let before = project(&h).clone();
        let mut expected = before.clone();
        let a = axis_mut(expected.graph_mut(id).unwrap(), layer, y);
        a.color = source.color;
        a.color_follows_plot = source.color_follows_plot;
        a.title_color_follows_plot = source.title_color_follows_plot;
        a.tick_color = source.tick_color;
        a.line_width_pt = source.line_width_pt;
        a.tick_width_pt = source.tick_width_pt;
        a.tick_font = source.tick_font.clone();
        a.title_font = source.title_font.clone();
        a.major_tick = source.major_tick;
        a.minor_tick = source.minor_tick;
        a.major_tick_len_pt = source.major_tick_len_pt;
        a.minor_tick_len_pt = source.minor_tick_len_pt;
        axis_menu(&mut h, layer, y);
        pick(&mut h, "Paste Format To");
        pick(&mut h, paste);
        assert_eq!(project(&h), &expected, "only existing style whitelist: {layer}/{y}");
        let a = axis(graph(&h), layer, y);
        assert_eq!((a.min, a.max, a.major_step, a.minor_ticks), (Some(0.), Some(10.), Some(2.), 3));
        assert_eq!(a.title.as_deref(), Some("Target title stays"));
        assert_eq!(h.state().doc.undo_label(), Some("Apply Theme"));
        let revision = h.state().doc.revision();
        axis_menu(&mut h, layer, y);
        pick(&mut h, "Paste Format To");
        pick(&mut h, paste);
        assert_eq!(h.state().doc.revision(), revision);
        menu(&mut h, "Edit", "Undo Apply Theme");
        assert_eq!(project(&h), &before);
        h.key_press_modifiers(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::Z);
        settle(&mut h);
        assert_eq!(project(&h), &expected);
        assert!(sheaf_plot::render_svg(project(&h), id).unwrap().contains("#FF0000"));
    }
}
#[test]
fn styles_copy_then_all_paste_keeps_target_scales_text_other_axes_and_layers() {
    exercise("Styles", "All");
}
#[test]
fn styles_paste_from_all_ignores_held_scales_and_has_one_undo() {
    exercise("All", "Styles");
}
#[test]
fn style_paste_from_scales_only_or_stale_targets_has_no_effect_or_history() {
    let (mut h, id) = fixture("no-op", 0, false);
    assert!(crate::format_clip_ui::copy_axis(&h.state().doc, id, 0, false, AxisPart::Scales));
    h.state_mut().doc.apply("different target styles", |p| {
        let a = axis_mut(p.graph_mut(id).unwrap(), 0, false);
        a.color = Color::BLUE;
        a.tick_font.size_pt = 11.0;
        a.line_width_pt = 1.0;
    });
    settle(&mut h);
    let before = project(&h).clone();
    let revision = h.state().doc.revision();
    let undo = h.state().doc.undo_label().map(str::to_owned);
    axis_menu(&mut h, 0, false);
    pick(&mut h, "Paste Format To");
    pick(&mut h, "Styles");
    assert_eq!(project(&h), &before);
    assert_eq!(h.state().doc.revision(), revision);
    assert_eq!(h.state().doc.undo_label(), undo.as_deref());
    for (graph, layer) in [(id, usize::MAX), (WindowId(u64::MAX), 0)] {
        let ctx = h.ctx.clone();
        h.state_mut().apply(Action::PasteAxisFormat { graph, layer, y: false, part: AxisPart::Styles }, &ctx);
        settle(&mut h);
        assert_eq!(project(&h), &before);
        assert_eq!(h.state().doc.revision(), revision);
        assert_eq!(h.state().doc.undo_label(), undo.as_deref());
    }
}
