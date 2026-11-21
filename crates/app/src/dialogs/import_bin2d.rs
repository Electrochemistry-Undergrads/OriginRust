//! File > Import > Binary 2D Array... (T199, matrices spec 7f, measured by T294): the options window for a
//! chosen file, with the options and defaults of Origin 2016's `impBin2D` window. OK reads the file into
//! the active matrix sheet, as Origin's Output does, keeping its ranges and names; with no matrix window
//! active, into a new matrix book named after the file (Sheaf's choice: Origin writes into the active
//! worksheet). One undo step.

use eframe::egui;
use sheaf_core::matrix::MatrixSheet;
use sheaf_core::{Document, WindowId};
use sheaf_io::bin2d::{BYTE_LENGTHS, Bin2dOptions, Sign, read_bin2d};
use std::path::{Path, PathBuf};

pub enum Outcome {
    Open,
    Closed,
    /// Imported; `true` when the file ended inside the last row, which is zeros.
    Done(bool),
}

pub struct ImportBin2dDialog {
    pub path: PathBuf,
    len: usize,
    pub options: Bin2dOptions,
    /// The matrix book and sheet to import into; `None` for a new book.
    pub target: Option<(WindowId, usize)>,
    problem: Option<String>,
}

/// What [`import_bin2d`] did.
#[derive(Debug, PartialEq, Eq)]
pub struct Imported {
    pub book: WindowId,
    /// The file ended inside the last row, which is zeros.
    pub partial: bool,
}

/// Reads `path` with `options` into the sheet `target` (keeping its ranges and names) or, with `None`, a
/// new matrix book named after the file; one undo step.
pub fn import_bin2d(
    doc: &mut Document,
    path: &Path,
    options: &Bin2dOptions,
    target: Option<(WindowId, usize)>,
) -> Result<Imported, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let a = read_bin2d(&bytes, options)?;
    if a.rows * a.cols > sheaf_core::matrix::MAX_CELLS {
        return Err(format!(
            "{} x {} numbers are more than a matrix holds ({} cells)",
            a.rows,
            a.cols,
            sheaf_core::matrix::MAX_CELLS
        ));
    }
    let name = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let book = doc.edit("Import Binary 2D Array", |p| {
        if let Some((book, sheet)) = target {
            let s = p.matrix_sheet_mut(book, sheet).map_err(|e| e.to_string())?;
            s.rows = a.rows;
            s.cols = a.cols;
            *s.values_mut() = a.values.clone();
            return Ok::<_, String>(book);
        }
        let mut sheet = MatrixSheet::new("MSheet1", 0, a.rows, a.cols);
        sheet.values_mut().copy_from_slice(&a.values);
        Ok(p.adopt_matrix_book(&name, vec![sheet]))
    })?;
    Ok(Imported { book, partial: a.partial })
}

impl ImportBin2dDialog {
    /// The window for `path` with Origin's defaults, importing into `target`.
    pub fn new(path: PathBuf, target: Option<(WindowId, usize)>) -> Result<Self, String> {
        let len = std::fs::metadata(&path).map_err(|e| format!("{}: {e}", path.display()))?.len() as usize;
        Ok(ImportBin2dDialog { path, len, options: Bin2dOptions::default(), target, problem: None })
    }

    pub fn show(&mut self, ctx: &egui::Context, doc: &mut Document) -> Outcome {
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            return Outcome::Closed;
        }
        let mut open = true;
        let mut result = Outcome::Open;
        let mut ok = false;
        let output = match self.target {
            Some((book, sheet)) => doc
                .project()
                .matrix_book(book)
                .and_then(|b| b.sheets.get(sheet).map(|s| format!("[{}]{}", b.name, s.name)))
                .unwrap_or_default(),
            None => "a new matrix book".to_string(),
        };
        egui::Window::new("Binary 2D Array")
            .id(egui::Id::new("import_bin2d"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                egui::Grid::new("import_bin2d_grid").num_columns(2).show(ui, |ui| {
                    ui.label("File Name");
                    ui.label(format!("{} ({} bytes)", self.path.display(), self.len));
                    ui.end_row();
                    ui.label("Number of Columns");
                    ui.add(egui::DragValue::new(&mut self.options.cols).range(1..=1_000_000));
                    ui.end_row();
                    ui.label("Byte Length");
                    egui::ComboBox::from_id_salt("import_bin2d_bytes")
                        .selected_text(self.options.bytes.to_string())
                        .show_ui(ui, |ui| {
                            for n in BYTE_LENGTHS {
                                ui.selectable_value(&mut self.options.bytes, n, n.to_string());
                            }
                        });
                    ui.end_row();
                    ui.label("Signed/Unsigned");
                    egui::ComboBox::from_id_salt("import_bin2d_sign").selected_text(self.options.sign.name()).show_ui(
                        ui,
                        |ui| {
                            for s in Sign::ALL {
                                ui.selectable_value(&mut self.options.sign, s, s.name());
                            }
                        },
                    );
                    ui.end_row();
                    ui.label("Big-Endian");
                    ui.checkbox(&mut self.options.big_endian, "");
                    ui.end_row();
                    ui.label("Header Bytes");
                    ui.add(egui::DragValue::new(&mut self.options.header).range(0..=self.len));
                    ui.end_row();
                    ui.label("Transpose after import");
                    ui.checkbox(&mut self.options.transpose, "");
                    ui.end_row();
                    ui.label("Output");
                    ui.label(&output);
                    ui.end_row();
                });
                // Sheaf's addition: the size the options give.
                match (self.options.check_type(), self.options.rows_for(self.len)) {
                    (Ok(()), Some(rows)) => ui.weak(format!("{rows} rows of {} numbers", self.options.cols)),
                    (Err(e), _) => ui.weak(e),
                    (_, None) => ui.weak("nothing to read after the header"),
                };
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
            match import_bin2d(doc, &self.path, &self.options, self.target) {
                Ok(done) => return Outcome::Done(done.partial),
                Err(e) => self.problem = Some(e),
            }
        }
        result
    }
}
