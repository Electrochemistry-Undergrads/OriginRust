//! The options of Statistics > Descriptive Statistics > Partial Correlation Coefficient and Statistics > ANOVA >
//! Three-Way ANOVA (T81), laid out as Origin's dialogs (`docs/spec/statistics.md` 24 and 25). The three-way
//! dialog has Origin's tabs Input, Model, Descriptive Statistics, Means Comparison and Power Analysis; its Mean
//! Plot tab, the Raw data layout and the Output tab are not built (the results go to a new sheet).

use eframe::egui::{self, Ui};
use sheaf_core::ColumnId;
use sheaf_core::analysis::PartialCorrSpec;

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
}
