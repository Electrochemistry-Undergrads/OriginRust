//! Graph gadgets read only the rows the plot shows (T209, workbook-model 13.2a).

use super::*;
use crate::gadget::{GadgetKind, Integrator, gadget_xy, plot_xy};

#[test]
fn gadgets_skip_filtered_and_masked_rows_and_the_reader_keeps_its_rows() {
    let mut h = start(vec![data_file("gadget-filters")]);
    let g = graph(&h).id;
    let plot = graph(&h).layers[0].plots[0].clone();
    let (book, _, _) = project(&h).locate(plot.y).unwrap();
    let book = book.id;
    let (all_x, all_y) = plot_xy(project(&h), g, 0, 0).unwrap();
    assert_eq!(gadget_xy(project(&h), g, 0, 0).unwrap(), (all_x.clone(), all_y.clone()), "nothing hidden");
    // Hide the first row with a filter on X and mask the last Y cell.
    let x = plot.x.expect("an X column");
    let x_name = project(&h).column(x).unwrap().short_name.clone();
    h.state_mut().doc.apply("Hide", |p| {
        p.set_row_filter(book, 0, &format!("col({x_name}) > {}", all_x[0])).unwrap();
        p.column_mut(plot.y).unwrap().mask.insert(all_y.len() - 1);
    });
    settle(&mut h);
    let (gx, gy) = gadget_xy(project(&h), g, 0, 0).unwrap();
    assert_eq!(gx, all_x[1..all_x.len() - 1]);
    assert_eq!(gy, all_y[1..all_y.len() - 1]);
    // The Data Reader's rows are the worksheet's.
    assert_eq!(plot_xy(project(&h), g, 0, 0).unwrap().0.len(), all_x.len());
    // The Statistics gadget over the whole X range counts the shown, unmasked points only.
    let stats = Integrator {
        kind: GadgetKind::Statistics,
        layer: 0,
        plot: 0,
        x0: all_x[0] - 1.0,
        x1: all_x[all_x.len() - 1] + 1.0,
        y0: 0.0,
        y1: 0.0,
        end_point_baseline: false,
        circle: false,
    };
    assert_eq!(stats.stats(&gx, &gy).unwrap().n, all_x.len() - 2);
}
