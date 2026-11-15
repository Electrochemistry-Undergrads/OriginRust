//! Tools > Autosave...: switches the autosave on or off and sets how often it writes.

use eframe::egui;
use sheaf_io::recovery::{AutosaveSettings, MAX_MINUTES, MIN_MINUTES};

/// What the window did in a frame.
pub enum AutosaveOutcome {
    Open,
    Cancelled,
    /// OK: use these settings (and keep them).
    Done(AutosaveSettings),
}

pub struct AutosaveDialog {
    settings: AutosaveSettings,
    /// False when this run has no recovery folder, so nothing is written whatever is chosen.
    available: bool,
}

impl AutosaveDialog {
    pub fn new(settings: AutosaveSettings, available: bool) -> Self {
        AutosaveDialog { settings, available }
    }

    pub fn show(&mut self, ctx: &egui::Context) -> AutosaveOutcome {
        let mut open = true;
        let mut outcome = AutosaveOutcome::Open;
        egui::Window::new("Autosave").open(&mut open).collapsible(false).resizable(false).show(ctx, |ui| {
            ui.checkbox(&mut self.settings.enabled, "Autosave the project");
            ui.add_enabled_ui(self.settings.enabled, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Every");
                    ui.add(egui::DragValue::new(&mut self.settings.minutes).range(MIN_MINUTES..=MAX_MINUTES));
                    ui.label("minutes");
                });
            });
            ui.weak("A modified project is written to a recovery file; after a crash the next start offers it back.");
            if !self.available {
                ui.colored_label(
                    ui.visuals().warn_fg_color,
                    "There is no recovery folder in this run, so nothing is written.",
                );
            }
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui.button("OK").clicked() {
                    outcome = AutosaveOutcome::Done(self.settings.clamped());
                }
                if ui.button("Cancel").clicked() {
                    outcome = AutosaveOutcome::Cancelled;
                }
            });
        });
        if !open || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            return AutosaveOutcome::Cancelled;
        }
        outcome
    }
}
