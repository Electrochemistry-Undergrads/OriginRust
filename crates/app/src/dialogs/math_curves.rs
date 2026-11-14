//! The option controls of Analysis > Mathematics > Average Multiple Curves and Intersect. (Polygon
//! Area has no options.) The window frame is the shared one of `analysis.rs`.

use eframe::egui::{self, DragValue};
use sheaf_core::analysis::{AverageMethod, AverageSpec, CommonX, IntersectSpec, Tool, XRange};

/// Origin's Averaged X choices (and Sheaf's "X values of one curve").
const X_MODES: [&str; 4] = ["Same as Source X", "Common X Range", "Full X Range", "X values of one curve"];

/// What the three tools keep between frames.
pub struct CurveOptions {
    pub average: AverageSpec,
    /// An index into [`X_MODES`]; Origin's default is Full X Range.
    x_mode: usize,
    /// Origin's Auto number of points (five times the mean number of points per curve, at most 1000).
    auto_points: bool,
    points: usize,
    /// 1-based number of the reference curve among the input columns.
    reference: usize,
    pub intersect: IntersectSpec,
}

impl Default for CurveOptions {
    fn default() -> Self {
        CurveOptions {
            average: AverageSpec::default(),
            x_mode: 2,
            auto_points: true,
            points: 100,
            reference: 1,
            intersect: IntersectSpec::default(),
        }
    }
}

impl CurveOptions {
    pub fn average_tool(&self) -> Tool {
        let evenly = if self.auto_points { CommonX::Auto } else { CommonX::Evenly(self.points) };
        let (x, range) = match self.x_mode {
            0 => (CommonX::Points, XRange::Intersection),
            1 => (evenly, XRange::Intersection),
            3 => (CommonX::Reference(self.reference.saturating_sub(1)), XRange::Intersection),
            _ => (evenly, XRange::Full),
        };
        let mut spec = self.average;
        spec.options.x = x;
        spec.options.range = range;
        Tool::AverageCurves(spec)
    }

    pub fn intersect_tool(&self) -> Tool {
        Tool::Intersect(self.intersect)
    }

    pub fn average_ui(&mut self, ui: &mut egui::Ui, curves: usize) {
        egui::Grid::new("avecurves").num_columns(2).spacing([14.0, 6.0]).show(ui, |ui| {
            ui.label("Averaged X");
            egui::ComboBox::from_id_salt("avecurves_x").selected_text(X_MODES[self.x_mode]).show_ui(ui, |ui| {
                for (i, name) in X_MODES.iter().enumerate() {
                    ui.selectable_value(&mut self.x_mode, i, *name);
                }
            });
            ui.end_row();
            match self.x_mode {
                1 | 2 => {
                    ui.label("Number of Points");
                    ui.horizontal(|ui| {
                        ui.add_enabled(!self.auto_points, DragValue::new(&mut self.points).range(2..=10_000_000));
                        ui.checkbox(&mut self.auto_points, "Auto");
                    });
                    ui.end_row();
                }
                3 => {
                    ui.label("Reference curve");
                    ui.add(DragValue::new(&mut self.reference).range(1..=curves.max(1)));
                    ui.end_row();
                }
                _ => {}
            }
            ui.label("Interpolate");
            egui::ComboBox::from_id_salt("avecurves_interp")
                .selected_text(match self.average.options.method {
                    AverageMethod::Linear => "Linear",
                    AverageMethod::CubicSpline => "Spline",
                    AverageMethod::Akima => "Akima Spline",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.average.options.method, AverageMethod::Linear, "Linear");
                    ui.selectable_value(&mut self.average.options.method, AverageMethod::CubicSpline, "Spline");
                    ui.selectable_value(&mut self.average.options.method, AverageMethod::Akima, "Akima Spline");
                });
            ui.end_row();
            ui.label("Output");
            ui.horizontal_wrapped(|ui| {
                ui.checkbox(&mut self.average.sd, "Std Dev");
                ui.checkbox(&mut self.average.se, "Std Err");
                ui.checkbox(&mut self.average.count, "N");
                ui.checkbox(&mut self.average.min, "Minimum");
                ui.checkbox(&mut self.average.max, "Maximum");
            });
            ui.end_row();
        });
        ui.weak(match self.x_mode {
            0 => "Curves that share their X values are averaged row by row.",
            2 => "Evenly spaced X from the smallest to the largest X of all curves; each curve is continued past its ends.",
            _ => "Each curve's X must rise or fall all the way: choose the rows of one sweep.",
        });
    }

    pub fn intersect_ui(&mut self, ui: &mut egui::Ui) {
        ui.checkbox(
            &mut self.intersect.include_touching,
            "Also list the points where the curves touch without crossing",
        );
        ui.weak("Choose exactly two columns. Each curve is a line through its points in row order.");
    }
}
