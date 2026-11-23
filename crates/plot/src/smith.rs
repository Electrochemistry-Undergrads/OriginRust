//! Smith charts (T115, graphing spec "Smith chart"): a layer whose plots read their X column as the
//! resistance and their Y column as the reactance of an impedance, normalised by the layer's Z0, and
//! draw the reflection coefficient gamma = (z - 1) / (z + 1) inside the unit circle. The layer's axes run
//! from -1 to 1. The grid is Origin 2016's (measured by T385): circles of constant resistance and arcs of
//! constant reactance at 0.2, 0.5, 1, 2, 5 and 20 in solid gray, dotted light gray ones halfway between
//! them in gamma (resistance 1/11, 1/3, 5/7, 1.4, 3 and 11, and the same reactances), the real axis and the
//! unit circle in black, the resistances 0.2 to 5.0 written along the axis and the reactances 0.2j to 5.0j
//! and -0.2j to -5.0j round the rim.

use crate::LayerGeometry;
use crate::data::PlotData;
use crate::scene::{self, Anchor, Item, Stroke, TextItem};
use sheaf_core::Color;
use sheaf_core::graph::Layer;
use std::borrow::Cow;

/// The resistances and reactances of the labelled grid lines.
pub const GRID_VALUES: [f64; 5] = [0.2, 0.5, 1.0, 2.0, 5.0];
/// All the solid (major) grid lines: the labelled ones and an unlabelled 20 (measured).
pub const MAJOR_VALUES: [f64; 6] = [0.2, 0.5, 1.0, 2.0, 5.0, 20.0];
/// The dotted (minor) grid lines: halfway between the major ones in gamma, (1 + u) / (1 - u) for u = +-1/6,
/// +-1/2, +-5/6 (measured: 0.09, 0.34, 0.72, 1.4, 3.0 read off Origin's picture).
pub const MINOR_VALUES: [f64; 6] = [1.0 / 11.0, 1.0 / 3.0, 5.0 / 7.0, 1.4, 3.0, 11.0];
/// Origin's Gray and LT Gray, the colours of the major and minor grid.
const MAJOR_GRAY: Color = Color::rgb(128, 128, 128);
const MINOR_GRAY: Color = Color::rgb(192, 192, 192);

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

/// The Smith grid of a layer with its labels, in the layer's tick-label font.
pub(crate) fn draw_grid(out: &mut Vec<Item>, layer: &Layer, geom: &LayerGeometry) {
    let major = Stroke::solid(MAJOR_GRAY, 0.5);
    let minor = Stroke { dash: vec![1.0, 2.0], ..Stroke::solid(MINOR_GRAY, 0.5) };
    let edge = Stroke::solid(Color::BLACK, 1.0);
    let circle = |cx: f64, cy: f64, rad: f64| {
        (0..=180).map(move |k| {
            let t = k as f64 / 180.0 * std::f64::consts::TAU;
            (cx + rad * t.cos(), cy + rad * t.sin())
        })
    };
    // Constant reactance: the impedances r + j x for r from 0 to (nearly) infinity.
    let rs: Vec<f64> = (0..=120).map(|k| (k as f64 / 12.0).exp() - 1.0).collect();
    for (values, stroke) in [(&MINOR_VALUES[..], &minor), (&MAJOR_VALUES[..], &major)] {
        for &r in values {
            path(out, geom, circle(r / (1.0 + r), 0.0, 1.0 / (1.0 + r)), stroke);
        }
        for &x in values {
            for s in [1.0, -1.0] {
                path(out, geom, rs.iter().map(|r| gamma(*r, s * x)).chain(std::iter::once((1.0, 0.0))), stroke);
            }
        }
    }
    path(out, geom, [(-1.0, 0.0), (1.0, 0.0)].into_iter(), &edge);
    path(out, geom, circle(0.0, 0.0, 1.0), &edge);
    // Labels: resistances just right of where their circles meet the axis, below it; reactances outside
    // the rim where their arcs end.
    let font = &layer.x_axis.tick_font;
    let text = |out: &mut Vec<Item>, (px, py): (f64, f64), t: String, anchor: Anchor| {
        out.push(Item::Text(TextItem {
            x: px,
            y: py,
            text: t,
            family: font.family.clone(),
            size: font.size_pt,
            bold: font.bold,
            italic: font.italic,
            color: Color::BLACK,
            anchor,
            rotate_ccw: 0.0,
        }));
    };
    for r in GRID_VALUES {
        let (px, py) = geom.data_to_page((r - 1.0) / (r + 1.0), 0.0);
        text(out, (px + 0.3 * font.size_pt, py + 1.1 * font.size_pt), format!("{r:.1}"), Anchor::Start);
    }
    for x in GRID_VALUES {
        for s in [1.0, -1.0] {
            let theta = s * (std::f64::consts::PI - 2.0 * x.atan());
            let (c, sn) = (theta.cos(), theta.sin());
            let (px, py) = geom.data_to_page(1.04 * c, 1.04 * sn);
            let anchor = if c > 0.2 {
                Anchor::Start
            } else if c < -0.2 {
                Anchor::End
            } else {
                Anchor::Middle
            };
            let dy = if sn < 0.0 { 0.9 * font.size_pt } else { 0.0 };
            text(out, (px, py + dy), format!("{}{:.1}j", if s < 0.0 { "-" } else { "" }, x), anchor);
        }
    }
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
        // The axes are fixed at -1..1. Origin's grid (T385): 6 major and 6 minor r circles, twice as many x
        // arcs, the axis and the edge, then 5 resistance labels and 10 reactance labels.
        let geom = &layout_geometry(&p, g).unwrap()[0];
        assert_eq!((geom.x.min, geom.x.max, geom.y.min, geom.y.max), (-1.0, 1.0, -1.0, 1.0));
        let s = layout::build_scene(&p, g, &FixedMetrics).unwrap();
        let grid = &s.layers[0].grid;
        assert_eq!(grid.len(), 12 + 24 + 1 + 1 + 5 + 10);
        let texts: Vec<&str> = grid
            .iter()
            .filter_map(|it| if let scene::Item::Text(t) = it { Some(t.text.as_str()) } else { None })
            .collect();
        for want in ["0.2", "1.0", "5.0", "0.5j", "-2.0j", "5.0j"] {
            assert!(texts.contains(&want), "{want} in {texts:?}");
        }
    }
}
