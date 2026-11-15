//! The option controls of Statistics > Multivariate Analysis > Principal Component Analysis (and the other
//! multivariate and ROC tools as they arrive). The window frame is the shared one of `analysis.rs`.

use super::analysis::AnalysisKind;
use eframe::egui;
use sheaf_core::analysis::{PcaSpec, Tool};

/// What the windows keep between frames.
#[derive(Default)]
pub struct MultiOptions {
    pca: PcaSpec,
}

impl MultiOptions {
    /// The tool the window runs.
    pub fn tool(&self, kind: AnalysisKind) -> Result<Tool, String> {
        match kind {
            AnalysisKind::Pca => {
                if self.pca.components == 0 {
                    return Err("the number of components must be at least 1".to_string());
                }
                Ok(Tool::Pca(self.pca))
            }
            _ => unreachable!("only the multivariate tools come here"),
        }
    }

    pub fn options(&mut self, ui: &mut egui::Ui, kind: AnalysisKind) {
        match kind {
            AnalysisKind::Pca => {
                let s = &mut self.pca;
                ui.label("Every chosen column is a variable, every row a case (at least two columns).");
                ui.horizontal(|ui| {
                    ui.label("Analyze");
                    ui.radio_value(&mut s.correlation, true, "Correlation matrix");
                    ui.radio_value(&mut s.correlation, false, "Covariance matrix");
                });
                ui.horizontal(|ui| {
                    ui.label("Missing values");
                    ui.radio_value(&mut s.pairwise, false, "Exclude listwise");
                    ui.radio_value(&mut s.pairwise, true, "Exclude pairwise");
                });
                ui.horizontal(|ui| {
                    ui.label("Number of components");
                    ui.add(egui::DragValue::new(&mut s.components).range(1..=1000));
                });
                ui.checkbox(&mut s.standardize_scores, "Standardize the scores (unit variance)");
                ui.checkbox(&mut s.scores, "Write the scores to the result sheet");
                ui.checkbox(&mut s.scree, "Scree plot (new graph)");
                ui.weak(
                    "The sign of each eigenvector is chosen so that its components add up to a positive number, as \
                     Origin does. A covariance analysis also gives the test that the remaining eigenvalues are equal.",
                );
            }
            _ => unreachable!("only the multivariate tools come here"),
        }
    }
}
