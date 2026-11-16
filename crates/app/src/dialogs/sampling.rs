//! Column > Set Sampling Interval... and Show X Column... (T84): the fields of Origin's `colint`
//! (Initial X Value, X Increment, Units, Long Name) and `colshowx` (Clear Sampling Interval). The
//! work is `Project::set_sampling_interval` and `Project::show_x_column`; rules in
//! `docs/spec/workbook-model.md`, "Sampling interval".

use eframe::egui::{self, DragValue, RichText};
use sheaf_core::ColumnId;
use sheaf_core::sampling::SamplingInterval;

pub enum SamplingOutcome {
    Open,
    Closed,
    Set { columns: Vec<ColumnId>, x0: f64, inc: f64, long_name: String, units: String },
}

pub struct SamplingDialog {
    columns: Vec<ColumnId>,
    x0: f64,
    inc: f64,
    units: String,
    long_name: String,
    problem: Option<String>,
}

impl SamplingDialog {
    /// A window for `columns`, filled with the first one's interval or Origin's 1 and 1.
    pub fn new(columns: Vec<ColumnId>, current: Option<&SamplingInterval>) -> Self {
        let (x0, inc, units, long_name) = match current {
            Some(s) => (s.x0, s.inc, s.units.clone(), s.long_name.clone()),
            None => (1.0, 1.0, String::new(), String::new()),
        };
        SamplingDialog { columns, x0, inc, units, long_name, problem: None }
    }

    pub fn set_problem(&mut self, p: String) {
        self.problem = Some(p);
    }

    pub fn show(&mut self, ctx: &egui::Context) -> SamplingOutcome {
        let mut outcome = SamplingOutcome::Open;
        let mut open = true;
        egui::Window::new("Set Sampling Interval")
            .id(egui::Id::new("set_sampling_interval"))
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                egui::Grid::new("sampling_grid").num_columns(2).show(ui, |ui| {
                    ui.label("Initial X Value");
                    ui.add(DragValue::new(&mut self.x0).speed(0.1));
                    ui.end_row();
                    ui.label("X Increment");
                    ui.add(DragValue::new(&mut self.inc).speed(0.1));
                    ui.end_row();
                    ui.label("Units");
                    ui.add(egui::TextEdit::singleline(&mut self.units).desired_width(160.0));
                    ui.end_row();
                    ui.label("Long Name");
                    ui.add(egui::TextEdit::singleline(&mut self.long_name).desired_width(160.0));
                    ui.end_row();
                });
                ui.weak("To clear the sampling interval, set both Initial X Value and X Increment to 1.");
                if let Some(p) = &self.problem {
                    ui.label(RichText::new(p).color(egui::Color32::from_rgb(190, 30, 30)));
                }
                ui.horizontal(|ui| {
                    if ui.button("OK").clicked() {
                        outcome = SamplingOutcome::Set {
                            columns: self.columns.clone(),
                            x0: self.x0,
                            inc: self.inc,
                            long_name: self.long_name.clone(),
                            units: self.units.clone(),
                        };
                    }
                    if ui.button("Cancel").clicked() {
                        outcome = SamplingOutcome::Closed;
                    }
                });
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    outcome = SamplingOutcome::Closed;
                }
            });
        if !open { SamplingOutcome::Closed } else { outcome }
    }
}

pub enum ShowXOutcome {
    Open,
    Closed,
    Show { columns: Vec<ColumnId>, clear: bool },
}

pub struct ShowXDialog {
    columns: Vec<ColumnId>,
    clear: bool,
    problem: Option<String>,
}

impl ShowXDialog {
    /// Origin's default leaves the interval in place.
    pub fn new(columns: Vec<ColumnId>) -> Self {
        ShowXDialog { columns, clear: false, problem: None }
    }

    pub fn set_problem(&mut self, p: String) {
        self.problem = Some(p);
    }

    pub fn show(&mut self, ctx: &egui::Context) -> ShowXOutcome {
        let mut outcome = ShowXOutcome::Open;
        let mut open = true;
        egui::Window::new("Show X Column")
            .id(egui::Id::new("show_x_column"))
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.checkbox(&mut self.clear, "Clear Sampling Interval");
                ui.weak(if self.clear {
                    "The new column becomes the X column and the interval is cleared."
                } else {
                    "The new column is a Disregard column; the interval stays."
                });
                if let Some(p) = &self.problem {
                    ui.label(RichText::new(p).color(egui::Color32::from_rgb(190, 30, 30)));
                }
                ui.horizontal(|ui| {
                    if ui.button("OK").clicked() {
                        outcome = ShowXOutcome::Show { columns: self.columns.clone(), clear: self.clear };
                    }
                    if ui.button("Cancel").clicked() {
                        outcome = ShowXOutcome::Closed;
                    }
                });
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    outcome = ShowXOutcome::Closed;
                }
            });
        if !open { ShowXOutcome::Closed } else { outcome }
    }
}
