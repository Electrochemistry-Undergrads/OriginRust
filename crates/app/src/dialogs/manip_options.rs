//! Options of the Data Manipulation windows (Subtract Straight Line, Reduce to Evenly Spaced X, Curve
//! Translate X and Y) and of Mathematics > Median Filter, for the shared analysis window in
//! `analysis.rs`. The tools themselves are `Tool::SubtractLine`, `ReduceX`, `Translate` and
//! `MedianFilter` of `sheaf_core::analysis` (spec: `docs/spec/analysis-tools.md` section 11).

use eframe::egui::{self, DragValue};
use sheaf_core::analysis::{
    EvenGrid, LinePoint, MedianEdge, MedianFilterSpec, ReduceMethod, ReduceOptions, Shift, SubtractLineSpec, Tool,
    TranslateAxis, TranslateSpec,
};

use super::analysis::AnalysisKind;

/// The kinds of window this module serves.
pub fn handles(kind: AnalysisKind) -> bool {
    matches!(
        kind,
        AnalysisKind::SubtractLine
            | AnalysisKind::ReduceX
            | AnalysisKind::TranslateX
            | AnalysisKind::TranslateY
            | AnalysisKind::MedianFilter
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GridChoice {
    Step,
    Count,
}

pub struct ManipOptions {
    /// Subtract Straight Line: the two points. A point's Y is used only when "Give Y" is ticked;
    /// otherwise the curve's own value at X is read.
    x1: f64,
    x2: f64,
    y1: f64,
    y2: f64,
    give_y: bool,
    /// Reduce to Evenly Spaced X.
    grid: GridChoice,
    step: f64,
    count: usize,
    method: ReduceMethod,
    own_range: bool,
    range: (f64, f64),
    /// Curve Translate: by a constant (false) or so that a point lands on a value (true).
    by_point: bool,
    by: f64,
    at: f64,
    to: f64,
    /// Median Filter.
    median: MedianFilterSpec,
}

impl ManipOptions {
    /// Starting values from the X range of the first input column.
    pub fn new(x_lo: f64, x_hi: f64) -> Self {
        let span = x_hi - x_lo;
        ManipOptions {
            x1: x_lo,
            x2: x_hi,
            y1: 0.0,
            y2: 0.0,
            give_y: false,
            grid: GridChoice::Count,
            step: if span > 0.0 { span / 100.0 } else { 1.0 },
            count: 100,
            method: ReduceMethod::Interpolate,
            own_range: false,
            range: (x_lo, x_hi),
            by_point: false,
            by: 0.0,
            at: x_lo,
            to: x_lo,
            median: MedianFilterSpec::default(),
        }
    }

    /// The tool for `kind` as set.
    pub fn tool(&self, kind: AnalysisKind) -> Tool {
        match kind {
            AnalysisKind::SubtractLine => Tool::SubtractLine(SubtractLineSpec {
                p1: LinePoint { x: self.x1, y: self.give_y.then_some(self.y1) },
                p2: LinePoint { x: self.x2, y: self.give_y.then_some(self.y2) },
            }),
            AnalysisKind::ReduceX => Tool::ReduceX(ReduceOptions {
                grid: match self.grid {
                    GridChoice::Step => EvenGrid::Step(self.step),
                    GridChoice::Count => EvenGrid::Count(self.count),
                },
                method: self.method,
                range: self.own_range.then_some(self.range),
            }),
            AnalysisKind::TranslateX | AnalysisKind::TranslateY => Tool::Translate(TranslateSpec {
                axis: if kind == AnalysisKind::TranslateX { TranslateAxis::X } else { TranslateAxis::Y },
                shift: if self.by_point { Shift::PointTo { at: self.at, to: self.to } } else { Shift::By(self.by) },
            }),
            _ => Tool::MedianFilter(self.median),
        }
    }

    pub fn options(&mut self, ui: &mut egui::Ui, kind: AnalysisKind) {
        match kind {
            AnalysisKind::SubtractLine => {
                ui.label("The line goes through two points of the curve, given by their X values:");
                ui.horizontal(|ui| {
                    ui.label("X1");
                    ui.add(DragValue::new(&mut self.x1).speed(0.01));
                    ui.add_enabled_ui(self.give_y, |ui| {
                        ui.label("Y1");
                        ui.add(DragValue::new(&mut self.y1).speed(0.01));
                    });
                });
                ui.horizontal(|ui| {
                    ui.label("X2");
                    ui.add(DragValue::new(&mut self.x2).speed(0.01));
                    ui.add_enabled_ui(self.give_y, |ui| {
                        ui.label("Y2");
                        ui.add(DragValue::new(&mut self.y2).speed(0.01));
                    });
                });
                ui.checkbox(&mut self.give_y, "Give the Y values (otherwise they are read from each curve)");
                ui.weak("Read from the curve, a point between two data points is found by a straight line.");
            }
            AnalysisKind::ReduceX => {
                ui.horizontal(|ui| {
                    ui.label("Evenly spaced by");
                    ui.radio_value(&mut self.grid, GridChoice::Count, "Number of points");
                    ui.add_enabled(
                        self.grid == GridChoice::Count,
                        DragValue::new(&mut self.count).range(2..=10_000_000),
                    );
                    ui.radio_value(&mut self.grid, GridChoice::Step, "Step");
                    ui.add_enabled(
                        self.grid == GridChoice::Step,
                        DragValue::new(&mut self.step).speed(0.001).range(1e-12..=1e12),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Y values");
                    ui.radio_value(&mut self.method, ReduceMethod::Interpolate, "Interpolated");
                    ui.radio_value(&mut self.method, ReduceMethod::Average, "Mean of the points in each bin");
                });
                ui.checkbox(&mut self.own_range, "Choose the X range of the result");
                ui.add_enabled_ui(self.own_range, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("X from");
                        ui.add(DragValue::new(&mut self.range.0).speed(0.01));
                        ui.label("to");
                        ui.add(DragValue::new(&mut self.range.1).speed(0.01));
                    });
                });
            }
            AnalysisKind::TranslateX | AnalysisKind::TranslateY => {
                let x = kind == AnalysisKind::TranslateX;
                ui.horizontal(|ui| {
                    ui.radio_value(&mut self.by_point, false, if x { "Shift X by" } else { "Shift Y by" });
                    ui.add_enabled(!self.by_point, DragValue::new(&mut self.by).speed(0.01));
                });
                ui.radio_value(
                    &mut self.by_point,
                    true,
                    if x { "Move a point to a new X" } else { "Move a point to a new Y value" },
                );
                ui.add_enabled_ui(self.by_point, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(if x { "Point at X" } else { "The curve at X" });
                        ui.add(DragValue::new(&mut self.at).speed(0.01));
                        ui.label(if x { "moves to X" } else { "moves to Y" });
                        ui.add(DragValue::new(&mut self.to).speed(0.01));
                    });
                });
                if x {
                    ui.weak("The shifted curve is written to a new sheet with its own X column.");
                }
            }
            _ => {
                ui.horizontal(|ui| {
                    ui.label("Points of window (odd)");
                    let mut n = self.median.npts;
                    ui.add(DragValue::new(&mut n).range(1..=100_001));
                    self.median.npts = n;
                });
                ui.horizontal(|ui| {
                    ui.label("At the ends");
                    ui.radio_value(&mut self.median.edge, MedianEdge::Shrink, "Shrink the window");
                    ui.radio_value(&mut self.median.edge, MedianEdge::Repeat, "Repeat the end value");
                    ui.radio_value(&mut self.median.edge, MedianEdge::Reflect, "Reflect");
                });
            }
        }
    }
}
