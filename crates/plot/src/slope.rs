//! Slope annotations (T52, graphing spec 17.6): the segment between two rows of a plot, optional
//! dashed legs (the run, then the rise) and a label "slope = <value>".
//!
//! The two points are the plot's rows as drawn (multipliers and offsets applied). The slope is
//! `dy / dx` in the axis-transformed coordinates: data units on a linear axis, decades on a log10
//! axis, e-folds on a ln axis and octaves on a log2 axis (so a Tafel plot of potential against
//! log10(current) reads in mV per decade).

use crate::LayerGeometry;
use crate::data::PlotData;
use crate::fonts::TextMeasurer;
use crate::hit;
use crate::plots::line_stroke;
use crate::scene::{Anchor, Item, ItemList, ObjectKind, SceneObject, line_to, move_to};
use sheaf_calc::trend::sig_str;
use sheaf_core::graph::{Dash, Font, Layer, LineStyle, ScaleKind};
use sheaf_core::{Color, Project};

/// Gap between the middle of the segment and the label, in points.
const LABEL_GAP: f64 = 8.0;

/// A value on an axis in the axis' own units: itself on a linear axis, its logarithm on a log axis.
/// Scales that have no such unit (probability, reciprocal, ...) use the data values.
pub(crate) fn axis_value(scale: ScaleKind, v: f64) -> f64 {
    match scale {
        ScaleKind::Log10 => v.log10(),
        ScaleKind::Ln => v.ln(),
        ScaleKind::Log2 => v.log2(),
        _ => v,
    }
}

/// What the axis' unit is called when it is a logarithm (`None` for the column's own unit).
fn log_unit(scale: ScaleKind) -> Option<&'static str> {
    match scale {
        ScaleKind::Log10 => Some("dec"),
        ScaleKind::Ln => Some("e-fold"),
        ScaleKind::Log2 => Some("oct"),
        _ => None,
    }
}

/// The two points of a slope annotation, as drawn, and its value.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Slope {
    pub a: (f64, f64),
    pub b: (f64, f64),
    pub value: f64,
}

/// Plots a slope can be marked on: those with one point per row (not pie, box, violin, ridgeline, hexbin
/// or histogram plots, whose data is not the worksheet's points).
fn usable(pd: &PlotData) -> bool {
    pd.pie.is_none()
        && pd.boxed.is_none()
        && pd.violin.is_none()
        && pd.ridge.is_none()
        && pd.hex.is_none()
        && pd.hist_raw.is_none()
        && pd.parallel.is_none()
        && pd.treemap.is_none()
        && pd.sunburst.is_none()
}

/// The point at `row`: shown, unmasked and finite (and placeable on the axes).
fn point(pd: &PlotData, row: usize, geom: &LayerGeometry) -> Option<(f64, f64)> {
    if !pd.rows().any(|r| r == row) || pd.mask.is_some_and(|m| m.contains(&row)) {
        return None;
    }
    let (x, y) = (pd.x_at(row), pd.y_at(row));
    let (tx, ty) = (axis_value(geom.x.scale, x), axis_value(geom.y.scale, y));
    (x.is_finite() && y.is_finite() && tx.is_finite() && ty.is_finite()).then_some((x, y))
}

/// The slope between rows `row_a` and `row_b` of `pd`; `None` when a row is missing, masked, filtered
/// or not a number, or when the two X values are the same (no slope).
pub(crate) fn compute(pd: &PlotData, row_a: usize, row_b: usize, geom: &LayerGeometry) -> Option<Slope> {
    if !usable(pd) {
        return None;
    }
    let a = point(pd, row_a, geom)?;
    let b = point(pd, row_b, geom)?;
    let dx = axis_value(geom.x.scale, b.0) - axis_value(geom.x.scale, a.0);
    let dy = axis_value(geom.y.scale, b.1) - axis_value(geom.y.scale, a.1);
    if dx == 0.0 {
        return None;
    }
    let value = dy / dx;
    value.is_finite().then_some(Slope { a, b, value })
}

/// The label text: `slope = 1.234`, with the unit `<y unit>/<x unit>` appended when both parts are
/// known (a log axis supplies `dec`, `e-fold` or `oct` as its unit; the other axis' column must have units).
pub(crate) fn label(value: f64, project: &Project, pd: &PlotData, geom: &LayerGeometry) -> String {
    let column_unit = |id: Option<sheaf_core::ColumnId>| {
        id.and_then(|id| project.column(id)).map(|c| c.units.trim().to_string()).filter(|u| !u.is_empty())
    };
    let num = log_unit(geom.y.scale).map(str::to_string).or_else(|| column_unit(Some(pd.plot.y)));
    let den = log_unit(geom.x.scale).map(str::to_string).or_else(|| column_unit(pd.plot.x));
    let mut text = format!("slope = {}", sig_str(value, 4));
    // A unit needs a known numerator; with a log X axis its denominator is the decade (spec 17.6).
    if let (Some(n), Some(d)) = (num, den)
        && (column_unit(pd.plot.x).is_some() || log_unit(geom.x.scale).is_some())
    {
        text.push_str(&format!(" {n}/{d}"));
    }
    text
}

fn path(points: &[(f64, f64)]) -> String {
    let mut d = String::new();
    for (i, p) in points.iter().enumerate() {
        if i == 0 {
            move_to(&mut d, p.0, p.1);
        } else {
            line_to(&mut d, p.0, p.1);
        }
    }
    d
}

/// Draws the slope annotations of a layer into its overlay and records each as a pickable object.
#[allow(clippy::too_many_arguments)]
pub(crate) fn draw(
    out: &mut Vec<Item>,
    objects: &mut Vec<SceneObject>,
    project: &Project,
    layer: &Layer,
    index: usize,
    pds: &[PlotData],
    geom: &LayerGeometry,
    m: &dyn TextMeasurer,
) {
    if geom.polar.is_some() || geom.ternary.is_some() {
        return;
    }
    for (k, s) in layer.slopes.iter().enumerate() {
        let Some(pd) = pds.iter().find(|pd| pd.plot.y == s.plot && usable(pd)) else { continue };
        let Some(sl) = compute(pd, s.row_a, s.row_b, geom) else { continue };
        let (Some(pa), Some(pb)) =
            (geom.checked_data_to_page(sl.a.0, sl.a.1), geom.checked_data_to_page(sl.b.0, sl.b.1))
        else {
            continue;
        };
        let f = geom.frame;
        let inside = |p: (f64, f64)| {
            p.0 >= f.x - 1e-6 && p.0 <= f.x + f.w + 1e-6 && p.1 >= f.y - 1e-6 && p.1 <= f.y + f.h + 1e-6
        };
        if !inside(pa) || !inside(pb) {
            continue;
        }
        let color: Color = s.color.unwrap_or_else(|| crate::rug::plot_color(pd));
        let start = out.len();
        let width = s.width_pt.max(0.0);
        if s.show_legs {
            let leg = LineStyle { dash: Dash::Dash, ..LineStyle::solid(color, (width * 0.75).max(0.25)) };
            out.push(Item::Path { d: path(&[pa, (pb.0, pa.1), pb]), fill: None, stroke: Some(line_stroke(&leg)) });
        }
        if width > 0.0 {
            let style = LineStyle::solid(color, width);
            out.push(Item::Path { d: path(&[pa, pb]), fill: None, stroke: Some(line_stroke(&style)) });
        }
        // The label sits at the middle of the segment, on the upper side of it.
        let (dx, dy) = (pb.0 - pa.0, pb.1 - pa.1);
        let len = dx.hypot(dy).max(1e-9);
        let (mut nx, mut ny) = (-dy / len, dx / len);
        if ny > 0.0 || (ny == 0.0 && nx < 0.0) {
            (nx, ny) = (-nx, -ny);
        }
        let size = layer.legend.font.size_pt;
        let font = Font { color, ..layer.legend.font.clone() };
        let at = (
            (pa.0 + pb.0) / 2.0 + nx * LABEL_GAP + s.label_offset_pt.0,
            (pa.1 + pb.1) / 2.0 + ny * LABEL_GAP + s.label_offset_pt.1 + if ny > 0.5 { size } else { 0.0 },
        );
        crate::text::place(out, m, &label(sl.value, project, pd, geom), &font, size, at, Anchor::Middle, 0.0);
        hit::record(objects, out, start, ObjectKind::Slope(k), index, ItemList::Overlay, &[], m);
    }
}
