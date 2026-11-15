//! Parallel Plot and Parallel Index (graphing spec 14.25): one vertical axis per column, each row a line through
//! the axes at its values in them.
//!
//! Origin documents the two plot types, the axis dialog's "Rescale Each Axis Independently" (each axis its own
//! range; cleared, one range from the variable with the greatest range), the axis title `%(?Y)` and the Plot
//! Details Parallel tab (Width, Transparency). The rest is Sheaf's own and unmeasured, named below and listed in
//! the spec:
//!
//! - Axes: the layer's X and Y axes are not drawn. Axis `i` of `n` stands at `(i + 0.5) / n` of the frame width and
//!   runs from the bottom to the top of the frame. Its ticks and labels (the layer's Y axis settings, a linear
//!   scale, automatic range) are left of it; its title is above the frame, centred on it, shrunk to fit the
//!   spacing of the axes.
//! - Rescale on: every axis has the nice range of its own column. Off: every axis has the nice range of the union
//!   of all the columns (the "variable with the greatest range", widened to include the others).
//! - Lines are straight, one scene path per row, in worksheet order. A row breaks at a missing cell (NaN, an empty
//!   cell or a masked point): the line is drawn through each run of two or more present cells, and a row with no
//!   such run draws nothing.
//! - Colour: a Parallel Plot with a numeric colour column colours each line with `plots::map_color` at the value's
//!   position between its smallest and largest value (a row whose colour cell is missing takes the plot colour), with the
//!   colour scale of the hexbin plot beside the frame, titled with the column's name; a Parallel Index gives each
//!   distinct text of its group column (by first appearance) one colour of the layer's palette and one legend
//!   entry, listed right of the frame. Without a colour column every line takes the plot colour.
//! - The Plot Details Width (default 0.5 pt) and Transparency (default 0) apply to every line.

use crate::LayerGeometry;
use crate::Rect;
use crate::data::PlotData;
use crate::fonts::TextMeasurer;
use crate::scene::{Anchor, Cap, Item, Join, Stroke, line_to, move_to};
use crate::text::{self, ASCENT, DESCENT};
use crate::ticks::{ResolvedAxis, resolve_axis};
use sheaf_core::graph::{Axis, Layer, LegendPosition, ParallelStyle, PlotKind, ScaleKind, opacity_for};
use sheaf_core::{Color, Project};

/// Length of the line sample of a group in the legend, in font sizes.
const SAMPLE_EM: f64 = 2.2;

/// One vertical axis.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ParAxis {
    /// The title: the plot's label, else the column's long name and unit (Origin's `%(?Y)`).
    pub title: String,
    /// The value of every drawn row, NaN where the cell is missing or masked.
    pub values: Vec<f64>,
    /// The range and ticks: the column's own, or the shared one when the axes are not rescaled.
    pub axis: ResolvedAxis,
}

/// What colours the lines.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ParColor {
    /// Every line in the plot colour.
    Plain,
    /// A numeric column through the colour ramp.
    Ramp {
        /// The value of every drawn row (NaN: the plot colour).
        values: Vec<f64>,
        /// The range the ramp spans: the smallest and largest value of the column.
        range: (f64, f64),
        /// The column's name, the title of the colour scale.
        title: String,
        /// Position along the bar (0..1) and label of each tick.
        ticks: Vec<(f64, String)>,
    },
    /// The groups of a Parallel Index.
    Groups {
        /// The text of each group, in order of first appearance.
        names: Vec<String>,
        colors: Vec<Color>,
        /// The group of every drawn row.
        of_row: Vec<usize>,
    },
}

/// Everything a Parallel Plot or Parallel Index draws, held by the first parallel plot of the layer.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ParallelData {
    pub axes: Vec<ParAxis>,
    pub rows: usize,
    pub color: ParColor,
    /// The colour of lines without a colour of their own: the first plot's.
    pub plain: Color,
    pub style: ParallelStyle,
}

/// The place of axis `i` of `n` across a frame: `x` and `w` of the frame.
pub(crate) fn axis_x(frame: Rect, n: usize, i: usize) -> f64 {
    frame.x + frame.w * (i as f64 + 0.5) / n.max(1) as f64
}

/// The axis settings the vertical axes are resolved with: the layer's Y axis, automatic and linear.
fn base_axis(layer: &Layer) -> Axis {
    Axis {
        min: None,
        max: None,
        major_step: None,
        scale: ScaleKind::Linear,
        reverse: false,
        axis_breaks: Vec::new(),
        first_tick: None,
        ..layer.y_axis.clone()
    }
}

fn finite_range(values: impl Iterator<Item = f64>) -> Option<(f64, f64)> {
    let r = values.filter(|v| v.is_finite()).fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), v| (a.min(v), b.max(v)));
    (r.0 <= r.1).then_some(r)
}

impl ParallelData {
    /// The data of the parallel plots `pds` of `layer` (the visible ones, in plot order).
    pub fn new(project: &Project, layer: &Layer, pds: &[&PlotData]) -> Option<ParallelData> {
        let first = pds.first()?;
        let style = first.plot.parallel;
        let rows: Vec<usize> = first.rows().collect();
        let base = base_axis(layer);
        let values_of = |pd: &PlotData| -> Vec<f64> {
            rows.iter()
                .map(|&r| {
                    let v = pd.y.get(r).copied().unwrap_or(f64::NAN);
                    if pd.mask.is_some_and(|m| m.contains(&r)) { f64::NAN } else { v }
                })
                .collect()
        };
        let columns: Vec<Vec<f64>> = pds.iter().map(|pd| values_of(pd)).collect();
        let shared = if style.rescale_each_axis {
            None
        } else {
            let range = finite_range(columns.iter().flatten().copied());
            Some(resolve_axis(&base, range))
        };
        let axes: Vec<ParAxis> = pds
            .iter()
            .zip(columns)
            .map(|(pd, values)| {
                let title = pd.plot.label.clone().unwrap_or_else(|| {
                    project.column(pd.plot.y).map(|c| c.axis_title()).unwrap_or_else(|| project.plot_label(pd.plot))
                });
                let axis = shared.clone().unwrap_or_else(|| resolve_axis(&base, finite_range(values.iter().copied())));
                ParAxis { title, values, axis }
            })
            .collect();
        let plain = first.plot.line.color;
        let color = match (first.plot.kind, style.color.and_then(|id| project.column(id))) {
            (PlotKind::ParallelIndex, Some(col)) => {
                let mut names: Vec<String> = Vec::new();
                let of_row: Vec<usize> = rows
                    .iter()
                    .map(|&r| {
                        let t = col.cell_text(r);
                        names.iter().position(|n| *n == t).unwrap_or_else(|| {
                            names.push(t);
                            names.len() - 1
                        })
                    })
                    .collect();
                let colors = (0..names.len()).map(|i| layer.palette.color(i, names.len())).collect();
                ParColor::Groups { names, colors, of_row }
            }
            (PlotKind::Parallel, Some(col)) if col.numeric().is_some() => {
                let all = col.numeric().unwrap_or(&[]);
                let values: Vec<f64> = rows.iter().map(|&r| all.get(r).copied().unwrap_or(f64::NAN)).collect();
                let data = finite_range(values.iter().copied()).unwrap_or((0.0, 1.0));
                let scale = resolve_axis(&base, Some(data));
                let span = data.1 - data.0;
                let ticks = scale
                    .major
                    .iter()
                    .zip(&scale.labels)
                    .filter(|(v, _)| **v >= data.0 - 1e-9 * span.abs() && **v <= data.1 + 1e-9 * span.abs())
                    .map(|(v, l)| (if span > 0.0 { (v - data.0) / span } else { 0.5 }, l.clone()))
                    .collect();
                ParColor::Ramp { values, range: data, title: col.display_name().to_string(), ticks }
            }
            _ => ParColor::Plain,
        };
        Some(ParallelData { axes, rows: rows.len(), color, plain, style })
    }

    /// The colour of the line of drawn row `i`, before transparency.
    pub fn row_color(&self, i: usize) -> Color {
        match &self.color {
            ParColor::Plain => self.plain,
            ParColor::Ramp { values, range, .. } => match values.get(i).copied().filter(|v| v.is_finite()) {
                Some(v) => {
                    let span = range.1 - range.0;
                    crate::plots::map_color(if span > 0.0 { (v - range.0) / span } else { 0.5 })
                }
                None => self.plain,
            },
            ParColor::Groups { colors, of_row, .. } => {
                of_row.get(i).and_then(|g| colors.get(*g)).copied().unwrap_or(self.plain)
            }
        }
    }

    /// The position (0 at the bottom, 1 at the top) of drawn row `i` on axis `k`; `None` where the cell is missing.
    pub fn unit(&self, k: usize, i: usize) -> Option<f64> {
        let a = &self.axes[k];
        let u = a.axis.to_unit(*a.values.get(i)?);
        u.is_finite().then_some(u)
    }

    /// The runs of consecutive axes where drawn row `i` has a value: `(axis, position)` pairs, runs of two or more.
    pub fn runs(&self, i: usize) -> Vec<Vec<(usize, f64)>> {
        let mut runs: Vec<Vec<(usize, f64)>> = Vec::new();
        let mut cur: Vec<(usize, f64)> = Vec::new();
        for k in 0..self.axes.len() {
            match self.unit(k, i) {
                Some(u) => cur.push((k, u)),
                None => runs.push(std::mem::take(&mut cur)),
            }
        }
        runs.push(cur);
        runs.retain(|r| r.len() >= 2);
        runs
    }
}

/// Builds the data of a layer's parallel plots into its first one.
pub(crate) fn attach(project: &Project, layer: &Layer, plots: &mut [PlotData]) {
    let idx: Vec<usize> = (0..plots.len()).filter(|&i| plots[i].plot.kind.is_parallel()).collect();
    let Some(&first) = idx.first() else { return };
    let data = {
        let refs: Vec<&PlotData> = idx.iter().map(|&i| &plots[i]).collect();
        ParallelData::new(project, layer, &refs)
    };
    plots[first].parallel = data.map(Box::new);
}

/// Draws the lines of the layer's parallel plots (the first plot holds them all).
pub(crate) fn draw_parallel(out: &mut Vec<Item>, pd: &PlotData, geom: &LayerGeometry) {
    let Some(par) = pd.parallel.as_deref() else { return };
    let f = geom.frame;
    let n = par.axes.len();
    let alpha = opacity_for(par.style.transparency_pct.round().clamp(0.0, 100.0) as u8);
    let width = par.style.width_pt.max(0.0);
    if width <= 0.0 {
        return;
    }
    for i in 0..par.rows {
        let mut d = String::new();
        for run in par.runs(i) {
            for (j, (k, u)) in run.into_iter().enumerate() {
                let (x, y) = (axis_x(f, n, k), f.y + (1.0 - u) * f.h);
                if j == 0 {
                    move_to(&mut d, x, y);
                } else {
                    line_to(&mut d, x, y);
                }
            }
        }
        if d.is_empty() {
            continue;
        }
        let mut color = par.row_color(i);
        color.a = (f64::from(color.a) * alpha).round().clamp(0.0, 255.0) as u8;
        out.push(Item::Path {
            d,
            fill: None,
            stroke: Some(Stroke { color, width, dash: Vec::new(), cap: Cap::Round, join: Join::Round }),
        });
    }
}

/// The size an axis title is drawn at: the title font's, shrunk so that the widest title fits the spacing of the
/// axes (never below 5 pt).
fn title_size(layer: &Layer, par: &ParallelData, frame_w: f64, m: &dyn TextMeasurer) -> f64 {
    let font = &layer.y_axis.title_font;
    let step = frame_w / par.axes.len().max(1) as f64;
    let widest = par.axes.iter().map(|a| text::width(m, &a.title, font, font.size_pt)).fold(0.0, f64::max);
    if widest > 0.95 * step { (font.size_pt * 0.95 * step / widest).max(5.0).min(font.size_pt) } else { font.size_pt }
}

/// Draws the axes: line, ticks, tick labels and title of each.
pub(crate) fn draw_axes(
    out: &mut Vec<Item>,
    layer: &Layer,
    pds: &[PlotData],
    geom: &LayerGeometry,
    m: &dyn TextMeasurer,
) {
    let Some(par) = pds.iter().find_map(|pd| pd.parallel.as_deref()) else { return };
    let f = geom.frame;
    let n = par.axes.len();
    let axis = &layer.y_axis;
    let ts = title_size(layer, par, f.w, m);
    let tick_size = axis.tick_font.size_pt;
    for (k, a) in par.axes.iter().enumerate() {
        let x = axis_x(f, n, k);
        let mut d = String::new();
        move_to(&mut d, x, f.y);
        line_to(&mut d, x, f.y + f.h);
        out.push(Item::Path { d, fill: None, stroke: Some(Stroke::solid(axis.color, axis.line_width_pt)) });
        let mut ticks = String::new();
        if let Some((from, to)) = crate::layout::tick_span(axis.major_tick, axis.major_tick_len_pt) {
            for &v in &a.axis.major {
                let u = a.axis.to_unit(v);
                if u.is_finite() {
                    let y = f.y + (1.0 - u) * f.h;
                    move_to(&mut ticks, x + from, y);
                    line_to(&mut ticks, x + to, y);
                }
            }
        }
        if !ticks.is_empty() {
            let stroke =
                Stroke::solid(axis.tick_color.unwrap_or(axis.color), axis.tick_width_pt.unwrap_or(axis.line_width_pt));
            out.push(Item::Path { d: ticks, fill: None, stroke: Some(stroke) });
        }
        if axis.show_tick_labels {
            let end = x - crate::layout::outward_extent(axis) - crate::layout::label_gap(axis);
            for (v, label) in a.axis.labelled() {
                let u = a.axis.to_unit(v);
                if u.is_finite() {
                    let y = f.y + (1.0 - u) * f.h;
                    text::place(
                        out,
                        m,
                        label,
                        &axis.tick_font,
                        tick_size,
                        (end, y + (ASCENT - DESCENT) / 2.0 * tick_size),
                        Anchor::End,
                        0.0,
                    );
                }
            }
        }
        if axis.show_title {
            let base = f.y - 0.6 * ts - DESCENT * ts;
            text::place(out, m, &a.title, &axis.title_font, ts, (x, base), Anchor::Middle, 0.0);
        }
    }
}

/// How far the colour scale is moved from its place right of the frame: the layer legend's position places it, as it
/// places the legend of the groups (automatic: not moved; a corner or a manual position puts the block of title,
/// bar and labels there, its height that of the frame).
fn scale_shift(layer: &Layer, f: Rect, title: &str, ticks: &[(f64, String)], m: &dyn TextMeasurer) -> (f64, f64) {
    let ya = &layer.y_axis;
    let labels = ticks.iter().map(|(_, t)| text::width(m, t, &ya.tick_font, ya.tick_font.size_pt)).fold(0.0, f64::max);
    let title_w = text::width(m, title, &ya.title_font, ya.title_font.size_pt);
    let w = (crate::hexbin::BAR_W_PT + 3.0 + crate::hexbin::LABEL_GAP_PT + labels).max(title_w);
    let edge = 0.8 * layer.legend.font.size_pt;
    let x0 = f.x + f.w + crate::hexbin::BAR_GAP_PT;
    let (tx, ty) = match layer.legend.position {
        LegendPosition::Automatic => return (0.0, 0.0),
        LegendPosition::TopLeft | LegendPosition::BottomLeft => (f.x + edge, f.y),
        LegendPosition::TopRight | LegendPosition::BottomRight => (f.x + f.w - edge - w, f.y),
        LegendPosition::Manual { x, y } => (f.x + x * f.w - w / 2.0, f.y + y * f.h - f.h / 2.0),
    };
    (tx - x0, ty - f.y)
}

/// The colour scale of a Parallel Plot beside the frame.
pub(crate) fn draw_scale(
    out: &mut Vec<Item>,
    layer: &Layer,
    pds: &[PlotData],
    geom: &LayerGeometry,
    m: &dyn TextMeasurer,
) {
    if let Some(ParallelData { color: ParColor::Ramp { title, ticks, .. }, .. }) =
        pds.iter().find_map(|pd| pd.parallel.as_deref())
    {
        let shift = scale_shift(layer, geom.frame, title, ticks, m);
        crate::hexbin::draw_color_bar(out, layer, geom, m, title, ticks, shift);
    }
}

/// The text of a group in the legend.
fn group_text(name: &str) -> &str {
    if name.is_empty() { "(blank)" } else { name }
}

/// The size of the legend of the groups of a Parallel Index: `(width, height)` in pt.
fn legend_size(layer: &Layer, names: &[String], m: &dyn TextMeasurer) -> (f64, f64) {
    let lg = &layer.legend;
    let fs = lg.font.size_pt;
    let widest = names.iter().map(|n| text::width(m, group_text(n), &lg.font, fs)).fold(0.0, f64::max);
    let pad = 0.4 * fs;
    (2.0 * pad + SAMPLE_EM * fs + 0.5 * fs + widest, 2.0 * pad + 1.3 * fs * names.len() as f64)
}

/// The legend of a Parallel Index: a line sample and the text of each group. Automatic places it right of the
/// frame, top aligned with it; the corner positions place it inside the frame as the ordinary legend does.
pub(crate) fn draw_legend(out: &mut Vec<Item>, layer: &Layer, pds: &[PlotData], f: Rect, m: &dyn TextMeasurer) {
    let lg = &layer.legend;
    let Some(ParallelData { color: ParColor::Groups { names, colors, .. }, style, .. }) =
        pds.iter().find_map(|pd| pd.parallel.as_deref())
    else {
        return;
    };
    if !lg.visible || names.is_empty() {
        return;
    }
    let fs = lg.font.size_pt;
    let (w, h) = legend_size(layer, names, m);
    let (pad, row_h) = (0.4 * fs, 1.3 * fs);
    let edge = 0.8 * fs;
    let (x0, y0) = match lg.position {
        LegendPosition::Automatic => (f.x + f.w + crate::hexbin::BAR_GAP_PT, f.y),
        LegendPosition::TopLeft => (f.x + edge, f.y + edge),
        LegendPosition::TopRight => (f.x + f.w - edge - w, f.y + edge),
        LegendPosition::BottomLeft => (f.x + edge, f.y + f.h - edge - h),
        LegendPosition::BottomRight => (f.x + f.w - edge - w, f.y + f.h - edge - h),
        LegendPosition::Manual { x, y } => (f.x + x * f.w - w / 2.0, f.y + y * f.h - h / 2.0),
    };
    if lg.background.is_some() || lg.frame {
        let stroke = lg.frame.then(|| Stroke::solid(lg.font.color, 0.75));
        out.push(Item::Rect { rect: Rect { x: x0, y: y0, w, h }, fill: lg.background, stroke });
    }
    let sample = SAMPLE_EM * fs;
    for (g, name) in names.iter().enumerate() {
        let cy = y0 + pad + row_h * (g as f64 + 0.5);
        let mut d = String::new();
        move_to(&mut d, x0 + pad, cy);
        line_to(&mut d, x0 + pad + sample, cy);
        let color = colors.get(g).copied().unwrap_or(Color::BLACK);
        let width = style.width_pt.max(1.0);
        out.push(Item::Path {
            d,
            fill: None,
            stroke: Some(Stroke { color, width, dash: Vec::new(), cap: Cap::Butt, join: Join::Round }),
        });
        let baseline = cy + (ASCENT - DESCENT) / 2.0 * fs;
        text::place(
            out,
            m,
            group_text(name),
            &lg.font,
            fs,
            (x0 + pad + sample + 0.5 * fs, baseline),
            Anchor::Start,
            0.0,
        );
    }
}

/// Widens the label edges of a group for a layer of parallel plots at frame `f`: the title row above, the tick
/// labels of the first axis on its left, and right of the frame the colour scale or the legend of the groups.
pub(crate) fn label_edges(
    edges: &mut crate::layout::LabelEdges,
    layer: &Layer,
    pds: &[PlotData],
    f: Rect,
    m: &dyn TextMeasurer,
) {
    let Some(par) = pds.iter().find_map(|pd| pd.parallel.as_deref()) else { return };
    if !layer.display.show_data {
        return;
    }
    let axis = &layer.y_axis;
    if axis.show_title {
        let ts = title_size(layer, par, f.w, m);
        edges.top = edges.top.min(f.y - 0.6 * ts - (ASCENT + DESCENT) * ts);
    }
    if axis.show_tick_labels
        && let Some(a) = par.axes.first()
    {
        let x = axis_x(f, par.axes.len(), 0);
        let widest = a
            .axis
            .labelled()
            .map(|(_, l)| text::width(m, l, &axis.tick_font, axis.tick_font.size_pt))
            .fold(0.0, f64::max);
        edges.left = edges.left.min(x - crate::layout::outward_extent(axis) - crate::layout::label_gap(axis) - widest);
    }
    match &par.color {
        ParColor::Ramp { .. } => {
            edges.right = edges.right.max(f.x + f.w + crate::hexbin::scale_reserve(axis.tick_font.size_pt));
        }
        ParColor::Groups { names, .. }
            if layer.legend.visible && layer.legend.position == LegendPosition::Automatic =>
        {
            let (w, _) = legend_size(layer, names, m);
            edges.right = edges.right.max(f.x + f.w + crate::hexbin::BAR_GAP_PT + w);
        }
        _ => {}
    }
}
