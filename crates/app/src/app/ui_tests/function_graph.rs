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

#[test]
fn graph_slicer_shows_the_ticked_values_in_one_undo_step() {
    // T406: Graph > Graph Slicer... on a line of Y whose sheet has a group column.
    let mut h = start(Vec::new());
    let ctx = h.ctx.clone();
    let mut graph = sheaf_core::WindowId(0);
    h.state_mut().doc.apply("data", |p| {
        let b = p.add_workbook("");
        let groups = ["a", "b", "a", "c"].iter().map(|s| s.to_string()).collect();
        p.add_column(b, 0, sheaf_core::Designation::Y, sheaf_core::ColumnData::text(groups)).unwrap();
        let y = p
            .add_column(b, 0, sheaf_core::Designation::Y, sheaf_core::ColumnData::numeric(vec![1.0, 2.0, 3.0, 4.0]))
            .unwrap();
        graph = p.plot_columns(sheaf_core::template::Template::Line, &[y]).unwrap();
    });
    settle(&mut h);
    h.state_mut().apply(Action::GraphSlicer { graph }, &ctx);
    settle(&mut h);
    {
        let d = h.state_mut().graph_slicer.as_mut().expect("the slicer window");
        let names: Vec<&str> = d.values.iter().map(|v| v.0.as_str()).collect();
        assert_eq!(names, ["a", "b", "c"], "the text column is chosen first");
        d.values[1].1 = false;
    }
    h.get_by_label("OK").click();
    settle(&mut h);
    assert!(h.state().graph_slicer.is_none());
    assert_eq!(h.state().doc.undo_label(), Some("Graph Slicer"));
    let s = project(&h).graph(graph).unwrap().slicer.clone().expect("a slicer");
    assert_eq!(s.values, ["a", "c"]);
    // Remove Slicer clears it.
    h.state_mut().apply(Action::GraphSlicer { graph }, &ctx);
    settle(&mut h);
    h.get_by_label("Remove Slicer").click();
    settle(&mut h);
    assert!(project(&h).graph(graph).unwrap().slicer.is_none());
}

#[test]
fn browser_graph_steps_through_the_columns_from_the_bar() {
    // T410: Graph > Browser Graph... on a line of the middle of three Y columns, then Next and Previous.
    let mut h = start(Vec::new());
    let ctx = h.ctx.clone();
    let mut graph = sheaf_core::WindowId(0);
    let mut ys = Vec::new();
    h.state_mut().doc.apply("data", |p| {
        let b = p.add_workbook("");
        for k in 0..3 {
            let v = vec![k as f64, k as f64 + 1.0];
            ys.push(p.add_column(b, 0, sheaf_core::Designation::Y, sheaf_core::ColumnData::numeric(v)).unwrap());
        }
        graph = p.plot_columns(sheaf_core::template::Template::Line, &[ys[1]]).unwrap();
    });
    settle(&mut h);
    h.state_mut().apply(Action::BrowserGraph { graph, layer: 0 }, &ctx);
    settle(&mut h);
    assert!(h.state().browser_graph.is_some(), "the browser window");
    h.get_by_label("OK").click();
    settle(&mut h);
    assert!(h.state().browser_graph.is_none());
    assert_eq!(h.state().doc.undo_label(), Some("Browser Graph"));
    assert_eq!(project(&h).graph(graph).unwrap().browser.as_ref().unwrap().items.len(), 3);
    h.get_by_label("Next").click();
    settle(&mut h);
    assert_eq!(project(&h).graph(graph).unwrap().layers[0].plots[0].y, ys[2]);
    h.get_by_label("Previous").click();
    settle(&mut h);
    h.get_by_label("Previous").click();
    settle(&mut h);
    assert_eq!(project(&h).graph(graph).unwrap().layers[0].plots[0].y, ys[0]);
    // The steps make one undo step: one Undo goes back to the dataset the browser started on.
    assert_eq!(h.state().doc.undo_label(), Some("Browse Graph"));
    h.state_mut().doc.undo();
    assert_eq!(project(&h).graph(graph).unwrap().layers[0].plots[0].y, ys[1]);
}

#[test]
fn time_series_explorer_steps_and_plays_from_the_bar() {
    // T416: a Time Series Explorer of 50 points; Next, Previous, then Play and Pause.
    let mut h = start(Vec::new());
    let mut graph = sheaf_core::WindowId(0);
    h.state_mut().doc.apply("data", |p| {
        let b = p.add_workbook("");
        let v = (0..50).map(|i| f64::from(i).sin()).collect();
        let y = p.add_column(b, 0, sheaf_core::Designation::Y, sheaf_core::ColumnData::numeric(v)).unwrap();
        graph = p.plot_columns(sheaf_core::template::Template::TimeSeriesExplorer, &[y]).unwrap();
    });
    settle(&mut h);
    let current = |h: &Harness<'static, SheafApp>| project(h).graph(graph).unwrap().explorer.unwrap().current;
    h.get_by_label("Next").click();
    settle(&mut h);
    h.get_by_label("Next").click();
    settle(&mut h);
    assert_eq!(current(&h), 2);
    h.get_by_label("Previous").click();
    settle(&mut h);
    assert_eq!(current(&h), 1);
    assert_eq!(h.state().doc.undo_label(), Some("Explore Interval"));
    h.get_by_label("Play").click();
    settle(&mut h);
    h.get_by_label("Pause").click();
    settle(&mut h);
    assert!(h.query_by_label("Play").is_some(), "paused");
}
