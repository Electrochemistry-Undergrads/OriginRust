//! The option controls of Analysis > Mathematics > Average Multiple Curves and Intersect. (Polygon
//! Area has no options.) The window frame is the shared one of `analysis.rs`.

use eframe::egui::{self, DragValue};
use sheaf_core::analysis::{AverageMethod, AverageSpec, CommonX, IntersectSpec, Tool, XRange};

/// What the three tools keep between frames.
pub struct CurveOptions {
    pub average: AverageSpec,
    /// 0 every X value, 1 evenly spaced, 2 the X of one curve.
    x_mode: usize,
    points: usize,
    /// 1-based number of the reference curve among the input columns.
    reference: usize,
    pub intersect: IntersectSpec,
}

impl Default for CurveOptions {
    fn default() -> Self {
        CurveOptions {
            average: AverageSpec::default(),
            x_mode: 0,
            points: 100,
            reference: 1,
            intersect: IntersectSpec::default(),
        }
    }
}

impl CurveOptions {
    pub fn average_tool(&self) -> Tool {
        let x = match self.x_mode {
            1 => CommonX::Evenly(self.points),
            2 => CommonX::Reference(self.reference.saturating_sub(1)),
            _ => CommonX::Points,
        };
        let mut spec = self.average;
        spec.options.x = x;
        Tool::AverageCurves(spec)
    }

    pub fn intersect_tool(&self) -> Tool {
        Tool::Intersect(self.intersect)
    }

    pub fn average_ui(&mut self, ui: &mut egui::Ui, curves: usize) {
        ui.horizontal(|ui| {
            ui.label("X values");
            ui.radio_value(&mut self.x_mode, 0, "All X values of the curves");
            ui.radio_value(&mut self.x_mode, 1, "Evenly spaced");
            ui.radio_value(&mut self.x_mode, 2, "Those of one curve");
        });
        match self.x_mode {
            1 => {
                ui.horizontal(|ui| {
                    ui.label("Number of points");
                    ui.add(DragValue::new(&mut self.points).range(2..=10_000_000));
                });
            }
            2 => {
                ui.horizontal(|ui| {
                    ui.label("Reference curve (number among the input columns)");
                    ui.add(DragValue::new(&mut self.reference).range(1..=curves.max(1)));
                });
            }
            _ => {}
        }
        ui.horizontal(|ui| {
            ui.label("X range");
            ui.radio_value(&mut self.average.options.range, XRange::Intersection, "Covered by every curve");
            ui.radio_value(&mut self.average.options.range, XRange::Union, "Covered by any curve");
        });
        ui.horizontal(|ui| {
            ui.label("Interpolation");
            ui.radio_value(&mut self.average.options.method, AverageMethod::Linear, "Linear");
            ui.radio_value(&mut self.average.options.method, AverageMethod::CubicSpline, "Cubic spline");
            ui.radio_value(&mut self.average.options.method, AverageMethod::Akima, "Akima spline");
        });
        ui.checkbox(&mut self.average.sd, "Standard deviation (as the error column of the mean)");
        ui.checkbox(&mut self.average.count, "Number of curves at each X");
        ui.weak(
            "Curves that have the same X values are averaged row by row. Otherwise each curve's X must \
             rise or fall all the way: choose the rows of one sweep.",
        );
    }

    pub fn intersect_ui(&mut self, ui: &mut egui::Ui) {
        ui.checkbox(
            &mut self.intersect.include_touching,
            "Also list the points where the curves touch without crossing",
        );
        ui.weak("Choose exactly two columns. Each curve is a line through its points in row order.");
    }
}
