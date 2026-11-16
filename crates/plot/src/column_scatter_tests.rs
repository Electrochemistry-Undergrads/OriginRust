//! Column Scatter (T79): each Y column as points standing at 1, 2, 3, ... with a small jitter.

use super::*;
use crate::data::Dim;
use crate::tests::{doc, svg_of, texts};
use sheaf_core::graph::{PlotKind, SymbolShape};
use sheaf_core::template::Template;
use sheaf_core::{ColumnData, Designation};

fn made() -> (Project, WindowId) {
    let mut p = Project::new();
    let b = p.add_workbook("data");
    let ys: Vec<_> =
        [(0..9).map(|i| (i * i) as f64).collect::<Vec<_>>(), (0..9).map(|i| 3.0 * i as f64 + 20.0).collect()]
            .into_iter()
            .enumerate()
            .map(|(i, v)| {
                let id = p.add_column(b, 0, Designation::Y, ColumnData::numeric(v)).unwrap();
                p.column_mut(id).unwrap().long_name = ["Left", "Right"][i].into();
                id
            })
            .collect();
    let g = p.plot_columns(Template::ColumnScatter, &ys).unwrap();
    (p, g)
}

#[test]
fn the_points_stand_at_their_position_within_the_jitter() {
    let (p, g) = made();
    let layer = &p.graph(g).unwrap().layers[0];
    assert!(layer.plots.iter().all(|q| q.kind == PlotKind::ColumnScatter));
    assert_eq!(layer.plots[0].symbol.shape, SymbolShape::Diamond);
    let l = data::collect(&p, layer);
    for (k, (lo, hi)) in [(0.82, 1.18), (1.82, 2.18)].into_iter().enumerate() {
        let pd = &l.plots[k];
        assert!(pd.rows().count() == 9);
        for i in pd.rows() {
            let x = pd.x_at(i);
            assert!(x >= lo && x <= hi, "plot {k} row {i}: {x}");
        }
        let xs: std::collections::HashSet<u64> = pd.rows().map(|i| pd.x_at(i).to_bits()).collect();
        assert!(xs.len() > 1, "the points are spread");
    }
}

#[test]
fn the_x_axis_runs_half_a_step_beyond_the_columns_named_by_them() {
    let (p, g) = made();
    let layer = &p.graph(g).unwrap().layers[0];
    let l = data::collect(&p, layer);
    let sources: Vec<&data::PlotData> = l.plots.iter().collect();
    let r = layout::resolve_dim(&p, &layer.x_axis, Dim::X, &sources, (layer, 0.0));
    assert!((r.min - 0.5).abs() < 1e-9 && (r.max - 2.5).abs() < 1e-9, "{} {}", r.min, r.max);
    let svg = svg_of(&p, g);
    let t = texts(&doc(&svg));
    for s in ["Left", "Right", "Range"] {
        assert!(t.iter().any(|x| x == s), "{s} in {t:?}");
    }
}

#[test]
fn only_symbols_are_drawn() {
    let (p, g) = made();
    let s = layout::build_scene(&p, g, &FixedMetrics).unwrap();
    assert!(!s.layers[0].plots.is_empty());
    assert!(
        s.layers[0].plots.iter().all(|it| matches!(it, scene::Item::Path { fill: Some(_), stroke: None, .. })),
        "filled symbols only: no line, no box"
    );
}
