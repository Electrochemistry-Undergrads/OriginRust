//! Plot > Specialized > Vector XYAM and Vector XYXY (T107, graphing spec "Vector plots").

use super::*;
use sheaf_core::graph::PlotKind;

#[test]
fn vector_xyam_plots_the_selected_y_angle_and_magnitude_columns() {
    let mut h = start(vec![data_file("vector-plots")]);
    show_window(&mut h, "Book1 - sample.csv");
    let book = h.state().books.keys().next().copied().unwrap();
    let ctx = h.ctx.clone();
    // A fourth column for the magnitudes.
    h.state_mut().doc.apply("Add", |p| {
        p.add_column(book, 0, sheaf_core::Designation::Y, sheaf_core::ColumnData::numeric(vec![1.0, 2.0, 3.0, 4.0]))
            .unwrap();
    });
    let cols: Vec<_> = project(&h).workbook(book).unwrap().sheet().columns.iter().map(|c| c.id).collect();
    h.state_mut().apply(
        Action::PlotColumns { book, columns: cols.clone(), template: sheaf_core::template::Template::VectorXyam },
        &ctx,
    );
    settle(&mut h);
    let g = project(&h).graphs().last().unwrap();
    let plot = &g.layers[0].plots[0];
    assert_eq!(plot.kind, PlotKind::VectorXyam);
    let v = plot.decor.vector.unwrap();
    assert_eq!((plot.y, v.a, v.b), (cols[1], cols[2], cols[3]));
}
