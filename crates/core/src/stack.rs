//! Where the plots of a layer are drawn when the layer is stacked (graphing spec section 13).
//!
//! Placement changes only the drawing: (x, y) is drawn at (mx * x + ox, my * y + oy).
//! The worksheet data are never changed. The renderer (`sheaf-plot`) and the Origin project writer
//! (`sheaf-io`) both place the plots with [`places`].

use crate::Project;
use crate::graph::{Layer, Plot, Stack};

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
