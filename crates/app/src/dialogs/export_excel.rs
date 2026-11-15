//! The export window of File > Export > Worksheet as Excel: which sheets of the book go into the
//! `.xlsx` file and which label rows are written above the data. Sheaf's own choices; Origin's
//! Excel export is not measured (`docs/spec/import-excel.md`).

use super::export::ExportOutcome;
use eframe::egui;
use sheaf_core::{Document, WindowId, Worksheet};
use sheaf_io::excel::{ExcelExportOptions, write_xlsx};
use std::path::{Path, PathBuf};

pub struct ExcelExportDialog {
    pub book: WindowId,
    /// Which sheets of the book are written; one entry per sheet.
    selected: Vec<bool>,
    opts: ExcelExportOptions,
    folder: Option<PathBuf>,
    /// Under test the file chooser is not opened; this is the file it would have returned.
    #[cfg(test)]
    pub chosen: Option<PathBuf>,
}

impl ExcelExportDialog {
    /// A dialog with every sheet of the book ticked.
    pub fn new(book: WindowId, sheets: usize, folder: Option<PathBuf>) -> Self {
        ExcelExportDialog {
            book,
            selected: vec![true; sheets],
            opts: ExcelExportOptions::default(),
            folder,
            #[cfg(test)]
            chosen: None,
        }
    }

    /// Writes the ticked sheets to `path`.
    fn save(&self, sheets: &[Worksheet], path: &Path) -> ExportOutcome {
        let chosen: Vec<&Worksheet> =
            sheets.iter().zip(&self.selected).filter(|(_, on)| **on).map(|(s, _)| s).collect();
        match write_xlsx(&chosen, path, &self.opts) {
            Ok(done) => ExportOutcome::SavedWithNotes(done.path, done.warnings),
            Err(e) => ExportOutcome::Failed(e.to_string()),
        }
    }

    /// The file to write, from the chooser.
    fn choose(&self, stem: &str) -> Option<PathBuf> {
        #[cfg(test)]
        if self.chosen.is_some() {
            return self.chosen.clone();
        }
        let mut chooser = rfd::FileDialog::new()
            .set_title("Export as Excel")
            .add_filter("Excel workbook", &["xlsx"])
            .set_file_name(format!("{stem}.xlsx"));
        if let Some(dir) = &self.folder {
            chooser = chooser.set_directory(dir);
        }
        chooser.save_file().map(|mut path| {
            if path.extension().is_none() {
                path.set_extension("xlsx");
            }
            path
        })
    }

    pub fn show(&mut self, ctx: &egui::Context, doc: &Document) -> ExportOutcome {
        let Some(book) = doc.project().workbook(self.book) else {
            return ExportOutcome::Closed;
        };
        self.selected.resize(book.sheets.len(), true);
        let mut open = true;
        let mut outcome = ExportOutcome::Open;
        egui::Window::new(format!("Export {} as Excel", book.name))
            .id(egui::Id::new("export_excel_dialog"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                egui::Grid::new("export_excel_grid").num_columns(2).spacing([14.0, 6.0]).show(ui, |ui| {
                    ui.label("Sheets");
                    ui.vertical(|ui| {
                        for (sheet, on) in book.sheets.iter().zip(&mut self.selected) {
                            ui.checkbox(on, &sheet.name);
                        }
                    });
                    ui.end_row();
                    ui.label("Label rows");
                    ui.vertical(|ui| {
                        ui.checkbox(&mut self.opts.short_names, "Short name");
                        ui.checkbox(&mut self.opts.long_names, "Long name");
                        ui.checkbox(&mut self.opts.units, "Units");
                        ui.checkbox(&mut self.opts.comments, "Comments");
                    });
                    ui.end_row();
                });
                ui.weak("A label row is written only when a column has that label.");
                ui.add_space(6.0);
                let any = self.selected.iter().any(|on| *on);
                ui.horizontal(|ui| {
                    if ui.add_enabled(any, egui::Button::new("Save as...")).clicked() {
                        let stem = if book.long_name.is_empty() { &book.name } else { &book.long_name };
                        if let Some(path) = self.choose(stem) {
                            outcome = self.save(&book.sheets, &path);
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
