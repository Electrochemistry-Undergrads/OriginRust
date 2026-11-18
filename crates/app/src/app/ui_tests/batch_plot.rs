//! Window > Duplicate (Batch Plotting) (T99, graphing.md "Batch plotting").

use super::*;

#[test]
fn duplicate_with_new_columns_makes_a_copy_per_ticked_column() {
    let mut h = start(vec![data_file("batch-plot")]);
    show_window(&mut h, "Book1 - sample.csv");
    let (b, c) = (column_id(&h, "B"), column_id(&h, "C"));
    let ctx = h.ctx.clone();
    // A graph of B only.
    let book = h.state().books.keys().next().copied().unwrap();
    h.state_mut()
        .apply(Action::PlotColumns { book, columns: vec![b], template: sheaf_core::template::Template::Line }, &ctx);
    settle(&mut h);
    let graph = project(&h)
        .graphs()
        .find(|g| g.layers[0].plots.iter().any(|p| p.y == b) && g.layers[0].plots.len() == 1)
        .unwrap()
        .id;
    let before = project(&h).graphs().count();
    h.state_mut().apply(Action::BatchPlot { graph, columns: true }, &ctx);
    settle(&mut h);
    let d = h.state_mut().batch_plot.as_mut().expect("the window opened");
    assert_eq!(d.column_items.iter().map(|c| c.0).collect::<Vec<_>>(), [c], "the other Y column");
    d.column_items[0].2 = true;
    settle(&mut h);
    click(&mut h, "OK");
    assert_eq!(project(&h).graphs().count(), before + 1);
    let copy = project(&h).graphs().last().unwrap();
    assert!(copy.long_name.ends_with(" - Copy"), "{}", copy.long_name);
    assert_eq!(copy.layers[0].plots[0].y, c);
    assert_eq!(h.state().doc.undo_label(), Some("Duplicate (batch plotting)"));
    h.state_mut().apply(Action::Undo, &ctx);
    settle(&mut h);
    assert_eq!(project(&h).graphs().count(), before);
}
