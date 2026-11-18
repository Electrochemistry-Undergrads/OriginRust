//! A Y column with a sampling interval stands at its own X (T84, workbook-model.md "Sampling
//! interval"): Origin 2016 drew B (x0 = 0, inc = 0.5, five rows) from 0 to 2 although an X column
//! of 1 to 5 stands to its left.

use super::*;
use sheaf_core::template::Template;
use sheaf_core::{ColumnData, Designation};

fn x_axis(p: &Project, g: WindowId) -> (f64, f64, Vec<f64>) {
    let graph = p.graph(g).unwrap();
    let pds: Vec<data::LayerData> = graph.layers.iter().map(|l| data::collect(p, l)).collect();
    let geom = &layout::graph_geometry(p, graph, &pds, &FixedMetrics)[0];
    (geom.x.min, geom.x.max, geom.x.major.clone())
}

#[test]
fn a_plot_of_a_column_with_an_interval_uses_it_and_its_name() {
    let ys = vec![10.0, 20.0, 30.0, 40.0, 50.0];
    let mut p = Project::new();
    let b = p.add_workbook("Book1");
    p.add_column(b, 0, Designation::X, ColumnData::numeric(vec![1.0, 2.0, 3.0, 4.0, 5.0])).unwrap();
    let y = p.add_column(b, 0, Designation::Y, ColumnData::numeric(ys.clone())).unwrap();
    p.set_sampling_interval(&[y], 0.0, 0.5, "Time", "s").unwrap();
    let g = p.plot_columns(Template::Scatter, &[y]).unwrap();

    // The same data with the X written out as a column.
    let mut q = Project::new();
    let qb = q.add_workbook("Book1");
    let qx = q.add_column(qb, 0, Designation::X, ColumnData::numeric(vec![0.0, 0.5, 1.0, 1.5, 2.0])).unwrap();
    q.column_mut(qx).unwrap().long_name = "Time".into();
    q.column_mut(qx).unwrap().units = "s".into();
    let qy = q.add_column(qb, 0, Designation::Y, ColumnData::numeric(ys)).unwrap();
    let qg = q.plot_columns(Template::Scatter, &[qy]).unwrap();

    assert_eq!(x_axis(&p, g), x_axis(&q, qg));
    let (min, max, _) = x_axis(&p, g);
    assert!(min <= 0.0 && (2.0..5.0).contains(&max), "{min}..{max}");
    let s = layout::build_scene(&p, g, &FixedMetrics).unwrap();
    let titles: Vec<String> = s
        .layers
        .iter()
        .flat_map(|l| l.overlay.iter())
        .filter_map(|it| match it {
            scene::Item::Text(t) => Some(t.text.clone()),
            _ => None,
        })
        .collect();
    assert!(titles.iter().any(|t| t == "Time (s)"), "{titles:?}");
}
