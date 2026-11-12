//! Reading the numbers of the plots of a layer.

use crate::Rect;
use sheaf_core::Project;
use sheaf_core::graph::{Frame, Layer, Page, Plot, ScaleKind};

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

/// Which coordinate of the plots an axis shows.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum Dim {
    X,
    Y,
}

/// Finite data range of one coordinate over all rows where both X and Y are
/// finite. On a log axis only positive values count.
pub(crate) fn axis_range<'p, 'a: 'p>(
    pds: impl IntoIterator<Item = &'p PlotData<'a>>,
    dim: Dim,
    scale: ScaleKind,
) -> Option<(f64, f64)> {
    let mut r: Option<(f64, f64)> = None;
    for pd in pds {
        for i in pd.start..pd.end {
            let (x, y) = (pd.x_at(i), pd.y[i]);
            if !(x.is_finite() && y.is_finite()) {
                continue;
            }
            let v = if dim == Dim::X { x } else { y };
            if scale == ScaleKind::Linear || v > 0.0 {
                r = Some(match r {
                    None => (v, v),
                    Some((lo, hi)) => (lo.min(v), hi.max(v)),
                });
            }
        }
    }
    r
}

/// A frame given as fractions of the page, in points.
pub(crate) fn page_frame(f: &Frame, page: &Page) -> Rect {
    Rect {
        x: f.left * page.width_pt,
        y: f.top * page.height_pt,
        w: f.width * page.width_pt,
        h: f.height * page.height_pt,
    }
}

/// A frame given as fractions of the parent layer's frame `parent` (a linked layer), in points.
pub(crate) fn linked_frame(f: &Frame, parent: Rect) -> Rect {
    Rect {
        x: parent.x + f.left * parent.w,
        y: parent.y + f.top * parent.h,
        w: f.width * parent.w,
        h: f.height * parent.h,
    }
}
