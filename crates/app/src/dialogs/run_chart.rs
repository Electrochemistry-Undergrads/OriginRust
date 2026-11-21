//! Plot > Statistics > Run Chart... (T270): the subgroup size, the statistic to connect (Mean or
//! Median) and whether the individual values show. OK makes the RunChart workbook and graph
//! (`Project::plot_run_chart`); rules in `docs/spec/graphing.md`, "Run chart".

use eframe::egui;
use sheaf_core::runchart::{MAX_SIZE, RunStat};
use sheaf_core::{ColumnId, Project};

pub enum RunChartOutcome {
    Open,
    Closed,
    Make { column: ColumnId, size: usize, stat: RunStat, points: bool },
}

pub struct RunChartDialog {
    pub column: ColumnId,
    /// `[Book1]Sheet1!B`-like name of the column for the window.
    pub name: String,
    pub size: usize,
    pub stat: RunStat,
    pub points: bool,
}

impl RunChartDialog {
    pub fn new(project: &Project, column: ColumnId) -> Option<Self> {
        let (book, _, col) = project.locate(column)?;
        Some(RunChartDialog {
            column,
            name: format!("{}_{}", book.name, col.short_name),
            size: 1,
            stat: RunStat::Mean,
            points: true,
        })
    }

    pub fn show(&mut self, ctx: &egui::Context) -> RunChartOutcome {
        let mut open = true;
        let mut outcome = RunChartOutcome::Open;
        egui::Window::new("Run Chart")
            .id(egui::Id::new("run_chart"))
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label(format!("Input: {}", self.name));
                egui::Grid::new("run_chart_grid").num_columns(2).show(ui, |ui| {
                    ui.label("Subgroup Size");
                    ui.add(egui::DragValue::new(&mut self.size).range(1..=MAX_SIZE).speed(0.1));
                    ui.end_row();
                    ui.label("Connect");
                    ui.horizontal(|ui| {
                        ui.radio_value(&mut self.stat, RunStat::Mean, "Mean");
                        ui.radio_value(&mut self.stat, RunStat::Median, "Median");
                    });
                    ui.end_row();
                });
                ui.checkbox(&mut self.points, "Show Individual Data Points");
                ui.weak("Origin 2025b's chart; not measured (Origin 2016 has none).");
                ui.horizontal(|ui| {
                    if ui.button("OK").clicked() {
                        outcome = RunChartOutcome::Make {
                            column: self.column,
                            size: self.size,
                            stat: self.stat,
                            points: self.points,
                        };
                    }
                    if ui.button("Cancel").clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        outcome = RunChartOutcome::Closed;
                    }
                });
            });
        if !open { RunChartOutcome::Closed } else { outcome }
    }
}
