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
    // Origin's Auto From/To (T322): the window starts over the range the layer's X axis shows.
    let geom = sheaf_plot::layout_geometry(project(&h), graph).unwrap()[0].clone();
    let (from, to) = (geom.x.min, geom.x.max);
    {
        let d = h.state_mut().function_graph.as_mut().unwrap();
        assert_eq!((d.from, d.to, d.points), (from, to, 100));
        d.expr = "2*x".into();
        d.kind = PlotKind::LineSymbol;
    }
    h.get_by_label("OK").click();
    settle(&mut h);
    assert!(h.state().function_graph.is_none());
    let plots = &project(&h).graph(graph).unwrap().layers[0].plots;
    assert_eq!(plots.len(), 2);
    assert_eq!(plots[1].kind, PlotKind::LineSymbol, "the Plot Type chosen");
    let ys = project(&h).numeric(plots[1].y).unwrap().to_vec();
    assert_eq!(ys.len(), 100);
    assert!((ys[ys.len() - 1] - 2.0 * to).abs() < 1e-9);
}

#[test]
fn file_new_function_plot_makes_its_graph_on_ok_in_one_undo_step() {
    let mut h = start(Vec::new());
    menu_path(&mut h, &["File", "New", "Function Plot"]);
    assert!(h.query_by_label("y(x) =").is_some(), "the function window is open");
    assert_eq!(project(&h).graphs().count(), 0, "no graph before OK");
    // Cancel makes nothing.
    h.get_by_label("Cancel").click();
    settle(&mut h);
    assert!(h.state().function_graph.is_none());
    assert_eq!(project(&h).graphs().count(), 0);
    menu_path(&mut h, &["File", "New", "Function Plot"]);
    h.state_mut().function_graph.as_mut().unwrap().expr = "x*x".into();
    h.get_by_label("OK").click();
    settle(&mut h);
    assert_eq!(h.state().doc.undo_label(), Some("New function plot"));
    let g = project(&h).graphs().next().expect("a new graph").clone();
    assert_eq!(g.layers[0].plots.len(), 1);
    assert_eq!(g.layers[0].plots[0].kind, PlotKind::Line);
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::Undo, &ctx);
    settle(&mut h);
    assert_eq!(project(&h).graphs().count(), 0, "one undo step takes the graph and its function away");
}
