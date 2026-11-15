//! Where the plots of a layer are drawn when the layer is stacked (graphing spec section 13).
//!
//! Placement changes only the drawing: (x, y) is drawn at (mx * x + ox, my * y + oy).
//! The worksheet data are never changed. The renderer (`sheaf-plot`) and the Origin project writer
//! (`sheaf-io`) both place the plots with [`places`].

use crate::Project;
use crate::graph::{Layer, Plot, PlotKind, Stack};

/// Where one plot of a layer is drawn relative to its data.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Place {
    /// Offset added to every X value, in axis units.
    pub ox: f64,
    /// Offset added to every Y value, in axis units.
    pub oy: f64,
    /// Multiplier of each original X/Y value, before adding its offset.
    pub mx: f64,
    pub my: f64,
    /// The plot's place in the stack (counted from 0), or that of the curve it follows. `None` for a
    /// plot without one: its Y column holds no numbers, or the curve it follows is not plotted.
    pub member: Option<usize>,
}

impl Default for Place {
    fn default() -> Self {
        Self { ox: 0.0, oy: 0.0, mx: 1.0, my: 1.0, member: None }
    }
}

impl Place {
    /// Display X, preserving the exact source value for identity placement.
    pub fn x(self, value: f64) -> f64 {
        let value = if self.mx == 1.0 { value } else { value * self.mx };
        if self.ox == 0.0 { value } else { value + self.ox }
    }

    /// Display Y, preserving the exact source value for identity placement.
    pub fn y(self, value: f64) -> f64 {
        let value = if self.my == 1.0 { value } else { value * self.my };
        if self.oy == 0.0 { value } else { value + self.oy }
    }

    /// Original X under a display coordinate. Resolved multipliers are strictly positive.
    pub fn source_x(self, value: f64) -> f64 {
        let value = if self.ox == 0.0 { value } else { value - self.ox };
        if self.mx == 1.0 { value } else { value / self.mx }
    }

    /// Original Y under a display coordinate. Resolved multipliers are strictly positive.
    pub fn source_y(self, value: f64) -> f64 {
        let value = if self.oy == 0.0 { value } else { value - self.oy };
        if self.my == 1.0 { value } else { value / self.my }
    }
}

/// The places of the plots of a layer, one per plot in plot order, and the number of members of
/// the stack. Hidden plots are members too (spec 13.3), so hiding a curve does not move the others.
pub fn places(project: &Project, layer: &Layer) -> (Vec<Place>, usize) {
    let mut places = vec![Place::default(); layer.plots.len()];
    // The members, as plot indexes in plot order.
    let members: Vec<usize> = (0..layer.plots.len())
        .filter(|&i| {
            let y = layer.plots[i].y;
            project.numeric(y).is_some() && layer.follows(y).is_none()
        })
        .collect();
    let offsets: Vec<(f64, f64)> = match layer.stack {
        Stack::None | Stack::Cumulative | Stack::Waterfall { .. } => vec![(0.0, 0.0); members.len()],
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
        let (mx, my) = if layer.stack == Stack::Individual {
            layer.plot_offset(layer.plots[i].y).map_or((1.0, 1.0), |o| {
                // Origin 2016 retains non-positive settings but draws with a factor of one.
                let scale = |v: f64| if v.is_finite() && v > 0.0 { v } else { 1.0 };
                (scale(o.x_multiplier), scale(o.y_multiplier))
            })
        } else {
            (1.0, 1.0)
        };
        places[i] = Place { ox, oy, mx, my, member: Some(k) };
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

/// The Z value of a waterfall curve for Z colour mapping (graphing spec 13.17): the number in the long name of
/// its Y column, else the number in the column's comments, else the curve's place in the stack counted from 1.
/// Sheaf's choice; Origin takes Z from the plot's own Z settings (unmeasured).
pub fn waterfall_z(project: &Project, plot: &Plot, member: usize) -> f64 {
    let column = project.column(plot.y);
    let number = |text: &str| text.trim().parse::<f64>().ok().filter(|v| v.is_finite());
    column.and_then(|c| number(&c.long_name).or_else(|| number(&c.comments))).unwrap_or((member + 1) as f64)
}

/// True for the plot types that [`Stack::Cumulative`] adds up: lines, scatter and line + symbol.
fn cumulates(plot: &Plot) -> bool {
    matches!(plot.kind, PlotKind::Line | PlotKind::Scatter | PlotKind::LineSymbol)
}

/// True when `layer` has a plot that [`Stack::Cumulative`] would add up (a line, scatter or line + symbol plot).
pub fn layer_cumulates(layer: &Layer) -> bool {
    layer.plots.iter().any(cumulates)
}

/// What [`Stack::Cumulative`] adds to the Y values of each plot of `layer` (graphing spec 13.15),
/// one entry per plot in plot order, each a vector with one number per row of the plot's Y column;
/// drawn Y of a row is its Y plus this number. `None` for a plot that is not stacked this way: the
/// layer is not cumulative, the plot is not a line, scatter or line + symbol plot, or its Y column
/// holds no numbers (a follower of such a curve gets `None` too).
///
/// Sheaf's rules (not measured in Origin): the members are summed in plot order, pointwise by row
/// index, whatever their X values. A member contributes its Y at a row when the row is shown (in
/// the plot's row range, passing its row filters, and not masked while Hide Masked Points is on) and finite; otherwise it contributes 0, so one
/// missing value does not break the curves above it. A hidden member still contributes (spec
/// 13.3). A curve that follows a member (a fitted curve) is moved by the leader's added amount
/// at its own X, interpolated linearly along the leader's X (clamped at the ends).
pub fn cumulative_add(project: &Project, layer: &Layer) -> Vec<Option<Vec<f64>>> {
    let mut out: Vec<Option<Vec<f64>>> = vec![None; layer.plots.len()];
    if layer.stack != Stack::Cumulative {
        return out;
    }
    let mut running: Vec<f64> = Vec::new();
    let mut members: Vec<usize> = Vec::new();
    for (i, plot) in layer.plots.iter().enumerate() {
        let Some(y) = project.numeric(plot.y) else { continue };
        if layer.follows(plot.y).is_some() {
            continue;
        }
        members.push(i);
        if !cumulates(plot) {
            continue;
        }
        if running.len() < y.len() {
            running.resize(y.len(), 0.0);
        }
        let mut add = running.clone();
        add.resize(y.len(), 0.0);
        out[i] = Some(add);
        let filters = project.plot_row_filters(plot);
        let (start, end) = row_range(plot, y.len());
        // With Hide Masked Points a masked row is not drawn, so it adds nothing either.
        let hidden = project.column(plot.y).and_then(|c| c.active_mask()).filter(|_| project.hide_masked);
        for (r, &v) in y.iter().enumerate().take(end).skip(start) {
            if v.is_finite() && filters.iter().all(|f| f.passes(r)) && !hidden.is_some_and(|m| m.contains(&r)) {
                running[r] += v;
            }
        }
    }
    for (i, plot) in layer.plots.iter().enumerate() {
        let Some(leader) = layer.follows(plot.y) else { continue };
        let Some(y) = project.numeric(plot.y) else { continue };
        let Some(&j) = members.iter().find(|&&j| layer.plots[j].y == leader) else { continue };
        let Some(base) = &out[j] else { continue };
        let lead = &layer.plots[j];
        let lx = lead.x.and_then(|id| project.numeric(id));
        // The leader's curve of added amounts, sorted by X.
        let mut curve: Vec<(f64, f64)> = base
            .iter()
            .enumerate()
            .filter_map(|(r, &b)| {
                let x = lx.map_or((r + 1) as f64, |x| x.get(r).copied().unwrap_or(f64::NAN));
                (x.is_finite() && b.is_finite()).then_some((x, b))
            })
            .collect();
        curve.sort_by(|a, b| a.0.total_cmp(&b.0));
        let fx = plot.x.and_then(|id| project.numeric(id));
        let add = (0..y.len())
            .map(|r| {
                let x = fx.map_or((r + 1) as f64, |x| x.get(r).copied().unwrap_or(f64::NAN));
                interpolate(&curve, x)
            })
            .collect();
        out[i] = Some(add);
    }
    out
}

/// Linear interpolation of the sorted points `curve` at `x`, clamped at the ends; 0 without points
/// or for a missing `x`.
fn interpolate(curve: &[(f64, f64)], x: f64) -> f64 {
    let (Some(first), Some(last)) = (curve.first(), curve.last()) else { return 0.0 };
    if !x.is_finite() {
        return 0.0;
    }
    if x <= first.0 {
        return first.1;
    }
    if x >= last.0 {
        return last.1;
    }
    let k = curve.partition_point(|p| p.0 <= x);
    let (a, b) = (curve[k - 1], curve[k]);
    if b.0 == a.0 { b.1 } else { a.1 + (b.1 - a.1) * (x - a.0) / (b.0 - a.0) }
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
    let filters = project.plot_row_filters(plot);
    let (start, end) = row_range(plot, y.len());
    let mut r: Option<(f64, f64)> = None;
    for (i, &v) in y.iter().enumerate().take(end).skip(start) {
        if !filters.iter().all(|f| f.passes(i)) {
            continue;
        }
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

/// The first row and the row after the last that `plot` shows, within `len` rows.
pub fn row_range(plot: &Plot, len: usize) -> (usize, usize) {
    let (start, end) = match plot.rows {
        Some((s, e)) => (s.min(len), e.min(len)),
        None => (0, len),
    };
    (start, end.max(start))
}

#[cfg(test)]
mod place_tests {
    use super::Place;

    #[test]
    fn source_values_are_the_inverse_of_the_display_values() {
        let p = Place { ox: 5.0, oy: 7.0, mx: 2.0, my: 3.0, member: None };
        assert_eq!((p.x(2.0), p.y(4.0)), (9.0, 19.0));
        assert_eq!((p.source_x(9.0), p.source_y(19.0)), (2.0, 4.0));
        let id = Place::default();
        assert_eq!((id.source_x(0.1), id.source_y(0.3)), (0.1, 0.3), "identity keeps every digit");
    }
}
