//! Reading the numbers of the plots of a layer.

use crate::{Rect, stack};
use sheaf_core::datetime::DatePattern;
use sheaf_core::format::NumberFormat;
use sheaf_core::graph::{
    Axis, BarSpacing, ErrorDirection, Frame, LabelForm, Layer, Page, Plot, PlotKind, ScaleKind, StatPlot, TickFormat,
    TickLabelSource,
};
use sheaf_core::{Column, Project};
use std::borrow::Cow;

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
    pub x: Option<Cow<'a, [f64]>>,
    /// How the X column shows its values (a date format makes a date axis).
    pub x_format: Option<NumberFormat>,
    pub y: Cow<'a, [f64]>,
    /// Rows of the Y column that are masked (none while masking is disabled): drawn in
    /// `mask_color`, red unless Column > Mask > Change Color chose another, as Origin does.
    pub mask: Option<&'a std::collections::BTreeSet<usize>>,
    pub mask_color: sheaf_core::Color,
    /// Active filters on the source worksheets. Indices remain original worksheet rows.
    pub filters: Vec<&'a sheaf_core::rowfilter::RowFilter>,
    pub err: Option<Cow<'a, [f64]>>,
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
    /// Individual plot multipliers, applied before offsets. Errors and label values stay unscaled.
    pub mx: f64,
    pub my: f64,
    /// The plot's place in the stack, for the waterfall shift.
    pub member: Option<usize>,
    /// Where the bars of a column plot stand; `None` for other kinds.
    pub bar: Option<BarSlot>,
    /// The statistics a box chart plot draws; `None` for other kinds.
    pub boxed: Option<BoxStats>,
    /// For a column scatter plot, `(position, column name)`: the plot stands at 1, 2, 3, ... in plot order
    /// and its X values are the position plus a jitter; `None` for other kinds.
    pub column_at: Option<(f64, String)>,
    /// The boxes of a grouped box chart plot of indexed data (T76), one per level of the main group present in
    /// its subset, at dodged positions; empty for other plots.
    pub gboxes: Vec<BoxStats>,
    /// The names of the categories at 1, 2, ... of a plot of indexed data (the levels of its main group).
    pub categories: Option<Vec<String>>,
    /// The name of the one position of an interval plot point (T83): its column's name.
    pub slot_label: Option<String>,
    /// The density silhouette a violin plot draws (its box statistics are in `boxed`); `None`
    /// for other kinds.
    pub violin: Option<crate::violin::ViolinData>,
    /// The density curve (or bars) a ridgeline plot draws; `None` for other kinds.
    pub ridge: Option<crate::ridgeline::RidgeData>,
    /// What a pie chart plot draws; `None` for other kinds.
    pub pie: Option<PieData>,
    /// The values that set the size of each symbol of a bubble plot.
    pub size_z: Option<&'a [f64]>,
    /// The values that set the colour of each symbol of a colour mapped plot.
    pub color_z: Option<&'a [f64]>,
    /// The spoke names of a plot of a radar layer (graphing spec 14.19); `None` elsewhere.
    pub spokes: Option<Vec<String>>,
    /// Source row repeated at the end to close a radar polygon. The line runs to the synthetic
    /// point, but no extra symbol, label or mask belongs to it.
    pub closing: Option<usize>,
    /// The cells of the label column of a plot whose data labels come from a column, one per row.
    pub label_texts: Option<Vec<String>>,
    /// The observations a histogram plot bins (shown, unmasked rows of its Y column), kept for rugs
    /// because `x`/`y` hold bin centres and counts; `None` for other kinds.
    pub hist_raw: Option<Vec<f64>>,
    /// The points a hexbin plot bins and their first binning; `None` for other kinds.
    pub hex: Option<crate::hexbin::HexData>,
    /// The arrows of a vector plot (T107); `None` for other kinds.
    pub vector: Option<crate::vector_plot::VectorData>,
    /// The prices of a stock plot (T108); `None` for other kinds.
    pub stock: Option<crate::stock_plot::StockData>,
    /// The bins or box of a marginal plot (T110); `None` for other kinds.
    pub marginal: Option<crate::marginal_plot::MarginalData>,
    /// The sectors of a wind rose band (T113); `None` for other kinds.
    pub wind_rose: Option<crate::wind_rose::WindRoseData>,
    /// The density grid of a 2D kernel density plot (T116); `None` for other kinds.
    pub kde2d: Option<crate::kde2d::Kde2dData>,
    /// The samples of a Piper diagram (T117); `None` for other kinds.
    pub piper: Option<crate::piper_plot::PiperData>,
    /// The layer's Data > Analysis Markers settings (T135).
    pub marker_display: sheaf_core::data_markers::MarkerDisplay,
    /// The lines and axes of a layer's parallel plots, held by the first of them; `None` for other plots.
    pub parallel: Option<Box<crate::parallel::ParallelData>>,
    /// What a treemap plot draws; `None` for other kinds.
    pub treemap: Option<Box<crate::treemap::TreemapData>>,
    /// What a sunburst plot draws; `None` for other kinds.
    pub sunburst: Option<Box<crate::sunburst::SunburstData>>,
    /// The colour mapping of a waterfall curve; `None` when the layer has none.
    pub wf: Option<WaterfallColors>,
}

/// How one curve of a colour mapped waterfall is coloured: `lo` and `hi` are the ends of the colour scale (over
/// the whole layer) and `z` the curve's own value for Z mapping.
#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) struct WaterfallColors {
    pub mode: sheaf_core::graph::WaterfallColor,
    pub lo: f64,
    pub hi: f64,
    pub z: f64,
}

impl WaterfallColors {
    /// Where `v` is on the scale, 0..1 (0.5 when the scale has no width).
    pub fn fraction(&self, v: f64) -> f64 {
        if self.hi > self.lo { ((v - self.lo) / (self.hi - self.lo)).clamp(0.0, 1.0) } else { 0.5 }
    }
}

/// The numbers of one box of a box chart (Origin's defaults: the box runs from the 25th to the
/// 75th percentile, the whiskers to the furthest values within 1.5 times the box height, the other
/// values are outliers).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BoxStats {
    /// Where the box stands on the X axis (1, 2, 3, ...).
    pub pos: f64,
    /// The tick label under the box.
    pub label: String,
    pub q1: f64,
    pub median: f64,
    pub q3: f64,
    pub mean: f64,
    pub whisker_low: f64,
    pub whisker_high: f64,
    pub outliers: Vec<f64>,
    /// The width of the box in X axis units ([`BOX_WIDTH`], less in a grouped box chart).
    pub width: f64,
}

/// The slices of a pie chart plot.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PieData {
    /// The value of each slice (rows that are not positive numbers are left out).
    pub values: Vec<f64>,
    /// The legend text of each slice: the X column's cell, or the row number.
    pub names: Vec<String>,
    pub colors: Vec<sheaf_core::Color>,
    /// The font of the percentages and of the legend.
    pub label_font: sheaf_core::graph::Font,
    pub legend_font: sheaf_core::graph::Font,
}

/// Width of a box, in X axis units (measured: 0.8 of the spacing of the boxes).
pub(crate) const BOX_WIDTH: f64 = 0.8;

/// The statistics of one box from the finite values of a column, `None` without any.
pub(crate) fn box_stats(values: &[f64], pos: f64, label: String) -> Option<BoxStats> {
    let mut v: Vec<f64> = values.iter().copied().filter(|x| x.is_finite()).collect();
    if v.is_empty() {
        return None;
    }
    v.sort_by(|a, b| a.total_cmp(b));
    let q = |p: f64| sheaf_calc::stats::quantile_origin(&v, p);
    let (q1, median, q3) = (q(0.25), q(0.5), q(0.75));
    let fence = 1.5 * (q3 - q1);
    let (lo_fence, hi_fence) = (q1 - fence, q3 + fence);
    let inside = v.iter().copied().filter(|x| *x >= lo_fence && *x <= hi_fence);
    let (whisker_low, whisker_high) = inside.fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), x| (a.min(x), b.max(x)));
    let outliers = v.iter().copied().filter(|x| *x < lo_fence || *x > hi_fence).collect();
    let mean = v.iter().sum::<f64>() / v.len() as f64;
    Some(BoxStats { pos, label, q1, median, q3, mean, whisker_low, whisker_high, outliers, width: BOX_WIDTH })
}

/// Reads the rows of an indexed plot (T76) as its bars or boxes: the Y values of the rows are split by the levels
/// of the main group column (the categories 1, 2, ... of the X axis, in sorted order, over every row with a data
/// value so that all subgroup plots share the positions) and, when the plot has a subset, by a level of the second
/// group column. A column plot gets the mean of each category (a category without rows in its subset has no bar)
/// and error values; a box plot gets the boxes of the categories of its subset, dodged next to those of the other
/// subgroup plots (`slot` of `slots`). Returns false when nothing can be drawn.
fn indexed_data(project: &Project, pd: &mut PlotData, slot: usize, slots: usize) -> bool {
    use sheaf_core::indexed::{level_text, sort_levels};
    let Some(ix) = pd.plot.indexed.as_ref() else { return false };
    let Some(group) = project.column(ix.group) else { return false };
    let subset = ix.subset.as_ref().and_then(|(c, l)| project.column(*c).map(|c| (c, l.as_str())));
    if ix.subset.is_some() && subset.is_none() {
        return false;
    }
    // Every shown row with a data value and a level, with its level and whether it is in the subset.
    let rows: Vec<(String, f64, bool)> = pd
        .rows()
        .filter_map(|r| {
            let v = pd.y.get(r).copied().filter(|v| v.is_finite())?;
            let level = level_text(group, r)?;
            let inside = subset.is_none_or(|(c, l)| level_text(c, r).as_deref() == Some(l));
            Some((level, v, inside))
        })
        .collect();
    let mut names: Vec<String> = Vec::new();
    for (l, _, _) in &rows {
        if !names.contains(l) {
            names.push(l.clone());
        }
    }
    sort_levels(&mut names);
    if names.is_empty() {
        return false;
    }
    let values_of = |k: usize| -> Vec<f64> {
        rows.iter().filter(|(l, _, inside)| *inside && *l == names[k]).map(|(_, v, _)| *v).collect()
    };
    pd.mask = None;
    pd.filters.clear();
    pd.xerr = None;
    pd.base = None;
    pd.start = 0;
    pd.x_format = None;
    if pd.plot.kind == PlotKind::Box {
        let band = BOX_WIDTH / slots as f64;
        let mut boxes = Vec::new();
        for (k, name) in names.iter().enumerate() {
            let centre = (k + 1) as f64 - BOX_WIDTH / 2.0 + (slot as f64 + 0.5) * band;
            if let Some(mut b) = box_stats(&values_of(k), centre, name.clone()) {
                b.width = band;
                boxes.push(b);
            }
        }
        if boxes.is_empty() {
            return false;
        }
        pd.x = Some(Cow::Owned(boxes.iter().map(|b| b.pos).collect()));
        pd.y = Cow::Owned(boxes.iter().map(|b| b.median).collect());
        pd.end = boxes.len();
        pd.err = None;
        pd.gboxes = boxes;
    } else {
        let mut means = Vec::new();
        let mut errs = Vec::new();
        for k in 0..names.len() {
            let v = values_of(k);
            let n = v.len() as f64;
            let mean = v.iter().sum::<f64>() / n;
            let sd = if v.len() > 1 {
                (v.iter().map(|x| (x - mean) * (x - mean)).sum::<f64>() / (n - 1.0)).sqrt()
            } else {
                f64::NAN
            };
            means.push(if v.is_empty() { f64::NAN } else { mean });
            errs.push(match ix_error(pd.plot) {
                sheaf_core::graph::IndexedError::None => f64::NAN,
                sheaf_core::graph::IndexedError::Sd => sd,
                sheaf_core::graph::IndexedError::Se => sd / n.sqrt(),
            });
        }
        pd.x = Some(Cow::Owned((1..=names.len()).map(|k| k as f64).collect()));
        pd.end = means.len();
        pd.y = Cow::Owned(means);
        pd.err = (ix_error(pd.plot) != sheaf_core::graph::IndexedError::None).then_some(Cow::Owned(errs));
    }
    pd.categories = Some(names);
    true
}

/// The points of an interval plot or Pareto chart (T83). An interval plot point is the mean of the plot's visible
/// numbers at position `slot` (1, 2, ... in plot order) with the half width of its error bar as `err` (NaN for
/// fewer than two numbers). A Pareto plot is read from the cells of its column: its categories are the distinct
/// values, most frequent first, at 1, 2, ...; the bars are the counts, the curve the running percent.
fn stat_data(project: &Project, pd: &mut PlotData, slot: usize) -> bool {
    use sheaf_core::graph::{IntervalError, StatPlot};
    let Some(stat) = pd.plot.stat.clone() else { return false };
    match stat {
        StatPlot::Interval { error, level } => {
            let values: Vec<f64> = pd.visible_values().iter().copied().filter(|v| v.is_finite()).collect();
            if values.is_empty() {
                return false;
            }
            let st = sheaf_calc::stats::stats(&values);
            let half = match error {
                IntervalError::Sd => st.sd,
                IntervalError::Se => st.sem,
                IntervalError::Ci => sheaf_calc::stats::mean_confidence_limits(&st, level / 100.0).1 - st.mean,
            };
            pd.x = Some(Cow::Owned(vec![slot as f64]));
            pd.y = Cow::Owned(vec![st.mean]);
            pd.err = Some(Cow::Owned(vec![half]));
            pd.slot_label = Some(project.plot_label(pd.plot));
            pd.end = 1;
        }
        StatPlot::Pareto | StatPlot::ParetoCumulative => {
            let Some(col) = project.column(pd.plot.y) else { return false };
            let counts = sheaf_core::indexed::counts_descending(col);
            let total: usize = counts.iter().map(|(_, n)| *n).sum();
            if total == 0 {
                return false;
            }
            let values: Vec<f64> = if stat == StatPlot::Pareto {
                counts.iter().map(|(_, n)| *n as f64).collect()
            } else {
                let mut sum = 0usize;
                counts
                    .iter()
                    .map(|(_, n)| {
                        sum += n;
                        100.0 * sum as f64 / total as f64
                    })
                    .collect()
            };
            pd.x = Some(Cow::Owned((1..=values.len()).map(|k| k as f64).collect()));
            pd.end = values.len();
            pd.y = Cow::Owned(values);
            pd.err = None;
            pd.categories = Some(counts.into_iter().map(|(l, _)| l).collect());
        }
    }
    pd.mask = None;
    pd.filters.clear();
    pd.xerr = None;
    pd.base = None;
    pd.start = 0;
    pd.x_format = None;
    true
}

fn ix_error(plot: &Plot) -> sheaf_core::graph::IndexedError {
    plot.indexed.as_ref().map_or(sheaf_core::graph::IndexedError::None, |i| i.error)
}

/// The bins of a histogram plot as the points of a column plot: bin centres and counts. `None`
/// without numbers or with a bin size that cannot be used.
fn histogram_points(values: &[f64], style: &sheaf_core::graph::HistogramStyle) -> Option<(Vec<f64>, Vec<f64>)> {
    let f = histogram_freq(values, style)?;
    Some((f.rows.iter().map(|r| r.center).collect(), f.rows.iter().map(|r| r.count as f64).collect()))
}

/// The bins of a histogram plot (also those of a ridgeline's block bars).
pub(crate) fn histogram_freq(
    values: &[f64],
    style: &sheaf_core::graph::HistogramStyle,
) -> Option<sheaf_calc::freq::Freq> {
    use sheaf_calc::freq::{BinSize, FreqOptions, freq_counts};
    let opts = FreqOptions {
        min: style.begin,
        max: style.end,
        size: style.bin_size.map_or(BinSize::Auto, BinSize::Width),
        below: false,
        above: false,
    };
    freq_counts(values, &opts)
}

impl PlotData<'_> {
    pub fn row_shown(&self, row: usize) -> bool {
        let row = if row == self.end.saturating_sub(1) { self.closing.unwrap_or(row) } else { row };
        self.filters.iter().all(|f| f.passes(row))
    }

    /// Plotted source rows, skipping filtered rows without turning them into missing data.
    /// A real missing value still breaks a line; a filtered row simply joins its neighbours.
    pub fn rows(&self) -> impl DoubleEndedIterator<Item = usize> + Clone + '_ {
        (self.start..self.end).filter(|&r| self.row_shown(r))
    }

    /// Rows in the order the line connects them: worksheet order, or with the plot's `sort_x`
    /// flag (line and line-symbol plots only) increasing drawn X. The sort is stable, so ties keep
    /// worksheet order; rows whose X is not a finite number are left out of the path altogether
    /// (they neither connect nor break it). A row with a missing Y stays in its sorted place and
    /// breaks the line there, as in worksheet order.
    pub fn line_rows(&self) -> Box<dyn DoubleEndedIterator<Item = usize> + '_> {
        if !(self.plot.line.sort_x && matches!(self.plot.kind, PlotKind::Line | PlotKind::LineSymbol)) {
            return Box::new(self.rows());
        }
        let mut rows: Vec<(f64, usize)> =
            self.rows().map(|r| (self.x_at(r), r)).filter(|(x, _)| x.is_finite()).collect();
        rows.sort_by(|a, b| a.0.total_cmp(&b.0));
        Box::new(rows.into_iter().map(|(_, r)| r))
    }

    /// Display frequency for point decorations. Lines, ranges and analysis still read every row.
    pub fn point_interval(&self) -> usize {
        if self.plot.kind.supports_skip_points() { self.plot.symbol.every.max(1) as usize } else { 1 }
    }

    /// Points selected by Drop Lines > Skip Points, retaining original worksheet row indices.
    /// Missing values count toward the frequency; filtered rows retain the existing compacted
    /// display order. A synthetic radar closing point has no extra symbol, error or label.
    pub fn point_rows(&self) -> impl Iterator<Item = usize> + Clone + '_ {
        let end = self.end - usize::from(self.closing.is_some());
        self.rows().take_while(move |&row| row < end).step_by(self.point_interval())
    }

    fn visible_values(&self) -> Cow<'_, [f64]> {
        if self.filters.is_empty() {
            Cow::Borrowed(&self.y[self.start..self.end])
        } else {
            Cow::Owned(self.rows().map(|r| self.y[r]).collect())
        }
    }

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

    /// The original Y value of row `i`, before display scaling and offsets (NaN past the end).
    pub fn source_y_at(&self, i: usize) -> f64 {
        self.y.get(i).copied().unwrap_or(f64::NAN)
    }

    /// The original X value (or row number), before display scaling and offsets.
    pub fn source_x_at(&self, i: usize) -> f64 {
        match &self.x {
            None => (i + 1) as f64,
            Some(x) => x.get(i).copied().unwrap_or(f64::NAN),
        }
    }

    /// The position of row `i` along the X column, with its multiplier and offset.
    pub fn cat_at(&self, i: usize) -> f64 {
        self.display_x(self.source_x_at(i))
    }

    pub fn display_x(&self, x: f64) -> f64 {
        let x = if self.mx == 1.0 { x } else { x * self.mx };
        // Adding 0 would turn -0 into +0; leave identity placements exactly as they are.
        if self.ox == 0.0 { x } else { x + self.ox }
    }

    pub fn display_y(&self, y: f64) -> f64 {
        let y = if self.my == 1.0 { y } else { y * self.my };
        if self.oy == 0.0 { y } else { y + self.oy }
    }

    /// The value of row `i` as drawn: the Y value with the stack offset, or the top of a stacked bar.
    pub fn val_at(&self, i: usize) -> f64 {
        if let Some((_, hi)) = &self.stack {
            return self.display_y(hi.get(i).copied().unwrap_or(f64::NAN));
        }
        self.display_y(self.y[i])
    }

    /// The lower end of a bar of a stacked or floating column plot at row `i`, when it has one.
    pub fn base_at(&self, i: usize) -> Option<f64> {
        if let Some((lo, _)) = &self.stack {
            return lo.get(i).map(|&v| self.display_y(v));
        }
        let b = self.base?.get(i).copied()?;
        Some(self.display_y(b))
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
        self.err.as_deref().and_then(|e| e.get(i)).copied().unwrap_or(f64::NAN)
    }
}

/// The plots of a layer as they are drawn.
pub(crate) struct LayerData<'a> {
    /// The visible plots whose Y column holds numbers, in plot order.
    pub plots: Vec<PlotData<'a>>,
    /// The number of members of the layer's stack, hidden plots included.
    pub members: usize,
    /// The least level step of the layer's significance brackets in the transformed space of its Y axis that
    /// the label height asks for at the scale last laid out (0 at first; the layout raises it until it settles).
    pub bracket_step: std::cell::Cell<f64>,
}

pub(crate) use sheaf_core::stack::row_range;

/// [`collect`] for every layer of a graph, with the hexbin plots binned for the nominal frame of their layer.
pub(crate) fn collect_graph<'a>(project: &'a Project, g: &'a sheaf_core::graph::Graph) -> Vec<LayerData<'a>> {
    let mut all: Vec<LayerData> = g.layers.iter().map(|layer| collect(project, layer)).collect();
    for (layer, d) in g.layers.iter().zip(&mut all) {
        let aspect = crate::hexbin::nominal_aspect(layer, g.page.width_pt, g.page.height_pt);
        for pd in &mut d.plots {
            let style = pd.plot.hexbin;
            if let Some(h) = &mut pd.hex {
                h.refit(&style, aspect);
            }
        }
    }
    place_pareto_cumulative(&mut all);
    all
}

/// Origin's Plot_ParetoRaw option symr (default 1): the symbols of the cumulative percent sit at the right edge of
/// each bar and the line starts at (0, 0), the left edge of the first bar. The edges are those of the drawn bars of
/// the layer that counts the same column (its bar width follows the gap), so they follow a change of the gap.
fn place_pareto_cumulative(all: &mut [LayerData]) {
    let bars: Vec<(sheaf_core::ColumnId, f64, f64)> = all
        .iter()
        .flat_map(|d| d.plots.iter())
        .filter(|pd| pd.plot.stat == Some(StatPlot::Pareto))
        .filter_map(|pd| pd.bar.as_ref().map(|b| (pd.plot.y, b.left, b.width)))
        .collect();
    for pd in all.iter_mut().flat_map(|d| d.plots.iter_mut()) {
        if pd.plot.stat != Some(StatPlot::ParetoCumulative) {
            continue;
        }
        let Some(&(_, left, width)) = bars.iter().find(|b| b.0 == pd.plot.y) else { continue };
        let (Some(x), y) = (pd.x.as_deref(), &pd.y) else { continue };
        let mut xs = vec![1.0 + left];
        xs.extend(x.iter().map(|k| k + left + width));
        let mut ys = vec![0.0];
        ys.extend(y.iter().copied());
        pd.end = xs.len();
        pd.x = Some(Cow::Owned(xs));
        pd.y = Cow::Owned(ys);
    }
}

/// Gives the curves of a colour mapped waterfall layer (graphing spec 13.17) the ends of the colour scale and their
/// Z value. The Y scale runs over the shown, finite Y values of every curve in the layer; the Z scale over the Z
/// values of the curves.
fn attach_waterfall_colors(project: &Project, layer: &Layer, plots: &mut [PlotData]) {
    use sheaf_core::graph::{Stack, WaterfallColor};
    let mode = layer.waterfall_color;
    if mode == WaterfallColor::None || !matches!(layer.stack, Stack::Waterfall { .. }) {
        return;
    }
    let zs: Vec<f64> =
        plots.iter().map(|pd| sheaf_core::stack::waterfall_z(project, pd.plot, pd.member.unwrap_or(0))).collect();
    let values: Vec<f64> = match mode {
        WaterfallColor::Z => zs.clone(),
        _ => plots
            .iter()
            .filter(|pd| pd.plot.kind.has_line())
            .flat_map(|pd| pd.rows().map(|r| pd.y[r]).collect::<Vec<_>>())
            .filter(|v| v.is_finite())
            .collect(),
    };
    let (lo, hi) = values.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), v| (a.min(*v), b.max(*v)));
    if !(lo.is_finite() && hi.is_finite()) {
        return;
    }
    for (pd, z) in plots.iter_mut().zip(zs) {
        pd.wf = Some(WaterfallColors { mode, lo, hi, z });
    }
}

/// The X value of the `k`-th row of the column scatter plot at position `g + 1` (0-based `g`): the position
/// plus a fixed pseudo-random shift of up to 0.15 either way (the golden-ratio sequence, so the picture is the
/// same every time). Origin's points stayed within about +-0.15 of their column (measured); its jitter rule
/// itself was not measured, so the sequence is Sheaf's choice.
fn column_jitter(g: usize, k: usize) -> f64 {
    let u = ((k as f64 + 1.0) * 0.618_033_988_749_894_9).fract();
    (g + 1) as f64 + (u - 0.5) * 0.30
}

/// The drawable plots of a layer, in plot order, with their stack offsets. A plot that is hidden
/// or whose Y column is missing or not numeric is skipped (a hidden plot still keeps its place in
/// the stack).
pub(crate) fn collect<'a>(project: &'a Project, layer: &'a Layer) -> LayerData<'a> {
    let (places, members) = stack::places(project, layer);
    let mut cumulative = sheaf_core::stack::cumulative_add(project, layer);
    let mut plots = Vec::new();
    let mut boxes = 0usize;
    let mut scatter_columns = 0usize;
    let mut ridges = 0usize;
    let mut intervals = 0usize;
    // The visible grouped box plots of indexed data share the width of a box; each has its slot.
    let indexed_boxes: Vec<usize> = layer
        .plots
        .iter()
        .enumerate()
        .filter(|(_, p)| p.visible && p.kind == PlotKind::Box && p.indexed.is_some())
        .map(|(i, _)| i)
        .collect();
    for (index, (plot, place)) in layer.plots.iter().zip(places).enumerate().filter(|(_, (p, _))| p.visible) {
        // A Pareto chart counts the cells of any column, text included.
        let counted = matches!(plot.stat, Some(StatPlot::Pareto | StatPlot::ParetoCumulative));
        let Some(y) = project.numeric(plot.y).or_else(|| counted.then_some(&[][..])) else { continue };
        // Without an X column, a Y column with a sampling interval stands at its own X (T84).
        let x: Option<Cow<'_, [f64]>> = match plot.x {
            Some(id) => project.numeric(id).map(Cow::Borrowed),
            None => project.column(plot.y).and_then(|c| c.sampling.as_ref()).map(|s| Cow::Owned(s.values(y.len()))),
        };
        let x_format = plot.x.and_then(|id| project.column(id)).map(|c| c.format);
        let err = plot.y_error.and_then(|id| project.numeric(id));
        let xerr = plot.x_error.and_then(|id| project.numeric(id));
        let base = plot.y_base.and_then(|id| project.numeric(id));
        let (start, end) = row_range(plot, y.len());
        // Set As Begin / Set As End: the rows outside the column's data range are left out (T87).
        let (start, end) = match project.column(plot.y).and_then(|c| c.data_range) {
            Some(r) => r.clip(start, end),
            None => (start, end),
        };
        let active_mask = project.column(plot.y).and_then(|c| c.active_mask());
        // Hide Masked Points: masked rows become gaps, like missing values, and nothing is drawn red.
        let (y_plot, mask): (Cow<'_, [f64]>, _) = match active_mask {
            Some(m) if project.hide_masked => {
                let mut owned = y.to_vec();
                for &r in m.range(..owned.len()) {
                    owned[r] = f64::NAN;
                }
                (Cow::Owned(owned), None)
            }
            _ => (Cow::Borrowed(y), active_mask),
        };
        // Cumulative stacking: each row is drawn at its Y plus the Y of the plots before it.
        let y_plot = match cumulative[index].take() {
            Some(add) => Cow::Owned(y_plot.iter().zip(&add).map(|(v, a)| v + a).collect()),
            None => y_plot,
        };
        let mut pd = PlotData {
            plot,
            x,
            x_format,
            y: y_plot,
            mask,
            mask_color: project.mask_color_rgb(),
            filters: project.plot_row_filters(plot),
            err: err.map(Cow::Borrowed),
            xerr,
            base,
            stack: None,
            start,
            end,
            ox: place.ox,
            oy: place.oy,
            mx: place.mx,
            my: place.my,
            member: place.member,
            bar: None,
            boxed: None,
            column_at: None,
            gboxes: Vec::new(),
            categories: None,
            slot_label: None,
            violin: None,
            ridge: None,
            pie: None,
            size_z: plot.size_from.and_then(|id| project.numeric(id)),
            color_z: plot.color_from.and_then(|id| project.numeric(id)),
            spokes: None,
            closing: None,
            hist_raw: None,
            hex: None,
            vector: None,
            stock: None,
            marginal: None,
            wind_rose: None,
            kde2d: None,
            piper: None,
            marker_display: Default::default(),
            parallel: None,
            treemap: None,
            sunburst: None,
            wf: None,
            label_texts: match plot.decor.labels.form {
                LabelForm::Column { column } if plot.decor.labels.enable => project.column(column).map(cell_texts),
                _ => None,
            },
        };
        match plot.kind {
            _ if plot.stat.is_some() => {
                if !stat_data(project, &mut pd, intervals + 1) {
                    continue;
                }
                if matches!(plot.stat, Some(StatPlot::Interval { .. })) {
                    intervals += 1;
                }
            }
            PlotKind::Column | PlotKind::Box if plot.indexed.is_some() => {
                let slot = indexed_boxes.iter().position(|&i| i == index).unwrap_or(0);
                if !indexed_data(project, &mut pd, slot, indexed_boxes.len().max(1)) {
                    continue;
                }
            }
            PlotKind::Histogram => {
                // The plot shows the counts of the bins of its Y values: one point per bin.
                let values = pd.visible_values();
                let Some((centres, counts)) = histogram_points(&values, &plot.hist) else { continue };
                pd.hist_raw =
                    Some(pd.rows().filter(|r| !pd.mask.is_some_and(|m| m.contains(r))).map(|r| pd.y[r]).collect());
                pd.start = 0;
                pd.end = counts.len();
                pd.x = Some(Cow::Owned(centres));
                pd.y = Cow::Owned(counts);
                pd.mask = None;
                pd.filters.clear();
                pd.err = None;
                pd.xerr = None;
                pd.base = None;
            }
            PlotKind::Pie | PlotKind::Pie3D | PlotKind::PieBw => {
                let labels = plot.x.and_then(|id| project.column(id));
                let rows: Vec<usize> =
                    pd.rows().filter(|&r| y.get(r).is_some_and(|v| v.is_finite() && *v > 0.0)).collect();
                pd.pie = Some(PieData {
                    values: rows.iter().map(|&r| y[r]).collect(),
                    names: rows
                        .iter()
                        .map(|&r| labels.map_or_else(|| (r + 1).to_string(), |c| c.cell_text(r)))
                        .collect(),
                    colors: (0..rows.len()).map(|i| layer.palette.color(i, rows.len())).collect(),
                    label_font: layer.x_axis.tick_font.clone(),
                    legend_font: layer.legend.font.clone(),
                });
                // A pie has no points on any axis.
                pd.end = pd.start;
                pd.mask = None;
                pd.err = None;
                pd.xerr = None;
                pd.base = None;
            }
            PlotKind::Treemap => {
                pd.treemap = crate::treemap::TreemapData::new(project, layer, &pd).map(Box::new);
                // A treemap has no points on any axis.
                pd.end = pd.start;
                pd.mask = None;
                pd.err = None;
                pd.xerr = None;
                pd.base = None;
            }
            PlotKind::Sunburst => {
                pd.sunburst = crate::sunburst::SunburstData::new(project, layer, &pd).map(Box::new);
                // A sunburst has no points on any axis.
                pd.end = pd.start;
                pd.mask = None;
                pd.err = None;
                pd.xerr = None;
                pd.base = None;
            }
            PlotKind::Box
            | PlotKind::Violin
            | PlotKind::ViolinBox
            | PlotKind::ViolinPoint
            | PlotKind::ViolinQuartile
            | PlotKind::ViolinStick
            | PlotKind::HalfViolin => {
                boxes += 1;
                let label = project.plot_label(plot);
                // A violin leaves out masked rows (Origin's masked data are excluded from the plot);
                // the box chart takes every visible row as it always did.
                let values: Cow<[f64]> = if plot.kind.is_violin() && pd.mask.is_some() {
                    Cow::Owned(pd.rows().filter(|r| !pd.mask.is_some_and(|m| m.contains(r))).map(|r| pd.y[r]).collect())
                } else {
                    pd.visible_values()
                };
                let Some(stats) = box_stats(&values, boxes as f64, label) else {
                    continue;
                };
                if plot.kind.is_violin() {
                    pd.violin = Some(crate::violin::ViolinData::new(&values));
                }
                // The plot is the one point at the middle of its box.
                pd.x = Some(Cow::Owned(vec![stats.pos]));
                pd.y = Cow::Owned(vec![stats.median]);
                pd.start = 0;
                pd.end = 1;
                pd.mask = None;
                pd.filters.clear();
                pd.err = None;
                pd.xerr = None;
                pd.base = None;
                pd.boxed = Some(stats);
            }
            PlotKind::ColumnScatter => {
                scatter_columns += 1;
                // Every row stands at the plot's position, spread by Sheaf's own deterministic jitter
                // (Origin's rule was not measured; its points stayed within about +-0.15).
                let pos = scatter_columns as f64;
                pd.x = Some(Cow::Owned((0..pd.y.len()).map(|k| column_jitter(scatter_columns - 1, k)).collect()));
                pd.column_at = Some((pos, project.plot_label(plot)));
            }
            PlotKind::Ridgeline => {
                // Masked rows are left out, as in a violin; filtered rows too.
                let values: Vec<f64> =
                    pd.rows().filter(|r| !pd.mask.is_some_and(|m| m.contains(r))).map(|r| pd.y[r]).collect();
                ridges += 1;
                let ridge = crate::ridgeline::RidgeData::new(
                    &values,
                    ridges as f64,
                    project.plot_label(plot),
                    &plot.ridge,
                    &plot.hist,
                    layer.x_axis.scale,
                );
                // The plot is the one point at the left end of its curve, on its baseline.
                pd.x = Some(Cow::Owned(vec![ridge.x_extent().map_or(0.0, |e| e.0)]));
                pd.y = Cow::Owned(vec![ridge.pos]);
                pd.start = 0;
                pd.end = 1;
                pd.mask = None;
                pd.filters.clear();
                pd.err = None;
                pd.xerr = None;
                pd.base = None;
                pd.ridge = Some(ridge);
            }
            PlotKind::Piper => {
                pd.piper = plot.decor.piper.map(|s| crate::piper_plot::PiperData::new(project, &pd, s));
                // The diagram's axes are fixed; the samples are not points of the axes.
                pd.end = pd.start;
                pd.mask = None;
                pd.err = None;
                pd.xerr = None;
                pd.base = None;
            }
            PlotKind::Kde2d => {
                pd.kde2d = crate::kde2d::Kde2dData::new(&pd);
                pd.end = pd.start;
                pd.mask = None;
                pd.err = None;
                pd.xerr = None;
                pd.base = None;
            }
            PlotKind::WindRose => {
                pd.wind_rose = plot.decor.wind_rose.map(|s| crate::wind_rose::WindRoseData::new(project, &pd, s));
                // The wedges are not points; their extent counts through `wind_rose`.
                pd.end = pd.start;
                pd.mask = None;
                pd.err = None;
                pd.xerr = None;
                pd.base = None;
            }
            PlotKind::MarginalHistogram | PlotKind::MarginalBox => {
                let values = pd.visible_values().into_owned();
                let side = plot.decor.marginal.unwrap_or(sheaf_core::marginal::MarginalSide::Top);
                let bins = plot.kind == PlotKind::MarginalHistogram;
                pd.marginal = crate::marginal_plot::MarginalData::new(&values, side, &plot.hist, bins);
                // The bars or box are not points on the axes; their extent counts through `marginal`.
                pd.end = pd.start;
                pd.mask = None;
                pd.err = None;
                pd.xerr = None;
                pd.base = None;
            }
            PlotKind::HighLowClose | PlotKind::Candlestick | PlotKind::OhlcBar => {
                let stock = plot.decor.stock.map(|s| crate::stock_plot::StockData::new(project, &pd, s, pd.y.len()));
                pd.stock = stock;
            }
            PlotKind::VectorXyam | PlotKind::VectorXyxy => {
                pd.vector = plot.decor.vector.map(|s| {
                    crate::vector_plot::VectorData::new(project, s, plot.kind == PlotKind::VectorXyam, pd.y.len())
                });
            }
            PlotKind::Hexbin => {
                // The nominal frame is not known here (`collect_graph` refits it); a square one for now.
                pd.hex =
                    Some(crate::hexbin::HexData::new(&pd, &plot.hexbin, 1.0, layer.x_axis.scale, layer.y_axis.scale));
            }
            _ => {}
        }
        if let Some(polar) = layer.coords.polar() {
            polar_columns(&mut pd, polar.radius);
        }
        if let Some(s) = layer.coords.smith() {
            crate::smith::columns(&mut pd, s.z0);
        }
        if layer.coords.ternary().is_some() {
            crate::ternary::columns(&mut pd, plot.size_from.and_then(|id| project.numeric(id)));
        }
        pd.marker_display = layer.markers;
        plots.push(pd);
    }
    if layer.coords.polar().is_some_and(|p| p.radar) {
        radar_columns(project, &mut plots);
    }
    crate::parallel::attach(project, layer, &mut plots);
    crate::ridgeline::set_scales(&mut plots);
    crate::wind_rose::pile(&mut plots);
    stack_bars(&mut plots);
    place_bars(&mut plots, &layer.bars);
    attach_waterfall_colors(project, layer, &mut plots);
    LayerData { plots, members, bracket_step: std::cell::Cell::new(0.0) }
}

/// Reads the two columns of a plot of a polar layer as `(angle, radius)`: `x` becomes the angle
/// and `y` the radius, whichever column the layer takes the radius from (a missing X column is
/// the row number). Error bars are not drawn in polar layers.
fn polar_columns(pd: &mut PlotData, radius: sheaf_core::graph::RadiusColumn) {
    pd.err = None;
    pd.xerr = None;
    pd.x_format = None;
    if radius == sheaf_core::graph::RadiusColumn::X {
        let angle = std::mem::replace(&mut pd.y, Cow::Owned(Vec::new()));
        let r = pd.x.take().unwrap_or_else(|| Cow::Owned((1..=angle.len()).map(|i| i as f64).collect()));
        pd.end = pd.end.min(r.len());
        pd.x = Some(angle);
        pd.y = r;
    }
}

/// Reads the plots of a radar layer (graphing spec 14.19) as `(angle, value)` pairs: row `i` of `n`
/// is the spoke at `360 i / n` degrees, `n` the number of rows of the longest plot, and every plot
/// ends with its first visible point again one turn later, which closes the polygon. The spokes are named
/// by the cells of the first plot's X column, or by the row numbers.
fn radar_columns(project: &Project, plots: &mut [PlotData]) {
    let n = plots.iter().map(|pd| pd.end.saturating_sub(pd.start)).max().unwrap_or(0);
    if n == 0 {
        return;
    }
    let first = &plots[0];
    let labels = first.plot.x.and_then(|id| project.column(id));
    let names: Vec<String> = (0..n)
        .map(|i| {
            let row = first.start + i;
            // An empty cell of the X column leaves the spoke with its row number.
            labels.map(|c| c.cell_text(row)).filter(|t| !t.is_empty()).unwrap_or_else(|| (row + 1).to_string())
        })
        .collect();
    let angle = |i: usize| 360.0 * i as f64 / n as f64;
    for pd in plots.iter_mut() {
        // Rows keep their place (the mask counts rows of the column); the first drawn row is
        // repeated once after the last, one turn later, to close the polygon.
        let (start, end) = (pd.start, pd.end.min(pd.y.len()));
        if start >= end {
            continue;
        }
        let mut ys: Vec<f64> = pd.y[..end].to_vec();
        let mut xs: Vec<f64> = (0..end).map(|r| angle(r.saturating_sub(start))).collect();
        let first_visible = pd.rows().next();
        if let Some(first) = first_visible {
            ys.push(ys[first]);
            xs.push(360.0 + angle(first - start));
        }
        pd.end = ys.len();
        pd.x = Some(Cow::Owned(xs));
        pd.y = Cow::Owned(ys);
        pd.closing = first_visible;
        pd.err = None;
        pd.xerr = None;
        pd.x_format = None;
        pd.spokes = Some(names.clone());
    }
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
    // Histogram bars touch.
    let gap = if plots[first].plot.kind == PlotKind::Histogram {
        0.0
    } else if let Some(w) = bars.width_pct {
        // Width (in %) replaces the gap: the bars together take that share of the spacing.
        1.0 - percent(w, 0.0, 100.0, 100.0 - sheaf_core::graph::DEFAULT_BAR_GAP_PCT) / 100.0
    } else {
        percent(bars.gap_pct, 0.0, 100.0, sheaf_core::graph::DEFAULT_BAR_GAP_PCT) / 100.0
    };
    let overlap = percent(bars.overlap_pct, -100.0, 100.0, 0.0) / 100.0;
    // Stacked plots share one slot, which stands where the first of them stands; so do floating ones.
    let mut slot_of = vec![0usize; plots.len()];
    let mut slots = 0usize;
    let mut stacked_slot = None;
    for &i in &columns {
        if plots[i].plot.kind.is_stacked_column() || plots[i].plot.kind.is_floating() {
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

/// Piles the stacked plots of a layer: the bar or area of each plot starts where those of the
/// plots before it (of the same family, columns or areas) end, row by row (a missing value counts
/// as 0). With a 100 % plot among them every pile is scaled to a total of 100.
fn stack_bars(plots: &mut [PlotData]) {
    for areas in [false, true] {
        let idx: Vec<usize> = (0..plots.len())
            .filter(|&i| {
                let k = plots[i].plot.kind;
                if areas { k.is_stacked_area() } else { k.is_stacked_column() }
            })
            .collect();
        pile(plots, &idx);
    }
}

fn pile(plots: &mut [PlotData], idx: &[usize]) {
    if idx.is_empty() {
        return;
    }
    let rows = idx.iter().map(|&i| plots[i].y.len()).max().unwrap_or(0);
    let percent = idx.iter().any(|&i| plots[i].plot.kind.is_percent_stacked());
    let value = |pd: &PlotData, r: usize| {
        if pd.row_shown(r) { pd.y.get(r).copied().filter(|v| v.is_finite()).unwrap_or(0.0) } else { 0.0 }
    };
    let totals: Vec<f64> = (0..rows).map(|r| idx.iter().map(|&i| value(&plots[i], r)).sum()).collect();
    let mut run = vec![0.0; rows];
    for &i in idx {
        let (mut lo, mut hi) = (vec![f64::NAN; rows], vec![f64::NAN; rows]);
        for r in 0..rows {
            let scale = if !percent {
                1.0
            } else if totals[r] != 0.0 {
                100.0 / totals[r]
            } else {
                0.0
            };
            let shown = plots[i].row_shown(r) && plots[i].y.get(r).is_some_and(|v| v.is_finite());
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
    // Indexed plots stand at the categories 1, 2, ..., whichever of them this subgroup lacks.
    if pd.x.is_none() || pd.plot.indexed.is_some() {
        return 1.0;
    }
    let rows = pd.rows().filter(|&i| pd.val_at(i).is_finite()).count();
    let xs = pd.rows().filter(|&i| pd.val_at(i).is_finite()).map(|i| pd.cat_at(i)).filter(|x| x.is_finite());
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
        if scale.accepts(v) {
            r = Some(match r {
                None => (v, v),
                Some((lo, hi)) => (lo.min(v), hi.max(v)),
            });
        }
    };
    for pd in pds {
        if let Some(k) = &pd.kde2d {
            let (a, b) = k.extent(dim == Dim::X);
            add(a);
            add(b);
            continue;
        }
        if let Some(w) = &pd.wind_rose {
            let (a, b) = w.extent(dim == Dim::X);
            add(a);
            add(b);
            continue;
        }
        if let Some(m) = &pd.marginal {
            let (a, b) = m.extent(dim == Dim::X);
            add(a);
            add(b);
            continue;
        }
        // Stock plots: every row's high and low (and open and close) count for the Y axis.
        if let Some(s) = &pd.stock
            && dim == Dim::Y
        {
            s.values(pd.start, pd.end).for_each(&mut add);
        }
        // Vector XYXY: the arrows' end points count for the axes too.
        if let Some(v) = &pd.vector {
            for &(ex, ey) in &v.ends()[pd.start.min(v.ends().len())..pd.end.min(v.ends().len())] {
                if dim == Dim::X {
                    add(pd.display_x(ex));
                } else {
                    add(pd.display_y(ey));
                }
            }
        }
        if let Some(hex) = &pd.hex {
            if let Some((x0, x1, y0, y1)) = hex.extent() {
                if dim == Dim::X {
                    add(x0);
                    add(x1);
                } else {
                    add(y0);
                    add(y1);
                }
            }
            continue;
        }
        if let Some(rd) = &pd.ridge {
            if dim == Dim::X {
                if let Some((lo, hi)) = rd.x_extent() {
                    add(pd.display_x(lo));
                    add(pd.display_x(hi));
                }
                // The data themselves keep the range valid where the curve's ends cannot be placed.
                if let (Some(&first), Some(&last)) = (rd.values.first(), rd.values.last()) {
                    add(pd.display_x(first));
                    add(pd.display_x(last));
                }
            } else {
                add(pd.display_y(rd.pos - rd.rug_reach(&pd.plot.ridge)));
                add(pd.display_y(rd.pos + rd.peak() * rd.scale));
            }
            continue;
        }
        if let (Some(b), Some(v)) = (&pd.boxed, &pd.violin) {
            if dim == Dim::X {
                add(pd.display_x(b.pos - crate::violin::VIOLIN_WIDTH / 2.0));
                add(pd.display_x(b.pos + crate::violin::VIOLIN_WIDTH / 2.0));
            } else if let Some((lo, hi)) = v.y_extent() {
                add(pd.display_y(lo));
                add(pd.display_y(hi));
                // The curve ends reach below the smallest value and can be <= 0 where the data are not,
                // which a log axis cannot show: the data and the box marks keep the range valid.
                if let (Some(&first), Some(&last)) = (v.values.first(), v.values.last()) {
                    add(pd.display_y(first));
                    add(pd.display_y(last));
                }
                for y in [b.whisker_low, b.q1, b.median, b.q3, b.whisker_high] {
                    add(pd.display_y(y));
                }
            }
            continue;
        }
        for b in &pd.gboxes {
            if dim == Dim::X {
                add(pd.display_x(b.pos - b.width / 2.0));
                add(pd.display_x(b.pos + b.width / 2.0));
            } else {
                add(pd.display_y(b.whisker_low));
                add(pd.display_y(b.whisker_high));
                b.outliers.iter().for_each(|v| add(pd.display_y(*v)));
                add(pd.display_y(b.q1));
                add(pd.display_y(b.q3));
            }
        }
        if !pd.gboxes.is_empty() {
            continue;
        }
        if let Some(b) = &pd.boxed {
            if dim == Dim::X {
                add(pd.display_x(b.pos - BOX_WIDTH / 2.0));
                add(pd.display_x(b.pos + BOX_WIDTH / 2.0));
            } else {
                add(pd.display_y(b.whisker_low));
                add(pd.display_y(b.whisker_high));
                b.outliers.iter().for_each(|v| add(pd.display_y(*v)));
                add(pd.display_y(b.q1));
                add(pd.display_y(b.q3));
            }
            continue;
        }
        let dir = pd.plot.error.direction;
        let transposed = pd.transposed();
        let value_dim = if transposed { Dim::X } else { Dim::Y };
        for i in pd.rows() {
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
