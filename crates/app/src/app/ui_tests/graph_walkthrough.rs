//! Graph UI walkthrough coverage (T217): a graph made from a brand-new table and the usual first
//! edits on it, each with Undo.

use super::*;
use crate::actions::GraphOp;
use sheaf_core::graph::PlotKind;

#[test]
fn a_graph_of_typed_data_takes_the_usual_edits_and_undoes_them() {
    let mut h = cell_regression_table(false, "graph-walk");
    for (col, values) in [(0, ["1", "2", "3"]), (1, ["4", "5", "6"])] {
        let at = worksheet_cell_pos(&h, col, 0);
        click_at(&mut h, at);
        for v in values {
            h.event(egui::Event::Text(v.into()));
            settle(&mut h);
            key(&mut h, egui::Key::Enter);
        }
    }
    let (a, b) = (column_id(&h, "A"), column_id(&h, "B"));
    let head = grid::head_name(sheet(&h), 1);
    click_text(&mut h, &head);
    menu_path(&mut h, &["Plot", "Line", "Line"]);
    let g = project(&h).graphs().last().expect("a graph").id;
    let plot = |h: &Harness<'static, SheafApp>| project(h).graph(g).unwrap().layers[0].plots[0].clone();
    assert_eq!((plot(&h).x, plot(&h).y, plot(&h).kind), (Some(a), b, PlotKind::Line));
    let ctx = h.ctx.clone();
    let edit = |h: &mut Harness<'static, SheafApp>, op: GraphOp| {
        h.state_mut().apply(Action::GraphEdit { graph: g, op }, &ctx);
        settle(h);
    };
    let undo = |h: &mut Harness<'static, SheafApp>| {
        let ctx = h.ctx.clone();
        h.state_mut().apply(Action::Undo, &ctx);
        settle(h);
    };

    // Change Plot to > Scatter, and back with Undo.
    edit(&mut h, GraphOp::SetPlotKind { layer: 0, plot: 0, kind: PlotKind::Scatter });
    assert_eq!(plot(&h).kind, PlotKind::Scatter);
    undo(&mut h);
    assert_eq!(plot(&h).kind, PlotKind::Line);

    // New Layer (Right-Y), and back.
    edit(&mut h, GraphOp::AddLayer { layer: 0, kind: sheaf_core::layers::NewLayerKind::RightY });
    assert_eq!(project(&h).graph(g).unwrap().layers.len(), 2);
    undo(&mut h);
    assert_eq!(project(&h).graph(g).unwrap().layers.len(), 1);

    // Fixed limits, then Rescale (Ctrl+R) goes back to automatic ones; Undo restores the limits.
    h.state_mut().doc.apply("Limits", |p| {
        let x = &mut p.graph_mut(g).unwrap().layers[0].x_axis;
        (x.min, x.max) = (Some(-10.0), Some(10.0));
    });
    settle(&mut h);
    edit(&mut h, GraphOp::Rescale { layer: 0 });
    let x = project(&h).graph(g).unwrap().layers[0].x_axis.clone();
    assert_eq!((x.min, x.max), (None, None));
    undo(&mut h);
    let x = project(&h).graph(g).unwrap().layers[0].x_axis.clone();
    assert_eq!((x.min, x.max), (Some(-10.0), Some(10.0)));

    // A value typed in the table afterwards reaches the graph's data.
    let title = project(&h).workbooks().next().unwrap().title();
    show_window(&mut h, &title);
    let at = worksheet_cell_pos(&h, 1, 3);
    click_at(&mut h, at);
    h.event(egui::Event::Text("7".into()));
    settle(&mut h);
    key(&mut h, egui::Key::Enter);
    assert_eq!(project(&h).numeric(plot(&h).y).unwrap(), [4.0, 5.0, 6.0, 7.0]);
    assert!(sheaf_plot::render_svg(project(&h), g).is_ok(), "the graph still draws");
}
