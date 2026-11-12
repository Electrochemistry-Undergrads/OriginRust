//! The export window of a graph: PNG or TIFF at a chosen resolution, SVG or PDF, at the page size
//! or scaled to a chosen width.

use eframe::egui::{self, DragValue};
use sheaf_core::{Document, WindowId};
use std::path::PathBuf;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ExportFormat {
    Png,
    Tiff,
    Svg,
    Pdf,
}

impl ExportFormat {
    fn extension(self) -> &'static str {
        match self {
            ExportFormat::Png => "png",
            ExportFormat::Tiff => "tif",
            ExportFormat::Svg => "svg",
            ExportFormat::Pdf => "pdf",
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum WidthUnit {
    Inch,
    Cm,
}

impl WidthUnit {
    fn per_inch(self) -> f64 {
        match self {
            WidthUnit::Inch => 1.0,
            WidthUnit::Cm => 2.54,
        }
    }
}

pub struct ExportDialog {
    pub graph: WindowId,
    format: ExportFormat,
    dpi: f64,
    /// When set, the picture is scaled to this width (in `width_unit`), text and lines included;
    /// the height follows the page's proportions.
    width: Option<f64>,
    width_unit: WidthUnit,
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
        ExportDialog { graph, format: ExportFormat::Png, dpi: 300.0, width: None, width_unit: WidthUnit::Inch, folder }
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
                        ui.selectable_value(&mut self.format, ExportFormat::Tiff, "TIFF image");
                        ui.selectable_value(&mut self.format, ExportFormat::Svg, "SVG (vector)");
                        ui.selectable_value(&mut self.format, ExportFormat::Pdf, "PDF (vector)");
                    });
                    ui.end_row();
                    ui.label("Page size");
                    ui.label(format!("{w_in:.2} x {h_in:.2} in"));
                    ui.end_row();
                    ui.label("Width");
                    ui.horizontal(|ui| {
                        let mut set = self.width.is_some();
                        if ui
                            .checkbox(&mut set, "Scale to")
                            .on_hover_text("Scale the whole picture, text and lines included, to this width")
                            .changed()
                        {
                            self.width = set.then_some(w_in * self.width_unit.per_inch());
                        }
                        if let Some(w) = &mut self.width {
                            let before = self.width_unit;
                            ui.add(DragValue::new(w).range(0.1..=200.0).speed(0.05).max_decimals(2));
                            egui::ComboBox::from_id_salt("export_width_unit")
                                .selected_text(if self.width_unit == WidthUnit::Inch { "in" } else { "cm" })
                                .width(48.0)
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut self.width_unit, WidthUnit::Inch, "in");
                                    ui.selectable_value(&mut self.width_unit, WidthUnit::Cm, "cm");
                                });
                            if self.width_unit != before {
                                *w *= self.width_unit.per_inch() / before.per_inch();
                            }
                        }
                    });
                    ui.end_row();
                    let scale = self.width.map_or(1.0, |w| w / self.width_unit.per_inch() / w_in);
                    let (w_in, h_in) = (w_in * scale, h_in * scale);
                    if self.width.is_some() {
                        ui.label("Picture size");
                        ui.label(format!("{w_in:.2} x {h_in:.2} in"));
                        ui.end_row();
                    }
                    if matches!(self.format, ExportFormat::Png | ExportFormat::Tiff) {
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
                            let size = sheaf_plot::ExportSize {
                                width_in: self.width.map(|w| w / self.width_unit.per_inch()),
                                height_in: None,
                            };
                            outcome =
                                match sheaf_plot::export_file_sized(doc.project(), self.graph, &path, self.dpi, size) {
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
