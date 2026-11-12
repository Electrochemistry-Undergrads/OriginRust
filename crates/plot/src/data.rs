//! Reading the numbers of the plots of a layer.

use crate::{Rect, stack};
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
    /// Offsets of a stacked layer, in axis units, added to every point (graphing spec 13).
    pub ox: f64,
    pub oy: f64,
    /// The plot's place in the stack, for the waterfall shift.
    pub member: Option<usize>,
}

impl PlotData<'_> {
    /// X value of row `i` as drawn, with the stack offset (NaN when the X column is shorter than
    /// the Y column).
    pub fn x_at(&self, i: usize) -> f64 {
        let x = match self.x {
            None => (i + 1) as f64,
            Some(x) => x.get(i).copied().unwrap_or(f64::NAN),
        };
        // Adding 0 would turn -0 into +0; leave unstacked values exactly as they are.
        if self.ox == 0.0 { x } else { x + self.ox }
    }

    /// Y value of row `i` as drawn, with the stack offset.
    pub fn y_at(&self, i: usize) -> f64 {
        if self.oy == 0.0 { self.y[i] } else { self.y[i] + self.oy }
    }

    pub fn err_at(&self, i: usize) -> f64 {
        self.err.and_then(|e| e.get(i)).copied().unwrap_or(f64::NAN)
    }
}

/// The plots of a layer as they are drawn.
pub(crate) struct LayerData<'a> {
    /// The visible plots whose Y column holds numbers, in plot order.
    pub plots: Vec<PlotData<'a>>,
    /// The number of members of the layer's stack, hidden plots included.
    pub members: usize,
}

/// The half-open row range of a plot whose Y column has `len` rows, clamped to the column.
pub(crate) fn row_range(plot: &Plot, len: usize) -> (usize, usize) {
    let (start, end) = match plot.rows {
        Some((s, e)) => (s.min(len), e.min(len)),
        None => (0, len),
    };
    (start, end.max(start))
}

/// The drawable plots of a layer, in plot order, with their stack offsets. A plot that is hidden
/// or whose Y column is missing or not numeric is skipped (a hidden plot still keeps its place in
/// the stack).
pub(crate) fn collect<'a>(project: &'a Project, layer: &'a Layer) -> LayerData<'a> {
    let (places, members) = stack::places(project, layer);
    let mut plots = Vec::new();
    for (plot, place) in layer.plots.iter().zip(places).filter(|(p, _)| p.visible) {
        let Some(y) = project.numeric(plot.y) else { continue };
        let x = plot.x.and_then(|id| project.numeric(id));
        let err = plot.y_error.and_then(|id| project.numeric(id));
        let (start, end) = row_range(plot, y.len());
        plots.push(PlotData { plot, x, y, err, start, end, ox: place.ox, oy: place.oy, member: place.member });
    }
    LayerData { plots, members }
}

/// Which coordinate of the plots an axis shows.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum Dim {
    X,
    Y,
}

/// Finite data range of one coordinate, as drawn (with the stack offsets), over all rows where
/// both X and Y are finite. On a log axis only positive values count.
pub(crate) fn axis_range<'p, 'a: 'p>(
    pds: impl IntoIterator<Item = &'p PlotData<'a>>,
    dim: Dim,
    scale: ScaleKind,
) -> Option<(f64, f64)> {
    let mut r: Option<(f64, f64)> = None;
    for pd in pds {
        for i in pd.start..pd.end {
            let (x, y) = (pd.x_at(i), pd.y_at(i));
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
