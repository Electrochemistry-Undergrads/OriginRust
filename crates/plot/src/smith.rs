//! Smith charts (T115, graphing spec "Smith chart"): a layer whose plots read their X column as the
//! resistance and their Y column as the reactance of an impedance, normalised by the layer's Z0, and
//! draw the reflection coefficient gamma = (z - 1) / (z + 1) inside the unit circle. The layer's axes run
//! from -1 to 1; the grid is the unit circle, circles of constant resistance and arcs of constant
//! reactance at 0.2, 0.5, 1, 2 and 5, and the real axis.

use crate::LayerGeometry;
use crate::data::PlotData;
use crate::scene::{self, Item, Stroke};
use sheaf_core::Color;
use std::borrow::Cow;

/// The resistances and reactances of the grid lines.
pub const GRID_VALUES: [f64; 5] = [0.2, 0.5, 1.0, 2.0, 5.0];

/// The reflection coefficient of the normalised impedance `r + j x`.
pub fn gamma(r: f64, x: f64) -> (f64, f64) {
    // (z - 1) / (z + 1) = ((r - 1) + j x) / ((r + 1) + j x)
    let d = (r + 1.0).powi(2) + x * x;
    if d == 0.0 || !d.is_finite() {
        return if r.is_infinite() || x.is_infinite() { (1.0, 0.0) } else { (f64::NAN, f64::NAN) };
    }
    ((r * r - 1.0 + x * x) / d, 2.0 * x / d)
}

/// Turns a plot's (resistance, reactance) rows into reflection coefficients.
pub(crate) fn columns(pd: &mut PlotData, z0: f64) {
    let z0 = if z0 > 0.0 && z0.is_finite() { z0 } else { 1.0 };
    let n = pd.y.len();
    let r: Vec<f64> = (0..n).map(|i| pd.x.as_ref().and_then(|x| x.get(i).copied()).unwrap_or((i + 1) as f64)).collect();
    let (gx, gy): (Vec<f64>, Vec<f64>) = r.iter().zip(pd.y.iter()).map(|(r, x)| gamma(r / z0, x / z0)).unzip();
    pd.x = Some(Cow::Owned(gx));
    pd.y = Cow::Owned(gy);
    pd.err = None;
    pd.xerr = None;
    pd.x_format = None;
}

fn path(out: &mut Vec<Item>, geom: &LayerGeometry, pts: impl Iterator<Item = (f64, f64)>, stroke: &Stroke) {
    let mut d = String::new();
    for (k, (x, y)) in pts.enumerate() {
        let (px, py) = geom.data_to_page(x, y);
        if k == 0 { scene::move_to(&mut d, px, py) } else { scene::line_to(&mut d, px, py) }
    }
    out.push(Item::Path { d, fill: None, stroke: Some(stroke.clone()) });
}

/// The Smith grid of a layer.
pub(crate) fn draw_grid(out: &mut Vec<Item>, geom: &LayerGeometry) {
    let grid = Stroke::solid(Color::rgb(204, 204, 204), 0.5);
    let edge = Stroke::solid(Color::BLACK, 1.0);
    let circle = |cx: f64, cy: f64, rad: f64| {
        (0..=180).map(move |k| {
            let t = k as f64 / 180.0 * std::f64::consts::TAU;
            (cx + rad * t.cos(), cy + rad * t.sin())
        })
    };
    for r in GRID_VALUES {
        path(out, geom, circle(r / (1.0 + r), 0.0, 1.0 / (1.0 + r)), &grid);
    }
    // Constant reactance: the impedances r + j x for r from 0 to (nearly) infinity.
    let rs: Vec<f64> = (0..=120).map(|k| (k as f64 / 12.0).exp() - 1.0).collect();
    for x in GRID_VALUES {
        for s in [1.0, -1.0] {
            path(out, geom, rs.iter().map(|r| gamma(*r, s * x)).chain(std::iter::once((1.0, 0.0))), &grid);
        }
    }
    path(out, geom, [(-1.0, 0.0), (1.0, 0.0)].into_iter(), &grid);
    path(out, geom, circle(0.0, 0.0, 1.0), &edge);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gamma_maps_matched_short_open_and_reactive_loads() {
        assert_eq!(gamma(1.0, 0.0), (0.0, 0.0), "a matched load is the centre");
        assert_eq!(gamma(0.0, 0.0), (-1.0, 0.0), "a short circuit is the left end");
        assert_eq!(gamma(f64::INFINITY, 0.0), (1.0, 0.0), "an open circuit is the right end");
        let (gx, gy) = gamma(0.0, 1.0);
        assert!((gx - 0.0).abs() < 1e-12 && (gy - 1.0).abs() < 1e-12, "pure +j is the top");
        // Constant resistance r lies on the circle centred at r / (1 + r) with radius 1 / (1 + r).
        let (gx, gy) = gamma(2.0, 3.7);
        assert!(((gx - 2.0 / 3.0).hypot(gy) - 1.0 / 3.0).abs() < 1e-12);
    }
}

#[cfg(test)]
mod layer_tests {
    use crate::*;
    use sheaf_core::template::Template;
    use sheaf_core::{ColumnData, Designation, Project};

    #[test]
    fn a_smith_layer_draws_the_reflection_coefficients_over_its_grid() {
        let mut p = Project::new();
        let b = p.add_workbook("");
        p.add_column(b, 0, Designation::X, ColumnData::numeric(vec![1.0, 0.0, 50.0])).unwrap();
        let y = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![0.0, 1.0, 0.0])).unwrap();
        let g = p.plot_columns(Template::SmithChart, &[y]).unwrap();
        let graph = p.graph(g).unwrap();
        assert!(graph.layers[0].coords.smith().is_some());
        let ld = data::collect(&p, &graph.layers[0]);
        let pd = &ld.plots[0];
        let pts: Vec<(f64, f64)> = (0..3).map(|i| (pd.x_at(i), pd.y_at(i))).collect();
        assert_eq!(pts[0], (0.0, 0.0), "z = 1 is the centre");
        assert!((pts[1].0).abs() < 1e-12 && (pts[1].1 - 1.0).abs() < 1e-12, "z = j is the top");
        assert!((pts[2].0 - 49.0 / 51.0).abs() < 1e-12, "z = 50 is near the right end");
        // The axes are fixed at -1..1 and the scene has the grid (5 r circles, 10 x arcs, the axis, the edge).
        let geom = &layout_geometry(&p, g).unwrap()[0];
        assert_eq!((geom.x.min, geom.x.max, geom.y.min, geom.y.max), (-1.0, 1.0, -1.0, 1.0));
        let s = layout::build_scene(&p, g, &FixedMetrics).unwrap();
        assert_eq!(s.layers[0].grid.len(), 5 + 10 + 1 + 1);
    }
}
