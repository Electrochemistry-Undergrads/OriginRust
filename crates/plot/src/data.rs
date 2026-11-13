//! Reading the numbers of the plots of a layer.

use crate::{Rect, stack};
use sheaf_core::datetime::DatePattern;
use sheaf_core::format::NumberFormat;
use sheaf_core::graph::{
    Axis, BarSpacing, ErrorDirection, Frame, Layer, Page, Plot, PlotKind, ScaleKind, TickFormat, TickLabelSource,
};
use sheaf_core::{Column, Project};

/// Where the bars of one column plot stand at each X position (graphing spec 14.4), in X axis units.
#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) struct BarSlot {
    /// Left edge of the bar, relative to the row's X.
    pub left: f64,
    /// Width of the bar.
    pub width: f64,
    /// The X spacing s of the layer's bars.
    pub spacing: f64,
}

impl BarSlot {
    /// The middle of the bar, relative to the row's X; error bars are drawn there.
    pub fn centre(&self) -> f64 {
        self.left + self.width / 2.0
    }
}

/// A visible plot whose Y column holds numbers, with its data resolved.
pub(crate) struct PlotData<'a> {
    pub plot: &'a Plot,
    /// X values; `None` plots against the row number (1, 2, 3, ...).
    pub x: Option<&'a [f64]>,
    /// How the X column shows its values (a date format makes a date axis).
    pub x_format: Option<NumberFormat>,
    pub y: &'a [f64],
    pub err: Option<&'a [f64]>,
    /// X error values, drawn as horizontal error bars.
    pub xerr: Option<&'a [f64]>,
    /// The lower ends of the bars of a floating column plot.
    pub base: Option<&'a [f64]>,
    /// Bottom and top of every bar of a stacked column plot, per row (100 % plots scaled).
    pub stack: Option<(Vec<f64>, Vec<f64>)>,
    /// Half-open row range that is drawn.
    pub start: usize,
    pub end: usize,
    /// Offsets of a stacked layer, in axis units, added to every point (graphing spec 13).
    pub ox: f64,
    pub oy: f64,
    /// The plot's place in the stack, for the waterfall shift.
    pub member: Option<usize>,
    /// Where the bars of a column plot stand; `None` for other kinds.
    pub bar: Option<BarSlot>,
}

impl PlotData<'_> {
    /// True for horizontal bars: the horizontal axis shows the values and the vertical axis the
    /// bar positions.
    pub fn transposed(&self) -> bool {
        self.plot.kind.is_horizontal()
    }

    /// X value of row `i` as drawn, with the stack offset (NaN when the X column is shorter than
    /// the Y column). For horizontal bars this is the bar's value.
    pub fn x_at(&self, i: usize) -> f64 {
        if self.transposed() { self.val_at(i) } else { self.cat_at(i) }
    }

    /// Y value of row `i` as drawn, with the stack offset. For horizontal bars this is the bar's
    /// position.
    pub fn y_at(&self, i: usize) -> f64 {
        if self.transposed() { self.cat_at(i) } else { self.val_at(i) }
    }

    /// The position of row `i` along the X column (the row number without one), with its offset.
    pub fn cat_at(&self, i: usize) -> f64 {
        let x = match self.x {
            None => (i + 1) as f64,
            Some(x) => x.get(i).copied().unwrap_or(f64::NAN),
        };
        // Adding 0 would turn -0 into +0; leave unstacked values exactly as they are.
        if self.ox == 0.0 { x } else { x + self.ox }
    }

    /// The value of row `i` as drawn: the Y value with the stack offset, or the top of a stacked bar.
    pub fn val_at(&self, i: usize) -> f64 {
        if let Some((_, hi)) = &self.stack {
            return hi.get(i).copied().unwrap_or(f64::NAN);
        }
        if self.oy == 0.0 { self.y[i] } else { self.y[i] + self.oy }
    }

    /// The lower end of a bar of a stacked or floating column plot at row `i`, when it has one.
    pub fn base_at(&self, i: usize) -> Option<f64> {
        if let Some((lo, _)) = &self.stack {
            return lo.get(i).copied();
        }
        let b = self.base?.get(i).copied()?;
        Some(if self.oy == 0.0 { b } else { b + self.oy })
    }

    pub fn xerr_at(&self, i: usize) -> f64 {
        self.xerr.and_then(|e| e.get(i)).copied().unwrap_or(f64::NAN)
    }

    /// The reach of an error bar below and above its point, as non-negative numbers (0 on the
    /// side the direction leaves out); `None` when the error value is not finite.
    pub fn reach(&self, e: f64, direction: ErrorDirection) -> Option<(f64, f64)> {
        if !e.is_finite() {
            return None;
        }
        let e = e.abs();
        Some(match direction {
            ErrorDirection::Both => (e, e),
            ErrorDirection::Plus => (0.0, e),
            ErrorDirection::Minus => (e, 0.0),
        })
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

pub(crate) use sheaf_core::stack::row_range;

/// The drawable plots of a layer, in plot order, with their stack offsets. A plot that is hidden
/// or whose Y column is missing or not numeric is skipped (a hidden plot still keeps its place in
/// the stack).
pub(crate) fn collect<'a>(project: &'a Project, layer: &'a Layer) -> LayerData<'a> {
    let (places, members) = stack::places(project, layer);
    let mut plots = Vec::new();
    for (plot, place) in layer.plots.iter().zip(places).filter(|(p, _)| p.visible) {
        let Some(y) = project.numeric(plot.y) else { continue };
        let x = plot.x.and_then(|id| project.numeric(id));
        let x_format = plot.x.and_then(|id| project.column(id)).map(|c| c.format);
        let err = plot.y_error.and_then(|id| project.numeric(id));
        let xerr = plot.x_error.and_then(|id| project.numeric(id));
        let base = plot.y_base.and_then(|id| project.numeric(id));
        let (start, end) = row_range(plot, y.len());
        plots.push(PlotData {
            plot,
            x,
            x_format,
            y,
            err,
            xerr,
            base,
            stack: None,
            start,
            end,
            ox: place.ox,
            oy: place.oy,
            member: place.member,
            bar: None,
        });
    }
    stack_bars(&mut plots);
    place_bars(&mut plots, &layer.bars);
    LayerData { plots, members }
}

/// A percentage of the model clamped to `lo..=hi`; a value that is not finite takes `default`.
fn percent(v: f64, lo: f64, hi: f64, default: f64) -> f64 {
    if v.is_finite() { v.clamp(lo, hi) } else { default }
}

/// Gives every column plot of a layer its place in the group of bars (graphing spec 14.4): the bars
/// at one X position together take `(1 - gap) * s` and stand side by side in plot order, each
/// `w` wide and `w * (1 - overlap)` from the one before it.
fn place_bars(plots: &mut [PlotData], bars: &BarSpacing) {
    let columns: Vec<usize> = (0..plots.len()).filter(|&i| plots[i].plot.kind.is_bar()).collect();
    let Some(&first) = columns.first() else { return };
    let s = bar_spacing(&plots[first]);
    let gap = percent(bars.gap_pct, 0.0, 100.0, sheaf_core::graph::DEFAULT_BAR_GAP_PCT) / 100.0;
    let overlap = percent(bars.overlap_pct, -100.0, 100.0, 0.0) / 100.0;
    // Stacked plots share one slot, which stands where the first of them stands.
    let mut slot_of = vec![0usize; plots.len()];
    let mut slots = 0usize;
    let mut stacked_slot = None;
    for &i in &columns {
        if plots[i].plot.kind.is_stacked() {
            slot_of[i] = *stacked_slot.get_or_insert_with(|| {
                slots += 1;
                slots - 1
            });
        } else {
            slot_of[i] = slots;
            slots += 1;
        }
    }
    let n = slots as f64;
    let group = (1.0 - gap) * s;
    let width = group / (1.0 + (n - 1.0) * (1.0 - overlap));
    let step = width * (1.0 - overlap);
    for &i in &columns {
        plots[i].bar = Some(BarSlot { left: -group / 2.0 + slot_of[i] as f64 * step, width, spacing: s });
    }
}

/// Piles the stacked column plots of a layer: the bar of each plot starts where the bars of the
/// plots before it end, row by row (a missing value counts as 0). With a 100 % plot among them
/// every pile is scaled to a total of 100.
fn stack_bars(plots: &mut [PlotData]) {
    let idx: Vec<usize> = (0..plots.len()).filter(|&i| plots[i].plot.kind.is_stacked()).collect();
    if idx.is_empty() {
        return;
    }
    let rows = idx.iter().map(|&i| plots[i].y.len()).max().unwrap_or(0);
    let percent = idx.iter().any(|&i| plots[i].plot.kind == PlotKind::Stacked100Column);
    let value = |pd: &PlotData, r: usize| pd.y.get(r).copied().filter(|v| v.is_finite()).unwrap_or(0.0);
    let totals: Vec<f64> = (0..rows).map(|r| idx.iter().map(|&i| value(&plots[i], r)).sum()).collect();
    let mut run = vec![0.0; rows];
    for &i in &idx {
        let (mut lo, mut hi) = (vec![f64::NAN; rows], vec![f64::NAN; rows]);
        for r in 0..rows {
            let scale = if !percent {
                1.0
            } else if totals[r] != 0.0 {
                100.0 / totals[r]
            } else {
                0.0
            };
            let shown = plots[i].y.get(r).is_some_and(|v| v.is_finite());
            lo[r] = run[r] * scale;
            run[r] += value(&plots[i], r);
            if shown {
                hi[r] = run[r] * scale;
            }
        }
        plots[i].stack = Some((lo, hi));
    }
}

/// The X spacing s of a column plot: 1 without an X column of numbers (the bars stand at row numbers),
/// otherwise the mean distance between X values, `(largest X - smallest X) / (n - 1)`, where the X
/// values are those of the drawn rows and n counts the rows of the row range that have a Y value (an
/// empty row between two groups of bars does not count, a row with only its X missing does); 1 when
/// that is not a number above 0.
fn bar_spacing(pd: &PlotData) -> f64 {
    if pd.x.is_none() {
        return 1.0;
    }
    let rows = (pd.start..pd.end).filter(|&i| pd.val_at(i).is_finite()).count();
    let xs = (pd.start..pd.end).filter(|&i| pd.val_at(i).is_finite()).map(|i| pd.cat_at(i)).filter(|x| x.is_finite());
    let (lo, hi) = xs.fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), x| (lo.min(x), hi.max(x)));
    let s = (hi - lo) / (rows as f64 - 1.0);
    if rows >= 2 && s.is_finite() && s > 0.0 { s } else { 1.0 }
}

/// The X column of a plot when it holds text (sample names).
fn text_x_column<'a>(project: &'a Project, pd: &PlotData) -> Option<&'a Column> {
    pd.plot.x.and_then(|id| project.column(id)).filter(|c| !c.data.is_numeric())
}

/// The column whose cells label the major ticks of an axis (graphing spec 14.3), if any. `first`
/// is the first plot the axis shows.
pub(crate) fn label_column<'a>(
    project: &'a Project,
    axis: &Axis,
    dim: Dim,
    first: Option<&PlotData>,
) -> Option<&'a Column> {
    match axis.tick_label_source {
        TickLabelSource::Numbers => None,
        TickLabelSource::Column { column } => project.column(column),
        TickLabelSource::Auto if dim == Dim::X => text_x_column(project, first?),
        TickLabelSource::Auto => None,
    }
}

/// Whether a linear X axis is a category axis (graphing spec 14.3): its first plot has no X column
/// of numbers, and that plot's X column holds text or the labels come from a chosen column. Its
/// rows then stand at the row numbers 1, 2, 3, ... and the axis gets half a step of room at each end.
pub(crate) fn is_category(project: &Project, axis: &Axis, first: Option<&PlotData>) -> bool {
    let Some(pd) = first else { return false };
    if axis.scale != ScaleKind::Linear || pd.x.is_some() {
        return false;
    }
    text_x_column(project, pd).is_some()
        || matches!(axis.tick_label_source, TickLabelSource::Column { column } if project.column(column).is_some())
}

/// The cells of a label column as they show in the worksheet.
pub(crate) fn cell_texts(column: &Column) -> Vec<String> {
    (0..column.len()).map(|row| column.cell_text(row)).collect()
}

/// Which coordinate of the plots an axis shows.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum Dim {
    X,
    Y,
}

/// Finite data range of one coordinate, as drawn (with the stack offsets), over all rows where
/// both X and Y are finite. The ends of error bars and the lower ends of floating and stacked
/// bars count too (Origin's automatic range reaches the ends of the error bars). On a log axis
/// only positive values count.
pub(crate) fn axis_range<'p, 'a: 'p>(
    pds: impl IntoIterator<Item = &'p PlotData<'a>>,
    dim: Dim,
    scale: ScaleKind,
) -> Option<(f64, f64)> {
    let mut r: Option<(f64, f64)> = None;
    let mut add = |v: f64| {
        if v.is_finite() && (scale == ScaleKind::Linear || v > 0.0) {
            r = Some(match r {
                None => (v, v),
                Some((lo, hi)) => (lo.min(v), hi.max(v)),
            });
        }
    };
    for pd in pds {
        let dir = pd.plot.error.direction;
        let transposed = pd.transposed();
        let value_dim = if transposed { Dim::X } else { Dim::Y };
        for i in pd.start..pd.end {
            let (x, y) = (pd.x_at(i), pd.y_at(i));
            if !(x.is_finite() && y.is_finite()) {
                continue;
            }
            add(if dim == Dim::X { x } else { y });
            if dim == value_dim {
                let v = if transposed { x } else { y };
                if let Some((below, above)) = pd.reach(pd.err_at(i), dir) {
                    add(v - below);
                    add(v + above);
                }
                if let Some(b) = pd.base_at(i) {
                    add(b);
                }
            }
            if dim == Dim::X
                && !pd.plot.kind.is_bar()
                && let Some((below, above)) = pd.reach(pd.xerr_at(i), dir)
            {
                add(x - below);
                add(x + above);
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

/// The X axis as it is drawn when it differs from the stored one: an X column in a date, time or
/// date-time format gives date tick labels, unless the axis has a tick format of its own.
pub(crate) fn date_axis<'p, 'a: 'p>(axis: &Axis, pds: impl IntoIterator<Item = &'p PlotData<'a>>) -> Option<Axis> {
    if axis.tick_format != TickFormat::Auto {
        return None;
    }
    let pattern = pds.into_iter().find_map(|pd| match pd.x_format {
        Some(NumberFormat::Date(d) | NumberFormat::DateTime(d, _)) => Some(d),
        Some(NumberFormat::Time(_)) => Some(DatePattern::default()),
        _ => None,
    })?;
    let mut drawn = axis.clone();
    drawn.tick_format = TickFormat::DateTime(pattern);
    Some(drawn)
}
