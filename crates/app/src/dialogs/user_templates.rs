//! File > Save Template As... and Plot > User Templates > Manage... (T56): the name prompt for a new
//! user graph template and the list that renames and deletes them. The files are written by
//! `sheaf_io::user_templates`; these windows only ask. Rules: `docs/spec/graphing.md`, section 24.

use eframe::egui::{self, RichText};
use sheaf_core::WindowId;

/// What the Save Template As window asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptOutcome {
    Open,
    Closed,
    /// Save the graph under this name (as typed; the store cleans it).
    Save(String),
}

/// The name prompt of File > Save Template As.
pub struct SaveTemplatePrompt {
    /// The graph whose look is saved.
    pub graph: WindowId,
    name: String,
    /// The saved names, to say that OK replaces one.
    existing: Vec<String>,
    error: Option<String>,
    focus: bool,
}

impl SaveTemplatePrompt {
    /// A prompt filled with `default_name` (the graph's short name).
    pub fn new(graph: WindowId, default_name: &str, existing: Vec<String>) -> Self {
        SaveTemplatePrompt { graph, name: default_name.to_string(), existing, error: None, focus: true }
    }

    /// Shows a problem of the last OK under the name; the window stays open.
    pub fn set_error(&mut self, error: String) {
        self.error = Some(error);
    }

    pub fn show(&mut self, ctx: &egui::Context) -> PromptOutcome {
        let mut outcome = PromptOutcome::Open;
        let mut open = true;
        egui::Window::new("Save Template As")
            .id(egui::Id::new("save_template_as"))
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label("Template name");
                let edit = ui.add(
                    egui::TextEdit::singleline(&mut self.name).desired_width(260.0).id(egui::Id::new("template_name")),
                );
                if self.focus {
                    edit.request_focus();
                    self.focus = false;
                }
                let lower = self.name.trim().to_lowercase();
                if self.existing.iter().any(|n| n.to_lowercase() == lower) {
                    ui.label("A template of this name is saved; OK replaces it.");
                }
                ui.weak("The template keeps the look of the graph, not its data (Plot > User Templates).");
                if let Some(e) = &self.error {
                    ui.label(RichText::new(e).color(egui::Color32::from_rgb(190, 30, 30)));
                }
                ui.horizontal(|ui| {
                    let enter = edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    if ui.button("OK").clicked() || enter {
                        outcome = PromptOutcome::Save(self.name.clone());
                    }
                    if ui.button("Cancel").clicked() {
                        outcome = PromptOutcome::Closed;
                    }
                });
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    outcome = PromptOutcome::Closed;
                }
            });
        if !open { PromptOutcome::Closed } else { outcome }
    }
}

/// What the Manage window asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManagerOutcome {
    Open,
    Closed,
    Rename { from: String, to: String },
    Delete(String),
}

/// Plot > User Templates > Manage...: the saved templates with Rename and Delete.
pub struct TemplateManager {
    names: Vec<String>,
    selected: Option<usize>,
    new_name: String,
    error: Option<String>,
}

impl TemplateManager {
    pub fn new(names: Vec<String>) -> Self {
        let mut m = TemplateManager { names: Vec::new(), selected: None, new_name: String::new(), error: None };
        m.set_names(names);
        m
    }

    /// The list after a change; the selection follows the name when it is still there.
    pub fn set_names(&mut self, names: Vec<String>) {
        let keep = self.selected.and_then(|i| self.names.get(i)).map(|n| n.to_lowercase());
        self.names = names;
        self.selected = keep.and_then(|k| self.names.iter().position(|n| n.to_lowercase() == k));
        self.new_name = self.selected.map(|i| self.names[i].clone()).unwrap_or_default();
        self.error = None;
    }

    /// Selects a template by name (letter case does not matter).
    pub fn select(&mut self, name: &str) {
        let lower = name.to_lowercase();
        self.selected = self.names.iter().position(|n| n.to_lowercase() == lower);
        self.new_name = self.selected.map(|i| self.names[i].clone()).unwrap_or_default();
    }

    pub fn set_error(&mut self, error: String) {
        self.error = Some(error);
    }

    pub fn show(&mut self, ctx: &egui::Context, folder: Option<&std::path::Path>) -> ManagerOutcome {
        let mut outcome = ManagerOutcome::Open;
        let mut open = true;
        egui::Window::new("User Templates")
            .id(egui::Id::new("user_templates_manager"))
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                if self.names.is_empty() {
                    ui.weak("No templates are saved. On a graph: File > Save Template As...");
                }
                egui::ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
                    for (i, name) in self.names.iter().enumerate() {
                        if ui.selectable_label(self.selected == Some(i), name).clicked() {
                            self.selected = Some(i);
                            self.new_name = name.clone();
                        }
                    }
                });
                if let Some(i) = self.selected {
                    let name = self.names[i].clone();
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label("New name");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.new_name)
                                .desired_width(200.0)
                                .id(egui::Id::new("template_new_name")),
                        );
                        if ui.button("Rename").clicked() {
                            outcome = ManagerOutcome::Rename { from: name.clone(), to: self.new_name.clone() };
                        }
                    });
                    if ui.button("Delete").clicked() {
                        outcome = ManagerOutcome::Delete(name);
                    }
                }
                if let Some(f) = folder {
                    ui.weak(format!("Folder: {}", f.display()));
                }
                if let Some(e) = &self.error {
                    ui.label(RichText::new(e).color(egui::Color32::from_rgb(190, 30, 30)));
                }
                if ui.button("Close").clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    outcome = ManagerOutcome::Closed;
                }
            });
        if !open { ManagerOutcome::Closed } else { outcome }
    }
}
