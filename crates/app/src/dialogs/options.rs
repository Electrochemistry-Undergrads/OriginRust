//! Tools > Options... (T134): the home of Sheaf's application settings, in tabs as Origin's Options
//! window (which has ten). Open/Close holds the autosave switch and interval (the same settings as
//! Tools > Autosave...); File Locations lists the folders Sheaf keeps its files in. Built from Origin's
//! documented behaviour, not measured; rules in `docs/spec/gui-look.md`, "Options".

use eframe::egui;
use sheaf_io::recovery::{AutosaveSettings, MAX_MINUTES, MIN_MINUTES};
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OptionsTab {
    OpenClose,
    FileLocations,
}

pub enum OptionsOutcome {
    Open,
    Cancelled,
    /// OK: use these settings (and keep them).
    Done(AutosaveSettings),
}

pub struct OptionsDialog {
    pub tab: OptionsTab,
    pub autosave: AutosaveSettings,
    /// False when this run has no recovery folder, so autosave writes nothing.
    available: bool,
    /// (what, folder): `None` when this run has no such folder.
    locations: Vec<(&'static str, Option<PathBuf>)>,
}

impl OptionsDialog {
    pub fn new(autosave: AutosaveSettings, available: bool, locations: Vec<(&'static str, Option<PathBuf>)>) -> Self {
        OptionsDialog { tab: OptionsTab::OpenClose, autosave, available, locations }
    }

    pub fn show(&mut self, ctx: &egui::Context) -> OptionsOutcome {
        let mut open = true;
        let mut outcome = OptionsOutcome::Open;
        egui::Window::new("Options")
            .id(egui::Id::new("options_window"))
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.tab, OptionsTab::OpenClose, "Open/Close");
                    ui.selectable_value(&mut self.tab, OptionsTab::FileLocations, "File Locations");
                });
                ui.separator();
                match self.tab {
                    OptionsTab::OpenClose => {
                        ui.checkbox(&mut self.autosave.enabled, "Autosave the project");
                        ui.add_enabled_ui(self.autosave.enabled, |ui| {
                            ui.horizontal(|ui| {
                                ui.label("Every");
                                ui.add(
                                    egui::DragValue::new(&mut self.autosave.minutes).range(MIN_MINUTES..=MAX_MINUTES),
                                );
                                ui.label("minutes");
                            });
                        });
                        if !self.available {
                            ui.colored_label(
                                ui.visuals().warn_fg_color,
                                "There is no recovery folder in this run, so nothing is written.",
                            );
                        }
                    }
                    OptionsTab::FileLocations => {
                        egui::Grid::new("options_locations").num_columns(2).striped(true).show(ui, |ui| {
                            for (what, dir) in &self.locations {
                                ui.label(*what);
                                match dir {
                                    Some(d) => ui.label(d.display().to_string()),
                                    None => ui.weak("(none in this run)"),
                                };
                                ui.end_row();
                            }
                        });
                    }
                }
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    if ui.button("OK").clicked() {
                        outcome = OptionsOutcome::Done(self.autosave.clamped());
                    }
                    if ui.button("Cancel").clicked() {
                        outcome = OptionsOutcome::Cancelled;
                    }
                });
            });
        if !open || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            return OptionsOutcome::Cancelled;
        }
        outcome
    }
}
