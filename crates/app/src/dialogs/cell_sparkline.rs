//! A data cell's Insert Sparklines... (T200): pick the column whose values the cell draws as a
//! sparkline. The window only asks; the app sets the cell. Rules: `docs/spec/workbook-model.md`,
//! "Sparklines in cells".

use eframe::egui;
use sheaf_core::ColumnId;

/// What the window asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellSparklineOutcome {
    Open,
    Closed,
    /// Draw this column's values in the cell.
    Pick(ColumnId),
}

/// The Insert Sparklines window for the data cell of `column` at `row`.
pub struct CellSparklinePicker {
    pub column: ColumnId,
    pub row: usize,
    /// The sheet's numeric columns: id and the name the list shows.
    sources: Vec<(ColumnId, String)>,
    selected: Option<usize>,
}

impl CellSparklinePicker {
    pub fn new(column: ColumnId, row: usize, sources: Vec<(ColumnId, String)>) -> Self {
        let selected = sources.iter().position(|(id, _)| *id == column).or((!sources.is_empty()).then_some(0));
        CellSparklinePicker { column, row, sources, selected }
    }

    pub fn show(&mut self, ctx: &egui::Context) -> CellSparklineOutcome {
        let mut outcome = CellSparklineOutcome::Open;
        let mut open = true;
        egui::Window::new("Insert Sparklines")
            .id(egui::Id::new("insert_cell_sparkline"))
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                if self.sources.is_empty() {
                    ui.weak("The sheet has no column of numbers.");
                }
                ui.label("Data column");
                egui::ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
                    for (i, (_, name)) in self.sources.iter().enumerate() {
                        if ui.selectable_label(self.selected == Some(i), name).clicked() {
                            self.selected = Some(i);
                        }
                    }
                });
                ui.weak("The cell draws the column's values as a small line, from the first value to the last.");
                ui.horizontal(|ui| {
                    let chosen = self.selected.and_then(|i| self.sources.get(i)).map(|(id, _)| *id);
                    if ui.add_enabled(chosen.is_some(), egui::Button::new("OK")).clicked()
                        && let Some(id) = chosen
                    {
                        outcome = CellSparklineOutcome::Pick(id);
                    }
                    if ui.button("Cancel").clicked() {
                        outcome = CellSparklineOutcome::Closed;
                    }
                });
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    outcome = CellSparklineOutcome::Closed;
                }
            });
        if !open { CellSparklineOutcome::Closed } else { outcome }
    }
}
