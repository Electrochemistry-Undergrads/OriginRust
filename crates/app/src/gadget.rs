//! The Integrate gadget of a graph window (Origin's Gadgets > Integrate): a yellow range over one
//! plot whose edges and body can be dragged; the area between the curve and the baseline inside
//! the range is shaded and its value shown at the top of the range, live. The right-click menu
//! switches the baseline (zero, or the straight line through the end points of the range), writes
//! the result to a sheet ("New Output", the analysis tool Integrate with the same rows) and closes
//! the gadget.
//!
//! Sheaf starts the gadget at once on the active plot over the middle half of the X axis, where
//! Origin first shows a settings dialog; the settings are in the right-click menu instead.

use crate::actions::Action;
use crate::menus::{self, MenuContext};
use eframe::egui::{self, Color32, Pos2, Rect, Sense, Stroke};
use sheaf_calc::integrate::{IntegrateOptions, IntegrateResult, integrate};
use sheaf_core::{Project, WindowId};
use sheaf_plot::LayerGeometry;

/// The state of an open Integrate gadget.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Integrator {
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
        let result = self.result(&x, &y);
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
        let text = match &result {
            Some(r) => format!("Area = {}", sheaf_core::format::format_general(r.area, 6)),
            None => "Area: too few points".to_string(),
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
            menus::show_context(ui, &menus::integrate_gadget_menu(self.end_point_baseline), c, actions)
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
        let mut g = Integrator { layer: 0, plot: 0, x0: 2.0, x1: 4.0, end_point_baseline: false };
        assert!((g.result(&x, &y).unwrap().area - 8.0).abs() < 1e-12, "from 2 to 4 under x + 1");
        g.end_point_baseline = true;
        assert!(g.result(&x, &y).unwrap().area.abs() < 1e-12, "a straight line has no area above its chord");
    }
}
