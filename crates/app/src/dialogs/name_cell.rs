//! A data cell's Define Name... (T396): names the cell so that formulas of its workbook can read it, or the
//! selected cells of its column (a named range, T400). The window only asks; the app defines the name. Rules:
//! `docs/spec/expression-language.md`, "Named cells".

use eframe::egui;
use sheaf_core::ColumnId;

/// What the window asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NameCellOutcome {
    Open,
    Closed,
    Define(String),
}

/// The Define Name window of one cell.
pub struct NameCellDialog {
    pub column: ColumnId,
    pub row: usize,
    /// The last row of a named range (T400); `row` for one cell.
    pub last: usize,
    /// The last column of a named block of several columns (T432); `None` for one column.
    pub last_column: Option<ColumnId>,
    pub name: String,
    /// Why the last OK did not work.
    pub problem: Option<String>,
}

impl NameCellDialog {
    pub fn new(column: ColumnId, row: usize, last: usize, current: Option<&str>) -> Self {
        NameCellDialog {
            column,
            row,
            last: last.max(row),
            last_column: None,
            name: current.unwrap_or_default().to_string(),
            problem: None,
        }
    }

    pub fn show(&mut self, ctx: &egui::Context) -> NameCellOutcome {
        let mut outcome = NameCellOutcome::Open;
        let mut open = true;
        egui::Window::new("Define Name")
            .id(egui::Id::new("define_cell_name"))
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let label = ui.label("Name");
                    ui.add(egui::TextEdit::singleline(&mut self.name).desired_width(160.0)).labelled_by(label.id);
                });
                if self.last_column.is_some() {
                    ui.weak(format!(
                        "Rows {} to {} of the selected columns. Formulas of this workbook use the name as all these \
                         cells, in statistics such as mean(Name) or total(Name).",
                        self.row + 1,
                        self.last + 1
                    ));
                } else if self.last > self.row {
                    ui.weak(format!(
                        "Rows {} to {} of this column. Formulas of this workbook use the name as these cells, \
                         in functions such as mean(Name) or total(Name).",
                        self.row + 1,
                        self.last + 1
                    ));
                } else {
                    ui.weak(format!(
                        "Row {} of this column. Formulas of this workbook read the name as the cell's value.",
                        self.row + 1
                    ));
                }
                if let Some(p) = &self.problem {
                    ui.colored_label(ui.visuals().error_fg_color, p);
                }
                ui.horizontal(|ui| {
                    if ui.button("OK").clicked() {
                        outcome = NameCellOutcome::Define(self.name.trim().to_string());
                    }
                    if ui.button("Cancel").clicked() {
                        outcome = NameCellOutcome::Closed;
                    }
                });
            });
        if !open {
            outcome = NameCellOutcome::Closed;
        }
        outcome
    }
}

/// What the Name Manager asks for (T434).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NameManagerOutcome {
    Open,
    Closed,
    Delete(String),
}

/// Worksheet > Name Manager... (T434): the names of one workbook, what they stand for, and Delete.
pub struct NameManager {
    pub book: sheaf_core::WindowId,
}

impl NameManager {
    /// `names` are (name, what it stands for) in the order they were defined.
    pub fn show(&mut self, ctx: &egui::Context, title: &str, names: &[(String, String)]) -> NameManagerOutcome {
        let mut outcome = NameManagerOutcome::Open;
        let mut open = true;
        egui::Window::new(format!("Name Manager - {title}"))
            .id(egui::Id::new("name_manager"))
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                if names.is_empty() {
                    ui.label("This workbook has no names. Right-click a data cell (or select cells) and choose Define Name...");
                } else {
                    egui::Grid::new("name_manager_grid").striped(true).num_columns(3).show(ui, |ui| {
                        ui.strong("Name");
                        ui.strong("Refers To");
                        ui.label("");
                        ui.end_row();
                        for (name, refers) in names {
                            ui.label(name);
                            ui.label(refers);
                            if ui.button("Delete").on_hover_text(format!("Delete the name {name}")).clicked() {
                                outcome = NameManagerOutcome::Delete(name.clone());
                            }
                            ui.end_row();
                        }
                    });
                }
                if ui.button("Close").clicked() {
                    outcome = NameManagerOutcome::Closed;
                }
            });
        if !open {
            outcome = NameManagerOutcome::Closed;
        }
        outcome
    }
}
