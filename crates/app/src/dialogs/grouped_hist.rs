//! Plot > Statistics > Grouped Histogram... (T271): the data column, the group column and Origin's
//! Multiple Data choice (Overlay on Same Graph or Separate Layers on Same Graph). OK makes the
//! GroupedHist workbook and graph (`Project::plot_grouped_histogram`); rules in
//! `docs/spec/graphing.md`, "Grouped histogram".

use eframe::egui;
use sheaf_core::{ColumnId, Project, WindowId};

pub enum GroupedHistOutcome {
    Open,
    Closed,
    Make { data: ColumnId, group: ColumnId, separate: bool },
}

pub struct GroupedHistDialog {
    /// The sheet's columns: id and name.
    pub columns: Vec<(ColumnId, String)>,
    pub data: usize,
    pub group: usize,
    pub separate: bool,
}

impl GroupedHistDialog {
    /// The window for the active sheet of `book`: the data is the first selected numeric column (else the
    /// first numeric Y column), the group the first other selected column (else the first text column, else
    /// the first X column). `None` when the sheet has fewer than two columns or no numbers.
    pub fn new(project: &Project, book: WindowId, selected: &[ColumnId]) -> Option<Self> {
        let b = project.workbook(book)?;
        let ws = b.sheets.get(b.active_sheet)?;
        if ws.columns.len() < 2 {
            return None;
        }
        let columns: Vec<(ColumnId, String)> =
            ws.columns.iter().map(|c| (c.id, c.display_name().to_string())).collect();
        let at = |id: ColumnId| ws.columns.iter().position(|c| c.id == id);
        let numeric = |k: &usize| ws.columns[*k].data.is_numeric();
        let data = selected
            .iter()
            .filter_map(|c| at(*c))
            .find(numeric)
            .or_else(|| {
                (0..ws.columns.len()).find(|k| numeric(k) && ws.columns[*k].designation == sheaf_core::Designation::Y)
            })
            .or_else(|| (0..ws.columns.len()).find(numeric))?;
        let group = selected
            .iter()
            .filter_map(|c| at(*c))
            .find(|k| *k != data)
            .or_else(|| (0..ws.columns.len()).find(|k| *k != data && !numeric(k)))
            .or_else(|| {
                (0..ws.columns.len()).find(|k| *k != data && ws.columns[*k].designation == sheaf_core::Designation::X)
            })
            .or_else(|| (0..ws.columns.len()).find(|k| *k != data))?;
        Some(GroupedHistDialog { columns, data, group, separate: false })
    }

    pub fn show(&mut self, ctx: &egui::Context) -> GroupedHistOutcome {
        let mut open = true;
        let mut outcome = GroupedHistOutcome::Open;
        egui::Window::new("Grouped Histogram")
            .id(egui::Id::new("grouped_hist"))
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                egui::Grid::new("grouped_hist_grid").num_columns(2).show(ui, |ui| {
                    for (label, pick) in [("Data", &mut self.data), ("Group", &mut self.group)] {
                        ui.label(label);
                        let shown = self.columns.get(*pick).map(|c| c.1.clone()).unwrap_or_default();
                        egui::ComboBox::from_id_salt(("grouped_hist", label)).selected_text(shown).show_ui(ui, |ui| {
                            for (k, (_, name)) in self.columns.iter().enumerate() {
                                ui.selectable_value(pick, k, name);
                            }
                        });
                        ui.end_row();
                    }
                    ui.label("Multiple Data");
                    ui.vertical(|ui| {
                        ui.radio_value(&mut self.separate, false, "Overlay on Same Graph");
                        ui.radio_value(&mut self.separate, true, "Separate Layers on Same Graph");
                    });
                    ui.end_row();
                });
                ui.weak("Origin 2026's plot; not measured (Origin 2016 has none). The bins are shared.");
                ui.horizontal(|ui| {
                    let ok = self.data != self.group;
                    if ui.add_enabled(ok, egui::Button::new("OK")).clicked() {
                        outcome = GroupedHistOutcome::Make {
                            data: self.columns[self.data].0,
                            group: self.columns[self.group].0,
                            separate: self.separate,
                        };
                    }
                    if ui.button("Cancel").clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        outcome = GroupedHistOutcome::Closed;
                    }
                });
            });
        if !open { GroupedHistOutcome::Closed } else { outcome }
    }
}
