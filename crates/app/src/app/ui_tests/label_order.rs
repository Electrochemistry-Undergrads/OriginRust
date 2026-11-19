//! A text label's Bring to the front / Push to the back (T137, graphing "Text label order").

use super::*;
use crate::actions::GraphOp;
use sheaf_plot::scene::ObjectKind;

#[test]
fn labels_move_to_the_top_or_bottom_and_stay_selected() {
    let mut h = start(vec![data_file("label-order")]);
    let ctx = h.ctx.clone();
    let graph = project(&h).graphs().next().unwrap().id;
    h.state_mut().doc.apply("Labels", |p| {
        let l = &mut p.graph_mut(graph).unwrap().layers[0];
        let font = l.new_label_font();
        for t in ["first", "second", "third"] {
            l.labels.push(sheaf_core::graph::TextLabel::new(t, 0.3, 0.3, font.clone()));
        }
    });
    let n = project(&h).graph(graph).unwrap().layers[0].labels.len();
    let texts = |h: &Harness<'static, SheafApp>| -> Vec<String> {
        project(h).graph(graph).unwrap().layers[0].labels.iter().map(|l| l.text.clone()).collect()
    };
    h.state_mut().graphs.get_mut(&graph).unwrap().selected = Some((0, ObjectKind::Label(n - 3)));
    h.state_mut()
        .apply(Action::GraphEdit { graph, op: GraphOp::OrderLabel { layer: 0, label: n - 3, front: true } }, &ctx);
    settle(&mut h);
    assert_eq!(texts(&h)[n - 3..], ["second", "third", "first"]);
    assert_eq!(h.state().graphs[&graph].selected, Some((0, ObjectKind::Label(n - 1))), "still the moved label");
    h.state_mut()
        .apply(Action::GraphEdit { graph, op: GraphOp::OrderLabel { layer: 0, label: n - 1, front: false } }, &ctx);
    settle(&mut h);
    assert_eq!(texts(&h)[0], "first");
    assert_eq!(h.state().graphs[&graph].selected, Some((0, ObjectKind::Label(0))));
    h.state_mut().doc.undo();
    assert_eq!(texts(&h)[n - 1], "first", "one undo step");
}

#[test]
fn the_legend_menu_orders_the_legend_and_puts_it_behind_the_data() {
    // T152: the legend's Bring to the front / Push to the back and front / back of data.
    let menu = menus::graph_legend_menu(0, false, false);
    for (label, op) in [
        ("Bring to the front", GraphOp::OrderLegend { layer: 0, front: true }),
        ("Push to the back", GraphOp::OrderLegend { layer: 0, front: false }),
        ("Bring to front of data", GraphOp::LegendBehindData { layer: 0, behind: false }),
        ("Push to back of data", GraphOp::LegendBehindData { layer: 0, behind: true }),
    ] {
        let found = menu.iter().any(|e| {
            matches!(e, menus::Entry::Item(i) if menus::plain(&i.label) == label && i.cmd == Some(menus::Cmd::GraphEdit(op)))
        });
        assert!(found, "{label} is live");
    }
    let mut h = start(vec![data_file("legend-order")]);
    let ctx = h.ctx.clone();
    let graph = project(&h).graphs().next().unwrap().id;
    let legend = |h: &Harness<'static, SheafApp>| project(h).graph(graph).unwrap().layers[0].legend.clone();
    h.state_mut().apply(Action::GraphEdit { graph, op: GraphOp::LegendBehindData { layer: 0, behind: true } }, &ctx);
    settle(&mut h);
    assert!(legend(&h).behind_data);
    assert_eq!(h.state().doc.undo_label(), Some("Push legend to back of data"));
    h.state_mut().apply(Action::GraphEdit { graph, op: GraphOp::OrderLegend { layer: 0, front: true } }, &ctx);
    settle(&mut h);
    assert!(legend(&h).on_top);
    assert_eq!(h.state().doc.undo_label(), Some("Bring legend to front"));
    h.state_mut().doc.undo();
    h.state_mut().doc.undo();
    assert!(!legend(&h).on_top && !legend(&h).behind_data, "one undo step each");
}
