//! File > Batch Processing... (T165): run a saved workbook or analysis template (T160, T163) on a list
//! of data files. The window only asks; the app reads the files and makes the books
//! (`Project::new_workbook_from_template_with_data`) and, with a summary sheet chosen (T233), the
//! summary book (`Project::batch_summary`). Rules: `docs/spec/workbook-model.md` 13p.

use eframe::egui::{self, RichText};
use std::path::PathBuf;

/// What the window asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BatchOutcome {
    Open,
    Closed,
    /// Pick data files with the system dialog and add them to the list.
    AddFiles,
    /// Make one book per file from the template of this name.
    Run {
        template: String,
        files: Vec<PathBuf>,
        summary: Option<BatchSummary>,
    },
}

/// The summary of a batch run (T233): one sheet of every book made, stacked in a new book.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchSummary {
    /// The template sheet whose rows are collected.
    pub sheet: String,
    /// The name of the summary's first column, which holds each row's file name.
    pub identifier: String,
    /// Delete the books made from the files once the summary holds their rows.
    pub delete_books: bool,
}

/// The Batch Processing window.
pub struct BatchDialog {
    /// The saved workbook templates with the names of their sheets.
    templates: Vec<(String, Vec<String>)>,
    template: usize,
    /// The chosen summary sheet of the template (T233), or none.
    summary_sheet: Option<String>,
    identifier: String,
    delete_books: bool,
    files: Vec<PathBuf>,
    selected: Option<usize>,
    error: Option<String>,
}

impl BatchDialog {
    pub fn new(templates: Vec<(String, Vec<String>)>) -> Self {
        BatchDialog {
            templates,
            template: 0,
            summary_sheet: None,
            identifier: "Dataset".into(),
            delete_books: false,
            files: Vec::new(),
            selected: None,
            error: None,
        }
    }

    /// Chooses the summary sheet (T233); `None` for no summary. Tests pick it this way.
    #[cfg(test)]
    pub fn set_summary(&mut self, sheet: Option<&str>, identifier: &str, delete_books: bool) {
        self.summary_sheet = sheet.map(String::from);
        self.identifier = identifier.to_string();
        self.delete_books = delete_books;
    }

    /// The summary the window asks for: the chosen sheet when the current template has it.
    fn summary(&self) -> Option<BatchSummary> {
        let sheets = &self.templates.get(self.template)?.1;
        let sheet = self.summary_sheet.as_ref().filter(|s| sheets.contains(s))?;
        let identifier = self.identifier.trim();
        Some(BatchSummary {
            sheet: sheet.clone(),
            identifier: if identifier.is_empty() { "Dataset".into() } else { identifier.to_string() },
            delete_books: self.delete_books,
        })
    }

    /// Adds files to the list; a file already listed is not added again.
    pub fn add_files(&mut self, files: impl IntoIterator<Item = PathBuf>) {
        for f in files {
            if !self.files.contains(&f) {
                self.files.push(f);
            }
        }
        self.error = None;
    }

    pub fn set_error(&mut self, error: String) {
        self.error = Some(error);
    }

    pub fn show(&mut self, ctx: &egui::Context) -> BatchOutcome {
        let mut outcome = BatchOutcome::Open;
        let mut open = true;
        egui::Window::new("Batch Processing")
            .id(egui::Id::new("batch_processing"))
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                if self.templates.is_empty() {
                    ui.weak(
                        "No workbook template is saved. On a workbook: File > Save Template As... or \
                         Save Workbook As Analysis Template...",
                    );
                } else {
                    ui.horizontal(|ui| {
                        ui.label("Template");
                        let current = self.templates.get(self.template).map(|t| t.0.clone()).unwrap_or_default();
                        egui::ComboBox::from_id_salt("batch_template").selected_text(current).show_ui(ui, |ui| {
                            for (i, (name, _)) in self.templates.iter().enumerate() {
                                ui.selectable_value(&mut self.template, i, name);
                            }
                        });
                    });
                    // T233: the summary report, one sheet of every book stacked in a new book.
                    let sheets = self.templates.get(self.template).map(|t| t.1.clone()).unwrap_or_default();
                    if self.summary_sheet.as_ref().is_some_and(|s| !sheets.contains(s)) {
                        self.summary_sheet = None;
                    }
                    ui.horizontal(|ui| {
                        ui.label("Summary Sheet");
                        let shown = self.summary_sheet.clone().unwrap_or_else(|| "(none)".into());
                        egui::ComboBox::from_id_salt("batch_summary_sheet").selected_text(shown).show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.summary_sheet, None, "(none)");
                            for s in &sheets {
                                ui.selectable_value(&mut self.summary_sheet, Some(s.clone()), s);
                            }
                        })
                        .response
                        .on_hover_text("The template sheet whose rows from every file are collected in a new summary book");
                    });
                    ui.add_enabled_ui(self.summary_sheet.is_some(), |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Dataset Identifier");
                            ui.add(egui::TextEdit::singleline(&mut self.identifier).desired_width(120.0))
                                .on_hover_text("The name of the summary's first column, which holds each row's file name");
                        });
                        ui.checkbox(&mut self.delete_books, "Delete Intermediate Books");
                    });
                }
                ui.label("Data files");
                egui::ScrollArea::vertical().max_height(180.0).show(ui, |ui| {
                    if self.files.is_empty() {
                        ui.weak("(none)");
                    }
                    for (i, f) in self.files.iter().enumerate() {
                        let name = f.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                        if ui.selectable_label(self.selected == Some(i), name).on_hover_text(f.display().to_string()).clicked()
                        {
                            self.selected = Some(i);
                        }
                    }
                });
                ui.horizontal(|ui| {
                    if ui.button("Add Files...").clicked() {
                        outcome = BatchOutcome::AddFiles;
                    }
                    if ui.add_enabled(self.selected.is_some(), egui::Button::new("Remove")).clicked()
                        && let Some(i) = self.selected.take()
                        && i < self.files.len()
                    {
                        self.files.remove(i);
                    }
                    if ui.add_enabled(!self.files.is_empty(), egui::Button::new("Clear")).clicked() {
                        self.files.clear();
                        self.selected = None;
                    }
                });
                ui.weak("Each file becomes a new workbook made from the template; its columns fill the template's data columns and the template's analysis runs again.");
                if let Some(e) = &self.error {
                    ui.label(RichText::new(e).color(egui::Color32::from_rgb(190, 30, 30)));
                }
                ui.horizontal(|ui| {
                    let ready = !self.templates.is_empty() && !self.files.is_empty();
                    if ui.add_enabled(ready, egui::Button::new("OK")).clicked() {
                        outcome = BatchOutcome::Run {
                            template: self.templates[self.template].0.clone(),
                            files: self.files.clone(),
                            summary: self.summary(),
                        };
                    }
                    if ui.button("Cancel").clicked() {
                        outcome = BatchOutcome::Closed;
                    }
                });
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    outcome = BatchOutcome::Closed;
                }
            });
        if !open { BatchOutcome::Closed } else { outcome }
    }
}
