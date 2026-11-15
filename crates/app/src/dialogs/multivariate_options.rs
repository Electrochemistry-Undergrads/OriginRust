//! The option controls of Statistics > Multivariate Analysis > Principal Component Analysis (and the other
//! multivariate and ROC tools as they arrive). The window frame is the shared one of `analysis.rs`.

use super::analysis::AnalysisKind;
use eframe::egui;
use sheaf_core::analysis::{KaplanMeierSpec, KmeansSpec, PcaSpec, RocSpec, Tool};

/// What the windows keep between frames.
#[derive(Default)]
pub struct MultiOptions {
    pca: PcaSpec,
    roc: RocSpec,
    km: KaplanMeierSpec,
    kmeans: KmeansSpec,
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
            AnalysisKind::Roc => {
                if self.roc.positive.trim().is_empty() {
                    return Err("give the value of the state column that is the positive state".to_string());
                }
                Ok(Tool::Roc(self.roc.clone()))
            }
            AnalysisKind::Kmeans => {
                if self.kmeans.clusters < 2 {
                    return Err("the number of clusters must be at least 2".to_string());
                }
                Ok(Tool::Kmeans(self.kmeans))
            }
            AnalysisKind::KaplanMeier => {
                if self.km.censor.trim().is_empty() {
                    return Err("give the censoring value".to_string());
                }
                Ok(Tool::KaplanMeier(self.km.clone()))
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
            AnalysisKind::Roc => {
                let s = &mut self.roc;
                ui.label("The chosen columns are the test variables; the last one is the state column.");
                ui.horizontal(|ui| {
                    ui.label("Positive state value");
                    ui.add(egui::TextEdit::singleline(&mut s.positive).desired_width(80.0));
                });
                ui.horizontal(|ui| {
                    ui.label("Test direction");
                    ui.radio_value(&mut s.higher_is_positive, true, "Positive v.s. High");
                    ui.radio_value(&mut s.higher_is_positive, false, "Positive v.s. Low");
                });
                ui.horizontal(|ui| {
                    ui.label("Confidence level (%)");
                    ui.add(egui::DragValue::new(&mut s.confidence).speed(0.1).range(1.0..=99.99));
                });
                ui.checkbox(&mut s.curve, "ROC curve (new graph, with the diagonal)");
                ui.weak(
                    "The area under the curve with its standard error (DeLong's method) and confidence interval, the                      curve points with their cut-offs, and the cut-off with the largest sensitivity + specificity - 1.",
                );
            }
            AnalysisKind::Kmeans => {
                let s = &mut self.kmeans;
                ui.label("Every chosen column is a variable, every row a case.");
                ui.horizontal(|ui| {
                    ui.label("Number of clusters");
                    ui.add(egui::DragValue::new(&mut s.clusters).range(2..=1000));
                });
                ui.horizontal(|ui| {
                    ui.label("Maximum passes");
                    ui.add(egui::DragValue::new(&mut s.max_passes).range(1..=10000));
                });
                ui.checkbox(&mut s.anova, "Analysis of variance of the variables by cluster");
                ui.checkbox(&mut s.membership, "Write the cluster of every row");
                ui.add_enabled(
                    s.membership,
                    egui::Checkbox::new(&mut s.member_distance, "... and its distance to the centre"),
                );
                ui.checkbox(&mut s.graph, "Scatter graph of the first two variables, one plot per cluster");
                ui.weak(
                    "The initial centres are picked from the rows by replacement in sheet order, as Origin does, so the \
                     cluster numbers depend on the order of the rows. Distances are Euclidean on the values as they are.",
                );
            }
            AnalysisKind::KaplanMeier => {
                let s = &mut self.km;
                ui.label("The chosen columns are the time, the censor column and, optionally, a grouping column.");
                ui.horizontal(|ui| {
                    ui.label("Censoring value");
                    ui.add(egui::TextEdit::singleline(&mut s.censor).desired_width(80.0));
                });
                ui.horizontal(|ui| {
                    ui.label("Confidence level (%)");
                    ui.add(egui::DragValue::new(&mut s.confidence).speed(0.1).range(1.0..=99.99));
                });
                ui.checkbox(&mut s.quartiles, "Quartile estimates");
                ui.checkbox(&mut s.mean, "Mean estimate");
                ui.label("Tests of groups");
                ui.checkbox(&mut s.log_rank, "Log-rank");
                ui.checkbox(&mut s.breslow, "Breslow");
                ui.checkbox(&mut s.tarone_ware, "Tarone-Ware");
                ui.checkbox(&mut s.curve, "Survival curves (new graph, censored cases marked)");
                ui.add_enabled(s.curve, egui::Checkbox::new(&mut s.band, "Add confidence limits to the graph"));
                ui.weak(
                    "Product-limit estimate with Greenwood standard errors. Quartile limits come from the linear                      limits of the curve; the mean is cut off at the largest time.",
                );
            }
            _ => unreachable!("only the multivariate tools come here"),
        }
    }
}
