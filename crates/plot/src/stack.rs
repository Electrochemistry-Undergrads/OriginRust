//! Offsets of stacked curves and the waterfall (graphing spec section 13).
//!
//! The offsets only move the drawing: every point (x, y) of a plot is drawn at (x + ox, y + oy).
//! The worksheet data are never changed.

use crate::Rect;
use crate::data::row_range;
use sheaf_core::Project;
use sheaf_core::graph::{Graph, Layer, Plot, Stack};

/// Where one plot of a layer is drawn relative to its data.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub(crate) struct Place {
    /// Offset added to every X value, in axis units.
    pub ox: f64,
    /// Offset added to every Y value, in axis units.
    pub oy: f64,
    /// The plot's place in the stack (counted from 0), or that of the curve it follows. `None` for a
    /// plot without one: its Y column holds no numbers, or the curve it follows is not plotted.
    pub member: Option<usize>,
}

/// The places of the plots of a layer, one per plot in plot order, and the number of members of
/// the stack. Hidden plots are members too (spec 13.3), so hiding a curve does not move the others.
pub(crate) fn places(project: &Project, layer: &Layer) -> (Vec<Place>, usize) {
    let mut places = vec![Place::default(); layer.plots.len()];
    // The members, as plot indexes in plot order.
    let members: Vec<usize> = (0..layer.plots.len())
        .filter(|&i| {
            let y = layer.plots[i].y;
            project.numeric(y).is_some() && layer.follows(y).is_none()
        })
        .collect();
    let offsets: Vec<(f64, f64)> = match layer.stack {
        Stack::None | Stack::Waterfall { .. } => vec![(0.0, 0.0); members.len()],
        Stack::Constant { step } => (0..members.len()).map(|k| (0.0, finite(k as f64 * finite(step)))).collect(),
        Stack::Individual => members
            .iter()
            .map(|&i| layer.plot_offset(layer.plots[i].y).map_or((0.0, 0.0), |o| (finite(o.x), finite(o.y))))
            .collect(),
        Stack::Auto { gap_percent } => {
            let extents: Vec<Option<(f64, f64)>> = members.iter().map(|&i| extent(project, &layer.plots[i])).collect();
            auto(&extents, finite(gap_percent))
        }
    };
    for (k, (&i, &(ox, oy))) in members.iter().zip(&offsets).enumerate() {
        places[i] = Place { ox, oy, member: Some(k) };
    }
    // A follower takes the place of the first member whose Y column it follows.
    for (i, plot) in layer.plots.iter().enumerate() {
        if let Some(leader) = layer.follows(plot.y)
            && let Some(&j) = members.iter().find(|&&j| layer.plots[j].y == leader)
        {
            places[i] = places[j];
        }
    }
    (places, members.len())
}

/// The offsets of the members in automatic mode (spec 13.4.1), from their extents.
fn auto(extents: &[Option<(f64, f64)>], gap_percent: f64) -> Vec<(f64, f64)> {
    let tallest = extents.iter().flatten().map(|(lo, hi)| hi - lo).fold(0.0, f64::max);
    let height = if tallest > 0.0 && tallest.is_finite() { tallest } else { 10.0 };
    let gap = gap_percent * height / 100.0;
    let mut top: Option<f64> = None;
    extents
        .iter()
        .map(|extent| {
            let Some((lo, hi)) = *extent else { return (0.0, 0.0) };
            let oy = match top {
                None => 0.0,
                Some(t) => finite(t + gap - lo),
            };
            top = Some(hi + oy);
            (0.0, oy)
        })
        .collect()
}

/// The smallest and largest Y of a plot over its plotted rows where X and Y are both finite
/// (without an X column, X is the row number).
fn extent(project: &Project, plot: &Plot) -> Option<(f64, f64)> {
    let y = project.numeric(plot.y)?;
    let x = plot.x.and_then(|id| project.numeric(id));
    let (start, end) = row_range(plot, y.len());
    let mut r: Option<(f64, f64)> = None;
    for (i, &v) in y.iter().enumerate().take(end).skip(start) {
        let xv = x.map_or((i + 1) as f64, |x| x.get(i).copied().unwrap_or(f64::NAN));
        if xv.is_finite() && v.is_finite() {
            r = Some(r.map_or((v, v), |(lo, hi)| (lo.min(v), hi.max(v))));
        }
    }
    r
}

/// A stored number that is not finite counts as 0.
fn finite(v: f64) -> f64 {
    if v.is_finite() { v } else { 0.0 }
}

/// The waterfall shift of layer `index` as fractions of the front frame `(sx, sy)`, when the layer
/// is drawn as a waterfall: its mode is [`Stack::Waterfall`] and it is neither linked to another
/// layer nor has linked layers. Otherwise `None`.
pub(crate) fn waterfall(g: &Graph, index: usize) -> Option<(f64, f64)> {
    let Stack::Waterfall { dx_percent, dy_percent } = g.layers.get(index)?.stack else { return None };
    (g.group(index).len() == 1).then(|| (finite(dx_percent) / 100.0, finite(dy_percent) / 100.0))
}

/// The front frame of a waterfall with `n` members inside the layer frame `f` (spec 13.6): the
/// frame shrinks so that the last member, moved by `(n - 1)` shifts, still ends at the edge of `f`.
pub(crate) fn front_frame(f: Rect, n: usize, sx: f64, sy: f64) -> Rect {
    if n <= 1 {
        return f;
    }
    let k = (n - 1) as f64;
    let w = f.w / (1.0 + k * sx.abs());
    let h = f.h / (1.0 + k * sy.abs());
    let x = if sx >= 0.0 { f.x } else { f.x + f.w - w };
    let y = if sy >= 0.0 { f.y + f.h - h } else { f.y };
    Rect { x, y, w, h }
}
