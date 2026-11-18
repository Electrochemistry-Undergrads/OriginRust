//! Plot > Stock > High-Low-Close, Japanese Candlestick and OHLC Bar Chart (T108, graphing spec "Stock
//! plots").

use super::*;
use sheaf_core::graph::PlotKind;
use sheaf_core::template::Template;

#[test]
fn stock_plots_take_exactly_three_or_four_y_columns() {
    let mut h = start(vec![data_file("stock-plots")]);
    show_window(&mut h, "Book1 - sample.csv");
    let book = h.state().books.keys().next().copied().unwrap();
    let ctx = h.ctx.clone();
    h.state_mut().doc.apply("Add", |p| {
        p.add_column(book, 0, sheaf_core::Designation::Y, sheaf_core::ColumnData::numeric(vec![3.0, 4.0, 5.0, 6.0]))
            .unwrap();
    });
    let cols: Vec<_> = project(&h).workbook(book).unwrap().sheet().columns.iter().map(|c| c.id).collect();
    let graphs = project(&h).graphs().count();
    // A, B, C, D: X and three Y columns, High-Low-Close's XYYY.
    h.state_mut().apply(Action::PlotColumns { book, columns: cols.clone(), template: Template::HighLowClose }, &ctx);
    settle(&mut h);
    let g = project(&h).graphs().last().unwrap();
    let plot = &g.layers[0].plots[0];
    assert_eq!(plot.kind, PlotKind::HighLowClose);
    let s = plot.decor.stock.unwrap();
    assert_eq!((plot.y, s.high, s.low, s.close, s.open), (cols[1], cols[1], cols[2], cols[3], None));
    // Three Y columns are one short of a candlestick: no graph.
    h.state_mut().apply(Action::PlotColumns { book, columns: cols, template: Template::Candlestick }, &ctx);
    settle(&mut h);
    assert_eq!(project(&h).graphs().count(), graphs + 1);
}
