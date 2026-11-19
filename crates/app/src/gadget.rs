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
        let (x, y) = plot_xy(project, graph, self.layer, self.plot)?;
        let f = geom.frame;
        let page_x = |v: f64| geom.data_to_page(placement.x(v), 0.0).0;
        let (sx0, sx1) = (to_screen((page_x(self.x0), f.y)).x, to_screen((page_x(self.x1), f.y)).x);
        let top = to_screen((f.x, f.y)).y;
        let bottom = to_screen((f.x, f.y + f.h)).y;
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
                | GadgetKind::Fft => menus::rise_time_gadget_menu(),
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
        let mut g =
            Integrator { kind: GadgetKind::Integrate, layer: 0, plot: 0, x0: 2.0, x1: 4.0, end_point_baseline: false };
        assert!((g.result(&x, &y).unwrap().area - 8.0).abs() < 1e-12, "from 2 to 4 under x + 1");
        g.end_point_baseline = true;
        assert!(g.result(&x, &y).unwrap().area.abs() < 1e-12, "a straight line has no area above its chord");
    }
}
