//! The option controls of Statistics > Descriptive Statistics > Distribution Fit, Probability Plot and Q-Q
//! Plot, and of the Power and Sample Size windows. The window frame is the shared one of `analysis.rs`.

use super::analysis::AnalysisKind;
use eframe::egui::{self, DragValue};
use sheaf_core::analysis::{
    DistFamily, DistFitSpec, PowerEffect, PowerSolve, PowerSpec, PowerTest, ProbPlotKind, ProbPlotSpec, Tail, Tool,
};

/// The power windows read no data.
pub fn is_power(kind: AnalysisKind) -> bool {
    matches!(
        kind,
        AnalysisKind::PowerOneSampleT
            | AnalysisKind::PowerTwoSampleT
            | AnalysisKind::PowerPairedT
            | AnalysisKind::PowerAnova
    )
}

/// What the windows keep between frames.
pub struct DistOptions {
    /// Distribution Fit: one tick per family, in the order of [`DistFamily::ALL`].
    fit: [bool; 5],
    /// Probability Plot and Q-Q Plot: the family, and whether to draw the graph.
    family: DistFamily,
    graph: bool,
    /// Power: the effect as a standardized size (0) or as differences or means with a standard deviation (1).
    effect_mode: usize,
    effect: f64,
    difference: f64,
    sd: f64,
    means: String,
    groups: usize,
    ratio: f64,
    /// Power: compute the sample size for wanted powers (true) or the power for sample sizes (false).
    want_size: bool,
    sizes: String,
    powers: String,
    tail: Tail,
    alpha: f64,
}

impl Default for DistOptions {
    fn default() -> Self {
        DistOptions {
            fit: [true; 5],
            family: DistFamily::Normal,
            graph: true,
            effect_mode: 0,
            effect: 0.5,
            difference: 1.0,
            sd: 2.0,
            means: "4, 5, 6".to_string(),
            groups: 3,
            ratio: 1.0,
            want_size: true,
            sizes: "10, 20, 30".to_string(),
            powers: "0.8, 0.9".to_string(),
            tail: Tail::Two,
            alpha: 0.05,
        }
    }
}

fn numbers<T: std::str::FromStr>(text: &str, what: &str) -> Result<Vec<T>, String> {
    let list: Result<Vec<T>, _> =
        text.split([',', ';', ' ']).map(str::trim).filter(|s| !s.is_empty()).map(str::parse).collect();
    match list {
        Ok(v) if !v.is_empty() => Ok(v),
        _ => Err(format!("write {what} as numbers separated by commas")),
    }
}

impl DistOptions {
    /// The tool the window runs.
    pub fn tool(&self, kind: AnalysisKind) -> Result<Tool, String> {
        Ok(match kind {
            AnalysisKind::DistFit => {
                let families: Vec<DistFamily> =
                    DistFamily::ALL.iter().zip(self.fit).filter(|(_, on)| *on).map(|(f, _)| *f).collect();
                if families.is_empty() {
                    return Err("choose at least one distribution".to_string());
                }
                Tool::DistFit(DistFitSpec { families })
            }
            AnalysisKind::ProbPlot | AnalysisKind::QQPlot => Tool::ProbPlot(ProbPlotSpec {
                kind: if kind == AnalysisKind::QQPlot { ProbPlotKind::QQ } else { ProbPlotKind::Probability },
                family: self.family,
                graph: self.graph,
            }),
            _ => {
                let anova = kind == AnalysisKind::PowerAnova;
                let test = match kind {
                    AnalysisKind::PowerOneSampleT => PowerTest::OneSampleT,
                    AnalysisKind::PowerPairedT => PowerTest::PairedT,
                    AnalysisKind::PowerAnova => PowerTest::OneWayAnova { groups: self.groups },
                    _ => PowerTest::TwoSampleT { ratio: self.ratio },
                };
                let effect = match (self.effect_mode, anova) {
                    (0, _) => PowerEffect::Standardized(self.effect),
                    (_, true) => PowerEffect::Means { means: numbers(&self.means, "the group means")?, sd: self.sd },
                    (_, false) => PowerEffect::Difference { difference: self.difference, sd: self.sd },
                };
                let solve = if self.want_size {
                    PowerSolve::SampleSize { powers: numbers(&self.powers, "the wanted powers")? }
                } else {
                    PowerSolve::Power { sample_sizes: numbers(&self.sizes, "the sample sizes")? }
                };
                Tool::Power(PowerSpec { test, effect, solve, tail: self.tail, alpha: self.alpha })
            }
        })
    }

    pub fn options(&mut self, ui: &mut egui::Ui, kind: AnalysisKind) {
        match kind {
            AnalysisKind::DistFit => {
                ui.label("Distributions to fit (maximum likelihood)");
                ui.horizontal_wrapped(|ui| {
                    for (family, on) in DistFamily::ALL.iter().zip(self.fit.iter_mut()) {
                        ui.checkbox(on, family.name());
                    }
                });
                ui.weak(
                    "The lognormal, exponential, gamma and Weibull distributions have their threshold fixed at 0, \
                     so they need positive numbers. A family that cannot take a column is left out for it.",
                );
            }
            AnalysisKind::ProbPlot | AnalysisKind::QQPlot => {
                ui.horizontal(|ui| {
                    ui.label("Distribution");
                    egui::ComboBox::from_id_salt("dist_family").selected_text(self.family.name()).show_ui(ui, |ui| {
                        for f in DistFamily::ALL {
                            ui.selectable_value(&mut self.family, f, f.name());
                        }
                    });
                });
                ui.checkbox(&mut self.graph, "Make a graph");
                ui.weak(if kind == AnalysisKind::QQPlot {
                    "The quantiles of the distribution against the ordered data, with the least-squares line. \
                     Families with a shape take the fitted shape."
                } else {
                    "The data against the plotting position in percent on a probability axis, with the fitted \
                     distribution function."
                });
            }
            _ => self.power_options(ui, kind),
        }
    }

    fn power_options(&mut self, ui: &mut egui::Ui, kind: AnalysisKind) {
        let anova = kind == AnalysisKind::PowerAnova;
        ui.horizontal(|ui| {
            ui.label("Compute");
            ui.radio_value(&mut self.want_size, false, "Power for sample sizes");
            ui.radio_value(&mut self.want_size, true, "Sample size for power");
        });
        let unit = match kind {
            AnalysisKind::PowerOneSampleT => "observations",
            AnalysisKind::PowerPairedT => "pairs",
            AnalysisKind::PowerTwoSampleT => "observations in the first sample",
            _ => "observations per group",
        };
        if self.want_size {
            ui.horizontal(|ui| {
                ui.label("Wanted powers");
                ui.add(egui::TextEdit::singleline(&mut self.powers).desired_width(140.0));
            });
        } else {
            ui.horizontal(|ui| {
                ui.label(format!("Sample sizes ({unit})"));
                ui.add(egui::TextEdit::singleline(&mut self.sizes).desired_width(140.0));
            });
        }
        ui.horizontal(|ui| {
            ui.label("Effect");
            ui.radio_value(&mut self.effect_mode, 0, if anova { "Effect size f" } else { "Effect size d" });
            ui.radio_value(&mut self.effect_mode, 1, if anova { "Group means" } else { "Difference" });
        });
        if self.effect_mode == 0 {
            ui.horizontal(|ui| {
                ui.label(if anova { "f" } else { "d" });
                ui.add(DragValue::new(&mut self.effect).speed(0.01).range(0.001..=100.0));
                ui.weak(if anova {
                    "(standard deviation of the group means over the common standard deviation)"
                } else {
                    "(difference over the standard deviation)"
                });
            });
            if anova {
                ui.horizontal(|ui| {
                    ui.label("Number of groups");
                    ui.add(DragValue::new(&mut self.groups).range(2..=1000));
                });
            }
        } else {
            if anova {
                ui.horizontal(|ui| {
                    ui.label("Group means");
                    ui.add(egui::TextEdit::singleline(&mut self.means).desired_width(180.0));
                });
            } else {
                ui.horizontal(|ui| {
                    ui.label("Difference to detect");
                    ui.add(DragValue::new(&mut self.difference).speed(0.01));
                });
            }
            ui.horizontal(|ui| {
                ui.label(if kind == AnalysisKind::PowerPairedT {
                    "Standard deviation of the differences"
                } else {
                    "Standard deviation"
                });
                ui.add(DragValue::new(&mut self.sd).speed(0.01).range(1e-12..=1e12));
            });
        }
        if kind == AnalysisKind::PowerTwoSampleT {
            ui.horizontal(|ui| {
                ui.label("Ratio of the sample sizes (second / first)");
                ui.add(DragValue::new(&mut self.ratio).speed(0.05).range(0.05..=20.0));
            });
        }
        if !anova {
            ui.horizontal(|ui| {
                ui.label("Alternative");
                ui.radio_value(&mut self.tail, Tail::Two, "Two-tailed");
                ui.radio_value(&mut self.tail, Tail::Upper, "One-tailed");
            });
        }
        ui.horizontal(|ui| {
            ui.label("Significance level");
            ui.add(DragValue::new(&mut self.alpha).speed(0.001).range(0.0001..=0.5));
        });
        ui.weak("Equal variances are assumed. The result goes to a new sheet; no data are read.");
    }
}
