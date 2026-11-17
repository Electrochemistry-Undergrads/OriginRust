//! Graph > Add Function Graph... (T119, graphing spec "Add Function Graph").

use super::*;
use sheaf_core::graph::PlotKind;

#[test]
fn the_function_curve_goes_into_the_active_layer() {
    let mut h = start(vec![data_file("function-graph")]);
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
    h.state_mut().apply(Action::AddFunctionGraph { graph, layer: 0 }, &ctx);
    settle(&mut h);
    assert!(h.query_by_label("y(x) =").is_some());
    h.state_mut().function_graph.as_mut().unwrap().expr = "2*x".into();
    h.get_by_label("OK").click();
    settle(&mut h);
    assert!(h.state().function_graph.is_none());
    let plots = &project(&h).graph(graph).unwrap().layers[0].plots;
    assert_eq!(plots.len(), 2);
    assert_eq!(plots[1].kind, PlotKind::Line);
    let ys = project(&h).numeric(plots[1].y).unwrap().to_vec();
    assert_eq!((ys.len(), ys[ys.len() - 1]), (100, 20.0));
}
