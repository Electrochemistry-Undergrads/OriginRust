//! The option controls of Statistics > Descriptive Statistics > Distribution Fit, Probability Plot and Q-Q
//! Plot. The window frame is the shared one of `analysis.rs`.

use super::analysis::AnalysisKind;
use eframe::egui;
use sheaf_core::analysis::{DistFamily, DistFitSpec, PlotScores, ProbPlotKind, ProbPlotSpec, Tool};

/// What the windows keep between frames.
pub struct DistOptions {
    /// Distribution Fit: one tick per family, in the order of [`DistFamily::ALL`].
    fit: [bool; 5],
    /// Probability Plot and Q-Q Plot: the family, and whether to draw the graph.
    family: DistFamily,
    graph: bool,
    /// The plotting positions, the probability plot's confidence band (level in percent) and the Q-Q
    /// plot's orientation, with the defaults Origin's `plot_prob` documents: Blom scores, a 95 %
    /// band, the data on X (T151; the band and scores measured by T308).
    scores: PlotScores,
    band: bool,
    level: f64,
    data_on_x: bool,
}

impl Default for DistOptions {
    fn default() -> Self {
        DistOptions {
            fit: [true; 5],
            family: DistFamily::Normal,
            graph: true,
            scores: PlotScores::Blom,
            band: true,
            level: 95.0,
            data_on_x: true,
        }
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
            _ => Tool::ProbPlot(ProbPlotSpec {
                kind: if kind == AnalysisKind::QQPlot { ProbPlotKind::QQ } else { ProbPlotKind::Probability },
                family: self.family,
                graph: self.graph,
                scores: self.scores,
                band: (kind != AnalysisKind::QQPlot && self.band).then_some(self.level / 100.0),
                data_on_x: self.data_on_x,
            }),
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
            _ => {
                ui.horizontal(|ui| {
                    ui.label("Distribution");
                    egui::ComboBox::from_id_salt("dist_family").selected_text(self.family.name()).show_ui(ui, |ui| {
                        for f in DistFamily::ALL {
                            ui.selectable_value(&mut self.family, f, f.name());
                        }
                    });
                });
                ui.horizontal(|ui| {
                    ui.label("Scores");
                    egui::ComboBox::from_id_salt("dist_scores").selected_text(self.scores.name()).show_ui(ui, |ui| {
                        for s in PlotScores::ALL {
                            ui.selectable_value(&mut self.scores, s, s.name());
                        }
                    });
                })
                .response
                .on_hover_text("How the cumulative probability of each ordered value is estimated");
                if kind == AnalysisKind::QQPlot {
                    ui.checkbox(&mut self.data_on_x, "Data on X, quantiles on Y");
                } else {
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut self.band, "Confidence band");
                        ui.add_enabled(
                            self.band,
                            egui::DragValue::new(&mut self.level).range(50.0..=99.99).speed(0.1).suffix(" %"),
                        );
                    })
                    .response
                    .on_hover_text("Drawn round the fitted line at 27 percents, as Origin draws it");
                }
                ui.checkbox(&mut self.graph, "Make a graph");
                ui.weak(if kind == AnalysisKind::QQPlot {
                    "The quantiles of the distribution against the ordered data, with the least-squares line. \
                     Families with a shape take the fitted shape."
                } else {
                    "The data against the plotting position in percent on a probability axis, with the fitted \
                     distribution function."
                });
            }
        }
    }
}
