//! File > Batch Processing... (T165): run a saved workbook or analysis template (T160, T163) on a list
//! of data files. The window only asks; the app reads the files and makes the books
//! (`Project::new_workbook_from_template_with_data`). Rules: `docs/spec/workbook-model.md` 13p.

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
    },
}

/// The Batch Processing window.
pub struct BatchDialog {
    /// The saved workbook templates.
    templates: Vec<String>,
    template: usize,
    files: Vec<PathBuf>,
    selected: Option<usize>,
    error: Option<String>,
}

impl BatchDialog {
    pub fn new(templates: Vec<String>) -> Self {
        BatchDialog { templates, template: 0, files: Vec::new(), selected: None, error: None }
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
                        let current = self.templates.get(self.template).cloned().unwrap_or_default();
                        egui::ComboBox::from_id_salt("batch_template").selected_text(current).show_ui(ui, |ui| {
                            for (i, name) in self.templates.iter().enumerate() {
                                ui.selectable_value(&mut self.template, i, name);
                            }
                        });
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
                            template: self.templates[self.template].clone(),
                            files: self.files.clone(),
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
