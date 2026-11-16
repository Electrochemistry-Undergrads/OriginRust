//! Graph themes (T102): an axis's Save Format as Theme... and Tools > Theme Organizer (F7), as Origin
//! 2016's windows (measured; `docs/spec/graphing.md`, "Graph themes"). The files are
//! `sheaf_io::graph_themes`.

use eframe::egui::{self, RichText};
use sheaf_io::graph_themes::ThemeEntry;

pub enum SaveThemeOutcome {
    Open,
    Closed,
    Save { name: String, description: String },
}

/// "Save Format as Theme": the new theme's name (the first free ThemeN) and a description.
pub struct SaveThemeDialog {
    pub name: String,
    pub description: String,
    pub existing: Vec<String>,
    pub error: Option<String>,
}

impl SaveThemeDialog {
    pub fn new(name: String, existing: Vec<String>) -> Self {
        SaveThemeDialog { name, description: String::new(), existing, error: None }
    }

    pub fn show(&mut self, ctx: &egui::Context) -> SaveThemeOutcome {
        let mut outcome = SaveThemeOutcome::Open;
        let mut open = true;
        egui::Window::new("Save Format as Theme")
            .id(egui::Id::new("save_format_as_theme"))
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                egui::Grid::new("save_theme_grid").num_columns(2).show(ui, |ui| {
                    ui.label("Name of the new theme:");
                    ui.add(egui::TextEdit::singleline(&mut self.name).desired_width(220.0));
                    ui.end_row();
                    ui.label("Description:");
                    ui.add(egui::TextEdit::singleline(&mut self.description).desired_width(220.0));
                    ui.end_row();
                });
                if self.existing.iter().any(|n| n.eq_ignore_ascii_case(self.name.trim())) {
                    ui.label(RichText::new("A theme of this name is replaced.").weak());
                }
                ui.label(RichText::new("Formats to Save: All (the axis's styles and scale)").weak());
                if let Some(e) = &self.error {
                    ui.label(RichText::new(e).color(egui::Color32::from_rgb(190, 30, 30)));
                }
                ui.horizontal(|ui| {
                    if ui.button("OK").clicked() {
                        outcome =
                            SaveThemeOutcome::Save { name: self.name.clone(), description: self.description.clone() };
                    }
                    if ui.button("Cancel").clicked() {
                        outcome = SaveThemeOutcome::Closed;
                    }
                });
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    outcome = SaveThemeOutcome::Closed;
                }
            });
        if !open { SaveThemeOutcome::Closed } else { outcome }
    }
}

pub enum OrganizerOutcome {
    Open,
    Closed,
    Apply(String),
    Delete(String),
}

/// Theme Organizer: the Graph tab's list (Name, Size, Path, Date) and Apply Now to the current graph.
pub struct ThemeOrganizer {
    pub entries: Vec<ThemeEntry>,
    pub selected: Option<String>,
    /// Whether a graph is active (Apply Now needs one).
    pub graph: bool,
}

impl ThemeOrganizer {
    pub fn show(&mut self, ctx: &egui::Context) -> OrganizerOutcome {
        let mut outcome = OrganizerOutcome::Open;
        let mut open = true;
        egui::Window::new("Theme Organizer")
            .id(egui::Id::new("theme_organizer"))
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label(RichText::new("Graph").strong());
                egui::ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
                    egui::Grid::new("theme_organizer_grid").striped(true).num_columns(4).show(ui, |ui| {
                        for head in ["Name", "Size", "Path", "Date"] {
                            ui.label(RichText::new(head).strong());
                        }
                        ui.end_row();
                        for e in &self.entries {
                            let picked = self.selected.as_deref() == Some(e.name.as_str());
                            if ui.selectable_label(picked, &e.name).clicked() {
                                self.selected = Some(e.name.clone());
                            }
                            ui.label(format!("{:.1} KB", e.size as f64 / 1024.0));
                            ui.label("User");
                            ui.label(&e.saved);
                            ui.end_row();
                        }
                    });
                    if self.entries.is_empty() {
                        ui.weak("No themes yet: right-click an axis and choose Save Format as Theme....");
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Apply Theme to: Current Graph");
                    let can = self.graph && self.selected.is_some();
                    if ui.add_enabled(can, egui::Button::new("Apply Now")).clicked() {
                        outcome = OrganizerOutcome::Apply(self.selected.clone().unwrap_or_default());
                    }
                    if ui.add_enabled(self.selected.is_some(), egui::Button::new("Delete")).clicked() {
                        outcome = OrganizerOutcome::Delete(self.selected.clone().unwrap_or_default());
                    }
                    if ui.button("Close").clicked() {
                        outcome = OrganizerOutcome::Closed;
                    }
                });
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    outcome = OrganizerOutcome::Closed;
                }
            });
        if !open { OrganizerOutcome::Closed } else { outcome }
    }
}
