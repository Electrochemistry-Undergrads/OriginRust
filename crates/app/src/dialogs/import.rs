//! The import window: choose how files are read, see a preview of the
//! result, then import all chosen files with the same settings.

use eframe::egui::{self, Color32, DragValue, RichText};
use egui_extras::{Column as TableColumn, TableBuilder};
use sheaf_core::template::Template;
use sheaf_io::ascii::{AsciiOptions, Delimiter, Thousands};
use sheaf_io::batch::ImportMode;
use sheaf_io::chi::ChiOptions;
use sheaf_io::{ImportKind, Imported};
use std::io::Read;
use std::path::{Path, PathBuf};

/// How much of the start of a file is looked at to detect the separator and header lines.
const DETECT_BYTES: u64 = 256 * 1024;

/// Rows of the preview table.
const PREVIEW_ROWS: usize = 40;

pub struct ImportDialog {
    files: Vec<PathBuf>,
    /// The file shown in the preview.
    current: usize,
    kind: ImportKind,
    chi: ChiOptions,
    ascii: AsciiOptions,
    other_delimiter: String,
    /// Make a graph of every imported file.
    plot: bool,
    /// The graph template; `None` uses what the importer suggests for each file.
    template: Option<Template>,
    /// Where the data goes: a new book, or into the sheet that is active in the project.
    mode: ImportMode,
    /// Name of the active sheet (book and sheet), when there is one to put data into.
    target: Option<String>,
    preview: Option<Result<Imported, String>>,
    /// What detection found for the file and options it was computed for, so that it is not
    /// repeated every frame.
    detection: Option<DetectionCache>,
    /// How many times detection has run (for tests).
    detect_runs: usize,
    /// The node of the tree at the left that is shown.
    page: ImportPage,
}

/// The nodes of the tree at the left: "Import Options" and its groups.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ImportPage {
    Options,
    ImportMode,
    HeaderLines,
    ColumnDesignation,
    PartialImport,
    Miscellaneous,
    FileInfo,
}

impl ImportPage {
    const ALL: [ImportPage; 7] = [
        ImportPage::Options,
        ImportPage::ImportMode,
        ImportPage::HeaderLines,
        ImportPage::ColumnDesignation,
        ImportPage::PartialImport,
        ImportPage::Miscellaneous,
        ImportPage::FileInfo,
    ];

    fn label(self) -> &'static str {
        match self {
            ImportPage::Options => "Import Options",
            ImportPage::ImportMode => "Import Mode",
            ImportPage::HeaderLines => "Header Lines",
            ImportPage::ColumnDesignation => "Column Designation",
            ImportPage::PartialImport => "Partial Import",
            ImportPage::Miscellaneous => "Miscellaneous",
            ImportPage::FileInfo => "File Info and Parameters",
        }
    }
}

struct DetectionCache {
    file: PathBuf,
    options: AsciiOptions,
    detected: Option<sheaf_io::ascii::Detected>,
}

/// Runs detection on the first part of a file only, cut at a line end.
fn detect_in_prefix(path: &Path, options: &AsciiOptions) -> Option<sheaf_io::ascii::Detected> {
    let mut bytes = Vec::new();
    std::fs::File::open(path).ok()?.take(DETECT_BYTES).read_to_end(&mut bytes).ok()?;
    if bytes.len() as u64 == DETECT_BYTES {
        if bytes.starts_with(&[0xFF, 0xFE]) || bytes.starts_with(&[0xFE, 0xFF]) {
            bytes.truncate(bytes.len() & !1);
        } else if let Some(end) = bytes.iter().rposition(|b| *b == b'\n') {
            bytes.truncate(end + 1);
        } else if let Err(e) = std::str::from_utf8(&bytes)
            && e.error_len().is_none()
        {
            // One very long line: only drop a character that the cut left incomplete.
            bytes.truncate(e.valid_up_to());
        }
    }
    let text = sheaf_io::decode_text(&bytes, path).ok()?;
    Some(sheaf_io::ascii::detect(&text, options))
}

/// What the dialog wants the application to do after a frame.
pub enum ImportOutcome {
    /// Keep showing the dialog.
    Open,
    Cancelled,
    /// The user pressed Import. Files that could not be read are in `errors`.
    Done {
        items: Vec<Imported>,
        errors: Vec<String>,
        plot: bool,
        template: Option<Template>,
        mode: ImportMode,
    },
}

impl ImportDialog {
    pub fn new(files: Vec<PathBuf>) -> Self {
        ImportDialog {
            files,
            current: 0,
            kind: ImportKind::Auto,
            chi: ChiOptions::default(),
            ascii: AsciiOptions::default(),
            other_delimiter: "|".to_string(),
            plot: true,
            mode: ImportMode::NewBook,
            target: None,
            template: None,
            preview: None,
            detection: None,
            detect_runs: 0,
            page: ImportPage::Options,
        }
    }

    fn read(&self, index: usize) -> Result<Imported, String> {
        sheaf_io::import_file(&self.files[index], self.kind, &self.chi, &self.ascii).map_err(|e| e.to_string())
    }

    pub fn show(&mut self, ctx: &egui::Context, target: Option<&str>) -> ImportOutcome {
        self.target = target.map(str::to_string);
        if self.target.is_none() {
            self.mode = ImportMode::NewBook;
        }
        if self.files.is_empty() {
            return ImportOutcome::Cancelled;
        }
        self.current = self.current.min(self.files.len() - 1);
        if self.preview.is_none() {
            self.preview = Some(self.read(self.current));
        }
        let before = (self.current, self.kind, self.chi.clone(), self.ascii.clone());

        let mut open = true;
        let mut outcome = ImportOutcome::Open;
        egui::Window::new("Import ASCII: impASC")
            .id(egui::Id::new("import_dialog"))
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_size([760.0, 640.0])
            .show(ctx, |ui| {
                // Windows order at the bottom right: OK, Cancel.
                egui::TopBottomPanel::bottom("import_buttons").show_inside(ui, |ui| {
                    ui.add_space(6.0);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Cancel").clicked() {
                            outcome = ImportOutcome::Cancelled;
                        }
                        if ui.button("OK").clicked() {
                            outcome = self.run();
                        }
                        if self.files.len() > 1 {
                            ui.weak(format!("{} files are imported with these options", self.files.len()));
                        }
                    });
                });
                egui::CentralPanel::default().show_inside(ui, |ui| {
                    self.files_ui(ui);
                    ui.separator();
                    // The tree of option groups at the left, the controls of the chosen group at
                    // the right.
                    ui.allocate_ui_with_layout(
                        egui::vec2(ui.available_width(), 235.0),
                        egui::Layout::left_to_right(egui::Align::Min),
                        |ui| {
                            ui.vertical(|ui| {
                                ui.set_width(170.0);
                                self.tree_ui(ui);
                            });
                            ui.separator();
                            ui.vertical(|ui| {
                                egui::ScrollArea::vertical()
                                    .id_salt("import_page")
                                    .auto_shrink([false, false])
                                    .show(ui, |ui| self.page_ui(ui));
                            });
                        },
                    );
                    ui.separator();
                    self.preview_ui(ui);
                });
            });

        if before != (self.current, self.kind, self.chi.clone(), self.ascii.clone()) {
            self.preview = None;
            ctx.request_repaint();
        }
        if !open {
            outcome = ImportOutcome::Cancelled;
        }
        outcome
    }

    fn run(&self) -> ImportOutcome {
        let mut items = Vec::new();
        let mut errors = Vec::new();
        for i in 0..self.files.len() {
            match self.read(i) {
                Ok(imported) => items.push(imported),
                Err(e) => errors.push(e),
            }
        }
        ImportOutcome::Done { items, errors, plot: self.plot, template: self.template, mode: self.mode }
    }

    fn files_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.label("Files:");
            let mut remove = None;
            for (i, f) in self.files.iter().enumerate() {
                let name = f.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                let response = ui.selectable_label(i == self.current, name).on_hover_text(f.display().to_string());
                if response.clicked() {
                    self.current = i;
                }
                response.context_menu(|ui| {
                    if ui.button("Remove from the list").clicked() {
                        remove = Some(i);
                        ui.close();
                    }
                });
            }
            if let Some(i) = remove {
                self.files.remove(i);
                self.preview = None;
            }
            if ui.small_button("Add files...").clicked()
                && let Some(more) = rfd::FileDialog::new().set_title("Add data files").pick_files()
            {
                self.files.extend(more);
            }
        });
    }

    /// The detected settings of the current file, recomputed only when the file or the options
    /// have changed since the last call.
    fn detected(&mut self) -> Option<sheaf_io::ascii::Detected> {
        let file = self.files.get(self.current)?;
        let fresh = matches!(&self.detection, Some(c) if c.file == *file && c.options == self.ascii);
        if !fresh {
            self.detect_runs += 1;
            self.detection = Some(DetectionCache {
                file: file.clone(),
                options: self.ascii.clone(),
                detected: detect_in_prefix(file, &self.ascii),
            });
        }
        self.detection.as_ref().and_then(|c| c.detected.clone())
    }

    /// What the format drop-down and the detection say about the files: "chi", "ascii" or "".
    fn shown_format(&self) -> &'static str {
        match (&self.preview, self.kind) {
            (_, ImportKind::Chi) => "chi",
            (_, ImportKind::Ascii) => "ascii",
            (Some(Ok(p)), ImportKind::Auto) => p.format,
            _ => "",
        }
    }

    /// The tree at the left: "Import Options" with its groups below it.
    fn tree_ui(&mut self, ui: &mut egui::Ui) {
        for page in ImportPage::ALL {
            let indent = if page == ImportPage::Options { 0.0 } else { 16.0 };
            ui.horizontal(|ui| {
                ui.add_space(indent);
                let text = if page == ImportPage::Options {
                    RichText::new(page.label()).strong()
                } else {
                    RichText::new(page.label())
                };
                if ui.selectable_label(self.page == page, text).clicked() {
                    self.page = page;
                }
            });
        }
    }

    fn page_ui(&mut self, ui: &mut egui::Ui) {
        match self.page {
            ImportPage::Options => self.options_page(ui),
            ImportPage::ImportMode => self.import_mode_page(ui),
            ImportPage::FileInfo => self.file_info_page(ui),
            ImportPage::Miscellaneous => {
                self.ascii_page(ui);
                self.chi_group(ui);
            }
            _ => self.ascii_page(ui),
        }
    }

    /// Import Options: how to read the files, and the separator between the columns.
    fn options_page(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Read the files as");
            ui.selectable_value(&mut self.kind, ImportKind::Auto, "Detect");
            ui.selectable_value(&mut self.kind, ImportKind::Chi, "CH Instruments");
            ui.selectable_value(&mut self.kind, ImportKind::Ascii, "Delimited text");
        });
        ui.add_space(4.0);
        if self.shown_format() == "ascii" {
            self.ascii_page(ui);
        } else {
            ui.add_enabled_ui(false, |ui| ui.label("The separator applies to delimited text files."));
        }
    }

    /// Import Mode: where the data goes, and the graph made after the import.
    fn import_mode_page(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Import Mode");
            egui::ComboBox::from_id_salt("import_mode").selected_text(self.mode.label()).show_ui(ui, |ui| {
                for m in ImportMode::ALL {
                    let usable = m == ImportMode::NewBook || self.target.is_some();
                    ui.add_enabled_ui(usable, |ui| {
                        ui.selectable_value(&mut self.mode, m, m.label());
                    });
                }
            });
        });
        match (&self.target, self.mode) {
            (Some(t), m) if m != ImportMode::NewBook => {
                ui.weak(format!("Active sheet: {t}"));
            }
            (None, _) => {
                ui.weak("Open or select a workbook to add to its sheets.");
            }
            _ => {}
        }
        ui.add_space(6.0);
        ui.checkbox(&mut self.plot, "Make a graph of each file");
        ui.add_enabled_ui(self.plot, |ui| {
            ui.horizontal(|ui| {
                ui.label("Template");
                let suggested = match &self.preview {
                    Some(Ok(p)) => format!("Suggested ({})", p.suggested_template.label()),
                    _ => "Suggested".to_string(),
                };
                let text = self.template.map(|t| t.label().to_string()).unwrap_or_else(|| suggested.clone());
                egui::ComboBox::from_id_salt("import_template").selected_text(text).show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.template, None, suggested);
                    for t in Template::ALL {
                        ui.selectable_value(&mut self.template, Some(t), t.label());
                    }
                });
            });
        });
    }

    /// File Info and Parameters: the header entries the importer found in the current file.
    fn file_info_page(&mut self, ui: &mut egui::Ui) {
        match &self.preview {
            Some(Ok(p)) if !p.meta.is_empty() => {
                ui.weak(format!("{} entries in the header of the file shown in the preview", p.meta.len()));
                for (k, v) in &p.meta {
                    ui.horizontal(|ui| {
                        ui.weak(k);
                        ui.label(v);
                    });
                }
            }
            _ => {
                ui.weak("The file has no header entries.");
            }
        }
    }

    /// The CH Instruments options (Miscellaneous), when the files are read as CH Instruments.
    fn chi_group(&mut self, ui: &mut egui::Ui) {
        ui.add_space(6.0);
        ui.strong("CH Instruments");
        if self.shown_format() != "chi" {
            ui.add_enabled_ui(false, |ui| ui.label("These options apply to CH Instruments files."));
            return;
        }
        ui.checkbox(&mut self.chi.split_scans, "Split cyclic voltammetry into scans");
        ui.checkbox(
            &mut self.chi.split_segments,
            "Add a sheet with each charge and discharge segment (chronopotentiometry)",
        );
        ui.checkbox(&mut self.chi.add_milliamp, "Add the current in mA");
        ui.checkbox(&mut self.chi.add_charge, "Add the charge to current-vs-time files");
        ui.checkbox(&mut self.chi.invert_current, "Invert the sign of the current");
        ui.checkbox(&mut self.chi.exact_time, "Recompute rounded times from the sampling interval").on_hover_text(
            "The file prints times with 4 significant digits (whole seconds above 1000 s). \
             When every time fits the sampling interval in the file header, the exact times are used.",
        );
    }

    /// The groups of delimited-text options: the separator (Import Options), Header Lines,
    /// Column Designation, Partial Import and the number and date options (Miscellaneous).
    fn ascii_page(&mut self, ui: &mut egui::Ui) {
        let page = self.page;
        if self.shown_format() != "ascii" {
            if page != ImportPage::Miscellaneous {
                ui.add_enabled_ui(false, |ui| ui.label("These options apply to delimited text files."));
            }
            return;
        }
        let detected = if matches!(self.preview, Some(Ok(_))) { self.detected() } else { None };
        let o = &mut self.ascii;
        egui::Grid::new(("ascii_options", page as u8)).num_columns(2).spacing([12.0, 5.0]).show(ui, |ui| {
            if matches!(page, ImportPage::Options) {
                ui.label("Delimiter");
                ui.horizontal(|ui| {
                    let name = |d: Delimiter| match d {
                        Delimiter::Auto => "Detect",
                        Delimiter::Comma => "Comma",
                        Delimiter::Tab => "Tab",
                        Delimiter::Semicolon => "Semicolon",
                        Delimiter::Whitespace => "Spaces",
                        Delimiter::Other(_) => "Other",
                    };
                    egui::ComboBox::from_id_salt("ascii_delimiter").selected_text(name(o.delimiter)).show_ui(
                        ui,
                        |ui| {
                            for d in [
                                Delimiter::Auto,
                                Delimiter::Comma,
                                Delimiter::Tab,
                                Delimiter::Semicolon,
                                Delimiter::Whitespace,
                            ] {
                                ui.selectable_value(&mut o.delimiter, d, name(d));
                            }
                            let other = self.other_delimiter.chars().next().unwrap_or('|');
                            let is_other = matches!(o.delimiter, Delimiter::Other(_));
                            if ui.selectable_label(is_other, "Other").clicked() {
                                o.delimiter = Delimiter::Other(other);
                            }
                        },
                    );
                    if let Delimiter::Other(c) = &mut o.delimiter {
                        if ui.add(egui::TextEdit::singleline(&mut self.other_delimiter).desired_width(24.0)).changed()
                            && let Some(typed) = self.other_delimiter.chars().next()
                        {
                            *c = typed;
                        }
                    } else if let (Delimiter::Auto, Some(d)) = (o.delimiter, &detected) {
                        ui.weak(format!("found: {}", name(d.delimiter).to_lowercase()));
                    }
                });
                ui.end_row();
            }
            if page == ImportPage::HeaderLines {
                ui.label("Lines before the data");
                optional_line(ui, &mut o.header_lines, detected.as_ref().map(|d| d.header_lines), "none");
                ui.end_row();
                ui.label("Line with the names");
                optional_line(ui, &mut o.long_name_line, detected.as_ref().map(|d| d.long_name_line), "none");
                ui.end_row();
                ui.label("Line with the units");
                optional_line(ui, &mut o.units_line, detected.as_ref().map(|d| d.units_line), "none");
                ui.end_row();
                ui.label("Line with comments");
                optional_line(ui, &mut o.comments_line, detected.as_ref().map(|d| d.comments_line), "none");
                ui.end_row();
            }
            if page == ImportPage::ColumnDesignation {
                ui.label("Columns");
                ui.checkbox(&mut o.first_column_x, "The first column is X");
                ui.end_row();
            }
            if page == ImportPage::PartialImport {
                ui.label("Skip data rows");
                ui.add(DragValue::new(&mut o.skip_rows).range(0..=usize::MAX));
                ui.end_row();
            }
            if page == ImportPage::Miscellaneous {
                ui.label("Numbers");
                ui.checkbox(&mut o.decimal_comma, "Comma as decimal separator");
                ui.end_row();
                ui.label("Thousands separator");
                ui.horizontal(|ui| {
                    let name = |t: Thousands| match t {
                        Thousands::Auto => "Detect",
                        Thousands::Off => "None",
                        Thousands::Comma => "Comma (1,234.5)",
                        Thousands::Dot => "Dot (1.234,5)",
                        Thousands::Space => "Space (1 234,5)",
                        Thousands::Apostrophe => "Apostrophe (1'234.5)",
                    };
                    egui::ComboBox::from_id_salt("ascii_thousands").selected_text(name(o.thousands)).show_ui(
                        ui,
                        |ui| {
                            for t in [
                                Thousands::Auto,
                                Thousands::Off,
                                Thousands::Comma,
                                Thousands::Dot,
                                Thousands::Space,
                                Thousands::Apostrophe,
                            ] {
                                ui.selectable_value(&mut o.thousands, t, name(t));
                            }
                        },
                    );
                    if let (Thousands::Auto, Some(d)) = (o.thousands, &detected)
                        && d.thousands != Thousands::Off
                    {
                        ui.weak(format!("found: {}", name(d.thousands).to_lowercase()));
                    }
                });
                ui.end_row();
                ui.label("Dates and times");
                let mut as_text = o.dates == sheaf_io::ascii::DateImport::AsText;
                if ui.checkbox(&mut as_text, "Keep as text").changed() {
                    o.dates =
                        if as_text { sheaf_io::ascii::DateImport::AsText } else { sheaf_io::ascii::DateImport::Detect };
                }
                ui.end_row();
            }
        });
    }

    fn preview_ui(&mut self, ui: &mut egui::Ui) {
        let imported = match &self.preview {
            Some(Ok(p)) => p,
            Some(Err(e)) => {
                ui.colored_label(Color32::from_rgb(170, 30, 30), format!("This file cannot be read: {e}"));
                return;
            }
            None => return,
        };
        for w in &imported.warnings {
            ui.colored_label(Color32::from_rgb(170, 100, 0), format!("Note: {w}"));
        }
        let book = &imported.workbook;
        let Some(sheet) = book.sheets.first() else {
            ui.label("The file has no data.");
            return;
        };
        let more_sheets = if book.sheets.len() > 1 { format!(", {} sheets", book.sheets.len()) } else { String::new() };
        ui.weak(format!(
            "Preview of sheet '{}': {} columns, {} rows{more_sheets}",
            sheet.name,
            sheet.columns.len(),
            sheet.row_count()
        ));
        let rows = sheet.row_count().min(PREVIEW_ROWS);
        egui::ScrollArea::horizontal().id_salt("import_preview_scroll").auto_shrink([false, false]).show(ui, |ui| {
            let mut table = TableBuilder::new(ui).id_salt("import_preview").striped(true).min_scrolled_height(0.0);
            for _ in &sheet.columns {
                table = table.column(TableColumn::auto().at_least(70.0));
            }
            table
                .header(64.0, |mut header| {
                    for c in &sheet.columns {
                        header.col(|ui| {
                            ui.vertical(|ui| {
                                ui.spacing_mut().item_spacing.y = 1.0;
                                let tag = c.designation.tag();
                                let head = if tag.is_empty() {
                                    c.short_name.clone()
                                } else {
                                    format!("{}({tag})", c.short_name)
                                };
                                ui.label(RichText::new(head).strong());
                                ui.label(&c.long_name);
                                ui.label(&c.units);
                                ui.weak(c.comments.lines().next().unwrap_or_default());
                            });
                        });
                    }
                })
                .body(|body| {
                    body.rows(18.0, rows, |mut row| {
                        let r = row.index();
                        for c in &sheet.columns {
                            row.col(|ui| {
                                ui.label(if r < c.len() { c.cell_text(r) } else { String::new() });
                            });
                        }
                    });
                });
        });
    }
}

/// A line number that is either detected or typed; 0 means "there is none".
fn optional_line(ui: &mut egui::Ui, value: &mut Option<usize>, detected: Option<usize>, zero_text: &str) {
    ui.horizontal(|ui| {
        let mut auto = value.is_none();
        if ui.checkbox(&mut auto, "Detect").changed() {
            *value = if auto { None } else { Some(detected.unwrap_or(0)) };
        }
        match value {
            Some(v) => {
                ui.add(DragValue::new(v).range(0..=100_000));
            }
            None => {
                let shown = match detected {
                    Some(0) => zero_text.to_string(),
                    Some(n) => n.to_string(),
                    None => String::new(),
                };
                ui.weak(shown);
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detection_reads_a_bounded_prefix_once_per_file_and_options() {
        let dir = std::env::temp_dir().join(format!("sheaf-import-detect-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("big.csv");
        let mut text = String::from(
            "x,y
",
        );
        for i in 0..200_000 {
            text.push_str(&format!(
                "{i},{}
",
                i * 2
            ));
        }
        std::fs::write(&path, &text).unwrap();
        assert!(text.len() as u64 > 4 * DETECT_BYTES, "the file is much larger than the prefix");

        let mut d = ImportDialog::new(vec![path.clone()]);
        let found = d.detected().expect("detected");
        assert_eq!(found.delimiter, Delimiter::Comma);
        assert_eq!(found.columns, 2);
        for _ in 0..50 {
            d.detected();
        }
        assert_eq!(d.detect_runs, 1, "repeated frames reuse the cached result");
        d.ascii.delimiter = Delimiter::Tab;
        d.detected();
        assert_eq!(d.detect_runs, 2, "changed options recompute");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn detection_works_when_the_prefix_has_no_line_end() {
        let dir = std::env::temp_dir().join(format!("sheaf-import-oneline-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("oneline.csv");
        // Seven-byte pieces, so the cut at the prefix size lands inside the first character.
        let mut text = String::new();
        while text.len() < 2 * DETECT_BYTES as usize {
            text.push_str("\u{e9}1,2,3");
        }
        std::fs::write(&path, &text).unwrap();
        let mut bytes = Vec::new();
        std::fs::File::open(&path).unwrap().take(DETECT_BYTES).read_to_end(&mut bytes).unwrap();
        assert!(std::str::from_utf8(&bytes).is_err(), "the cut splits a character");
        let found = detect_in_prefix(&path, &AsciiOptions::default()).expect("detected");
        assert_eq!(found.delimiter, Delimiter::Comma);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
