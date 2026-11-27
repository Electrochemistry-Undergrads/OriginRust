//! Statistics > Power and Sample Size: one window for the power of a t test, the one-way ANOVA or a proportion or
//! variance test (T415), or the sample size that reaches a wanted power. It needs no worksheet data; the result goes to a new workbook.

use eframe::egui;
use egui::DragValue;
use sheaf_core::Document;
use sheaf_core::analysis::Report;
use sheaf_core::analysis::{PowerGoal, PowerKind, PowerSpec};

/// The window of one power analysis.
pub struct PowerDialog {
    kind: PowerKind,
    /// True: compute sample sizes for the wanted powers; false: the powers of the sample sizes.
    find_size: bool,
    alpha: f64,
    two_sided: bool,
    mean1: f64,
    mean2: f64,
    sd: f64,
    levels: f64,
    css: f64,
    sizes_text: String,
    powers_text: String,
    /// T500: the second group's size over the first's, the pooled two-proportion test, the exact one-proportion test.
    ratio: f64,
    pooled: bool,
    exact: bool,
    problem: Option<String>,
    undo_token: u64,
}

/// What the window did in a frame.
pub enum PowerOutcome {
    Open,
    Closed,
    Done(Box<Report>),
}

fn parse_list(text: &str) -> Result<Vec<f64>, String> {
    text.split([',', ' ', ';'])
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.trim().parse::<f64>().map_err(|_| format!("'{}' is not a number", s.trim())))
        .collect()
}

impl PowerDialog {
    pub fn new(kind: PowerKind) -> Self {
        let (mean1, mean2) = match kind {
            PowerKind::OneProportion | PowerKind::TwoProportions => (0.6, 0.5),
            PowerKind::OneVariance | PowerKind::TwoVariances => (2.0, 1.0),
            _ => (0.0, 1.0),
        };
        PowerDialog {
            kind,
            find_size: false,
            alpha: 0.05,
            two_sided: true,
            mean1,
            mean2,
            sd: 1.0,
            levels: 3.0,
            css: 1.0,
            sizes_text: "50 100 200".to_string(),
            powers_text: "0.9 0.95 0.99".to_string(),
            ratio: 1.0,
            pooled: false,
            exact: false,
            problem: None,
            undo_token: crate::dialogs::analysis::next_undo_token(),
        }
    }

    #[cfg(test)]
    pub fn kind(&self) -> PowerKind {
        self.kind
    }

    /// The inputs as typed, or what is wrong with them.
    pub fn spec(&self) -> Result<PowerSpec, String> {
        let goal = if self.find_size {
            PowerGoal::SampleSize(parse_list(&self.powers_text)?)
        } else {
            PowerGoal::Power(parse_list(&self.sizes_text)?)
        };
        Ok(PowerSpec {
            kind: self.kind,
            goal,
            alpha: self.alpha,
            two_sided: self.two_sided,
            mean1: self.mean1,
            mean2: if self.kind == PowerKind::PairedT { 0.0 } else { self.mean2 },
            sd: self.sd,
            levels: self.levels,
            css: self.css,
            ratio: if matches!(self.kind, PowerKind::TwoProportions | PowerKind::TwoVariances) {
                self.ratio
            } else {
                1.0
            },
            pooled: self.pooled && self.kind == PowerKind::TwoProportions,
            exact: self.exact && self.kind == PowerKind::OneProportion,
        })
    }

    fn run(&mut self, doc: &mut Document) -> Result<Report, String> {
        let spec = self.spec()?;
        doc.apply_merging("Power and sample size", self.undo_token, |p| p.power_analysis(&spec))
            .map_err(|e| e.to_string())
    }

    pub fn show(&mut self, ctx: &egui::Context, doc: &mut Document) -> PowerOutcome {
        let mut open = true;
        let mut outcome = PowerOutcome::Open;
        let title = format!("Power and Sample Size: {}", self.kind.label());
        egui::Window::new(title).open(&mut open).collapsible(false).resizable(false).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Compute");
                ui.radio_value(&mut self.find_size, false, "Power");
                ui.radio_value(&mut self.find_size, true, "Sample Size");
            });
            egui::Grid::new("power_grid").num_columns(2).show(ui, |ui| {
                let (first, second) = match self.kind {
                    PowerKind::OneSampleT => (Some("Mean"), Some("Hypothesized mean")),
                    PowerKind::TwoSampleT => (Some("Mean 1"), Some("Mean 2")),
                    PowerKind::PairedT => (Some("Mean of the differences"), None),
                    PowerKind::Anova => (None, None),
                    PowerKind::OneProportion => (Some("Proportion"), Some("Hypothesized proportion")),
                    PowerKind::TwoProportions => (Some("Proportion 1"), Some("Proportion 2")),
                    PowerKind::OneVariance => (Some("Variance"), Some("Hypothesized variance")),
                    PowerKind::TwoVariances => (Some("Variance 1"), Some("Variance 2")),
                };
                let rate = self.kind.rate_test().is_some();
                if let Some(l) = first {
                    ui.label(l);
                    ui.add(DragValue::new(&mut self.mean1).speed(0.01));
                    ui.end_row();
                }
                if let Some(l) = second {
                    ui.label(l);
                    ui.add(DragValue::new(&mut self.mean2).speed(0.01));
                    ui.end_row();
                }
                if self.kind == PowerKind::Anova {
                    ui.label("Number of groups");
                    ui.add(DragValue::new(&mut self.levels).speed(1.0).range(2.0..=100.0));
                    ui.end_row();
                    ui.label("Corrected sum of squares of the means");
                    ui.add(DragValue::new(&mut self.css).speed(0.01).range(0.0..=1e9));
                    ui.end_row();
                }
                if !rate {
                    ui.label(if self.kind == PowerKind::Anova {
                        "Standard deviation (root MSE)"
                    } else {
                        "Standard deviation"
                    });
                    ui.add(DragValue::new(&mut self.sd).speed(0.01).range(1e-12..=1e12));
                    ui.end_row();
                }
                ui.label("Significance level");
                ui.add(DragValue::new(&mut self.alpha).speed(0.001).range(0.0001..=0.5));
                ui.end_row();
                if self.kind != PowerKind::Anova {
                    ui.label("Test");
                    ui.horizontal(|ui| {
                        ui.radio_value(&mut self.two_sided, true, "2-sided");
                        ui.radio_value(&mut self.two_sided, false, "1-sided");
                    });
                    ui.end_row();
                }
                // T500: the test variants and unequal groups.
                if self.kind == PowerKind::OneProportion {
                    ui.label("Method");
                    ui.horizontal(|ui| {
                        ui.radio_value(&mut self.exact, false, "Normal approximation");
                        ui.radio_value(&mut self.exact, true, "Exact binomial");
                    });
                    ui.end_row();
                }
                if self.kind == PowerKind::TwoProportions {
                    ui.label("Standard error");
                    ui.horizontal(|ui| {
                        ui.radio_value(&mut self.pooled, false, "Unpooled");
                        ui.radio_value(&mut self.pooled, true, "Pooled");
                    });
                    ui.end_row();
                }
                if matches!(self.kind, PowerKind::TwoProportions | PowerKind::TwoVariances) {
                    ui.label("Group 2 size / group 1 size");
                    ui.add(DragValue::new(&mut self.ratio).speed(0.01).range(0.01..=100.0));
                    ui.end_row();
                }
                if self.find_size {
                    ui.label("Powers");
                    ui.add(egui::TextEdit::singleline(&mut self.powers_text).desired_width(160.0));
                } else {
                    ui.label(if self.kind.per_group() { "Sample sizes (per group)" } else { "Sample sizes" });
                    ui.add(egui::TextEdit::singleline(&mut self.sizes_text).desired_width(160.0));
                }
                ui.end_row();
            });
            ui.weak("Separate several values with spaces or commas.");
            if let Some(p) = &self.problem {
                ui.colored_label(ui.visuals().error_fg_color, p);
            }
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("OK").clicked() {
                    match self.run(doc) {
                        Ok(report) => outcome = PowerOutcome::Done(Box::new(report)),
                        Err(e) => self.problem = Some(e),
                    }
                }
                if ui.button("Cancel").clicked() {
                    outcome = PowerOutcome::Closed;
                }
            });
        });
        if !open && matches!(outcome, PowerOutcome::Open) {
            outcome = PowerOutcome::Closed;
        }
        outcome
    }
}
