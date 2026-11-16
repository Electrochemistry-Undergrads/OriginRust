//! The sheet tab's Navigate... (T93): Origin 2016's "Navigate Worksheets - Book1" window lists the
//! workbook's sheets in a table (Index, Creation Index, Name, Columns, Size(KB), Comments, Type, Source),
//! the active one selected; OK goes to the selected sheet, Close closes. Sheaf shows the columns it can
//! fill: Index, Name, Long Name, Columns and Rows. Rules in `docs/spec/workbook-model.md` 13h.

use eframe::egui::{self, RichText};
use sheaf_core::{Project, WindowId};

pub enum NavigateOutcome {
    Open,
    Closed,
    Go { book: WindowId, sheet: usize },
}

pub struct SheetNavigateDialog {
    pub book: WindowId,
    pub selected: usize,
}

impl SheetNavigateDialog {
    pub fn new(book: WindowId, active: usize) -> Self {
        SheetNavigateDialog { book, selected: active }
    }

    pub fn show(&mut self, ctx: &egui::Context, project: &Project) -> NavigateOutcome {
        let Some(book) = project.workbook(self.book) else { return NavigateOutcome::Closed };
        let mut outcome = NavigateOutcome::Open;
        let mut open = true;
        egui::Window::new(format!("Navigate Worksheets - {}", book.name))
            .id(egui::Id::new("sheet_navigate"))
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                egui::Grid::new("sheet_navigate_grid").striped(true).num_columns(5).show(ui, |ui| {
                    for head in ["Index", "Name", "Long Name", "Columns", "Rows"] {
                        ui.label(RichText::new(head).strong());
                    }
                    ui.end_row();
                    for (i, sheet) in book.sheets.iter().enumerate() {
                        let picked = i == self.selected;
                        let index = ui.selectable_label(picked, (i + 1).to_string());
                        let name = ui.selectable_label(picked, &sheet.name);
                        ui.label(&sheet.long_name);
                        ui.label(sheet.columns.len().to_string());
                        ui.label(sheet.row_count().to_string());
                        ui.end_row();
                        if index.clicked() || name.clicked() {
                            self.selected = i;
                        }
                        if index.double_clicked() || name.double_clicked() {
                            outcome = NavigateOutcome::Go { book: self.book, sheet: i };
                        }
                    }
                });
                ui.horizontal(|ui| {
                    if ui.button("OK").clicked() {
                        outcome = NavigateOutcome::Go { book: self.book, sheet: self.selected };
                    }
                    if ui.button("Close").clicked() {
                        outcome = NavigateOutcome::Closed;
                    }
                });
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    outcome = NavigateOutcome::Closed;
                }
            });
        if !open { NavigateOutcome::Closed } else { outcome }
    }
}
