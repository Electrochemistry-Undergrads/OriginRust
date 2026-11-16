//! The options of Statistics > Descriptive Statistics > Partial Correlation Coefficient and Statistics > ANOVA >
//! Three-Way ANOVA (T81), laid out as Origin's dialogs (`docs/spec/statistics.md` 24 and 25). The three-way
//! dialog has Origin's tabs Input, Model, Descriptive Statistics, Means Comparison and Power Analysis; its Mean
//! Plot tab, the Raw data layout and the Output tab are not built (the results go to a new sheet).

use eframe::egui::{self, DragValue, Ui};
use sheaf_core::ColumnId;
use sheaf_core::analysis::{MeansComparison, PartialCorrSpec, ThreeWayAnovaSpec, ThreeWayEffect, ThreeWayTerms};

/// The seven means comparison methods of the three-way ANOVA, in Origin's order.
const METHODS: [MeansComparison; 7] = [
    MeansComparison::Tukey,
    MeansComparison::Bonferroni,
    MeansComparison::Sidak,
    MeansComparison::FisherLsd,
    MeansComparison::Scheffe,
    MeansComparison::HolmBonferroni,
    MeansComparison::HolmSidak,
];

/// The state of the partial correlation window.
/// Origin's defaults are all off: pairwise, two-tailed, no significance blocks, no flags.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PartCorrState {
    /// Indexes into the sheet's columns: the controlling variables.
    pub controls: Vec<usize>,
    pub listwise: bool,
    pub one_tailed: bool,
    pub show_significance: bool,
    pub flag_significant: bool,
}

impl PartCorrState {
    pub fn spec(&self, sheet_columns: &[(ColumnId, String)]) -> Result<PartialCorrSpec, String> {
        let controls: Vec<ColumnId> =
            self.controls.iter().filter_map(|&i| sheet_columns.get(i)).map(|(id, _)| *id).collect();
        if controls.is_empty() {
            return Err("choose at least one controlling variable".to_string());
        }
        Ok(PartialCorrSpec {
            controls,
            listwise: self.listwise,
            one_tailed: self.one_tailed,
            show_significance: self.show_significance,
            flag_significant: self.flag_significant,
        })
    }

    /// Re-chooses the controls of the window after the variables changed: a variable cannot control itself.
    pub fn drop_variables(&mut self, sheet_columns: &[(ColumnId, String)], variables: &[ColumnId]) {
        self.controls.retain(|&i| sheet_columns.get(i).is_some_and(|(id, _)| !variables.contains(id)));
    }

    pub fn ui(&mut self, ui: &mut Ui, sheet_columns: &[(ColumnId, String)], variables: &[ColumnId]) {
        ui.horizontal(|ui| {
            ui.label("Controlling Variables");
            let chosen: Vec<&str> = self
                .controls
                .iter()
                .filter_map(|&i| sheet_columns.get(i))
                .map(|(_, n)| n.split("  ").next().unwrap_or(""))
                .collect();
            let text = if chosen.is_empty() { "(none)".to_string() } else { chosen.join(", ") };
            egui::ComboBox::from_id_salt("partcorr_controls").selected_text(text).width(220.0).show_ui(ui, |ui| {
                for (j, (id, name)) in sheet_columns.iter().enumerate() {
                    if variables.contains(id) {
                        continue;
                    }
                    let mut on = self.controls.contains(&j);
                    if ui.checkbox(&mut on, name).changed() {
                        if on {
                            self.controls.push(j);
                        } else {
                            self.controls.retain(|k| *k != j);
                        }
                    }
                }
            });
        });
        ui.horizontal(|ui| {
            ui.label("Missing values");
            ui.radio_value(&mut self.listwise, false, "Pairwise");
            ui.radio_value(&mut self.listwise, true, "Listwise");
        });
        ui.horizontal(|ui| {
            ui.label("Test of significance");
            ui.radio_value(&mut self.one_tailed, false, "Two-tailed");
            ui.radio_value(&mut self.one_tailed, true, "One-tailed");
        });
        ui.checkbox(&mut self.show_significance, "Show Significance in Result Table")
            .on_hover_text("Adds the p-value and the degrees of freedom of every coefficient.");
        ui.checkbox(&mut self.flag_significant, "Flag Significant Correlations")
            .on_hover_text("A star after the coefficients that are significantly different from zero at 0.05.");
        ui.weak("Pearson's coefficient only, as in Origin. The variables are the Input columns above.");
    }
}

/// The tabs of the three-way window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
    Input,
    Model,
    Descriptive,
    Means,
    Power,
}

/// The state of the three-way ANOVA window.
#[derive(Debug, Clone, PartialEq)]
pub struct Anova3State {
    tab: Tab,
    /// Indexes into the sheet's columns: factor A, factor B, factor C and the data.
    pub columns: [usize; 4],
    pub alpha: f64,
    pub levene: bool,
    /// A*B, A*C, B*C, A*B*C in the model.
    pub terms: [bool; 4],
    /// Descriptive statistics of A, B, C, A*B, A*C, B*C, A*B*C.
    pub descriptive: [bool; 7],
    pub overall: bool,
    pub methods: [bool; 7],
    pub comparison_alpha: f64,
    pub power_actual: bool,
    pub power_hypothetical: bool,
    /// The hypothetical sample sizes as typed (Origin's default `50 100 200`).
    pub power_sizes: String,
    pub power_alpha: f64,
}

impl Default for Anova3State {
    /// Sheaf's own defaults for what Origin's dialog starts with (not read from Origin): all descriptive
    /// statistics, Tukey, no Levene, no power; the model has every interaction.
    fn default() -> Self {
        Anova3State {
            tab: Tab::Input,
            columns: [0, 1, 2, 3],
            alpha: 0.05,
            levene: false,
            terms: [true; 4],
            descriptive: [true; 7],
            overall: true,
            methods: [true, false, false, false, false, false, false],
            comparison_alpha: 0.05,
            power_actual: false,
            power_hypothetical: false,
            power_sizes: "50 100 200".to_string(),
            power_alpha: 0.05,
        }
    }
}

/// The numbers in `text`, separated by spaces, commas or semicolons.
fn parse_sizes(text: &str) -> Result<Vec<f64>, String> {
    text.split(|c: char| c.is_whitespace() || c == ',' || c == ';')
        .filter(|t| !t.is_empty())
        .map(|t| match t.parse::<f64>() {
            Ok(v) if v.is_finite() && v > 0.0 => Ok(v),
            _ => Err(format!("'{t}' is not a valid sample size")),
        })
        .collect()
}

impl Anova3State {
    /// Starts with the chosen columns as factor A, B, C and data, in this order.
    pub fn with_columns(chosen: &[usize], total: usize) -> Self {
        let last = total.saturating_sub(1);
        let columns = if chosen.len() >= 4 {
            [chosen[0], chosen[1], chosen[2], chosen[3]]
        } else {
            [0, 1.min(last), 2.min(last), 3.min(last)]
        };
        Anova3State { columns, ..Default::default() }
    }

    pub fn spec(&self) -> Result<ThreeWayAnovaSpec, String> {
        let power_sizes = if self.power_hypothetical { parse_sizes(&self.power_sizes)? } else { Vec::new() };
        if self.power_hypothetical && power_sizes.is_empty() {
            return Err("give at least one hypothetical sample size, or switch the hypothetical power off".to_string());
        }
        Ok(ThreeWayAnovaSpec {
            terms: ThreeWayTerms { ab: self.terms[0], ac: self.terms[1], bc: self.terms[2], abc: self.terms[3] },
            levene: self.levene,
            descriptive: ThreeWayEffect::ALL
                .into_iter()
                .zip(self.descriptive)
                .filter_map(|(e, on)| on.then_some(e))
                .collect(),
            descriptive_overall: self.overall,
            comparisons: METHODS.into_iter().zip(self.methods).filter_map(|(m, on)| on.then_some(m)).collect(),
            comparison_alpha: self.comparison_alpha,
            power_actual: self.power_actual,
            power_sizes,
            power_alpha: self.power_alpha,
            alpha: self.alpha,
        })
    }

    /// The four chosen columns: A, B, C and the data.
    pub fn picked(&self, sheet_columns: &[(ColumnId, String)]) -> Result<Vec<ColumnId>, String> {
        let ids: Vec<ColumnId> = self.columns.iter().filter_map(|&i| sheet_columns.get(i)).map(|(id, _)| *id).collect();
        if ids.len() != 4 {
            return Err("the sheet needs four columns: factor A, factor B, factor C and the data".to_string());
        }
        for (i, a) in ids.iter().enumerate() {
            if ids[..i].contains(a) {
                return Err("choose four different columns for factor A, B, C and the data".to_string());
            }
        }
        Ok(ids)
    }

    pub fn ui(&mut self, ui: &mut Ui, sheet_columns: &[(ColumnId, String)]) {
        ui.horizontal(|ui| {
            for (tab, label) in [
                (Tab::Input, "Input"),
                (Tab::Model, "Model"),
                (Tab::Descriptive, "Descriptive Statistics"),
                (Tab::Means, "Means Comparison"),
                (Tab::Power, "Power Analysis"),
            ] {
                ui.selectable_value(&mut self.tab, tab, label);
            }
        });
        ui.separator();
        match self.tab {
            Tab::Input => {
                ui.label("Indexed data: one row of the sheet is one observation.");
                for (i, label) in ["Factor A", "Factor B", "Factor C", "Data"].into_iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(label);
                        let current = sheet_columns.get(self.columns[i]).map_or("", |c| c.1.as_str());
                        egui::ComboBox::from_id_salt(("anova3_column", i)).selected_text(current).width(220.0).show_ui(
                            ui,
                            |ui| {
                                for (j, (_, text)) in sheet_columns.iter().enumerate() {
                                    ui.selectable_value(&mut self.columns[i], j, text);
                                }
                            },
                        );
                    });
                }
                ui.weak("The factor columns may hold text or numbers. The Raw data layout of Origin is not built.");
                ui.checkbox(&mut self.levene, "Homogeneity Tests (Levene's Test)");
                ui.horizontal(|ui| {
                    ui.label("Significance level");
                    ui.add(DragValue::new(&mut self.alpha).speed(0.001).range(0.0001..=0.5));
                });
            }
            Tab::Model => {
                ui.label("Data = A + B + C");
                for (on, label) in self.terms.iter_mut().zip(["A*B", "A*C", "B*C", "A*B*C"]) {
                    ui.checkbox(on, label);
                }
                ui.weak("Sums of squares are adjusted for all the other terms (type III), as in Origin.");
            }
            Tab::Descriptive => {
                for (on, e) in self.descriptive.iter_mut().zip(ThreeWayEffect::ALL) {
                    ui.checkbox(on, e.letters());
                }
                ui.checkbox(&mut self.overall, "Overall");
            }
            Tab::Means => {
                ui.horizontal(|ui| {
                    ui.label("Significance level");
                    ui.add(DragValue::new(&mut self.comparison_alpha).speed(0.001).range(0.0001..=0.5));
                });
                for (on, m) in self.methods.iter_mut().zip(METHODS) {
                    ui.checkbox(on, m.label());
                }
                ui.weak(
                    "Pairs of the levels of A, B and C and of the cells of A*B. Origin's means comparison of the \
                     other interactions was not measured; its Holm columns pair each p-value with another \
                     threshold, which is not reproduced (Sheaf lists the pairs by increasing p).",
                );
            }
            Tab::Power => {
                ui.checkbox(&mut self.power_actual, "Actual Power");
                ui.checkbox(&mut self.power_hypothetical, "Hypothetical Power");
                ui.add_enabled_ui(self.power_hypothetical, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Hypothetical Sample Size(s)");
                        ui.add(egui::TextEdit::singleline(&mut self.power_sizes).desired_width(140.0));
                    });
                });
                ui.horizontal(|ui| {
                    ui.label("Significance level");
                    ui.add(DragValue::new(&mut self.power_alpha).speed(0.001).range(0.0001..=0.5));
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn columns(n: usize) -> Vec<(ColumnId, String)> {
        let mut p = sheaf_core::Project::new();
        let book = p.add_workbook("t");
        (0..n)
            .map(|i| {
                let id = p
                    .add_column(book, 0, sheaf_core::Designation::Y, sheaf_core::ColumnData::numeric(vec![1.0]))
                    .unwrap();
                (id, format!("C{i}"))
            })
            .collect()
    }

    #[test]
    fn the_partial_correlation_window_needs_a_control_and_starts_on_origins_defaults() {
        let cols = columns(4);
        let mut s = PartCorrState::default();
        assert!(s.spec(&cols).is_err());
        s.controls = vec![2, 3];
        let spec = s.spec(&cols).unwrap();
        assert_eq!(spec.controls, vec![cols[2].0, cols[3].0]);
        assert!(!spec.listwise && !spec.one_tailed && !spec.show_significance && !spec.flag_significant);
        s.drop_variables(&cols, &[cols[3].0]);
        assert_eq!(s.controls, vec![2]);
    }

    #[test]
    fn the_three_way_window_builds_the_spec_from_its_tabs() {
        let mut s = Anova3State::default();
        let spec = s.spec().unwrap();
        assert_eq!(spec.comparisons, vec![MeansComparison::Tukey]);
        assert_eq!(spec.descriptive.len(), 7);
        assert!(spec.power_sizes.is_empty() && !spec.power_actual && spec.terms == ThreeWayTerms::default());
        s.terms[3] = false;
        s.methods = [true; 7];
        s.power_hypothetical = true;
        let spec = s.spec().unwrap();
        assert!(!spec.terms.abc && spec.comparisons.len() == 7);
        assert_eq!(spec.power_sizes, vec![50.0, 100.0, 200.0]);
        s.power_sizes = "50, x".to_string();
        assert!(s.spec().is_err());
        s.power_sizes = "  ".to_string();
        assert!(s.spec().is_err());
        let cols = columns(5);
        assert_eq!(Anova3State::with_columns(&[3, 1, 0, 2], 5).picked(&cols).unwrap().len(), 4);
        assert!(Anova3State::with_columns(&[], 2).picked(&columns(2)).is_err());
    }
}
