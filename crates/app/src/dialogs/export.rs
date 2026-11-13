//! The export window of a graph: PNG or TIFF at a chosen resolution, SVG, PDF, EPS or EMF, of the whole
//! page or of the box round what is drawn, at the page size or scaled to a chosen width, in inches,
//! centimetres, pixels or percent of the page (Origin's "size and margin options").

use eframe::egui::{self, DragValue};
use sheaf_core::{Document, WindowId};
use sheaf_plot::{ExportLayout, ExportSize, Margin, SizeUnit};
use std::path::PathBuf;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ExportFormat {
    Png,
    Tiff,
    Svg,
    Pdf,
    Eps,
    Emf,
}

impl ExportFormat {
    fn extension(self) -> &'static str {
        match self {
            ExportFormat::Png => "png",
            ExportFormat::Tiff => "tif",
            ExportFormat::Svg => "svg",
            ExportFormat::Pdf => "pdf",
            ExportFormat::Eps => "eps",
            ExportFormat::Emf => "emf",
        }
    }

    fn is_raster(self) -> bool {
        matches!(self, ExportFormat::Png | ExportFormat::Tiff)
    }
}

fn unit_label(u: SizeUnit) -> &'static str {
    match u {
        SizeUnit::Inch => "in",
        SizeUnit::Cm => "cm",
        SizeUnit::Pixel => "px",
        SizeUnit::PageRatio => "% of page",
    }
}

fn margin_label(m: Margin) -> &'static str {
    match m {
        Margin::Auto => "Auto (Copy Page setting)",
        Margin::Border => "Border",
        Margin::Tight => "Tight",
        Margin::Page => "Page",
        Margin::TightInPage => "Tight in page",
    }
}

pub struct ExportDialog {
    pub graph: WindowId,
    format: ExportFormat,
    dpi: f64,
    /// When set, the picture is scaled to this width (in `width_unit`), text and lines included;
    /// the height follows the proportions of the exported area.
    width: Option<f64>,
    width_unit: SizeUnit,
    /// Which part of the page is exported. Origin's default `Auto` is `Border`.
    margin: Margin,
    /// The border width of `Margin::Border` in pixels; `None` is the default.
    border_px: Option<f64>,
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
        ExportDialog {
            graph,
            format: ExportFormat::Png,
            dpi: 300.0,
            width: None,
            width_unit: SizeUnit::Inch,
            margin: Margin::Auto,
            border_px: None,
            folder,
        }
    }

    fn layout(&self) -> ExportLayout {
        ExportLayout { margin: self.margin, border_px: self.border_px }
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
                // Pixels only exist for raster formats.
                if !self.format.is_raster() && self.width_unit == SizeUnit::Pixel {
                    self.width_unit = SizeUnit::Inch;
                    self.width = None;
                }
                let raster_dpi = self.format.is_raster().then_some(self.dpi);
                let area = sheaf_plot::export_area(doc.project(), self.graph, self.layout(), raster_dpi).ok();
                let (page_w, page_h) = (graph.page.width_pt / 72.0, graph.page.height_pt / 72.0);
                let (w_in, h_in) = area.map_or((page_w, page_h), |a| (a.w / 72.0, a.h / 72.0));
                egui::Grid::new("export_grid").num_columns(2).spacing([14.0, 6.0]).show(ui, |ui| {
                    ui.label("Format");
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut self.format, ExportFormat::Png, "PNG image");
                        ui.selectable_value(&mut self.format, ExportFormat::Tiff, "TIFF image");
                        ui.selectable_value(&mut self.format, ExportFormat::Svg, "SVG (vector)");
                        ui.selectable_value(&mut self.format, ExportFormat::Pdf, "PDF (vector)");
                        ui.selectable_value(&mut self.format, ExportFormat::Eps, "EPS (vector)");
                        ui.selectable_value(&mut self.format, ExportFormat::Emf, "EMF (vector)");
                    });
                    ui.end_row();
                    ui.label("Page size");
                    ui.label(format!("{page_w:.2} x {page_h:.2} in"));
                    ui.end_row();
                    ui.label("Margin");
                    ui.horizontal(|ui| {
                        egui::ComboBox::from_id_salt("export_margin")
                            .selected_text(margin_label(self.margin))
                            .show_ui(ui, |ui| {
                                for m in
                                    [Margin::Auto, Margin::Border, Margin::Tight, Margin::Page, Margin::TightInPage]
                                {
                                    ui.selectable_value(&mut self.margin, m, margin_label(m));
                                }
                            })
                            .response
                            .on_hover_text(
                                "Border: everything drawn plus a border. Tight: everything drawn. Page: the whole \
                                 page. Tight in page: the objects that are over half inside the page.",
                            );
                        if self.margin.resolved() == Margin::Border {
                            let mut px = self.border_px.unwrap_or(sheaf_plot::DEFAULT_BORDER_PT);
                            if ui
                                .add(
                                    DragValue::new(&mut px).range(0.0..=500.0).speed(0.5).max_decimals(1).suffix(" px"),
                                )
                                .on_hover_text("Border width (pixels of the picture; points for vector formats)")
                                .changed()
                            {
                                self.border_px = Some(px);
                            }
                        }
                    });
                    ui.end_row();
                    ui.label("Width");
                    ui.horizontal(|ui| {
                        let mut set = self.width.is_some();
                        if ui
                            .checkbox(&mut set, "Scale to")
                            .on_hover_text("Scale the whole picture, text and lines included, to this width")
                            .changed()
                        {
                            self.width = set.then_some(self.width_unit.from_inches(w_in, self.dpi, w_in));
                        }
                        if let Some(w) = &mut self.width {
                            let before = self.width_unit;
                            ui.add(DragValue::new(w).range(0.1..=20000.0).speed(0.05).max_decimals(2));
                            egui::ComboBox::from_id_salt("export_width_unit")
                                .selected_text(unit_label(self.width_unit))
                                .width(80.0)
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut self.width_unit, SizeUnit::Inch, "in");
                                    ui.selectable_value(&mut self.width_unit, SizeUnit::Cm, "cm");
                                    if self.format.is_raster() {
                                        ui.selectable_value(&mut self.width_unit, SizeUnit::Pixel, "px");
                                    }
                                    ui.selectable_value(&mut self.width_unit, SizeUnit::PageRatio, "% of page");
                                });
                            if self.width_unit != before {
                                let inches = before.to_inches(*w, self.dpi, w_in);
                                *w = self.width_unit.from_inches(inches, self.dpi, w_in);
                            }
                        }
                    });
                    ui.end_row();
                    let scale = self.width.map_or(1.0, |w| self.width_unit.to_inches(w, self.dpi, w_in) / w_in);
                    let (w_in, h_in) = (w_in * scale, h_in * scale);
                    if self.width.is_some() || self.margin.resolved() != Margin::Page {
                        ui.label("Picture size");
                        ui.label(format!("{w_in:.2} x {h_in:.2} in"));
                        ui.end_row();
                    }
                    if self.format.is_raster() {
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
                            let size = self.width.map_or_else(ExportSize::default, |w| {
                                ExportSize::from_unit(Some(w), None, self.width_unit, self.dpi)
                            });
                            outcome = match sheaf_plot::export_file_with(
                                doc.project(),
                                self.graph,
                                &path,
                                self.dpi,
                                size,
                                self.layout(),
                            ) {
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
            return ExportOutcome::Closed;
        }
        outcome
    }
}
