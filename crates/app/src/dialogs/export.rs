//! The export window of a graph: PNG at a chosen resolution, SVG or PDF.

use eframe::egui::{self, DragValue};
use sheaf_core::{Document, WindowId};
use std::path::PathBuf;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ExportFormat {
    Png,
    Svg,
    Pdf,
}

impl ExportFormat {
    fn extension(self) -> &'static str {
        match self {
            ExportFormat::Png => "png",
            ExportFormat::Svg => "svg",
            ExportFormat::Pdf => "pdf",
        }
    }
}

pub struct ExportDialog {
    pub graph: WindowId,
    format: ExportFormat,
    dpi: f64,
    /// Folder the file chooser starts in.
    folder: Option<PathBuf>,
}

/// What happened in one frame of the dialog.
pub enum ExportOutcome {
    Open,
    Closed,
    /// A file was written.
    Saved(PathBuf),
    Failed(String),
}

impl ExportDialog {
    pub fn new(graph: WindowId, folder: Option<PathBuf>) -> Self {
        ExportDialog { graph, format: ExportFormat::Png, dpi: 300.0, folder }
    }

    pub fn show(&mut self, ctx: &egui::Context, doc: &Document) -> ExportOutcome {
        let Some(graph) = doc.project().graph(self.graph) else {
            return ExportOutcome::Closed;
        };
        let mut open = true;
        let mut outcome = ExportOutcome::Open;
        egui::Window::new(format!("Export {}", graph.name))
            .id(egui::Id::new("export_dialog"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                let (w_in, h_in) = (graph.page.width_pt / 72.0, graph.page.height_pt / 72.0);
                egui::Grid::new("export_grid").num_columns(2).spacing([14.0, 6.0]).show(ui, |ui| {
                    ui.label("Format");
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut self.format, ExportFormat::Png, "PNG image");
                        ui.selectable_value(&mut self.format, ExportFormat::Svg, "SVG (vector)");
                        ui.selectable_value(&mut self.format, ExportFormat::Pdf, "PDF (vector)");
                    });
                    ui.end_row();
                    ui.label("Page size");
                    ui.label(format!("{w_in:.2} x {h_in:.2} in"));
                    ui.end_row();
                    if self.format == ExportFormat::Png {
                        ui.label("Resolution");
                        ui.horizontal(|ui| {
                            ui.add(DragValue::new(&mut self.dpi).range(36.0..=2400.0).speed(5.0).suffix(" dpi"));
                            for preset in [150.0, 300.0, 600.0, 1200.0] {
                                if ui.small_button(format!("{preset}")).clicked() {
                                    self.dpi = preset;
                                }
                            }
                        });
                        ui.end_row();
                        ui.label("Image size");
                        ui.label(format!(
                            "{} x {} pixels",
                            (w_in * self.dpi).round() as u64,
                            (h_in * self.dpi).round() as u64
                        ));
                        ui.end_row();
                    }
                });
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.button("Save as...").clicked() {
                        let ext = self.format.extension();
                        let stem =
                            if graph.long_name.is_empty() { graph.name.clone() } else { graph.long_name.clone() };
                        let mut chooser = rfd::FileDialog::new()
                            .set_title("Export graph")
                            .add_filter(ext.to_uppercase(), &[ext])
                            .set_file_name(format!("{stem}.{ext}"));
                        if let Some(dir) = &self.folder {
                            chooser = chooser.set_directory(dir);
                        }
                        if let Some(mut path) = chooser.save_file() {
                            if path.extension().is_none() {
                                path.set_extension(ext);
                            }
                            outcome = match sheaf_plot::export_file(doc.project(), self.graph, &path, self.dpi) {
                                Ok(()) => ExportOutcome::Saved(path),
                                Err(e) => ExportOutcome::Failed(e.to_string()),
                            };
                        }
                    }
                    if ui.button("Close").clicked() {
                        outcome = ExportOutcome::Closed;
                    }
                });
            });
        if !open {
            outcome = ExportOutcome::Closed;
        }
        outcome
    }
}
