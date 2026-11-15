//! The Export Graphs window, laid out as Origin's: image type, which graphs, the file name (with
//! `<long name>` style substitution), the folder and what to do about an existing file, then the
//! groups Export Settings (margin), Image Size and Image Settings (resolution, colour depth, TIFF
//! compression, JPEG quality), a preview pane and the buttons Preview, OK, Cancel and Apply.

use eframe::egui::{self, DragValue};
use sheaf_core::{Document, WindowId};
use sheaf_plot::{ColorDepth, ExportLayout, ExportSize, Margin, RasterOptions, SizeUnit, TiffCompression};
use std::path::{Path, PathBuf};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ExportFormat {
    Png,
    Tiff,
    Jpeg,
    Bmp,
    Gif,
    Tga,
    Pcx,
    Psd,
    Svg,
    Pdf,
    Eps,
    Emf,
}

impl ExportFormat {
    const ALL: [ExportFormat; 12] = [
        ExportFormat::Png,
        ExportFormat::Tiff,
        ExportFormat::Jpeg,
        ExportFormat::Bmp,
        ExportFormat::Gif,
        ExportFormat::Tga,
        ExportFormat::Pcx,
        ExportFormat::Psd,
        ExportFormat::Svg,
        ExportFormat::Pdf,
        ExportFormat::Eps,
        ExportFormat::Emf,
    ];

    pub fn extension(self) -> &'static str {
        match self {
            ExportFormat::Png => "png",
            ExportFormat::Tiff => "tif",
            ExportFormat::Jpeg => "jpg",
            ExportFormat::Bmp => "bmp",
            ExportFormat::Gif => "gif",
            ExportFormat::Tga => "tga",
            ExportFormat::Pcx => "pcx",
            ExportFormat::Psd => "psd",
            ExportFormat::Svg => "svg",
            ExportFormat::Pdf => "pdf",
            ExportFormat::Eps => "eps",
            ExportFormat::Emf => "emf",
        }
    }

    fn label(self) -> String {
        let name = match self {
            ExportFormat::Png => "Portable Network Graphics",
            ExportFormat::Tiff => "Tagged Image File Format",
            ExportFormat::Jpeg => "JPEG",
            ExportFormat::Bmp => "Windows Bitmap",
            ExportFormat::Gif => "Graphics Interchange Format",
            ExportFormat::Tga => "Truevision Targa",
            ExportFormat::Pcx => "ZSoft PC Paintbrush",
            ExportFormat::Psd => "Photoshop Document",
            ExportFormat::Svg => "Scalable Vector Graphics",
            ExportFormat::Pdf => "Portable Document Format",
            ExportFormat::Eps => "Encapsulated PostScript",
            ExportFormat::Emf => "Enhanced Metafile",
        };
        format!("{name} (*.{})", self.extension())
    }

    fn is_raster(self) -> bool {
        matches!(
            self,
            ExportFormat::Png
                | ExportFormat::Tiff
                | ExportFormat::Jpeg
                | ExportFormat::Bmp
                | ExportFormat::Gif
                | ExportFormat::Tga
                | ExportFormat::Pcx
                | ExportFormat::Psd
        )
    }

    /// The colour depths the format can store.
    fn depths(self) -> &'static [ColorDepth] {
        match self {
            // Origin 2016's Color Depth lists, in its order (graphing.md 9.4); its 16-bit Color for
            // BMP and TGA is not offered.
            ExportFormat::Png | ExportFormat::Bmp | ExportFormat::Pcx | ExportFormat::Tiff => {
                &[ColorDepth::Mono, ColorDepth::Colors16, ColorDepth::Colors256, ColorDepth::Bit24, ColorDepth::Grey8]
            }
            ExportFormat::Psd => &[ColorDepth::Mono, ColorDepth::Colors256, ColorDepth::Bit24, ColorDepth::Grey8],
            ExportFormat::Tga => &[ColorDepth::Colors256, ColorDepth::Bit24, ColorDepth::Grey8],
            ExportFormat::Jpeg => &[ColorDepth::Bit24, ColorDepth::Grey8],
            ExportFormat::Gif => &[ColorDepth::Mono, ColorDepth::Colors16, ColorDepth::Colors256, ColorDepth::Grey8],
            _ => &[],
        }
    }
}

fn depth_label(d: ColorDepth) -> &'static str {
    match d {
        ColorDepth::Mono => "Monochrome",
        ColorDepth::Colors16 => "16 Colors",
        ColorDepth::Colors256 => "256 Colors",
        ColorDepth::Bit24 => "24-bit Color",
        ColorDepth::Grey8 => "8-bit Gray Scale",
    }
}

fn compression_label(c: TiffCompression) -> &'static str {
    match c {
        TiffCompression::None => "None",
        TiffCompression::PackBits => "PackBits",
        TiffCompression::Lzw => "LZW",
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

/// Which graphs are exported.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Which {
    ActivePage,
    AllGraphs,
}

/// What to do when the file exists.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Overwrite {
    Ask,
    /// Add a number to the name until it is free.
    Rename,
    Replace,
    Skip,
}

fn which_label(w: Which) -> &'static str {
    match w {
        Which::ActivePage => "Active Page",
        Which::AllGraphs => "All Graphs in Project",
    }
}

fn overwrite_label(o: Overwrite) -> &'static str {
    match o {
        Overwrite::Ask => "Ask",
        Overwrite::Rename => "Rename (Auto Enumerate)",
        Overwrite::Replace => "Replace",
        Overwrite::Skip => "Skip",
    }
}

/// The file name for graph number `index` (from 1): `<long name>`, `<short name>` (also `<name>`)
/// and `<index>` are replaced; an empty long name gives the short name; characters a file name
/// cannot hold become `_`. With `enabled` false the text is used as typed.
pub fn substitute(template: &str, short: &str, long: &str, index: usize, enabled: bool) -> String {
    let mut s = template.to_string();
    if enabled {
        let long = if long.is_empty() { short } else { long };
        s = s.replace("<long name>", long).replace("<short name>", short).replace("<name>", short);
        s = s.replace("<index>", &index.to_string());
    }
    let s: String = s.chars().map(|c| if r#"\/:*?"<>|"#.contains(c) { '_' } else { c }).collect();
    let s = s.trim().trim_end_matches('.').to_string();
    if s.is_empty() { short.to_string() } else { s }
}

pub struct ExportDialog {
    pub graph: WindowId,
    format: ExportFormat,
    which: Which,
    /// The file name, with substitution tokens.
    file_name: String,
    substitution: bool,
    /// The folder the files go in.
    path: String,
    overwrite: Overwrite,
    dpi: f64,
    depth: ColorDepth,
    tiff_compression: TiffCompression,
    jpeg_quality: u8,
    /// When set, the picture is scaled to this width (in `width_unit`), text and lines included;
    /// the height follows the proportions of the exported area.
    width: Option<f64>,
    width_unit: SizeUnit,
    /// Which part of the page is exported. Origin's default `Auto` is `Border`.
    margin: Margin,
    /// The border width of `Margin::Border` in pixels; `None` is the default.
    border_px: Option<f64>,
    auto_preview: bool,
    show_preview: bool,
    preview: Option<(egui::TextureHandle, String)>,
    /// Set when existing files need a yes: the files, and whether the window closes after writing.
    pending: Option<bool>,
}

/// What happened in one frame of the dialog.
pub enum ExportOutcome {
    Open,
    Closed,
    /// A file was written and the window closes.
    Saved(PathBuf),
    /// Apply: files were written (or skipped) and the window stays; the text is for the status bar.
    Applied(String),
    Failed(String),
}

impl ExportDialog {
    pub fn new(graph: WindowId, folder: Option<PathBuf>) -> Self {
        let folder = folder.or_else(|| std::env::current_dir().ok()).unwrap_or_default();
        ExportDialog {
            graph,
            format: ExportFormat::Png,
            which: Which::ActivePage,
            file_name: "<long name>".to_string(),
            substitution: true,
            path: folder.display().to_string(),
            overwrite: Overwrite::Ask,
            dpi: 300.0,
            depth: ColorDepth::Bit24,
            tiff_compression: TiffCompression::Lzw,
            jpeg_quality: 90,
            width: None,
            width_unit: SizeUnit::Inch,
            margin: Margin::Auto,
            border_px: None,
            auto_preview: true,
            show_preview: true,
            preview: None,
            pending: None,
        }
    }

    fn layout(&self) -> ExportLayout {
        ExportLayout { margin: self.margin, border_px: self.border_px }
    }

    fn raster_options(&self) -> RasterOptions {
        RasterOptions { depth: self.depth, tiff_compression: self.tiff_compression, jpeg_quality: self.jpeg_quality }
    }

    fn size(&self) -> ExportSize {
        self.width.map_or_else(ExportSize::default, |w| ExportSize::from_unit(Some(w), None, self.width_unit, self.dpi))
    }

    /// The file name (without extension) the graph with this index would get.
    pub fn name_for(&self, doc: &Document, graph: WindowId) -> String {
        let (short, long) = doc
            .project()
            .graph(graph)
            .map_or((String::new(), String::new()), |g| (g.name.clone(), g.long_name.clone()));
        let index = doc.project().graphs().position(|g| g.id == graph).map_or(1, |i| i + 1);
        substitute(&self.file_name, &short, &long, index, self.substitution)
    }

    #[cfg(test)]
    pub fn set_file_name(&mut self, name: &str) {
        self.file_name = name.to_string();
    }

    #[cfg(test)]
    pub fn set_path(&mut self, path: &Path) {
        self.path = path.display().to_string();
    }

    /// Whether Apply and OK can write: there is a file name and a folder.
    pub fn can_apply(&self) -> bool {
        !self.file_name.trim().is_empty() && !self.path.trim().is_empty()
    }

    fn graph_ids(&self, doc: &Document) -> Vec<WindowId> {
        match self.which {
            Which::ActivePage => vec![self.graph],
            Which::AllGraphs => doc.project().graphs().map(|g| g.id).collect(),
        }
    }

    /// The files to write: the overwrite rule applied. The flag says an existing file needs a yes.
    fn plan(&self, doc: &Document) -> (Vec<(WindowId, PathBuf)>, bool) {
        let ext = self.format.extension();
        let mut jobs: Vec<(WindowId, PathBuf)> = Vec::new();
        let mut asks = false;
        for id in self.graph_ids(doc) {
            let name = self.name_for(doc, id);
            let mut file = Path::new(self.path.trim()).join(format!("{name}.{ext}"));
            let taken = |f: &Path, jobs: &[(WindowId, PathBuf)]| jobs.iter().any(|(_, j)| j == f);
            if taken(&file, &jobs) || (self.overwrite == Overwrite::Rename && file.exists()) {
                let mut n = 1;
                loop {
                    file = Path::new(self.path.trim()).join(format!("{name}_{n}.{ext}"));
                    if !file.exists() && !taken(&file, &jobs) {
                        break;
                    }
                    n += 1;
                }
            } else if file.exists() {
                match self.overwrite {
                    Overwrite::Skip => continue,
                    Overwrite::Ask => asks = true,
                    _ => {}
                }
            }
            jobs.push((id, file));
        }
        (jobs, asks)
    }

    /// Writes the files. With `close` the window closes afterwards (OK); otherwise it stays (Apply).
    fn run(&mut self, doc: &Document, close: bool, confirmed: bool) -> ExportOutcome {
        let (jobs, asks) = self.plan(doc);
        if asks && !confirmed {
            self.pending = Some(close);
            return ExportOutcome::Open;
        }
        self.pending = None;
        if jobs.is_empty() {
            return if close { ExportOutcome::Closed } else { ExportOutcome::Applied("Nothing was exported".into()) };
        }
        if let Err(e) = std::fs::create_dir_all(self.path.trim()) {
            return ExportOutcome::Failed(format!("cannot use the folder '{}': {e}", self.path.trim()));
        }
        for (id, file) in &jobs {
            let r = sheaf_plot::export_file_opts(
                doc.project(),
                *id,
                file,
                self.dpi,
                self.size(),
                self.layout(),
                &self.raster_options(),
            );
            if let Err(e) = r {
                return ExportOutcome::Failed(e.to_string());
            }
        }
        if close {
            ExportOutcome::Saved(jobs[jobs.len() - 1].1.clone())
        } else if jobs.len() == 1 {
            ExportOutcome::Applied(format!("Wrote {}", jobs[0].1.display()))
        } else {
            ExportOutcome::Applied(format!("Wrote {} files to {}", jobs.len(), self.path.trim()))
        }
    }

    fn settings_key(&self) -> String {
        format!(
            "{:?}{:?}{:?}{:?}{:?}{:?}{:?}",
            self.margin, self.border_px, self.width, self.width_unit, self.dpi, self.format, self.graph
        )
    }

    fn make_preview(&mut self, ctx: &egui::Context, doc: &Document) {
        let layout = self.layout();
        let Ok(area) = sheaf_plot::export_area(doc.project(), self.graph, layout, None) else {
            self.preview = None;
            return;
        };
        let dpi = 72.0 * 320.0 / area.w.max(area.h).max(1.0);
        match sheaf_plot::export_raster(doc.project(), self.graph, dpi, ExportSize::default(), layout) {
            Ok(r) => {
                let img = egui::ColorImage::from_rgba_unmultiplied([r.width as usize, r.height as usize], &r.rgba);
                let tex = ctx.load_texture("export_preview", img, egui::TextureOptions::LINEAR);
                self.preview = Some((tex, self.settings_key()));
            }
            Err(_) => self.preview = None,
        }
    }

    pub fn show(&mut self, ctx: &egui::Context, doc: &Document) -> ExportOutcome {
        let Some(graph) = doc.project().graph(self.graph) else {
            return ExportOutcome::Closed;
        };
        // The depth must be one the format can store.
        if self.format.is_raster() && !self.format.depths().contains(&self.depth) {
            let d = self.format.depths();
            // Not offered by this format: 24-bit, else 256 colours (GIF), else the first depth.
            self.depth = [ColorDepth::Bit24, ColorDepth::Colors256].into_iter().find(|c| d.contains(c)).unwrap_or(d[0]);
        }
        if !self.format.is_raster() && self.width_unit == SizeUnit::Pixel {
            self.width_unit = SizeUnit::Inch;
            self.width = None;
        }
        let mut open = true;
        let mut outcome = ExportOutcome::Open;
        let mut want_preview = false;
        egui::Window::new("Export Graphs")
            .id(egui::Id::new("export_dialog"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                let raster_dpi = self.format.is_raster().then_some(self.dpi);
                let area = sheaf_plot::export_area(doc.project(), self.graph, self.layout(), raster_dpi).ok();
                let (page_w, page_h) = (graph.page.width_pt / 72.0, graph.page.height_pt / 72.0);
                let (w_in, h_in) = area.map_or((page_w, page_h), |a| (a.w / 72.0, a.h / 72.0));
                ui.horizontal_top(|ui| {
                    ui.vertical(|ui| {
                        ui.set_width(430.0);
                        egui::Grid::new("export_grid").num_columns(2).spacing([14.0, 6.0]).show(ui, |ui| {
                            ui.label("Image Type");
                            egui::ComboBox::from_id_salt("export_type")
                                .selected_text(self.format.label())
                                .width(280.0)
                                .show_ui(ui, |ui| {
                                    for f in ExportFormat::ALL {
                                        ui.selectable_value(&mut self.format, f, f.label());
                                    }
                                });
                            ui.end_row();
                            ui.label("Select Graph(s)");
                            egui::ComboBox::from_id_salt("export_which")
                                .selected_text(which_label(self.which))
                                .width(280.0)
                                .show_ui(ui, |ui| {
                                    for w in [Which::ActivePage, Which::AllGraphs] {
                                        ui.selectable_value(&mut self.which, w, which_label(w));
                                    }
                                });
                            ui.end_row();
                            ui.label("File Name(s)");
                            ui.add(egui::TextEdit::singleline(&mut self.file_name).desired_width(280.0));
                            ui.end_row();
                            ui.label("");
                            ui.checkbox(&mut self.substitution, "Enable Substitution").on_hover_text(
                                "Replace <long name>, <short name> and <index> in the file name; \
                                 an empty long name gives the short name",
                            );
                            ui.end_row();
                            ui.label("");
                            ui.label(format!(
                                "File will be saved as {}.{}",
                                self.name_for(doc, self.graph),
                                self.format.extension()
                            ));
                            ui.end_row();
                            ui.label("Path");
                            ui.horizontal(|ui| {
                                ui.add(egui::TextEdit::singleline(&mut self.path).desired_width(250.0));
                                if ui.button("...").on_hover_text("Choose the folder").clicked()
                                    && let Some(dir) = rfd::FileDialog::new().set_title("Export folder").pick_folder()
                                {
                                    self.path = dir.display().to_string();
                                }
                            });
                            ui.end_row();
                            ui.label("Overwrite Existing");
                            egui::ComboBox::from_id_salt("export_overwrite")
                                .selected_text(overwrite_label(self.overwrite))
                                .width(280.0)
                                .show_ui(ui, |ui| {
                                    for o in [Overwrite::Ask, Overwrite::Rename, Overwrite::Replace, Overwrite::Skip] {
                                        ui.selectable_value(&mut self.overwrite, o, overwrite_label(o));
                                    }
                                });
                            ui.end_row();
                            ui.label("Graph Theme");
                            ui.add_enabled(false, egui::TextEdit::singleline(&mut "<Original>".to_string()));
                            ui.end_row();
                        });
                        egui::CollapsingHeader::new("Export Settings").default_open(true).show(ui, |ui| {
                            egui::Grid::new("export_settings").num_columns(2).spacing([14.0, 6.0]).show(ui, |ui| {
                                ui.label("Margin");
                                ui.horizontal(|ui| {
                                    egui::ComboBox::from_id_salt("export_margin")
                                        .selected_text(margin_label(self.margin))
                                        .show_ui(ui, |ui| {
                                            for m in [
                                                Margin::Auto,
                                                Margin::Border,
                                                Margin::Tight,
                                                Margin::Page,
                                                Margin::TightInPage,
                                            ] {
                                                ui.selectable_value(&mut self.margin, m, margin_label(m));
                                            }
                                        })
                                        .response
                                        .on_hover_text(
                                            "Border: everything drawn plus a border. Tight: everything drawn. \
                                             Page: the whole page. Tight in page: the objects that are over \
                                             half inside the page.",
                                        );
                                    if self.margin.resolved() == Margin::Border {
                                        let mut px = self.border_px.unwrap_or(sheaf_plot::DEFAULT_BORDER_PT);
                                        if ui
                                            .add(
                                                DragValue::new(&mut px)
                                                    .range(0.0..=500.0)
                                                    .speed(0.5)
                                                    .max_decimals(1)
                                                    .suffix(" px"),
                                            )
                                            .on_hover_text(
                                                "Border width (pixels of the picture; points for vector formats)",
                                            )
                                            .changed()
                                        {
                                            self.border_px = Some(px);
                                        }
                                    }
                                });
                                ui.end_row();
                            });
                        });
                        egui::CollapsingHeader::new("Image Size").default_open(true).show(ui, |ui| {
                            egui::Grid::new("export_size").num_columns(2).spacing([14.0, 6.0]).show(ui, |ui| {
                                ui.label("Page size");
                                ui.label(format!("{page_w:.2} x {page_h:.2} in"));
                                ui.end_row();
                                ui.label("Width");
                                ui.horizontal(|ui| {
                                    let mut set = self.width.is_some();
                                    if ui
                                        .checkbox(&mut set, "Scale to")
                                        .on_hover_text(
                                            "Scale the whole picture, text and lines included, to this width",
                                        )
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
                                                ui.selectable_value(
                                                    &mut self.width_unit,
                                                    SizeUnit::PageRatio,
                                                    "% of page",
                                                );
                                            });
                                        if self.width_unit != before {
                                            let inches = before.to_inches(*w, self.dpi, w_in);
                                            *w = self.width_unit.from_inches(inches, self.dpi, w_in);
                                        }
                                    }
                                });
                                ui.end_row();
                                let scale =
                                    self.width.map_or(1.0, |w| self.width_unit.to_inches(w, self.dpi, w_in) / w_in);
                                let (w_in, h_in) = (w_in * scale, h_in * scale);
                                ui.label("Picture size");
                                ui.label(format!("{w_in:.2} x {h_in:.2} in"));
                                ui.end_row();
                                if self.format.is_raster() {
                                    ui.label("Image size");
                                    ui.label(format!(
                                        "{} x {} pixels",
                                        (w_in * self.dpi).round() as u64,
                                        (h_in * self.dpi).round() as u64
                                    ));
                                    ui.end_row();
                                }
                            });
                        });
                        egui::CollapsingHeader::new("Image Settings").default_open(true).show(ui, |ui| {
                            egui::Grid::new("export_image").num_columns(2).spacing([14.0, 6.0]).show(ui, |ui| {
                                if self.format.is_raster() {
                                    ui.label("Resolution");
                                    ui.horizontal(|ui| {
                                        ui.add(
                                            DragValue::new(&mut self.dpi)
                                                .range(36.0..=2400.0)
                                                .speed(5.0)
                                                .suffix(" dpi"),
                                        );
                                        for preset in [150.0, 300.0, 600.0, 1200.0] {
                                            if ui.small_button(format!("{preset}")).clicked() {
                                                self.dpi = preset;
                                            }
                                        }
                                    });
                                    ui.end_row();
                                    ui.label("Color Depth");
                                    ui.add_enabled_ui(self.format.depths().len() > 1, |ui| {
                                        egui::ComboBox::from_id_salt("export_depth")
                                            .selected_text(depth_label(self.depth))
                                            .show_ui(ui, |ui| {
                                                for d in self.format.depths() {
                                                    ui.selectable_value(&mut self.depth, *d, depth_label(*d));
                                                }
                                            });
                                    });
                                    ui.end_row();
                                }
                                if self.format == ExportFormat::Tiff {
                                    ui.label("Compression");
                                    egui::ComboBox::from_id_salt("export_tiff_compression")
                                        .selected_text(compression_label(self.tiff_compression))
                                        .show_ui(ui, |ui| {
                                            for c in
                                                [TiffCompression::None, TiffCompression::PackBits, TiffCompression::Lzw]
                                            {
                                                ui.selectable_value(
                                                    &mut self.tiff_compression,
                                                    c,
                                                    compression_label(c),
                                                );
                                            }
                                        });
                                    ui.end_row();
                                }
                                if self.format == ExportFormat::Jpeg {
                                    ui.label("Quality");
                                    ui.add(DragValue::new(&mut self.jpeg_quality).range(1..=100).suffix(" %"))
                                        .on_hover_text("100 is the best picture and the largest file");
                                    ui.end_row();
                                }
                                if !self.format.is_raster() {
                                    ui.label("");
                                    ui.label("A vector format has no resolution or colour depth.");
                                    ui.end_row();
                                }
                            });
                            if self.format == ExportFormat::Psd {
                                ui.small("Exports a flattened image, without editable graph layers.");
                            }
                        });
                    });
                    if self.show_preview {
                        ui.add_space(8.0);
                        ui.vertical(|ui| {
                            ui.set_width(330.0);
                            ui.label("Image");
                            let (rect, _) = ui.allocate_exact_size(egui::vec2(330.0, 330.0), egui::Sense::hover());
                            ui.painter().rect_filled(rect, 0.0, ui.visuals().extreme_bg_color);
                            match &self.preview {
                                Some((tex, _)) => {
                                    let sz = tex.size_vec2();
                                    let k = (rect.width() / sz.x).min(rect.height() / sz.y);
                                    let r = egui::Rect::from_center_size(rect.center(), sz * k);
                                    ui.painter().image(
                                        tex.id(),
                                        r,
                                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                                        egui::Color32::WHITE,
                                    );
                                }
                                None => {
                                    ui.painter().text(
                                        rect.center(),
                                        egui::Align2::CENTER_CENTER,
                                        "No Preview",
                                        egui::FontId::proportional(14.0),
                                        ui.visuals().weak_text_color(),
                                    );
                                }
                            }
                        });
                    }
                });
                if let Some(close) = self.pending {
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label("A file with this name exists. Replace it?");
                        if ui.button("Replace").clicked() {
                            outcome = self.run(doc, close, true);
                        }
                        if ui.button("Don't replace").clicked() {
                            self.pending = None;
                        }
                    });
                }
                ui.separator();
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.auto_preview, "Auto Preview");
                    if ui.button("Preview").clicked() {
                        want_preview = true;
                    }
                    if ui
                        .button(if self.show_preview { "<<" } else { ">>" })
                        .on_hover_text("Hide or show the preview")
                        .clicked()
                    {
                        self.show_preview = !self.show_preview;
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let can = self.can_apply();
                        if ui.add_enabled(can, egui::Button::new("Apply")).clicked() {
                            outcome = self.run(doc, false, false);
                        }
                        if ui.button("Cancel").clicked() {
                            outcome = ExportOutcome::Closed;
                        }
                        if ui.add_enabled(can, egui::Button::new("OK")).clicked() {
                            outcome = self.run(doc, true, false);
                        }
                    });
                });
            });
        if self.show_preview {
            let stale = self.preview.as_ref().is_none_or(|(_, k)| *k != self.settings_key());
            if want_preview || (self.auto_preview && stale) {
                self.make_preview(ctx, doc);
            }
        }
        if !open {
            return ExportOutcome::Closed;
        }
        outcome
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn substitution_replaces_tokens_and_falls_back_to_the_short_name() {
        assert_eq!(substitute("<long name>", "Graph1", "CV scan", 1, true), "CV scan");
        assert_eq!(substitute("<long name>", "Graph1", "", 1, true), "Graph1");
        assert_eq!(substitute("fig_<short name>_<index>", "Graph2", "x", 2, true), "fig_Graph2_2");
        assert_eq!(substitute("<name>-a", "Graph3", "x", 3, true), "Graph3-a");
        // Off: used as typed (characters a file cannot hold still go).
        assert_eq!(substitute("<long name>", "Graph1", "CV", 1, false), "_long name_");
        assert_eq!(substitute("a/b:c", "G", "", 1, true), "a_b_c");
        assert_eq!(substitute("  ", "G", "", 1, true), "G");
    }

    #[test]
    fn every_format_has_a_depth_it_can_store() {
        for f in ExportFormat::ALL {
            assert_eq!(f.is_raster(), !f.depths().is_empty(), "{f:?}");
        }
    }
}
