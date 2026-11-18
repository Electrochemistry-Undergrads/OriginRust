//! Vector XYAM and XYXY (T107, graphing spec "Vector plots").

use super::*;
use sheaf_core::template::Template;
use sheaf_core::{ColumnData, Designation};

fn geometry(p: &Project, g: WindowId) -> LayerGeometry {
    let graph = p.graph(g).unwrap();
    let pds: Vec<data::LayerData> = graph.layers.iter().map(|l| data::collect(p, l)).collect();
    layout::graph_geometry(p, graph, &pds, &FixedMetrics).remove(0)
}

#[test]
fn xyxy_axes_take_in_the_end_points() {
    // Origin's probe: (1,1) -> (2,1.5) and (2,2) -> (2.5,3); its axes went to 2.7 and 3.25.
    let mut p = Project::new();
    let b = p.add_workbook("");
    p.add_column(b, 0, Designation::X, ColumnData::numeric(vec![1.0, 2.0])).unwrap();
    let y1 = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![1.0, 2.0])).unwrap();
    p.add_column(b, 0, Designation::X, ColumnData::numeric(vec![2.0, 2.5])).unwrap();
    let y2 = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![1.5, 3.0])).unwrap();
    let g = p.plot_columns(Template::VectorXyxy, &[y1, y2]).unwrap();
    let geom = geometry(&p, g);
    assert!(geom.x.max >= 2.5 && geom.y.max >= 3.0, "x to {}, y to {}", geom.x.max, geom.y.max);
}

#[test]
fn an_xyam_arrow_runs_magnitude_times_multiplier_points_at_its_angle() {
    let mut p = Project::new();
    let b = p.add_workbook("");
    p.add_column(b, 0, Designation::X, ColumnData::numeric(vec![1.0, 2.0])).unwrap();
    let y = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![1.0, 2.0])).unwrap();
    let a = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![0.0, std::f64::consts::FRAC_PI_2])).unwrap();
    let m = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![100.0, 50.0])).unwrap();
    let g = p.plot_columns(Template::VectorXyam, &[y, a, m]).unwrap();
    let geom = geometry(&p, g);
    let s = layout::build_scene(&p, g, &FixedMetrics).unwrap();
    let shafts: Vec<Vec<f64>> = s.layers[0]
        .plots
        .iter()
        .filter_map(|it| match it {
            scene::Item::Path { d, fill: None, stroke: Some(st) } if st.width == 0.5 => {
                Some(d.split(['M', 'L', ' ']).filter_map(|t| t.parse().ok()).collect())
            }
            _ => None,
        })
        .collect();
    assert_eq!(shafts.len(), 2, "{shafts:?}");
    // Row 1: angle 0, 100 x 0.16 = 16 pt to the right (the shaft stops at the 4.5 pt head).
    let (x0, y0) = geom.data_to_page(1.0, 1.0);
    let r1 = &shafts[0];
    assert!((r1[0] - x0).abs() < 0.01 && (r1[1] - y0).abs() < 0.01, "{r1:?}");
    assert!((r1[2] - (x0 + 16.0 - 4.5)).abs() < 0.01 && (r1[3] - y0).abs() < 0.01, "{r1:?}");
    // Row 2: pi/2 radians points straight up, 8 pt.
    let (x1, y1) = geom.data_to_page(2.0, 2.0);
    let r2 = &shafts[1];
    assert!((r2[2] - x1).abs() < 0.01 && (r2[3] - (y1 - 8.0 + 4.5)).abs() < 0.01, "{r2:?}");
}
