//! Column header > Slide Show of Dependent Graphs (T129, gui-look "Slide Show of Dependent Graphs").

use super::*;
use sheaf_core::template::Template;

#[test]
fn the_slide_show_takes_the_graphs_that_plot_the_column() {
    let mut h = start(vec![data_file("slide-show")]);
    show_window(&mut h, "Book1 - sample.csv");
    let book = h.state().books.keys().next().copied().unwrap();
    let ctx = h.ctx.clone();
    let cols: Vec<_> = project(&h).workbook(book).unwrap().sheet().columns.iter().map(|c| c.id).collect();
    for (col, t) in [(cols[1], Template::Line), (cols[2], Template::Scatter), (cols[1], Template::Scatter)] {
        h.state_mut().apply(Action::PlotColumns { book, columns: vec![col], template: t }, &ctx);
        settle(&mut h);
    }
    let graphs: Vec<_> = project(&h).graphs().map(|g| g.id).collect();
    // The opened file comes with a graph of B and C; column B is also in the first and third new graph.
    assert_eq!(graphs.len(), 4);
    h.state_mut().apply(Action::SlideShow(vec![cols[1]]), &ctx);
    let fs = h.state().full_screen.as_ref().expect("the slide show is on");
    assert_eq!(fs.slides.as_deref(), Some(&[graphs[0], graphs[1], graphs[3]][..]));
    assert_eq!((fs.graph, fs.reminder), (graphs[0], false));
    // A column that no graph plots: a warning, no slide show.
    h.state_mut().full_screen = None;
    let free = h
        .state_mut()
        .doc
        .edit("Add", |p| p.add_column(book, 0, sheaf_core::Designation::Y, sheaf_core::ColumnData::numeric(vec![1.0])));
    h.state_mut().apply(Action::SlideShow(vec![free.unwrap()]), &ctx);
    assert!(h.state().full_screen.is_none());
}
