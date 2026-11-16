//! Plot > Statistics > Scatter Matrix (T114, graphing spec "Scatter matrix").

use super::*;
use sheaf_core::graph::PlotKind;
use sheaf_core::scatter_matrix::{MatrixDiagonal, MatrixDisplay};
use sheaf_core::template::Template;

#[test]
fn the_scatter_matrix_asks_for_its_options_then_makes_one_graph_in_one_undo_step() {
    let mut h = start(vec![data_file("scatter-matrix")]);
    show_window(&mut h, "Book1 - sample.csv");
    let book = h.state().books.keys().next().copied().unwrap();
    let ctx = h.ctx.clone();
    let cols: Vec<_> = project(&h).workbook(book).unwrap().sheet().columns.iter().map(|c| c.id).collect();
    let graphs = project(&h).graphs().count();
    // The X column counts as a variable too, as in Origin.
    h.state_mut().apply(Action::PlotColumns { book, columns: cols.clone(), template: Template::ScatterMatrix }, &ctx);
    settle(&mut h);
    assert!(h.query_by_label_contains("Matrix Display").is_some());
    assert_eq!(project(&h).graphs().count(), graphs, "nothing is made before OK");
    let d = h.state_mut().scatter_matrix.as_mut().unwrap();
    assert_eq!(d.columns, cols);
    d.options.display = MatrixDisplay::Lower;
    d.options.diagonal = MatrixDiagonal::Histogram;
    h.get_by_label("OK").click();
    settle(&mut h);
    assert!(h.state().scatter_matrix.is_none());
    assert_eq!(project(&h).graphs().count(), graphs + 1);
    let g = project(&h).graphs().last().unwrap();
    assert_eq!(g.layers.len(), 6);
    assert_eq!(g.layers[0].plots[0].kind, PlotKind::Histogram);
    assert_eq!(g.layers[1].plots[0].x, Some(cols[0]));
    assert_eq!(g.layers[1].plots[0].y, cols[1]);
    // One undo step removes the graph.
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::Undo, &ctx);
    settle(&mut h);
    assert_eq!(project(&h).graphs().count(), graphs);
}

#[test]
fn the_scatter_matrix_refuses_a_single_column_and_cancel_makes_nothing() {
    let mut h = start(vec![data_file("scatter-matrix-one")]);
    show_window(&mut h, "Book1 - sample.csv");
    let book = h.state().books.keys().next().copied().unwrap();
    let ctx = h.ctx.clone();
    let cols: Vec<_> = project(&h).workbook(book).unwrap().sheet().columns.iter().map(|c| c.id).collect();
    let graphs = project(&h).graphs().count();
    h.state_mut().apply(Action::PlotColumns { book, columns: vec![cols[1]], template: Template::ScatterMatrix }, &ctx);
    settle(&mut h);
    assert!(h.state().scatter_matrix.is_none());
    h.state_mut()
        .apply(Action::PlotColumns { book, columns: cols[1..].to_vec(), template: Template::ScatterMatrix }, &ctx);
    settle(&mut h);
    assert!(h.state().scatter_matrix.is_some());
    h.get_by_label("Cancel").click();
    settle(&mut h);
    assert!(h.state().scatter_matrix.is_none());
    assert_eq!(project(&h).graphs().count(), graphs);
}

#[test]
fn the_scatter_matrix_refuses_a_selection_with_a_text_column() {
    let dir = std::env::temp_dir().join(format!("sheaf-app-ui-scatter-matrix-text-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("sample.csv");
    std::fs::write(&path, "t,up,name\n1,2.5,a\n2,4.5,b\n3,6.5,c\n").unwrap();
    let mut h = start(vec![path]);
    show_window(&mut h, "Book1 - sample.csv");
    let book = h.state().books.keys().next().copied().unwrap();
    let ctx = h.ctx.clone();
    let cols: Vec<_> = project(&h).workbook(book).unwrap().sheet().columns.iter().map(|c| c.id).collect();
    assert!(!project(&h).column(cols[2]).unwrap().data.is_numeric());
    h.state_mut().apply(Action::PlotColumns { book, columns: cols, template: Template::ScatterMatrix }, &ctx);
    settle(&mut h);
    assert!(h.state().scatter_matrix.is_none(), "no options window for a selection with text");
}
