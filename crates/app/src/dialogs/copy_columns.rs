//! The column header's Copy Columns to... (T89): the destination and the check boxes of Origin's
//! `colcopy`. The copy is `Project::copy_columns_to`; rules in `docs/spec/workbook-model.md`, "Copy
//! Columns to".

use eframe::egui::{self, RichText};
use sheaf_core::copy_columns::{CopyDestination, CopyParts};
use sheaf_core::{ColumnId, Project, WindowId};

pub enum CopyColumnsOutcome {
    Open,
    Closed,
    Copy { columns: Vec<ColumnId>, dest: CopyDestination, parts: CopyParts },
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Where {
    NewBook,
    NewSheet,
    Sheet,
}

pub struct CopyColumnsDialog {
    columns: Vec<ColumnId>,
    to: Where,
    /// The workbook and sheet chosen for a new sheet or an existing sheet.
    book: Option<WindowId>,
    sheet: usize,
    /// The first column overwritten; `None` adds the copies after the last column.
    at: Option<usize>,
    parts: CopyParts,
    problem: Option<String>,
}

impl CopyColumnsDialog {
    /// Origin's defaults: a new book, everything copied.
    pub fn new(columns: Vec<ColumnId>, project: &Project) -> Self {
        let book = columns.first().and_then(|c| project.locate(*c)).map(|(b, _, _)| b.id);
        CopyColumnsDialog {
            columns,
            to: Where::NewBook,
            book,
            sheet: 0,
            at: None,
            parts: CopyParts::default(),
            problem: None,
        }
    }

    pub fn set_problem(&mut self, p: String) {
        self.problem = Some(p);
    }

    fn destination(&self) -> Result<CopyDestination, String> {
        let book = || self.book.ok_or_else(|| "choose a workbook".to_string());
        Ok(match self.to {
            Where::NewBook => CopyDestination::NewBook,
            Where::NewSheet => CopyDestination::NewSheet(book()?),
            Where::Sheet => CopyDestination::Sheet { book: book()?, sheet: self.sheet, at: self.at },
        })
    }

    pub fn show(&mut self, ctx: &egui::Context, project: &Project) -> CopyColumnsOutcome {
        let mut outcome = CopyColumnsOutcome::Open;
        let mut open = true;
        let books: Vec<(WindowId, String, Vec<(String, Vec<String>)>)> = project
            .workbooks()
            .map(|b| {
                let sheets = b
                    .sheets
                    .iter()
                    .map(|s| (s.name.clone(), s.columns.iter().map(|c| c.short_name.clone()).collect()))
                    .collect();
                (b.id, b.name.clone(), sheets)
            })
            .collect();
        egui::Window::new("Copy Columns to")
            .id(egui::Id::new("copy_columns_to"))
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label(format!("{} column(s)", self.columns.len()));
                ui.radio_value(&mut self.to, Where::NewBook, "New book");
                ui.radio_value(&mut self.to, Where::NewSheet, "New sheet in a workbook");
                ui.radio_value(&mut self.to, Where::Sheet, "Columns of a sheet");
                if self.to != Where::NewBook {
                    egui::Grid::new("copy_columns_dest").num_columns(2).show(ui, |ui| {
                        ui.label("Workbook");
                        let name = books.iter().find(|b| Some(b.0) == self.book).map_or("", |b| b.1.as_str());
                        egui::ComboBox::from_id_salt("copy_cols_book").selected_text(name).show_ui(ui, |ui| {
                            for (id, n, _) in &books {
                                if ui.selectable_label(Some(*id) == self.book, n).clicked() {
                                    self.book = Some(*id);
                                    self.sheet = 0;
                                    self.at = None;
                                }
                            }
                        });
                        ui.end_row();
                        if self.to == Where::Sheet {
                            let sheets =
                                books.iter().find(|b| Some(b.0) == self.book).map(|b| b.2.clone()).unwrap_or_default();
                            ui.label("Sheet");
                            let sname = sheets.get(self.sheet).map_or("", |s| s.0.as_str());
                            egui::ComboBox::from_id_salt("copy_cols_sheet").selected_text(sname).show_ui(ui, |ui| {
                                for (k, (n, _)) in sheets.iter().enumerate() {
                                    if ui.selectable_label(k == self.sheet, n).clicked() {
                                        self.sheet = k;
                                        self.at = None;
                                    }
                                }
                            });
                            ui.end_row();
                            let cols = sheets.get(self.sheet).map(|s| s.1.clone()).unwrap_or_default();
                            ui.label("From column");
                            let cname =
                                self.at.and_then(|a| cols.get(a)).map_or("<new> (after the last)", |s| s.as_str());
                            egui::ComboBox::from_id_salt("copy_cols_at").selected_text(cname).show_ui(ui, |ui| {
                                if ui.selectable_label(self.at.is_none(), "<new> (after the last)").clicked() {
                                    self.at = None;
                                }
                                for (k, n) in cols.iter().enumerate() {
                                    if ui.selectable_label(self.at == Some(k), n).clicked() {
                                        self.at = Some(k);
                                    }
                                }
                            });
                            ui.end_row();
                        }
                    });
                }
                ui.separator();
                ui.checkbox(&mut self.parts.data, "Copy Data");
                ui.add_enabled(
                    self.parts.data,
                    egui::Checkbox::new(&mut self.parts.ignore_hidden_rows, "Ignore Hidden Rows"),
                );
                ui.checkbox(&mut self.parts.format, "Copy Format");
                ui.checkbox(&mut self.parts.long_name, "Long Name");
                ui.checkbox(&mut self.parts.units, "Units");
                ui.checkbox(&mut self.parts.comments, "Comments");
                ui.checkbox(&mut self.parts.params, "User-Defined Parameters");
                if let Some(p) = &self.problem {
                    ui.label(RichText::new(p).color(egui::Color32::from_rgb(190, 30, 30)));
                }
                ui.horizontal(|ui| {
                    if ui.button("OK").clicked() {
                        match self.destination() {
                            Ok(dest) => {
                                outcome =
                                    CopyColumnsOutcome::Copy { columns: self.columns.clone(), dest, parts: self.parts }
                            }
                            Err(e) => self.problem = Some(e),
                        }
                    }
                    if ui.button("Cancel").clicked() {
                        outcome = CopyColumnsOutcome::Closed;
                    }
                });
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    outcome = CopyColumnsOutcome::Closed;
                }
            });
        if !open { CopyColumnsOutcome::Closed } else { outcome }
    }
}
