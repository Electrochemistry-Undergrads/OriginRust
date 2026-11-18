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
