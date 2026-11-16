//! Plot > Statistics > QC (X bar R) Chart (T111, graphing spec "QC (X bar R) chart").

use super::*;
use sheaf_core::graph::PlotKind;
use sheaf_core::template::Template;

#[test]
fn the_qc_chart_asks_for_the_subgroup_size_then_makes_the_workbook_and_graph() {
    let mut h = start(vec![data_file("qc-chart")]);
    show_window(&mut h, "Book1 - sample.csv");
    let book = h.state().books.keys().next().copied().unwrap();
    let ctx = h.ctx.clone();
    let cols: Vec<_> = project(&h).workbook(book).unwrap().sheet().columns.iter().map(|c| c.id).collect();
    let graphs = project(&h).graphs().count();
    h.state_mut().apply(Action::PlotColumns { book, columns: vec![cols[1]], template: Template::QcXbarR }, &ctx);
    settle(&mut h);
    // Origin's prompt, 3 by default; nothing is made before OK.
    assert!(h.query_by_label_contains("Please specify subgroup size for Book1_B").is_some());
    assert_eq!(project(&h).graphs().count(), graphs);
    h.state_mut().qc_chart.as_mut().unwrap().size = 2;
    h.get_by_label("OK").click();
    settle(&mut h);
    assert!(h.state().qc_chart.is_none());
    let g = project(&h).graphs().last().unwrap();
    assert_eq!(g.layers.len(), 2);
    assert_eq!(g.layers[1].plots[0].kind, PlotKind::QcMeans);
    // Four values in subgroups of 2: two means.
    let qc = project(&h).workbooks().find(|b| b.name == "QC1").unwrap();
    assert_eq!(qc.sheet().columns[0].data.len(), 2);
}
