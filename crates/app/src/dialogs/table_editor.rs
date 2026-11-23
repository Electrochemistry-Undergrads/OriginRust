//! Graph > New Table (T136) and a double-click on a table: the cells in a grid of text boxes, with buttons
//! to add or remove the last row or column. OK makes the table (in the active layer) or changes it, one
//! undo step. Rules in `docs/spec/graphing.md`, "Tables".

use eframe::egui;
use sheaf_core::WindowId;

pub enum TableOutcome {
    Open,
    Closed,
    /// OK: `label` is `None` for a new table.
    Apply {
        graph: WindowId,
        layer: usize,
        label: Option<usize>,
        cells: Vec<Vec<String>>,
    },
}

pub struct TableEditor {
    pub graph: WindowId,
    pub layer: usize,
    pub label: Option<usize>,
    pub cells: Vec<Vec<String>>,
}

/// The rows and columns of a new table: Origin 2016's New Table defaults, Number of Rows 8 and Number of Columns
/// 5 (measured, T395).
pub const NEW_ROWS: usize = 8;
pub const NEW_COLS: usize = 5;
/// The most rows and columns Origin's New Table offers (its lists run 1 to 9).
pub const MAX_SIZE: usize = 9;

impl TableEditor {
    pub fn new_table(graph: WindowId, layer: usize) -> Self {
        TableEditor { graph, layer, label: None, cells: vec![vec![String::new(); NEW_COLS]; NEW_ROWS] }
    }

    pub fn edit(graph: WindowId, layer: usize, label: usize, mut cells: Vec<Vec<String>>) -> Self {
        let cols = cells.iter().map(Vec::len).max().unwrap_or(1).max(1);
        for row in &mut cells {
            row.resize(cols, String::new());
        }
        TableEditor { graph, layer, label: Some(label), cells }
    }

    fn cols(&self) -> usize {
        self.cells.first().map_or(0, Vec::len)
    }

    pub fn show(&mut self, ctx: &egui::Context) -> TableOutcome {
        let mut open = true;
        let mut outcome = TableOutcome::Open;
        let title = if self.label.is_some() { "Table" } else { "New Table" };
        egui::Window::new(title)
            .id(egui::Id::new("table_editor"))
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                egui::Grid::new("table_editor_grid").num_columns(self.cols()).show(ui, |ui| {
                    for (r, row) in self.cells.iter_mut().enumerate() {
                        for (c, cell) in row.iter_mut().enumerate() {
                            ui.add(egui::TextEdit::singleline(cell).desired_width(70.0).id(egui::Id::new((
                                "table_cell",
                                r,
                                c,
                            ))));
                        }
                        ui.end_row();
                    }
                });
                ui.horizontal(|ui| {
                    let cols = self.cols();
                    if ui.add_enabled(self.cells.len() < MAX_SIZE, egui::Button::new("Add Row")).clicked() {
                        self.cells.push(vec![String::new(); cols]);
                    }
                    if ui.add_enabled(self.cells.len() > 1, egui::Button::new("Remove Row")).clicked() {
                        self.cells.pop();
                    }
                    if ui.add_enabled(cols < MAX_SIZE, egui::Button::new("Add Column")).clicked() {
                        self.cells.iter_mut().for_each(|r| r.push(String::new()));
                    }
                    if ui.add_enabled(cols > 1, egui::Button::new("Remove Column")).clicked() {
                        self.cells.iter_mut().for_each(|r| {
                            r.pop();
                        });
                    }
                });
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("OK").clicked() {
                        outcome = TableOutcome::Apply {
                            graph: self.graph,
                            layer: self.layer,
                            label: self.label,
                            cells: self.cells.clone(),
                        };
                    }
                    if ui.button("Cancel").clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        outcome = TableOutcome::Closed;
                    }
                });
            });
        if !open { TableOutcome::Closed } else { outcome }
    }
}
