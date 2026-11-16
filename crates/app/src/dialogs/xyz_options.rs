//! The input of Analysis > Mathematics > XYZ Surface Area (T91): the X, Y and Z columns of the sheet. Origin's
//! `xyzarea` takes one range of three columns (`iz`); its window was not measured (`docs/spec/analysis-tools.md`,
//! section 18), so the three boxes are Sheaf's layout.

use eframe::egui::{self, Ui};
use sheaf_core::ColumnId;

/// The state of the XYZ Surface Area window.
#[derive(Debug, Clone, PartialEq)]
pub struct XyzState {
    /// Indexes into the sheet's columns: X, Y and Z.
    pub columns: [usize; 3],
}

impl XyzState {
    /// Starts with the chosen columns when there are three of them, else with the sheet's own X, Y and Z
    /// (`sheaf_core::analysis::xyz_default_columns`), else with the first three columns.
    pub fn new(chosen: &[usize], default: Option<[usize; 3]>, total: usize) -> Self {
        let last = total.saturating_sub(1);
        let columns = if chosen.len() >= 3 {
            [chosen[0], chosen[1], chosen[2]]
        } else {
            default.unwrap_or([0, 1.min(last), 2.min(last)])
        };
        XyzState { columns }
    }

    /// The three chosen columns, different from each other.
    pub fn picked(&self, sheet_columns: &[(ColumnId, String)]) -> Result<Vec<ColumnId>, String> {
        let ids: Vec<ColumnId> = self.columns.iter().filter_map(|&i| sheet_columns.get(i)).map(|(id, _)| *id).collect();
        if ids.len() != 3 {
            return Err("the sheet needs three columns: X, Y and Z".to_string());
        }
        if ids[0] == ids[1] || ids[0] == ids[2] || ids[1] == ids[2] {
            return Err("choose three different columns for X, Y and Z".to_string());
        }
        Ok(ids)
    }

    pub fn ui(&mut self, ui: &mut Ui, sheet_columns: &[(ColumnId, String)]) {
        for (i, label) in ["X", "Y", "Z"].into_iter().enumerate() {
            ui.horizontal(|ui| {
                ui.label(label);
                let current = sheet_columns.get(self.columns[i]).map_or("", |c| c.1.as_str());
                egui::ComboBox::from_id_salt(("xyz_column", i)).selected_text(current).width(220.0).show_ui(ui, |ui| {
                    for (j, (_, text)) in sheet_columns.iter().enumerate() {
                        ui.selectable_value(&mut self.columns[i], j, text);
                    }
                });
            });
        }
        ui.weak(
            "The area of the surface through the points: the Delaunay triangulation of X and Y, lifted to Z. Rows \
             with the same X and Y are merged with the mean of their Z.",
        );
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
    fn the_boxes_start_on_the_selection_then_the_defaults_and_refuse_repeats() {
        assert_eq!(XyzState::new(&[3, 1, 0], Some([0, 1, 2]), 5).columns, [3, 1, 0]);
        assert_eq!(XyzState::new(&[1], Some([2, 3, 4]), 5).columns, [2, 3, 4]);
        assert_eq!(XyzState::new(&[], None, 2).columns, [0, 1, 1]);
        let cols = columns(4);
        let mut s = XyzState::new(&[], Some([0, 1, 2]), 4);
        assert_eq!(s.picked(&cols).unwrap(), vec![cols[0].0, cols[1].0, cols[2].0]);
        s.columns = [0, 0, 2];
        assert!(s.picked(&cols).is_err());
        s.columns = [0, 1, 9];
        assert!(s.picked(&cols).is_err());
    }
}
