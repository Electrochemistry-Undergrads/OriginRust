//! The Integrate gadget of a graph window (Origin's Gadgets > Integrate): a yellow range over one
//! plot whose edges and body can be dragged; the area between the curve and the baseline inside
//! the range is shaded and its value shown at the top of the range, live. The right-click menu
//! switches the baseline (zero, or the straight line through the end points of the range), writes
//! the result to a sheet ("New Output", the analysis tool Integrate with the same rows) and closes
//! the gadget.
//!
//! Sheaf starts the gadget at once on the active plot over the middle half of the X axis, where
//! Origin first shows a settings dialog; the settings are in the right-click menu instead.
//!
//! The Rise Time gadget (T138, Gadgets > Rise Time, not measured) uses the same range: it draws the 10 %
//! and 90 % levels of the curve's amplitude inside the range, marks where the curve crosses them and shows
//! the rise time (or fall time) between the crossings (`sheaf_calc::risetime`).
//!
//! The Intersect gadget (T139, Gadgets > Intersect, not measured) marks where its plot crosses the
//! layer's other plots inside the range and writes each point's X and Y beside it
//! (`sheaf_calc::curves::intersections`).
//!
//! The Quick Peaks gadget (T140, Gadgets > Quick Peaks, not measured) marks the positive peaks of the
//! curve inside the range (`sheaf_calc::peaks::find_peaks` with Peak Analyzer's defaults: 2 neighbours on
//! each side, peaks of at least 20 % of the largest magnitude) and writes each one's X above it.
//!
//! The Quick Sigmoidal Fit gadget (T141, Gadgets > Quick Sigmoidal Fit, not measured) fits the built-in
//! Boltzmann function `A2 + (A1 - A2) / (1 + exp((x - x0) / dx))` to the rows inside the range, draws the
//! fitted curve over the range and shows x0 and dx.
//!
//! The FFT gadget (T142, Gadgets > FFT, not measured) takes the rows inside the range at their mean X step,
//! computes the one-sided amplitude spectrum (`sheaf_calc::fft::fft_spectrum`, Origin's FFT defaults with
//! that sampling interval), draws it as a small chart at the top of the range and names the strongest
//! frequency above zero.
//!
//! The Statistics gadget (T143, Gadgets > Statistics, not measured) shows the count, mean, standard
//! deviation, minimum and maximum of the Y values inside the range; the Quick Fit gadget (T143, Gadgets >
//! Quick Fit, linear only) fits a straight line to them by least squares, draws it over the range and
//! shows its equation and R².

use crate::actions::Action;
use crate::menus::{self, MenuContext};
use eframe::egui::{self, Color32, Pos2, Rect, Sense, Stroke};
use sheaf_calc::integrate::{IntegrateOptions, IntegrateResult, integrate};
use sheaf_core::{Project, WindowId};
use sheaf_plot::LayerGeometry;

/// Which gadget the range belongs to.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GadgetKind {
    #[default]
    Integrate,
    /// Gadgets > Rise Time (T138).
    RiseTime,
    /// Gadgets > Intersect (T139).
    Intersect,
    /// Gadgets > Quick Peaks (T140).
    QuickPeaks,
    /// Gadgets > Quick Sigmoidal Fit (T141).
    Sigmoid,
    /// Gadgets > FFT (T142).
    Fft,
    /// Gadgets > Statistics (T143).
    Statistics,
    /// Gadgets > Quick Fit, a straight line (T143).
    QuickFit,
    /// Gadgets > Vertical Cursor (T144): the range is a thin band whose centre is the cursor.
    VerticalCursor,
    /// Gadgets > Differentiate (T145).
    Differentiate,
    /// Gadgets > Interpolate (T145).
    Interpolate,
    /// Gadgets > Cluster (T146): the range is a rectangle (`y0`..`y1` as well).
    Cluster,
    /// Gadgets > 2D Integrate (T189): a rectangle over a heat map or contour plot; the volume under
    /// the surface inside it, from a zero base.
    Integrate2d,
}

/// What the 2D Integrate gadget shows (T189): of the grid cells whose centres are inside the
/// rectangle, how many, the least, greatest, mean, standard deviation and sum of Z, and the volume
/// (the sum of Z times the area of a cell).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Volume {
    pub n: usize,
    pub min: f64,
    pub max: f64,
    pub mean: f64,
    pub sd: f64,
    pub sum: f64,
    pub volume: f64,
}

/// The column names of the 2D Integrate gadget's output table (T198), one row per New Output.
pub const VOLUME_COLUMNS: [&str; 11] =
    ["X From", "X To", "Y From", "Y To", "Volume", "N", "Mean", "SD", "Min", "Max", "Sum"];

/// The long name of the workbook that collects the 2D Integrate gadget's outputs (T198).
pub const VOLUME_BOOK: &str = "2D Integrate Results";

impl Integrator {
    /// The 2D Integrate gadget's numbers for its rectangle (T198), `None` without a field or cells.
    pub fn volume(&self, project: &Project, graph: WindowId) -> Option<Volume> {
        let layer = project.graph(graph)?.layers.get(self.layer)?;
        let (cells, area) = sheaf_plot::field_cells(project, layer)?;
        volume_in(&cells, area, (self.x0, self.x1), (self.y0, self.y1))
    }
}

/// Adds a row with the rectangle and its [`Volume`] to the workbook named [`VOLUME_BOOK`], made with
/// [`VOLUME_COLUMNS`] the first time (T198). Returns the workbook.
pub fn write_volume(p: &mut Project, rect: [f64; 4], v: &Volume) -> WindowId {
    use sheaf_core::{ColumnData, Designation};
    let existing = p.workbooks().find(|b| b.long_name == VOLUME_BOOK).map(|b| b.id);
    let book = match existing {
        Some(book) => book,
        None => {
            let book = p.add_workbook(VOLUME_BOOK);
            for (k, name) in VOLUME_COLUMNS.iter().enumerate() {
                let designation = if k == 0 { Designation::X } else { Designation::Y };
                if let Ok(id) = p.add_column(book, 0, designation, ColumnData::numeric(Vec::new()))
                    && let Some(c) = p.column_mut(id)
                {
                    c.long_name = (*name).to_string();
                }
            }
            book
        }
    };
    let row = [rect[0], rect[1], rect[2], rect[3], v.volume, v.n as f64, v.mean, v.sd, v.min, v.max, v.sum];
    let ids: Vec<sheaf_core::ColumnId> = p
        .workbook(book)
        .and_then(|b| b.sheets.first())
        .map(|s| s.columns.iter().map(|c| c.id).collect())
        .unwrap_or_default();
    for (id, value) in ids.into_iter().zip(row) {
        if let Some(c) = p.column_mut(id) {
            let mut values = c.numeric().map(<[f64]>::to_vec).unwrap_or_default();
            values.push(value);
            c.data = ColumnData::numeric(values);
        }
    }
    book
}

/// [`Volume`] of `cells` (X, Y, Z) whose X is between `x0` and `x1` and Y between `y0` and `y1` (either
/// order, ends included); `None` when no cell is inside.
pub fn volume_in(cells: &[[f64; 3]], area: f64, (x0, x1): (f64, f64), (y0, y1): (f64, f64)) -> Option<Volume> {
    let inside = |v: f64, a: f64, b: f64| v >= a.min(b) && v <= a.max(b);
    let z: Vec<f64> = cells.iter().filter(|p| inside(p[0], x0, x1) && inside(p[1], y0, y1)).map(|p| p[2]).collect();
    let n = z.len();
    if n == 0 {
        return None;
    }
    let sum: f64 = z.iter().sum();
    let mean = sum / n as f64;
    let sd = if n > 1 { (z.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (n - 1) as f64).sqrt() } else { 0.0 };
    let (min, max) = z.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), v| (a.min(*v), b.max(*v)));
    Some(Volume { n, min, max, mean, sd, sum, volume: sum * area })
}

/// The Cluster gadget's numbers: how many points are inside and their mean X and Y.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClusterStats {
    pub n: usize,
    pub mean_x: f64,
    pub mean_y: f64,
}

/// A small line chart of `(u, v)` in the top third of `range`, both scaled to their own extent.
fn mini_chart(painter: &egui::Painter, range: Rect, pts: &[(f64, f64)]) {
    let chart = Rect::from_min_max(
        range.left_top() + egui::vec2(4.0, 20.0),
        Pos2::new(range.right() - 4.0, range.top() + 20.0 + range.height() / 3.0),
    );
    if chart.width() <= 20.0 || chart.height() <= 20.0 {
        return;
    }
    painter.rect_filled(chart, 0.0, Color32::from_rgba_unmultiplied(255, 255, 255, 220));
    painter.rect_stroke(chart, 0.0, Stroke::new(1.0_f32, Color32::GRAY), egui::StrokeKind::Inside);
    let finite: Vec<(f64, f64)> = pts.iter().copied().filter(|(u, v)| u.is_finite() && v.is_finite()).collect();
    let ext = |f: &dyn Fn(&(f64, f64)) -> f64| {
        finite.iter().map(f).fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), v| (a.min(v), b.max(v)))
    };
    let ((u0, u1), (v0, v1)) = (ext(&|p| p.0), ext(&|p| p.1));
    if u1 <= u0 {
        return;
    }
    let vspan = if v1 > v0 { v1 - v0 } else { 1.0 };
    let line: Vec<Pos2> = finite
        .iter()
        .map(|(u, v)| {
            Pos2::new(
                chart.left() + ((u - u0) / (u1 - u0)) as f32 * chart.width(),
                chart.bottom() - ((v - v0) / vspan) as f32 * chart.height(),
            )
        })
        .collect();
    painter.add(egui::Shape::line(line, Stroke::new(1.0_f32, Color32::from_rgb(200, 30, 30))));
}

/// The Y of a polyline at `xv`: straight interpolation on the first segment that spans it.
pub fn y_at(x: &[f64], y: &[f64], xv: f64) -> Option<f64> {
    (0..x.len().min(y.len()).saturating_sub(1)).find_map(|i| {
        let (x0, x1, y0, y1) = (x[i], x[i + 1], y[i], y[i + 1]);
        let ok = [x0, x1, y0, y1].iter().all(|v| v.is_finite());
        let inside = (x0 <= xv && xv <= x1) || (x1 <= xv && xv <= x0);
        (ok && inside).then(|| if x1 == x0 { y0 } else { y0 + (y1 - y0) * (xv - x0) / (x1 - x0) })
    })
}

/// The finite (x, y) pairs of the rows inside a range.
fn points_in(x: &[f64], y: &[f64], x0: f64, x1: f64) -> Vec<(f64, f64)> {
    let Some((a, b)) = rows_in(x, x0, x1) else { return Vec::new() };
    x[a - 1..b]
        .iter()
        .zip(&y[a - 1..b])
        .map(|(u, v)| (*u, *v))
        .filter(|(u, v)| u.is_finite() && v.is_finite())
        .collect()
}

/// The Statistics gadget's numbers: count, mean, sample standard deviation, minimum, maximum.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RangeStats {
    pub n: usize,
    pub mean: f64,
    pub sd: f64,
    pub min: f64,
    pub max: f64,
}

/// The Quick Fit gadget's line y = intercept + slope x with its R².
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LineFit {
    pub intercept: f64,
    pub slope: f64,
    pub r2: f64,
}

/// The Quick Sigmoidal Fit result: the fitted parameters A1, A2, x0, dx and the function to draw.
pub struct SigmoidFit {
    pub params: Vec<f64>,
    pub func: sheaf_fit::nlsf::FitFunction,
}

impl SigmoidFit {
    pub fn value(&self, name: &str) -> f64 {
        let i = ["A1", "A2", "x0", "dx"].iter().position(|n| *n == name).unwrap_or(0);
        self.params.get(i).copied().unwrap_or(f64::NAN)
    }
}

/// The levels of the Rise Time gadget, in percent of the amplitude.
pub const RISE_LEVELS: (f64, f64) = (10.0, 90.0);

/// The state of an open gadget (Integrate or Rise Time).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Integrator {
    pub kind: GadgetKind,
    pub layer: usize,
    pub plot: usize,
    /// The ends of the range, in X data units (in either order).
    pub x0: f64,
    pub x1: f64,
    /// Cluster gadget only: the bottom and top of the rectangle, in Y data units (in either order).
    pub y0: f64,
    pub y1: f64,
    /// Subtract the straight line through the first and last point of the range.
    pub end_point_baseline: bool,
}

/// What the gadget asks the app to do.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GadgetOp {
    /// Switch the baseline between zero and the line through the end points.
    ToggleBaseline,
    /// Write the result into a sheet.
    NewOutput,
    Close,
}

/// The X and Y values of a plot as drawn (no stack offset), and its rows.
pub fn plot_xy(project: &Project, graph: WindowId, layer: usize, plot: usize) -> Option<(Vec<f64>, Vec<f64>)> {
    let p = project.graph(graph)?.layers.get(layer)?.plots.get(plot)?;
    let y = project.column(p.y)?.numeric()?.to_vec();
    let x = match p.x.and_then(|x| project.column(x)).and_then(|c| c.numeric()) {
        Some(x) => x.to_vec(),
        None => (1..=y.len()).map(|i| i as f64).collect(),
    };
    Some((x, y))
}

/// The first unbroken run of rows whose X lies inside the range, 1-based inclusive (the rule of
/// the analysis tools' "X from .. to").
pub fn rows_in(x: &[f64], x0: f64, x1: f64) -> Option<(usize, usize)> {
    let (lo, hi) = (x0.min(x1), x0.max(x1));
    let inside = |v: f64| v >= lo && v <= hi;
    let start = x.iter().position(|v| inside(*v))?;
    let len = x[start..].iter().take_while(|v| inside(**v)).count();
    Some((start + 1, start + len))
}

impl Integrator {
    /// Intersect gadget: where the gadget's plot crosses each other visible plot of numbers of its layer,
    /// with X inside the range, sorted by X.
    pub fn crossings(&self, project: &Project, graph: WindowId) -> Vec<sheaf_calc::curves::Intersection> {
        let Some((ax, ay)) = plot_xy(project, graph, self.layer, self.plot) else { return Vec::new() };
        let count = project.graph(graph).and_then(|g| g.layers.get(self.layer)).map_or(0, |l| l.plots.len());
        let visible = |i: usize| {
            project
                .graph(graph)
                .and_then(|g| g.layers.get(self.layer))
                .and_then(|l| l.plots.get(i))
                .is_some_and(|p| p.visible)
        };
        let (lo, hi) = (self.x0.min(self.x1), self.x0.max(self.x1));
        let mut out: Vec<sheaf_calc::curves::Intersection> = (0..count)
            .filter(|&i| i != self.plot && visible(i))
            .filter_map(|i| plot_xy(project, graph, self.layer, i))
            .flat_map(|(bx, by)| sheaf_calc::curves::intersections(&ax, &ay, &bx, &by, false))
            .filter(|p| p.x >= lo && p.x <= hi)
            .collect();
        out.sort_by(|a, b| a.x.total_cmp(&b.x));
        out
    }

    /// Quick Sigmoidal Fit gadget: the Boltzmann fit of the rows inside the range; `None` with fewer than five
    /// points or when the fit fails.
    pub fn sigmoid(&self, x: &[f64], y: &[f64]) -> Option<SigmoidFit> {
        let (a, b) = rows_in(x, self.x0, self.x1)?;
        let (xs, ys) = (&x[a - 1..b], &y[a - 1..b]);
        if xs.iter().zip(ys).filter(|(u, v)| u.is_finite() && v.is_finite()).count() < 5 {
            return None;
        }
        let func = sheaf_fit::library::get("Boltzmann")?;
        let report = sheaf_fit::nlsf::fit(&func, xs, ys, &Default::default()).ok()?;
        let params: Vec<f64> = report.params.iter().map(|p| p.value).collect();
        params.iter().all(|v| v.is_finite()).then_some(SigmoidFit { params, func })
    }

    /// Differentiate gadget: the first derivative at the rows inside the range, as (x, dy/dx).
    pub fn derivative(&self, x: &[f64], y: &[f64]) -> Vec<(f64, f64)> {
        let Some((a, b)) = rows_in(x, self.x0, self.x1) else { return Vec::new() };
        let (xs, ys) = (&x[a - 1..b], &y[a - 1..b]);
        if xs.len() < 2 {
            return Vec::new();
        }
        let d = sheaf_calc::differentiate::differentiate(xs, ys, &Default::default());
        xs.iter().copied().zip(d).collect()
    }

    /// Interpolate gadget: the not-a-knot cubic spline through the rows inside the range at 200 even
    /// steps; empty with fewer than three points.
    pub fn interpolated(&self, x: &[f64], y: &[f64]) -> Vec<(f64, f64)> {
        let Some((a, b)) = rows_in(x, self.x0, self.x1) else { return Vec::new() };
        let pts: Vec<(f64, f64)> = x[a - 1..b]
            .iter()
            .zip(&y[a - 1..b])
            .map(|(u, v)| (*u, *v))
            .filter(|(u, v)| u.is_finite() && v.is_finite())
            .collect();
        if pts.len() < 3 {
            return Vec::new();
        }
        let (xs, ys): (Vec<f64>, Vec<f64>) = pts.into_iter().unzip();
        let (lo, hi) =
            (xs.iter().copied().fold(f64::INFINITY, f64::min), xs.iter().copied().fold(f64::NEG_INFINITY, f64::max));
        let at: Vec<f64> = (0..=200).map(|k| lo + (hi - lo) * f64::from(k) / 200.0).collect();
        let opts = sheaf_calc::interp::InterpOptions {
            method: sheaf_calc::interp::InterpMethod::CubicSpline,
            ..Default::default()
        };
        let v = sheaf_calc::interp::interp1(&xs, &ys, &at, &opts);
        at.into_iter().zip(v).collect()
    }

    /// Cluster gadget: the points (with numbers) inside the rectangle, as (row, x, y).
    pub fn cluster_points(&self, x: &[f64], y: &[f64]) -> Vec<(usize, f64, f64)> {
        let (xl, xh) = (self.x0.min(self.x1), self.x0.max(self.x1));
        let (yl, yh) = (self.y0.min(self.y1), self.y0.max(self.y1));
        x.iter()
            .zip(y)
            .enumerate()
            .filter(|(_, (u, v))| u.is_finite() && v.is_finite() && **u >= xl && **u <= xh && **v >= yl && **v <= yh)
            .map(|(i, (u, v))| (i, *u, *v))
            .collect()
    }

    /// Cluster gadget: count and means of the points inside the rectangle; `None` when it holds none.
    pub fn cluster(&self, x: &[f64], y: &[f64]) -> Option<ClusterStats> {
        let pts = self.cluster_points(x, y);
        let n = pts.len();
        (n > 0).then(|| ClusterStats {
            n,
            mean_x: pts.iter().map(|p| p.1).sum::<f64>() / n as f64,
            mean_y: pts.iter().map(|p| p.2).sum::<f64>() / n as f64,
        })
    }

    /// Vertical Cursor gadget: the cursor's X (the middle of the band).
    pub fn cursor_x(&self) -> f64 {
        (self.x0 + self.x1) / 2.0
    }

    /// Vertical Cursor gadget: (plot index, Y at the cursor) for every visible plot of the layer that spans it.
    pub fn cursor_values(&self, project: &Project, graph: WindowId) -> Vec<(usize, f64)> {
        let Some(l) = project.graph(graph).and_then(|g| g.layers.get(self.layer)) else { return Vec::new() };
        let xc = self.cursor_x();
        (0..l.plots.len())
            .filter(|&i| l.plots[i].visible)
            .filter_map(|i| plot_xy(project, graph, self.layer, i).and_then(|(x, y)| y_at(&x, &y, xc)).map(|v| (i, v)))
            .collect()
    }

    /// Statistics gadget: the Y values inside the range; `None` without any.
    pub fn stats(&self, x: &[f64], y: &[f64]) -> Option<RangeStats> {
        let ys: Vec<f64> = points_in(x, y, self.x0, self.x1).into_iter().map(|p| p.1).collect();
        let n = ys.len();
        if n == 0 {
            return None;
        }
        let mean = ys.iter().sum::<f64>() / n as f64;
        let sd =
            if n > 1 { (ys.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (n - 1) as f64).sqrt() } else { f64::NAN };
        let (min, max) = ys.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), v| (a.min(*v), b.max(*v)));
        Some(RangeStats { n, mean, sd, min, max })
    }

    /// Quick Fit gadget: the least-squares line through the points inside the range; `None` with fewer
    /// than two points or no spread in X.
    pub fn line_fit(&self, x: &[f64], y: &[f64]) -> Option<LineFit> {
        let pts = points_in(x, y, self.x0, self.x1);
        let n = pts.len() as f64;
        if pts.len() < 2 {
            return None;
        }
        let (mx, my) = (pts.iter().map(|p| p.0).sum::<f64>() / n, pts.iter().map(|p| p.1).sum::<f64>() / n);
        let sxx: f64 = pts.iter().map(|p| (p.0 - mx).powi(2)).sum();
        let sxy: f64 = pts.iter().map(|p| (p.0 - mx) * (p.1 - my)).sum();
        let syy: f64 = pts.iter().map(|p| (p.1 - my).powi(2)).sum();
        if sxx <= 0.0 {
            return None;
        }
        let slope = sxy / sxx;
        let intercept = my - slope * mx;
        let r2 = if syy > 0.0 { sxy * sxy / (sxx * syy) } else { 1.0 };
        Some(LineFit { intercept, slope, r2 })
    }

    /// FFT gadget: the frequencies and amplitudes of the rows inside the range (at least four, with a
    /// positive mean X step).
    pub fn spectrum(&self, x: &[f64], y: &[f64]) -> Option<(Vec<f64>, Vec<f64>)> {
        let (a, b) = rows_in(x, self.x0, self.x1)?;
        let (xs, ys) = (&x[a - 1..b], &y[a - 1..b]);
        if xs.len() < 4 || ys.iter().any(|v| !v.is_finite()) {
            return None;
        }
        let interval = (xs[xs.len() - 1] - xs[0]).abs() / (xs.len() - 1) as f64;
        if !(interval > 0.0 && interval.is_finite()) {
            return None;
        }
        let r = sheaf_calc::fft::fft_spectrum(ys, &sheaf_calc::fft::FftOptions { interval, ..Default::default() });
        Some((r.frequency, r.amplitude))
    }

    /// Quick Peaks gadget: the positive peaks of the curve among the rows inside the range, as (x, y).
    pub fn peaks(&self, x: &[f64], y: &[f64]) -> Vec<(f64, f64)> {
        let Some((a, b)) = rows_in(x, self.x0, self.x1) else { return Vec::new() };
        let opts = sheaf_calc::peaks::PeakOptions {
            direction: sheaf_calc::peaks::PeakDirection::Positive,
            ..Default::default()
        };
        let found = sheaf_calc::peaks::find_peaks(&x[a - 1..b], &y[a - 1..b], &opts);
        found.x.into_iter().zip(found.y).collect()
    }

    /// The rise or fall time over the range (Rise Time gadget).
    pub fn rise(&self, x: &[f64], y: &[f64]) -> Option<sheaf_calc::risetime::RiseTime> {
        let (a, b) = rows_in(x, self.x0, self.x1)?;
        sheaf_calc::risetime::rise_time(x, y, (a - 1, b - 1), RISE_LEVELS.0, RISE_LEVELS.1)
    }

    /// The integration over the range, as the analysis tool computes it.
    pub fn result(&self, x: &[f64], y: &[f64]) -> Option<IntegrateResult> {
        let rows = rows_in(x, self.x0, self.x1)?;
        integrate(
            x,
            y,
            &IntegrateOptions { end_point_baseline: self.end_point_baseline, rows: Some(rows), ..Default::default() },
        )
    }

    /// Draws the range, the shaded area and the value, and handles dragging and the menu. Returns
    /// the range's rectangle on the screen.
    /// `geom` is the layout of the gadget's layer, `to_screen` maps page points to the screen,
    /// `placement` maps the source values to their displayed multipliers and offsets.
    #[allow(clippy::too_many_arguments)]
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        project: &Project,
        graph: WindowId,
        geom: &LayerGeometry,
        fit: f32,
        to_screen: &dyn Fn((f64, f64)) -> Pos2,
        placement: sheaf_core::stack::Place,
        c: &MenuContext,
        actions: &mut Vec<Action>,
    ) -> Option<Rect> {
        // The 2D Integrate gadget (T189) reads the layer's field, not a plot.
        let (x, y) = if self.kind == GadgetKind::Integrate2d {
            (Vec::new(), Vec::new())
        } else {
            plot_xy(project, graph, self.layer, self.plot)?
        };
        let f = geom.frame;
        let page_x = |v: f64| geom.data_to_page(placement.x(v), 0.0).0;
        let (sx0, sx1) = (to_screen((page_x(self.x0), f.y)).x, to_screen((page_x(self.x1), f.y)).x);
        let frame_top = to_screen((f.x, f.y)).y;
        let frame_bottom = to_screen((f.x, f.y + f.h)).y;
        // Cluster and 2D Integrate take a rectangle, the others a band over the whole frame height.
        let cluster = matches!(self.kind, GadgetKind::Cluster | GadgetKind::Integrate2d);
        let x_at = placement.x(self.x0);
        let screen_y = |v: f64| to_screen(geom.data_to_page(x_at, placement.y(v))).y;
        let (top, bottom) = if cluster {
            let (a, b) = (screen_y(self.y0), screen_y(self.y1));
            let t = a.min(b).clamp(frame_top, frame_bottom);
            (t, a.max(b).clamp(t, frame_bottom))
        } else {
            (frame_top, frame_bottom)
        };
        let range = Rect::from_x_y_ranges(sx0.min(sx1)..=sx0.max(sx1), top..=bottom);
        let painter = ui.painter().clone();

        // The shaded area, strip by strip (each strip is convex), at most about 2000 strips.
        let integrating = self.kind == GadgetKind::Integrate;
        let result = if integrating { self.result(&x, &y) } else { None };
        let rise = if self.kind == GadgetKind::RiseTime { self.rise(&x, &y) } else { None };
        let crossings = if self.kind == GadgetKind::Intersect { self.crossings(project, graph) } else { Vec::new() };
        let peaks = if self.kind == GadgetKind::QuickPeaks { self.peaks(&x, &y) } else { Vec::new() };
        let sigmoid = if self.kind == GadgetKind::Sigmoid { self.sigmoid(&x, &y) } else { None };
        let spectrum = if self.kind == GadgetKind::Fft { self.spectrum(&x, &y) } else { None };
        let stats = if self.kind == GadgetKind::Statistics { self.stats(&x, &y) } else { None };
        let line = if self.kind == GadgetKind::QuickFit { self.line_fit(&x, &y) } else { None };
        let cursor =
            if self.kind == GadgetKind::VerticalCursor { self.cursor_values(project, graph) } else { Vec::new() };
        if self.kind == GadgetKind::VerticalCursor {
            let xc = to_screen((geom.data_to_page(placement.x(self.cursor_x()), 0.0).0, f.y)).x;
            painter.line_segment(
                [Pos2::new(xc, top), Pos2::new(xc, bottom)],
                Stroke::new(1.5_f32, Color32::from_rgb(200, 30, 30)),
            );
            for &(i, v) in &cursor {
                let pl = sheaf_plot::stack_places(project, graph, self.layer)
                    .ok()
                    .and_then(|p| p.get(i).copied())
                    .unwrap_or_default();
                let at = to_screen(geom.data_to_page(pl.x(self.cursor_x()), pl.y(v)));
                painter.circle_stroke(at, 4.0, Stroke::new(1.5_f32, Color32::from_rgb(200, 30, 30)));
                painter.text(
                    at + egui::vec2(6.0, -4.0),
                    egui::Align2::LEFT_BOTTOM,
                    sheaf_core::format::format_general(v, 5),
                    egui::FontId::proportional(11.0),
                    Color32::from_rgb(120, 20, 20),
                );
            }
        }
        if let Some(l) = &line {
            let (lo, hi) = (self.x0.min(self.x1), self.x0.max(self.x1));
            let at = |xv: f64| to_screen(geom.data_to_page(placement.x(xv), placement.y(l.intercept + l.slope * xv)));
            painter.line_segment([at(lo), at(hi)], Stroke::new(2.0_f32, Color32::from_rgb(200, 30, 30)));
        }
        // The strongest frequency above zero.
        let strongest = spectrum.as_ref().and_then(|(f, a)| {
            f.iter()
                .zip(a)
                .filter(|(fv, av)| **fv > 0.0 && av.is_finite())
                .max_by(|p, q| p.1.total_cmp(q.1))
                .map(|(fv, _)| *fv)
        });
        if let Some((f, a)) = &spectrum {
            // A small chart in the top third of the range: frequency across, amplitude up.
            let chart = Rect::from_min_max(
                range.left_top() + egui::vec2(4.0, 20.0),
                Pos2::new(range.right() - 4.0, range.top() + 20.0 + range.height() / 3.0),
            );
            if chart.width() > 20.0 && chart.height() > 20.0 {
                painter.rect_filled(chart, 0.0, Color32::from_rgba_unmultiplied(255, 255, 255, 220));
                painter.rect_stroke(chart, 0.0, Stroke::new(1.0_f32, Color32::GRAY), egui::StrokeKind::Inside);
                let fmax = f.iter().copied().fold(0.0, f64::max);
                let amax = a.iter().copied().filter(|v| v.is_finite()).fold(0.0, f64::max);
                if fmax > 0.0 && amax > 0.0 {
                    let pts: Vec<Pos2> = f
                        .iter()
                        .zip(a)
                        .filter(|(_, av)| av.is_finite())
                        .map(|(fv, av)| {
                            Pos2::new(
                                chart.left() + (fv / fmax) as f32 * chart.width(),
                                chart.bottom() - (av / amax) as f32 * chart.height(),
                            )
                        })
                        .collect();
                    painter.add(egui::Shape::line(pts, Stroke::new(1.0_f32, Color32::from_rgb(200, 30, 30))));
                }
            }
        }
        if let Some(s) = &sigmoid {
            // The fitted curve across the range, 200 steps.
            let (lo, hi) = (self.x0.min(self.x1), self.x0.max(self.x1));
            let pts: Vec<Pos2> = (0..=200)
                .map(|k| lo + (hi - lo) * f64::from(k) / 200.0)
                .map(|xv| (xv, s.func.eval(&s.params, xv)))
                .filter(|(_, yv)| yv.is_finite())
                .map(|(xv, yv)| to_screen(geom.data_to_page(placement.x(xv), placement.y(yv))))
                .collect();
            painter.add(egui::Shape::line(pts, Stroke::new(2.0_f32, Color32::from_rgb(200, 30, 30))));
        }
        for &(px, py) in &peaks {
            let at = to_screen(geom.data_to_page(placement.x(px), placement.y(py)));
            let tip = at + egui::vec2(0.0, -4.0);
            let red = Color32::from_rgb(200, 30, 30);
            painter.add(egui::Shape::convex_polygon(
                vec![tip, tip + egui::vec2(-5.0, -8.0), tip + egui::vec2(5.0, -8.0)],
                red,
                Stroke::NONE,
            ));
            painter.text(
                tip + egui::vec2(0.0, -10.0),
                egui::Align2::CENTER_BOTTOM,
                sheaf_core::format::format_general(px, 4),
                egui::FontId::proportional(11.0),
                Color32::from_rgb(120, 20, 20),
            );
        }
        for p in &crossings {
            let at = to_screen(geom.data_to_page(placement.x(p.x), placement.y(p.y)));
            painter.circle_stroke(at, 4.0, Stroke::new(1.5_f32, Color32::from_rgb(200, 30, 30)));
            let g = |v: f64| sheaf_core::format::format_general(v, 4);
            painter.text(
                at + egui::vec2(6.0, -6.0),
                egui::Align2::LEFT_BOTTOM,
                format!("({}, {})", g(p.x), g(p.y)),
                egui::FontId::proportional(11.0),
                Color32::from_rgb(120, 20, 20),
            );
        }
        if let Some(r) = &rise {
            // The two levels across the range and the two crossings.
            let level = |v: f64| to_screen(geom.data_to_page(placement.x(self.x0), placement.y(v))).y;
            let line = Stroke::new(1.0_f32, Color32::from_rgb(60, 60, 200));
            for v in [r.level_lo, r.level_hi] {
                painter.line_segment([Pos2::new(range.left(), level(v)), Pos2::new(range.right(), level(v))], line);
            }
            let (first, second) = if r.rising { (r.level_lo, r.level_hi) } else { (r.level_hi, r.level_lo) };
            for (xv, yv) in [(r.x_start, first), (r.x_end, second)] {
                let p = to_screen(geom.data_to_page(placement.x(xv), placement.y(yv)));
                painter.circle_stroke(p, 4.0, Stroke::new(1.5_f32, Color32::from_rgb(200, 30, 30)));
            }
        }
        if let (Some(r), Some((a, b))) = (&result, rows_in(&x, self.x0, self.x1)) {
            let (a, b) = (a - 1, b - 1);
            let base = |i: usize| -> f64 {
                if !self.end_point_baseline || r.x2 == r.x1 {
                    return 0.0;
                }
                let (ya, yb) = (y[a], y[b]);
                ya + (yb - ya) * (x[i] - x[a]) / (x[b] - x[a])
            };
            let step = (b - a).div_ceil(2000).max(1);
            let fill = Color32::from_rgba_unmultiplied(120, 120, 255, 90);
            let pt = |xv: f64, yv: f64| to_screen(geom.data_to_page(placement.x(xv), placement.y(yv)));
            let mut i = a;
            while i < b {
                let j = (i + step).min(b);
                let ok = [x[i], x[j], y[i], y[j]].iter().all(|v| v.is_finite());
                if ok {
                    let quad = vec![pt(x[i], base(i)), pt(x[i], y[i]), pt(x[j], y[j]), pt(x[j], base(j))];
                    // A strip whose curve crosses the baseline is drawn as two triangles.
                    let (da, db) = (y[i] - base(i), y[j] - base(j));
                    if da * db < 0.0 {
                        let t = da / (da - db);
                        let xc = x[i] + t * (x[j] - x[i]);
                        let yc = base(i) + t * (base(j) - base(i));
                        let c = pt(xc, yc);
                        painter.add(egui::Shape::convex_polygon(vec![quad[0], quad[1], c], fill, Stroke::NONE));
                        painter.add(egui::Shape::convex_polygon(vec![c, quad[2], quad[3]], fill, Stroke::NONE));
                    } else {
                        painter.add(egui::Shape::convex_polygon(quad, fill, Stroke::NONE));
                    }
                }
                i = j;
            }
            if self.end_point_baseline {
                painter.line_segment(
                    [pt(x[a], y[a]), pt(x[b], y[b])],
                    Stroke::new(1.0_f32, Color32::from_rgb(60, 60, 200)),
                );
            }
        }

        // The range itself, with its value at the top.
        painter.rect_filled(range, 0.0, Color32::from_rgba_unmultiplied(255, 230, 0, 40));
        let edge = Stroke::new(1.5_f32, Color32::from_rgb(230, 170, 0));
        painter.line_segment([range.left_top(), range.left_bottom()], edge);
        painter.line_segment([range.right_top(), range.right_bottom()], edge);
        let text = match (self.kind, &result, &rise) {
            (GadgetKind::Integrate, Some(r), _) => format!("Area = {}", sheaf_core::format::format_general(r.area, 6)),
            (GadgetKind::Integrate, None, _) => "Area: too few points".to_string(),
            (GadgetKind::RiseTime, _, Some(r)) => format!(
                "{} = {}",
                if r.rising { "Rise Time" } else { "Fall Time" },
                sheaf_core::format::format_general(r.time, 6)
            ),
            (GadgetKind::RiseTime, _, None) => "Rise Time: no step in the range".to_string(),
            (GadgetKind::Cluster, _, _) => {
                for (_, u, v) in self.cluster_points(&x, &y) {
                    let at = to_screen(geom.data_to_page(placement.x(u), placement.y(v)));
                    painter.circle_stroke(at, 4.0, Stroke::new(1.5_f32, Color32::from_rgb(200, 30, 30)));
                }
                match self.cluster(&x, &y) {
                    Some(c) => {
                        let g = |v: f64| sheaf_core::format::format_general(v, 5);
                        format!("N = {}, Mean X = {}, Mean Y = {}", c.n, g(c.mean_x), g(c.mean_y))
                    }
                    None => "No points in the region".to_string(),
                }
            }
            (GadgetKind::Differentiate, _, _) => {
                let d = self.derivative(&x, &y);
                let xc = (self.x0 + self.x1) / 2.0;
                let at_centre =
                    d.iter().filter(|p| p.1.is_finite()).min_by(|p, q| (p.0 - xc).abs().total_cmp(&(q.0 - xc).abs()));
                mini_chart(&painter, range, &d);
                match at_centre {
                    Some(&(px, slope)) => format!(
                        "dy/dx = {} at x = {}",
                        sheaf_core::format::format_general(slope, 5),
                        sheaf_core::format::format_general(px, 5)
                    ),
                    None => "Differentiate: too few points in the range".to_string(),
                }
            }
            (GadgetKind::Interpolate, _, _) => {
                let curve = self.interpolated(&x, &y);
                let line: Vec<Pos2> = curve
                    .iter()
                    .filter(|p| p.1.is_finite())
                    .map(|&(u, v)| to_screen(geom.data_to_page(placement.x(u), placement.y(v))))
                    .collect();
                let n = line.len();
                painter.add(egui::Shape::line(line, Stroke::new(2.0_f32, Color32::from_rgb(200, 30, 30))));
                if n > 0 {
                    "Cubic spline through the range".to_string()
                } else {
                    "Interpolate: too few points in the range".to_string()
                }
            }
            (GadgetKind::Integrate2d, _, _) => {
                let layer = project.graph(graph).and_then(|g| g.layers.get(self.layer));
                let volume = layer
                    .and_then(|l| sheaf_plot::field_cells(project, l))
                    .and_then(|(cells, area)| volume_in(&cells, area, (self.x0, self.x1), (self.y0, self.y1)));
                match volume {
                    Some(v) => {
                        let g = |v: f64| sheaf_core::format::format_general(v, 5);
                        format!(
                            "Volume = {}\nN = {}, Mean = {}, SD = {}\nMin = {}, Max = {}, Sum = {}",
                            g(v.volume),
                            v.n,
                            g(v.mean),
                            g(v.sd),
                            g(v.min),
                            g(v.max),
                            g(v.sum)
                        )
                    }
                    None => "No cells in the region".to_string(),
                }
            }
            (GadgetKind::VerticalCursor, _, _) => {
                format!("x = {}", sheaf_core::format::format_general(self.cursor_x(), 6))
            }
            (GadgetKind::Statistics, _, _) => match &stats {
                Some(s) => {
                    let g = |v: f64| sheaf_core::format::format_general(v, 5);
                    format!(
                        "N = {}, Mean = {}, SD = {}\nMin = {}, Max = {}",
                        s.n,
                        g(s.mean),
                        g(s.sd),
                        g(s.min),
                        g(s.max)
                    )
                }
                None => "No points in the range".to_string(),
            },
            (GadgetKind::QuickFit, _, _) => match &line {
                Some(l) => {
                    let g = |v: f64| sheaf_core::format::format_general(v, 5);
                    format!("y = {} + {} x\nR² = {}", g(l.intercept), g(l.slope), g(l.r2))
                }
                None => "Quick Fit: too few points in the range".to_string(),
            },
            (GadgetKind::Fft, _, _) => match strongest {
                Some(fv) => format!("Strongest frequency = {}", sheaf_core::format::format_general(fv, 6)),
                None => "FFT: too few points in the range".to_string(),
            },
            (GadgetKind::Sigmoid, _, _) => match &sigmoid {
                Some(s) => format!(
                    "x0 = {}, dx = {}",
                    sheaf_core::format::format_general(s.value("x0"), 6),
                    sheaf_core::format::format_general(s.value("dx"), 4)
                ),
                None => "Sigmoidal fit failed in the range".to_string(),
            },
            (GadgetKind::QuickPeaks, _, _) => match peaks.len() {
                0 => "No peak in the range".to_string(),
                1 => "1 peak".to_string(),
                n => format!("{n} peaks"),
            },
            (GadgetKind::Intersect, _, _) => match crossings.len() {
                0 => "No intersection in the range".to_string(),
                1 => "1 intersection".to_string(),
                n => format!("{n} intersections"),
            },
        };
        painter.text(
            range.center_top() + egui::vec2(0.0, 3.0),
            egui::Align2::CENTER_TOP,
            text,
            egui::FontId::proportional(12.0),
            Color32::from_rgb(20, 20, 120),
        );

        // Dragging: an edge moves that end, the body moves both.
        let id = ui.id().with(("integrate_gadget", graph.0));
        let grip = 4.0;
        let left_is_x0 = sx0 <= sx1;
        let body = ui.interact(range.shrink2(egui::vec2(grip, 0.0)), id.with("body"), Sense::click_and_drag());
        let left = ui.interact(
            Rect::from_x_y_ranges(range.left() - grip..=range.left() + grip, top..=bottom),
            id.with("left"),
            Sense::drag(),
        );
        let right = ui.interact(
            Rect::from_x_y_ranges(range.right() - grip..=range.right() + grip, top..=bottom),
            id.with("right"),
            Sense::drag(),
        );
        for r in [&left, &right] {
            if r.hovered() || r.dragged() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
            }
        }
        // The data X under a screen x (inverse of `page_x`).
        let to_x = |sx: f32| {
            let page = f64::from((sx - to_screen((0.0, 0.0)).x) / fit);
            placement.source_x(geom.page_to_data(page, f.y).0)
        };
        let clamp = |sx: f32| sx.clamp(to_screen((f.x, f.y)).x, to_screen((f.x + f.w, f.y)).x);
        let moved = |r: &egui::Response| r.dragged().then(|| r.interact_pointer_pos()).flatten();
        if let Some(p) = moved(&left) {
            let v = to_x(clamp(p.x));
            if left_is_x0 { self.x0 = v } else { self.x1 = v }
        }
        if let Some(p) = moved(&right) {
            let v = to_x(clamp(p.x));
            if left_is_x0 { self.x1 = v } else { self.x0 = v }
        }
        // Cluster: the top and bottom edges move the rectangle's Y ends, and the body moves it up and down too.
        if cluster {
            let to_y = |sy: f32| {
                let page = f64::from((sy - to_screen((0.0, 0.0)).y) / fit);
                placement.source_y(geom.page_to_data(f.x, page).1)
            };
            let clamp_y = |sy: f32| sy.clamp(frame_top, frame_bottom);
            let edge = |y: f32, name: &str| {
                ui.interact(
                    Rect::from_x_y_ranges(range.left()..=range.right(), y - grip..=y + grip),
                    id.with(name),
                    Sense::drag(),
                )
            };
            let (t, b) = (edge(range.top(), "top"), edge(range.bottom(), "bottom"));
            for r in [&t, &b] {
                if r.hovered() || r.dragged() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
                }
            }
            let top_is_y1 = screen_y(self.y1) <= screen_y(self.y0);
            if let Some(p) = moved(&t) {
                let v = to_y(clamp_y(p.y));
                if top_is_y1 { self.y1 = v } else { self.y0 = v }
            }
            if let Some(p) = moved(&b) {
                let v = to_y(clamp_y(p.y));
                if top_is_y1 { self.y0 = v } else { self.y1 = v }
            }
            if body.dragged() {
                let dy = body.drag_delta().y;
                let (tt, bb) = (range.top() + dy, range.bottom() + dy);
                if tt >= frame_top && bb <= frame_bottom {
                    let (a, b) = (to_y(bb), to_y(tt));
                    if top_is_y1 { (self.y0, self.y1) = (a, b) } else { (self.y1, self.y0) = (a, b) }
                }
            }
        }
        if body.dragged() {
            let d = body.drag_delta().x;
            let (l, r) = (range.left() + d, range.right() + d);
            if l >= to_screen((f.x, f.y)).x && r <= to_screen((f.x + f.w, f.y)).x {
                let (a, b) = (to_x(l), to_x(r));
                if left_is_x0 { (self.x0, self.x1) = (a, b) } else { (self.x1, self.x0) = (a, b) }
            }
        }
        body.context_menu(|ui| {
            let menu = match self.kind {
                GadgetKind::Integrate => menus::integrate_gadget_menu(self.end_point_baseline),
                GadgetKind::RiseTime
                | GadgetKind::Intersect
                | GadgetKind::QuickPeaks
                | GadgetKind::Sigmoid
                | GadgetKind::Fft
                | GadgetKind::Statistics
                | GadgetKind::QuickFit
                | GadgetKind::VerticalCursor
                | GadgetKind::Differentiate
                | GadgetKind::Interpolate
                | GadgetKind::Cluster => menus::rise_time_gadget_menu(),
                GadgetKind::Integrate2d => menus::integrate_2d_gadget_menu(),
            };
            menus::show_context(ui, &menu, c, actions)
        });
        Some(range)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_rows_are_the_first_run_inside_the_range() {
        let x = [0.0, 1.0, 2.0, 3.0, 2.0, 1.0];
        assert_eq!(rows_in(&x, 0.5, 2.5), Some((2, 3)));
        assert_eq!(rows_in(&x, 2.5, 0.5), Some((2, 3)), "either order");
        assert_eq!(rows_in(&x, 5.0, 6.0), None);
    }

    #[test]
    fn the_area_matches_the_trapezoid_rule_with_and_without_baseline() {
        let x: Vec<f64> = (0..=10).map(f64::from).collect();
        let y: Vec<f64> = x.iter().map(|v| v + 1.0).collect();
        let mut g = Integrator {
            kind: GadgetKind::Integrate,
            layer: 0,
            plot: 0,
            x0: 2.0,
            x1: 4.0,
            y0: 0.0,
            y1: 0.0,
            end_point_baseline: false,
        };
        assert!((g.result(&x, &y).unwrap().area - 8.0).abs() < 1e-12, "from 2 to 4 under x + 1");
        g.end_point_baseline = true;
        assert!(g.result(&x, &y).unwrap().area.abs() < 1e-12, "a straight line has no area above its chord");
    }

    #[test]
    fn the_2d_volume_sums_the_cells_inside_the_rectangle() {
        // A 3 x 3 grid at 0, 1, 2 with Z = x + y; cells of area 1.
        let cells: Vec<[f64; 3]> =
            (0..3).flat_map(|r| (0..3).map(move |c| [c as f64, r as f64, (c + r) as f64])).collect();
        let all = volume_in(&cells, 1.0, (-1.0, 3.0), (3.0, -1.0)).unwrap();
        assert_eq!((all.n, all.sum, all.volume, all.min, all.max), (9, 18.0, 18.0, 0.0, 4.0));
        assert!((all.mean - 2.0).abs() < 1e-12);
        // The corner X, Y in 1..2: Z = 2, 3, 3, 4 (ends included), cells of area 0.5.
        let corner = volume_in(&cells, 0.5, (1.0, 2.0), (1.0, 2.0)).unwrap();
        assert_eq!((corner.n, corner.sum, corner.volume), (4, 12.0, 6.0));
        assert!((corner.sd - (2.0f64 / 3.0).sqrt()).abs() < 1e-12);
        assert!(volume_in(&cells, 1.0, (5.0, 6.0), (0.0, 1.0)).is_none());
    }
}
