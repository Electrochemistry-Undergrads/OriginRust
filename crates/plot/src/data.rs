//! Reading the numbers of the plots of a layer.

use crate::ticks::{ResolvedAxis, resolve_axis};
use crate::{LayerGeometry, Rect};
use sheaf_core::Project;
use sheaf_core::graph::{Layer, Page, Plot, ScaleKind};

/// A visible plot whose Y column holds numbers, with its data resolved.
pub(crate) struct PlotData<'a> {
    pub plot: &'a Plot,
    /// X values; `None` plots against the row number (1, 2, 3, ...).
    pub x: Option<&'a [f64]>,
    pub y: &'a [f64],
    pub err: Option<&'a [f64]>,
    /// Half-open row range that is drawn.
    pub start: usize,
    pub end: usize,
}

impl PlotData<'_> {
    /// X value of row `i` (NaN when the X column is shorter than the Y column).
    pub fn x_at(&self, i: usize) -> f64 {
        match self.x {
            None => (i + 1) as f64,
            Some(x) => x.get(i).copied().unwrap_or(f64::NAN),
        }
    }

    pub fn err_at(&self, i: usize) -> f64 {
        self.err.and_then(|e| e.get(i)).copied().unwrap_or(f64::NAN)
    }
}

/// The drawable plots of a layer, in plot order. A plot whose Y column is
/// missing or not numeric is skipped.
pub(crate) fn collect<'a>(project: &'a Project, layer: &'a Layer) -> Vec<PlotData<'a>> {
    let mut out = Vec::new();
    for plot in layer.plots.iter().filter(|p| p.visible) {
        let Some(y) = project.numeric(plot.y) else { continue };
        let x = plot.x.and_then(|id| project.numeric(id));
        let err = plot.y_error.and_then(|id| project.numeric(id));
        let (start, end) = match plot.rows {
            Some((s, e)) => (s.min(y.len()), e.min(y.len())),
            None => (0, y.len()),
        };
        out.push(PlotData { plot, x, y, err, start, end: end.max(start) });
    }
    out
}

/// Finite data range per axis over all rows where both X and Y are finite.
/// On a log axis only positive values count.
fn data_range(pds: &[PlotData], x_scale: ScaleKind, y_scale: ScaleKind) -> (Option<(f64, f64)>, Option<(f64, f64)>) {
    let mut xr: Option<(f64, f64)> = None;
    let mut yr: Option<(f64, f64)> = None;
    let widen = |r: &mut Option<(f64, f64)>, v: f64| {
        *r = Some(match *r {
            None => (v, v),
            Some((lo, hi)) => (lo.min(v), hi.max(v)),
        });
    };
    for pd in pds {
        for i in pd.start..pd.end {
            let (x, y) = (pd.x_at(i), pd.y[i]);
            if !(x.is_finite() && y.is_finite()) {
                continue;
            }
            if x_scale == ScaleKind::Linear || x > 0.0 {
                widen(&mut xr, x);
            }
            if y_scale == ScaleKind::Linear || y > 0.0 {
                widen(&mut yr, y);
            }
        }
    }
    (xr, yr)
}

/// Frame rectangle in points and the two resolved axes of a layer.
pub(crate) fn resolve_layer(layer: &Layer, page: &Page, pds: &[PlotData]) -> LayerGeometry {
    let f = layer.frame;
    let frame = Rect {
        x: f.left * page.width_pt,
        y: f.top * page.height_pt,
        w: f.width * page.width_pt,
        h: f.height * page.height_pt,
    };
    let (xr, yr) = data_range(pds, layer.x_axis.scale, layer.y_axis.scale);
    let x: ResolvedAxis = resolve_axis(&layer.x_axis, xr);
    let y: ResolvedAxis = resolve_axis(&layer.y_axis, yr);
    LayerGeometry { frame, x, y }
}
