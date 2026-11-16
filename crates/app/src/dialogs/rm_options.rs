//! The options of Statistics > ANOVA > One-Way Repeated Measures ANOVA and Two-Way Repeated Measures ANOVA (T81),
//! laid out as Origin's dialogs (`docs/spec/statistics.md` 26): Input, [Descriptive Statistics (two-way)] and Means
//! Comparison tabs. The Raw data layout, the Plots and Output tabs are not built (the results go to a new sheet),
//! and the means comparisons of the two-way design were not measured, so that tab only says so.

use eframe::egui::{self, DragValue, Ui};
use sheaf_core::ColumnId;
use sheaf_core::analysis::{MeansComparison, RmAnovaSpec, RmTerm};

/// The seven means comparison methods of the one-way dialog without Dunnett, in Origin's order.
const METHODS: [MeansComparison; 7] = [
    MeansComparison::Tukey,
    MeansComparison::Bonferroni,
    MeansComparison::Sidak,
    MeansComparison::FisherLsd,
    MeansComparison::Scheffe,
    MeansComparison::HolmBonferroni,
    MeansComparison::HolmSidak,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
    Input,
    Descriptive,
    Means,
}

/// The state of a repeated measures window.
#[derive(Debug, Clone, PartialEq)]
pub struct RmState {
    tab: Tab,
    pub two_way: bool,
    /// Indexes into the sheet's columns: factor (A), factor B (two-way only), data, subject.
    pub columns: [usize; 4],
    pub alpha: f64,
    /// The interaction of the two-way design (Origin's "Interactions" box, off by default).
    pub interaction: bool,
    /// Descriptive statistics of the levels of A, of B and of the A*B cells.
    pub descriptive: [bool; 3],
    pub methods: [bool; 7],
    pub comparison_alpha: f64,
}

impl RmState {
    /// Starts with the chosen columns in the order of the dialog (factor(s), data, subject).
    pub fn new(two_way: bool, chosen: &[usize], total: usize) -> Self {
        let need = if two_way { 4 } else { 3 };
        let last = total.saturating_sub(1);
        let mut columns = [0, 1.min(last), 2.min(last), 3.min(last)];
        if chosen.len() >= need {
            columns[..need].copy_from_slice(&chosen[..need]);
        }
        RmState {
            tab: Tab::Input,
            two_way,
            columns,
            alpha: 0.05,
            interaction: false,
            descriptive: [true, two_way, false],
            methods: [true, false, false, false, false, false, false],
            comparison_alpha: 0.05,
        }
    }

    fn factors(&self) -> usize {
        if self.two_way { 2 } else { 1 }
    }

    pub fn spec(&self) -> Result<RmAnovaSpec, String> {
        let mut spec = RmAnovaSpec::new(self.two_way);
        spec.interaction = self.two_way && self.interaction;
        spec.descriptive = [RmTerm::A, RmTerm::B, RmTerm::Ab]
            .into_iter()
            .zip(self.descriptive)
            .filter(|(t, on)| *on && (self.two_way || *t == RmTerm::A) && (*t != RmTerm::Ab || spec.interaction))
            .map(|(t, _)| t)
            .collect();
        spec.comparisons = if self.two_way {
            Vec::new()
        } else {
            METHODS.into_iter().zip(self.methods).filter_map(|(m, on)| on.then_some(m)).collect()
        };
        spec.comparison_alpha = self.comparison_alpha;
        spec.alpha = self.alpha;
        Ok(spec)
    }

    /// The chosen columns: the factor(s), the data and the subject.
    pub fn picked(&self, sheet_columns: &[(ColumnId, String)]) -> Result<Vec<ColumnId>, String> {
        let need = self.factors() + 2;
        let ids: Vec<ColumnId> =
            self.columns[..need].iter().filter_map(|&i| sheet_columns.get(i)).map(|(id, _)| *id).collect();
        if ids.len() != need {
            return Err(format!("the sheet needs {need} columns for this ANOVA"));
        }
        for (i, a) in ids.iter().enumerate() {
            if ids[..i].contains(a) {
                return Err("choose different columns for the factors, the data and the subject".to_string());
            }
        }
        Ok(ids)
    }

    pub fn ui(&mut self, ui: &mut Ui, sheet_columns: &[(ColumnId, String)]) {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.tab, Tab::Input, "Input");
            if self.two_way {
                ui.selectable_value(&mut self.tab, Tab::Descriptive, "Descriptive Statistics");
            }
            ui.selectable_value(&mut self.tab, Tab::Means, "Means Comparison");
        });
        ui.separator();
        match self.tab {
            Tab::Input => {
                ui.label("Indexed data: one row of the sheet is one observation of one subject.");
                let labels: &[&str] = if self.two_way {
                    &["Factor A", "Factor B", "Data", "Subject"]
                } else {
                    &["Factor", "Data", "Subject"]
                };
                for (i, label) in labels.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(*label);
                        let current = sheet_columns.get(self.columns[i]).map_or("", |c| c.1.as_str());
                        egui::ComboBox::from_id_salt(("rm_column", i)).selected_text(current).width(220.0).show_ui(
                            ui,
                            |ui| {
                                for (j, (_, text)) in sheet_columns.iter().enumerate() {
                                    ui.selectable_value(&mut self.columns[i], j, text);
                                }
                            },
                        );
                    });
                }
                ui.weak(
                    "The factor and subject columns may hold text or numbers. Every subject needs one number in \
                     every cell. The Raw data layout of Origin is not built.",
                );
                if self.two_way {
                    ui.checkbox(&mut self.interaction, "Interactions");
                } else {
                    ui.checkbox(&mut self.descriptive[0], "Descriptive Statistics");
                }
                ui.horizontal(|ui| {
                    ui.label("Significance level");
                    ui.add(DragValue::new(&mut self.alpha).speed(0.001).range(0.0001..=0.5));
                });
            }
            Tab::Descriptive => {
                ui.checkbox(&mut self.descriptive[0], "FactorA");
                ui.checkbox(&mut self.descriptive[1], "FactorB");
                ui.add_enabled(self.interaction, egui::Checkbox::new(&mut self.descriptive[2], "Interaction"));
                if !self.interaction {
                    ui.weak("The interaction cells are listed when Interactions is ticked on the Input tab.");
                }
            }
            Tab::Means => {
                if self.two_way {
                    ui.weak(
                        "Origin's two-way dialog has means comparisons; they were not measured against Origin and \
                         are not built.",
                    );
                } else {
                    ui.horizontal(|ui| {
                        ui.label("Significance level");
                        ui.add(DragValue::new(&mut self.comparison_alpha).speed(0.001).range(0.0001..=0.5));
                    });
                    for (on, m) in self.methods.iter_mut().zip(METHODS) {
                        ui.checkbox(on, m.label());
                    }
                    ui.weak(
                        "Pairs of the level means with the error mean square of the table. Origin also offers \
                         Dunnett's test here; Sheaf's Dunnett test is not part of this tool.",
                    );
                }
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
    fn the_windows_build_the_spec_from_their_tabs() {
        let cols = columns(4);
        let one = RmState::new(false, &[], 4);
        let spec = one.spec().unwrap();
        assert!(!spec.two_way && !spec.interaction);
        assert_eq!(spec.comparisons, vec![MeansComparison::Tukey]);
        assert_eq!(spec.descriptive, vec![RmTerm::A]);
        assert_eq!(one.picked(&cols).unwrap().len(), 3);

        let mut two = RmState::new(true, &[3, 2, 1, 0], 4);
        assert_eq!(two.columns, [3, 2, 1, 0]);
        let spec = two.spec().unwrap();
        assert!(spec.two_way && !spec.interaction && spec.comparisons.is_empty());
        assert_eq!(spec.descriptive, vec![RmTerm::A, RmTerm::B]);
        two.interaction = true;
        two.descriptive[2] = true;
        let spec = two.spec().unwrap();
        assert!(spec.interaction && spec.descriptive.contains(&RmTerm::Ab));
        assert_eq!(two.picked(&cols).unwrap().len(), 4);
        two.columns = [0, 0, 1, 2];
        assert!(two.picked(&cols).is_err());
        assert!(RmState::new(true, &[], 2).picked(&columns(2)).is_err());
    }
}
