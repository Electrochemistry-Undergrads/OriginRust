//! Plot > Statistics > QC (X bar R) Chart (T111): Origin 2016's "X bar R chart" prompt ("Please specify
//! subgroup size for Book1_B", 3 by default; measured 2026-11-16). OK makes the QC workbook and the chart
//! (`Project::plot_qc_xbar_r`); rules in `docs/spec/graphing.md`, "QC (X bar R) chart".

use eframe::egui;
use sheaf_core::{ColumnId, Project};

pub enum QcOutcome {
    Open,
    Closed,
    Make { column: ColumnId, size: usize },
}

pub struct QcDialog {
    pub column: ColumnId,
    /// `Book1_B`: the book and the column's short name.
    pub name: String,
    pub size: usize,
}

impl QcDialog {
    pub fn new(project: &Project, column: ColumnId) -> Option<Self> {
        let (book, _, col) = project.locate(column)?;
        Some(QcDialog { column, name: format!("{}_{}", book.name, col.short_name), size: 3 })
    }

    pub fn show(&mut self, ctx: &egui::Context) -> QcOutcome {
        let mut open = true;
        let mut outcome = QcOutcome::Open;
        egui::Window::new("X bar R chart")
            .id(egui::Id::new("qc_chart"))
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label(format!("Please specify subgroup size for {}", self.name));
                ui.add(
                    egui::DragValue::new(&mut self.size)
                        .range(sheaf_calc::qc::MIN_GROUP..=sheaf_calc::qc::MAX_GROUP)
                        .speed(0.1),
                );
                ui.horizontal(|ui| {
                    if ui.button("OK").clicked() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        outcome = QcOutcome::Make { column: self.column, size: self.size };
                    }
                    if ui.button("Cancel").clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        outcome = QcOutcome::Closed;
                    }
                });
            });
        if !open { QcOutcome::Closed } else { outcome }
    }
}
