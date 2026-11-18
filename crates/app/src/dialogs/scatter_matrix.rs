//! Plot > Statistics > Scatter Matrix (T114): the options of Origin's `plot_matrix` dialog that Sheaf builds
//! (Matrix Display, Show in Diagonal Cells, Variables in Diagonal Cells, Gap), with the ones it does not build
//! (confidence ellipse, linear fit, listwise missing values) shown disabled. OK makes the graph
//! (`Project::plot_scatter_matrix`); rules in `docs/spec/graphing.md`, "Scatter matrix".

use eframe::egui;
use sheaf_core::scatter_matrix::{MAX_GAP_PCT, MatrixDiagonal, MatrixDisplay, ScatterMatrixOptions};
use sheaf_core::{ColumnId, Project};

pub enum ScatterMatrixOutcome {
    Open,
    Closed,
    Make { columns: Vec<ColumnId>, options: ScatterMatrixOptions },
}

pub struct ScatterMatrixDialog {
    pub columns: Vec<ColumnId>,
    /// The input range as shown: the columns' short names.
    pub range: String,
    pub options: ScatterMatrixOptions,
}

impl ScatterMatrixDialog {
    pub fn new(project: &Project, columns: Vec<ColumnId>) -> Self {
        let names: Vec<String> = columns
            .iter()
            .filter_map(|c| project.locate(*c).map(|(book, _, col)| format!("[{}]{}", book.name, col.short_name)))
            .collect();
        ScatterMatrixDialog { columns, range: names.join(", "), options: ScatterMatrixOptions::default() }
    }

    pub fn show(&mut self, ctx: &egui::Context) -> ScatterMatrixOutcome {
        let mut open = true;
        let mut outcome = ScatterMatrixOutcome::Open;
        let o = &mut self.options;
        egui::Window::new("Plotting: plot_matrix")
            .id(egui::Id::new("scatter_matrix"))
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                egui::Grid::new("scatter_matrix_grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
                    ui.label("Input Data");
                    ui.label(&self.range);
                    ui.end_row();
                    ui.label("Matrix Display");
                    egui::ComboBox::from_id_salt("scatter_matrix_display").selected_text(o.display.label()).show_ui(
                        ui,
                        |ui| {
                            for d in MatrixDisplay::ALL {
                                ui.selectable_value(&mut o.display, d, d.label());
                            }
                        },
                    );
                    ui.end_row();
                    ui.label("Show in Diagonal Cells");
                    egui::ComboBox::from_id_salt("scatter_matrix_diagonal").selected_text(o.diagonal.label()).show_ui(
                        ui,
                        |ui| {
                            for d in MatrixDiagonal::ALL {
                                ui.selectable_value(&mut o.diagonal, d, d.label());
                            }
                        },
                    );
                    ui.end_row();
                    ui.label("Variables in Diagonal Cells");
                    ui.checkbox(&mut o.names, "");
                    ui.end_row();
                    ui.label("Gap (% of page)");
                    ui.add(egui::DragValue::new(&mut o.gap_pct).range(0.0..=MAX_GAP_PCT).speed(0.1));
                    ui.end_row();
                    // Origin's further options, not in Sheaf yet.
                    for label in ["Confidence Ellipse", "Linear Fit", "Exclude Missing Values Listwise"] {
                        ui.add_enabled_ui(false, |ui| ui.label(label))
                            .response
                            .on_disabled_hover_text(crate::menus::NOT_YET);
                        ui.add_enabled(false, egui::Checkbox::new(&mut false, ""));
                        ui.end_row();
                    }
                });
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("OK").clicked() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        outcome = ScatterMatrixOutcome::Make { columns: self.columns.clone(), options: *o };
                    }
                    if ui.button("Cancel").clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        outcome = ScatterMatrixOutcome::Closed;
                    }
                });
            });
        if !open { ScatterMatrixOutcome::Closed } else { outcome }
    }
}
