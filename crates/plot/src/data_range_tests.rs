//! Set As Begin / Set As End (T87, workbook-model.md "Column data range"): a plot leaves out the
//! rows outside the Y column's data range. Origin 2016 drew B (10..60) begun at row 2 and ended at
//! row 4 with its X axis from 1.5 to 4.5: only rows 2 to 4.

use super::*;
use sheaf_core::data_range::RangeEnd;
use sheaf_core::template::Template;
use sheaf_core::{ColumnData, Designation};

fn axes(p: &Project, g: WindowId) -> ((f64, f64), (f64, f64)) {
    let graph = p.graph(g).unwrap();
    let pds: Vec<data::LayerData> = graph.layers.iter().map(|l| data::collect(p, l)).collect();
    let geom = &layout::graph_geometry(p, graph, &pds, &FixedMetrics)[0];
    ((geom.x.min, geom.x.max), (geom.y.min, geom.y.max))
}

fn graph(xs: Vec<f64>, ys: Vec<f64>, range: Option<(usize, usize)>) -> (Project, WindowId) {
    let mut p = Project::new();
    let b = p.add_workbook("Book1");
    p.add_column(b, 0, Designation::X, ColumnData::numeric(xs)).unwrap();
    let y = p.add_column(b, 0, Designation::Y, ColumnData::numeric(ys)).unwrap();
    if let Some((first, last)) = range {
        p.set_data_range_end(&[y], RangeEnd::Begin, first).unwrap();
        p.set_data_range_end(&[y], RangeEnd::End, last).unwrap();
    }
    let g = p.plot_columns(Template::Scatter, &[y]).unwrap();
    (p, g)
}

#[test]
fn a_plot_leaves_out_the_rows_outside_the_data_range() {
    let all: Vec<f64> = (1..=6).map(f64::from).collect();
    let ys: Vec<f64> = (1..=6).map(|k| 10.0 * k as f64).collect();
    let (p, g) = graph(all, ys, Some((1, 3)));
    let (q, qg) = graph(vec![2.0, 3.0, 4.0], vec![20.0, 30.0, 40.0], None);
    assert_eq!(axes(&p, g), axes(&q, qg));
    let ((x0, x1), _) = axes(&p, g);
    assert!(x0 > 1.0 && x1 < 5.0, "{x0}..{x1}");
}
