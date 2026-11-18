//! Data > Mark Data Range, Set Display Range, Reset to Full Range and Clear Data Markers (T118, graphing
//! spec "Data markers and display range").

use super::*;
use crate::actions::{DataTool, GraphOp, GraphTool, MarkerOp};

#[test]
fn marking_then_setting_the_display_range_restricts_the_plot() {
    let mut h = start(vec![data_file("data-markers")]);
    show_window(&mut h, "Book1 - sample.csv");
    let book = h.state().books.keys().next().copied().unwrap();
    let ctx = h.ctx.clone();
    let cols: Vec<_> = project(&h).workbook(book).unwrap().sheet().columns.iter().map(|c| c.id).collect();
    h.state_mut().apply(
        Action::PlotColumns { book, columns: vec![cols[1]], template: sheaf_core::template::Template::Line },
        &ctx,
    );
    settle(&mut h);
    let graph = project(&h).graphs().last().unwrap().id;
    // Mark Data Range: markers on the first and last rows, and the marker tool is on.
    h.state_mut().apply(Action::GraphTool { graph, tool: GraphTool::Data(DataTool::MarkRange) }, &ctx);
    settle(&mut h);
    let plot = |h: &Harness<'static, SheafApp>| project(h).graph(graph).unwrap().layers[0].plots[0].clone();
    assert_eq!(plot(&h).decor.markers, Some((0, 3)));
    // A marker moved to row 1, then Set Display Range: rows 1..=3.
    let edit = |op| Action::GraphEdit { graph, op: GraphOp::DataMarkers { layer: 0, op } };
    h.state_mut().apply(edit(MarkerOp::Move { plot: 0, a: 1, b: 3 }), &ctx);
    h.state_mut().apply(edit(MarkerOp::SetRange), &ctx);
    settle(&mut h);
    assert_eq!((plot(&h).rows, plot(&h).decor.markers), (Some((1, 4)), None));
    h.state_mut().apply(edit(MarkerOp::ResetRange), &ctx);
    settle(&mut h);
    assert_eq!(plot(&h).rows, None);
    h.state_mut().apply(edit(MarkerOp::Mark { plot: 0 }), &ctx);
    h.state_mut().apply(edit(MarkerOp::Clear), &ctx);
    settle(&mut h);
    assert_eq!(plot(&h).decor.markers, None);
}
