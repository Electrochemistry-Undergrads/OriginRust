//! The export window of a worksheet: file type and separator, which label rows to write, row
//! labels, full precision and the text of a missing value, after Origin's Export ASCII (`expASC`).

use super::export::ExportOutcome;
use eframe::egui;
use sheaf_core::{Document, WindowId};
use sheaf_io::export::{AsciiExportOptions, Separator, write_delimited};
use std::path::PathBuf;

/// The file types of the "File type" list: Origin's csv (comma or semicolon) and its text type
/// with each separator.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FileType {
    CsvComma,
    CsvSemicolon,
    Text(Separator),
}

impl FileType {
    const ALL: [FileType; 7] = [
        FileType::CsvComma,
        FileType::CsvSemicolon,
        FileType::Text(Separator::Tab),
        FileType::Text(Separator::Comma),
        FileType::Text(Separator::Space),
        FileType::Text(Separator::CommaTab),
        FileType::Text(Separator::CommaSpace),
    ];

    fn label(self) -> &'static str {
        match self {
            FileType::CsvComma => "CSV, comma",
            FileType::CsvSemicolon => "CSV, semicolon",
            FileType::Text(Separator::Tab) => "Text, tab",
            FileType::Text(Separator::Comma) => "Text, comma",
            FileType::Text(Separator::Space) => "Text, space",
            FileType::Text(Separator::CommaTab) => "Text, comma and tab",
            FileType::Text(Separator::CommaSpace) => "Text, comma and space",
            FileType::Text(Separator::Semicolon) => "Text, semicolon",
        }
    }

    fn extension(self) -> &'static str {
        match self {
            FileType::CsvComma | FileType::CsvSemicolon => "csv",
            FileType::Text(_) => "txt",
        }
    }
}

pub struct SheetExportDialog {
    pub book: WindowId,
    file_type: FileType,
    opts: AsciiExportOptions,
    folder: Option<PathBuf>,
}

impl SheetExportDialog {
    pub fn new(book: WindowId, folder: Option<PathBuf>) -> Self {
        SheetExportDialog { book, file_type: FileType::CsvComma, opts: AsciiExportOptions::default(), folder }
    }

    /// The options with the file type applied.
    fn options(&self) -> AsciiExportOptions {
        let (csv, separator) = match self.file_type {
            FileType::CsvComma => (true, Separator::Comma),
            FileType::CsvSemicolon => (true, Separator::Semicolon),
            FileType::Text(s) => (false, s),
        };
        AsciiExportOptions { csv, separator, ..self.opts.clone() }
    }

    pub fn show(&mut self, ctx: &egui::Context, doc: &Document) -> ExportOutcome {
        let Some(book) = doc.project().workbook(self.book) else {
            return ExportOutcome::Closed;
        };
        let mut open = true;
        let mut outcome = ExportOutcome::Open;
        egui::Window::new(format!("Export {}", book.name))
            .id(egui::Id::new("export_sheet_dialog"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label(format!("Sheet: {}", book.sheet().name));
                egui::Grid::new("export_sheet_grid").num_columns(2).spacing([14.0, 6.0]).show(ui, |ui| {
                    ui.label("File type");
                    egui::ComboBox::from_id_salt("export_sheet_type").selected_text(self.file_type.label()).show_ui(
                        ui,
                        |ui| {
                            for t in FileType::ALL {
                                ui.selectable_value(&mut self.file_type, t, t.label());
                            }
                        },
                    );
                    ui.end_row();
                    ui.label("Label rows");
                    ui.vertical(|ui| {
                        ui.checkbox(&mut self.opts.short_names, "Short name");
                        ui.checkbox(&mut self.opts.long_names, "Long name");
                        ui.checkbox(&mut self.opts.units, "Units");
                        ui.checkbox(&mut self.opts.comments, "Comments");
                        ui.checkbox(&mut self.opts.user_params, "User parameters");
                    });
                    ui.end_row();
                    ui.label("Row labels");
                    ui.checkbox(&mut self.opts.row_labels, "Row numbers and the names of the label rows");
                    ui.end_row();
                    ui.label("Numbers");
                    ui.checkbox(&mut self.opts.full_precision, "Full precision (not the displayed digits)");
                    ui.end_row();
                    ui.label("Missing values");
                    ui.checkbox(&mut self.opts.missing_dashes, "Write -- (otherwise leave the field empty)");
                    ui.end_row();
                });
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.button("Save as...").clicked() {
                        let ext = self.file_type.extension();
                        let stem = if book.long_name.is_empty() { &book.name } else { &book.long_name };
                        let mut chooser = rfd::FileDialog::new()
                            .set_title("Export worksheet")
                            .add_filter("Comma-separated", &["csv"])
                            .add_filter("Text", &["txt", "dat", "tsv"])
                            .set_file_name(format!("{stem}.{ext}"));
                        if let Some(dir) = &self.folder {
                            chooser = chooser.set_directory(dir);
                        }
                        if let Some(mut path) = chooser.save_file() {
                            if path.extension().is_none() {
                                path.set_extension(ext);
                            }
                            let opts = self.options();
                            let sheet = book.sheet();
                            let result = std::fs::File::create(&path).map_err(|e| e.to_string()).and_then(|file| {
                                let mut out = std::io::BufWriter::new(file);
                                write_delimited(sheet, &mut out, &opts).map_err(|e| e.to_string())
                            });
                            outcome = match result {
                                Ok(()) => ExportOutcome::Saved(path),
                                Err(e) => ExportOutcome::Failed(e),
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
