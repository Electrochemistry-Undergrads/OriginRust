//! Analysis > Peaks and Baseline > Batch Peak Analysis Using Theme... (T169): pick a saved Peak Analyzer
//! theme and run it on many curves at once. The window only asks; the app runs the analysis. Rules:
//! `docs/spec/analysis-tools.md`, "Peak Analyzer themes and batch peak analysis".

use eframe::egui::{self, RichText};
use sheaf_core::ColumnId;

/// What the window asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BatchPeaksOutcome {
    Open,
    Closed,
    /// Run the theme of this name on the window's columns.
    Run(String),
}

/// The Batch Peak Analysis window.
pub struct BatchPeaksDialog {
    themes: Vec<String>,
    theme: usize,
    /// The curves (Y columns) the analysis runs on, and how the window names them.
    pub columns: Vec<ColumnId>,
    names: String,
    error: Option<String>,
}

impl BatchPeaksDialog {
    pub fn new(themes: Vec<String>, columns: Vec<ColumnId>, names: String) -> Self {
        BatchPeaksDialog { themes, theme: 0, columns, names, error: None }
    }

    pub fn set_error(&mut self, error: String) {
        self.error = Some(error);
    }

    pub fn show(&mut self, ctx: &egui::Context) -> BatchPeaksOutcome {
        let mut outcome = BatchPeaksOutcome::Open;
        let mut open = true;
        egui::Window::new("Batch Peak Analysis")
            .id(egui::Id::new("batch_peak_analysis"))
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                if self.themes.is_empty() {
                    ui.weak(
                        "No Peak Analyzer theme is saved. Open the Peak Analyzer, set it up and use Save as Theme.",
                    );
                } else {
                    ui.horizontal(|ui| {
                        ui.label("Theme");
                        let current = self.themes.get(self.theme).cloned().unwrap_or_default();
                        egui::ComboBox::from_id_salt("batch_peak_theme").selected_text(current).show_ui(ui, |ui| {
                            for (i, name) in self.themes.iter().enumerate() {
                                ui.selectable_value(&mut self.theme, i, name);
                            }
                        });
                    });
                }
                ui.label(format!("Curves ({}): {}", self.columns.len(), self.names));
                ui.weak("Each curve is analysed with the theme's settings; the results go to one new sheet.");
                if let Some(e) = &self.error {
                    ui.label(RichText::new(e).color(egui::Color32::from_rgb(190, 30, 30)));
                }
                ui.horizontal(|ui| {
                    let ready = !self.themes.is_empty() && !self.columns.is_empty();
                    if ui.add_enabled(ready, egui::Button::new("OK")).clicked() {
                        outcome = BatchPeaksOutcome::Run(self.themes[self.theme].clone());
                    }
                    if ui.button("Cancel").clicked() {
                        outcome = BatchPeaksOutcome::Closed;
                    }
                });
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    outcome = BatchPeaksOutcome::Closed;
                }
            });
        if !open { BatchPeaksOutcome::Closed } else { outcome }
    }
}
