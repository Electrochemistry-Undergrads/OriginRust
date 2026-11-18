//! File > Open Sample Projects and Help > Open Sample Projects > Sample Projects (T133): the built-in
//! samples of `sheaf_core::samples` with their descriptions; Open replaces the project with the chosen one
//! (after the usual question about unsaved changes). Rules in `docs/spec/gui-look.md`, "Sample Projects".

use eframe::egui;
use sheaf_core::samples::SAMPLES;

pub enum SampleOutcome {
    Open,
    Closed,
    Load(String),
}

#[derive(Default)]
pub struct SampleProjects {
    pub selected: usize,
}

impl SampleProjects {
    pub fn show(&mut self, ctx: &egui::Context) -> SampleOutcome {
        let mut open = true;
        let mut outcome = SampleOutcome::Open;
        egui::Window::new("Sample Projects")
            .id(egui::Id::new("sample_projects"))
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                for (i, (name, about)) in SAMPLES.iter().enumerate() {
                    let r = ui.selectable_label(self.selected == i, *name);
                    if r.clicked() {
                        self.selected = i;
                    }
                    if r.double_clicked() {
                        outcome = SampleOutcome::Load(name.to_string());
                    }
                    ui.weak(format!("    {about}"));
                }
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Open").clicked() {
                        outcome = SampleOutcome::Load(SAMPLES[self.selected.min(SAMPLES.len() - 1)].0.to_string());
                    }
                    if ui.button("Cancel").clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        outcome = SampleOutcome::Closed;
                    }
                });
            });
        if !open { SampleOutcome::Closed } else { outcome }
    }
}
