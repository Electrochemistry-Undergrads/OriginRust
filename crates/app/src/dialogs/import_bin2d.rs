//! File > Import > Binary 2D Array... (T199, matrices spec 7f): the options window for a chosen file.
//! OK reads the file into a new matrix book, named after the file, as one undo step.

use eframe::egui;
use sheaf_core::matrix::MatrixSheet;
use sheaf_core::{Document, WindowId};
use sheaf_io::bin2d::{Bin2dOptions, BinType, read_bin2d};
use std::path::{Path, PathBuf};

pub enum Outcome {
    Open,
    Closed,
    Done,
}

pub struct ImportBin2dDialog {
    pub path: PathBuf,
    len: usize,
    pub options: Bin2dOptions,
    problem: Option<String>,
}

/// Reads `path` with `options` into a new matrix book (one undo step) and returns its id.
pub fn import_bin2d(doc: &mut Document, path: &Path, options: &Bin2dOptions) -> Result<WindowId, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let values = read_bin2d(&bytes, options)?;
    let name = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    doc.edit("Import Binary 2D Array", |p| {
        let mut sheet = MatrixSheet::new("MSheet1", 0, options.rows, options.cols);
        sheet.values_mut().copy_from_slice(&values);
        Ok::<_, String>(p.adopt_matrix_book(&name, vec![sheet]))
    })
}

impl ImportBin2dDialog {
    /// The window for `path`, with a guess from its size (32-bit floats, square if they make one).
    pub fn new(path: PathBuf) -> Result<Self, String> {
        let len = std::fs::metadata(&path).map_err(|e| format!("{}: {e}", path.display()))?.len() as usize;
        Ok(ImportBin2dDialog { path, len, options: Bin2dOptions::guess(len, BinType::F32), problem: None })
    }

    pub fn show(&mut self, ctx: &egui::Context, doc: &mut Document) -> Outcome {
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            return Outcome::Closed;
        }
        let mut open = true;
        let mut result = Outcome::Open;
        let mut ok = false;
        egui::Window::new("Import Binary 2D Array")
            .id(egui::Id::new("import_bin2d"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label(format!("{} ({} bytes)", self.path.display(), self.len));
                egui::Grid::new("import_bin2d_grid").num_columns(2).show(ui, |ui| {
                    ui.label("Data Type");
                    let before = self.options.kind;
                    egui::ComboBox::from_id_salt("import_bin2d_type").selected_text(self.options.kind.name()).show_ui(
                        ui,
                        |ui| {
                            for t in BinType::ALL {
                                ui.selectable_value(&mut self.options.kind, t, t.name());
                            }
                        },
                    );
                    if self.options.kind != before {
                        let g = Bin2dOptions::guess(self.len.saturating_sub(self.options.header), self.options.kind);
                        (self.options.cols, self.options.rows) = (g.cols, g.rows);
                    }
                    ui.end_row();
                    ui.label("Columns");
                    ui.add(egui::DragValue::new(&mut self.options.cols).range(1..=1_000_000));
                    ui.end_row();
                    ui.label("Rows");
                    ui.add(egui::DragValue::new(&mut self.options.rows).range(1..=1_000_000));
                    ui.end_row();
                    ui.label("Header Bytes");
                    ui.add(egui::DragValue::new(&mut self.options.header).range(0..=self.len));
                    ui.end_row();
                    ui.label("Byte Order");
                    ui.horizontal(|ui| {
                        ui.radio_value(&mut self.options.big_endian, false, "Little Endian");
                        ui.radio_value(&mut self.options.big_endian, true, "Big Endian");
                    });
                    ui.end_row();
                });
                if let Some(p) = &self.problem {
                    ui.colored_label(egui::Color32::DARK_RED, p);
                }
                ui.separator();
                ui.horizontal(|ui| {
                    ok = ui.button("OK").clicked();
                    if ui.button("Cancel").clicked() {
                        result = Outcome::Closed;
                    }
                });
            });
        if !open {
            return Outcome::Closed;
        }
        if ok {
            match import_bin2d(doc, &self.path, &self.options) {
                Ok(_) => return Outcome::Done,
                Err(e) => self.problem = Some(e),
            }
        }
        result
    }
}
