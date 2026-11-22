//! The option controls of Statistics > Multivariate Analysis > Principal Component Analysis (and the other
//! multivariate and ROC tools as they arrive). The window frame is the shared one of `analysis.rs`.

use super::analysis::AnalysisKind;
use eframe::egui;
use sheaf_core::analysis::{
    BinRule, BinScatterSpec, ChangeKind, ChangePenalty, ChangePointSpec, KaplanMeierSpec, KmeansSpec, ManhattanSpec,
    PcaSpec, PivotPeriod, PivotStat, QuantRegSpec, RocSpec, TimePivotSpec, Tool,
};

/// What the windows keep between frames.
#[derive(Default)]
pub struct MultiOptions {
    pca: PcaSpec,
    roc: RocSpec,
    quantreg: QuantRegSpec,
    changepoint: ChangePointSpec,
    binscatter: BinScatterSpec,
    timepivot: TimePivotSpec,
    manhattan: ManhattanSpec,
    /// The value of the Manual penalty while another penalty is chosen.
    manual_penalty: Option<f64>,
    /// The quantiles as typed (comma separated), read when the tool runs.
    quantiles: Option<String>,
    km: KaplanMeierSpec,
    kmeans: KmeansSpec,
}

impl MultiOptions {
    /// The quantiles as the window shows them.
    fn quantiles_text(&self) -> String {
        self.quantiles
            .clone()
            .unwrap_or_else(|| self.quantreg.quantiles.iter().map(|q| q.to_string()).collect::<Vec<_>>().join(", "))
    }

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
            AnalysisKind::QuantReg => {
                let text = self.quantiles_text();
                let quantiles: Vec<f64> = text
                    .split(',')
                    .map(str::trim)
                    .filter(|t| !t.is_empty())
                    .map(|t| t.parse::<f64>().map_err(|_| format!("\"{t}\" is not a number")))
                    .collect::<Result<_, _>>()?;
                if quantiles.is_empty() {
                    return Err("give at least one quantile".to_string());
                }
                if let Some(bad) = quantiles.iter().find(|t| !(**t > 0.0 && **t < 1.0)) {
                    return Err(format!("a quantile must lie between 0 and 1 (got {bad})"));
                }
                Ok(Tool::QuantReg(QuantRegSpec { quantiles, ..self.quantreg.clone() }))
            }
            AnalysisKind::TimePivot => Ok(Tool::TimePivot(self.timepivot)),
            AnalysisKind::Manhattan => {
                if !(self.manhattan.threshold > 0.0 && self.manhattan.threshold < 1.0) {
                    return Err("the significance level must lie between 0 and 1".to_string());
                }
                Ok(Tool::Manhattan(self.manhattan))
            }
            AnalysisKind::BinScatter => {
                let (BinRule::EqualWidth(n) | BinRule::EqualCount(n)) = self.binscatter.rule;
                if n == 0 {
                    return Err("the number of bins must be at least 1".to_string());
                }
                Ok(Tool::BinScatter(self.binscatter))
            }
            AnalysisKind::ChangePoint => {
                let need = if self.changepoint.kind == ChangeKind::Mean { 1 } else { 2 };
                if self.changepoint.min_segment < need {
                    return Err(format!("the minimum segment length must be at least {need}"));
                }
                Ok(Tool::ChangePoint(self.changepoint))
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
                ui.horizontal(|ui| {
                    ui.label("Plots of two components: horizontal axis");
                    ui.add(egui::DragValue::new(&mut s.x_component).range(1..=1000));
                    ui.label("vertical axis");
                    ui.add(egui::DragValue::new(&mut s.y_component).range(1..=1000));
                });
                ui.checkbox(&mut s.loading_plot, "Loading plot (the loadings as vectors from the origin)");
                ui.checkbox(&mut s.score_plot, "Score plot");
                ui.checkbox(&mut s.biplot, "Biplot (scores as points, loadings as vectors on the top and right axes)");
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
                    "The area under the curve with Origin's standard error and confidence interval, the                      curve points with their cut-offs, and the cut-off with the largest sensitivity + specificity - 1.",
                );
            }
            AnalysisKind::QuantReg => {
                ui.label("The chosen columns are the independent variables; the last one is the dependent variable.");
                let mut text = self.quantiles_text();
                ui.horizontal(|ui| {
                    ui.label("Quantiles");
                    ui.add(egui::TextEdit::singleline(&mut text).desired_width(200.0))
                        .on_hover_text("Comma separated, each between 0 and 1");
                });
                self.quantiles = Some(text);
                let s = &mut self.quantreg;
                ui.checkbox(&mut s.intercept, "Fit an intercept");
                ui.checkbox(&mut s.graph, "Graph of the data and the fitted lines (one independent column)");
                ui.weak(
                    "The exact solution of each quantile (the sum of check losses is as small as it can be); no standard errors.",
                );
            }
            AnalysisKind::Manhattan => {
                ui.label("The chosen columns are the group (chromosome), the position and the p-value, in that order.");
                let s = &mut self.manhattan;
                ui.horizontal(|ui| {
                    ui.label("Significance line at p =");
                    ui.add(egui::DragValue::new(&mut s.threshold).speed(1e-9).range(1e-300..=0.999_999));
                });
                ui.checkbox(&mut s.graph, "Manhattan graph (one scatter per group and the line)");
            }
            AnalysisKind::TimePivot => {
                ui.label(
                    "Every chosen column is summarized per period of its X column, which holds dates or date-times.",
                );
                let s = &mut self.timepivot;
                ui.horizontal(|ui| {
                    ui.label("Period");
                    egui::ComboBox::from_id_salt("timepivot_period").selected_text(s.period.name()).show_ui(ui, |ui| {
                        for p in PivotPeriod::ALL {
                            ui.selectable_value(&mut s.period, p, p.name());
                        }
                    });
                    ui.label("Statistic");
                    egui::ComboBox::from_id_salt("timepivot_stat").selected_text(s.stat.name()).show_ui(ui, |ui| {
                        for st in PivotStat::ALL {
                            ui.selectable_value(&mut s.stat, st, st.name());
                        }
                    });
                });
                ui.weak("Weeks start on Monday; periods without a value are left out.");
            }
            AnalysisKind::BinScatter => {
                ui.label("Every chosen column is binned against its X column (or the row number).");
                let s = &mut self.binscatter;
                let (mut n, mut width) = match s.rule {
                    BinRule::EqualWidth(n) => (n, true),
                    BinRule::EqualCount(n) => (n, false),
                };
                ui.horizontal(|ui| {
                    ui.label("Number of bins");
                    ui.add(egui::DragValue::new(&mut n).range(1..=100_000));
                });
                ui.horizontal(|ui| {
                    ui.label("Bins of");
                    ui.radio_value(&mut width, false, "Equal counts");
                    ui.radio_value(&mut width, true, "Equal width");
                });
                s.rule = if width { BinRule::EqualWidth(n) } else { BinRule::EqualCount(n) };
                ui.checkbox(&mut s.graph, "Graph of the data with the bin means");
            }
            AnalysisKind::ChangePoint => {
                ui.label("Every chosen column is a series of its own, in row order.");
                let s = &mut self.changepoint;
                ui.horizontal(|ui| {
                    ui.label("Change in");
                    ui.radio_value(&mut s.kind, ChangeKind::Mean, "Mean");
                    ui.radio_value(&mut s.kind, ChangeKind::Variance, "Variance");
                    ui.radio_value(&mut s.kind, ChangeKind::MeanVariance, "Mean and Variance");
                });
                let manual = self.manual_penalty.get_or_insert(10.0);
                ui.horizontal(|ui| {
                    ui.label("Penalty");
                    let name = match s.penalty {
                        ChangePenalty::Mbic => "MBIC",
                        ChangePenalty::Bic => "BIC (SIC)",
                        ChangePenalty::Aic => "AIC",
                        ChangePenalty::HannanQuinn => "Hannan-Quinn",
                        ChangePenalty::Manual(_) => "Manual",
                    };
                    egui::ComboBox::from_id_salt("changepoint_penalty").selected_text(name).show_ui(ui, |ui| {
                        ui.selectable_value(&mut s.penalty, ChangePenalty::Mbic, "MBIC");
                        ui.selectable_value(&mut s.penalty, ChangePenalty::Bic, "BIC (SIC)");
                        ui.selectable_value(&mut s.penalty, ChangePenalty::Aic, "AIC");
                        ui.selectable_value(&mut s.penalty, ChangePenalty::HannanQuinn, "Hannan-Quinn");
                        if ui.selectable_label(matches!(s.penalty, ChangePenalty::Manual(_)), "Manual").clicked() {
                            s.penalty = ChangePenalty::Manual(*manual);
                        }
                    });
                    if let ChangePenalty::Manual(v) = &mut s.penalty {
                        ui.add(egui::DragValue::new(v).speed(0.5).range(0.0..=1e6));
                        *manual = *v;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Minimum segment length");
                    ui.add(egui::DragValue::new(&mut s.min_segment).range(1..=100_000));
                });
                ui.checkbox(&mut s.graph, "Graph of the data with the segment means");
                ui.weak("The exact optimum of normal costs plus the penalty per change (PELT).");
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
