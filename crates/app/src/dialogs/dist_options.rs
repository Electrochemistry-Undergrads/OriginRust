//! The option controls of Statistics > Descriptive Statistics > Distribution Fit, Probability Plot and Q-Q
//! Plot. The window frame is the shared one of `analysis.rs`.

use super::analysis::AnalysisKind;
use eframe::egui;
use sheaf_core::analysis::{DistFamily, DistFitSpec, ProbPlotKind, ProbPlotSpec, Tool};

/// What the windows keep between frames.
pub struct DistOptions {
    /// Distribution Fit: one tick per family, in the order of [`DistFamily::ALL`].
    fit: [bool; 5],
    /// Probability Plot and Q-Q Plot: the family, and whether to draw the graph.
    family: DistFamily,
    graph: bool,
}

impl Default for DistOptions {
    fn default() -> Self {
        DistOptions { fit: [true; 5], family: DistFamily::Normal, graph: true }
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
