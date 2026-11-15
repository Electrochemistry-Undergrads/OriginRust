//! The Import Wizard (File > Import > Import Wizard, Ctrl+3): the same import options as the
//! import window, taken one page at a time. Pages, as Origin's ASCII wizard (measured,
//! import-ascii.md): Source, Header Lines, Variable Extraction (greyed), File Name Options, Data
//! Columns, Data Selection and Save Filters (greyed: Sheaf has no saved import filters). The
//! buttons are Cancel at the left and "<< Back", "Next >>", "Finish" at the right, as in Origin's
//! first page. Finish imports the file with the chosen options, exactly as OK does in the import
//! window.

use super::frame::read_only_box;
use super::graph_props::group_box;
use super::import::{ImportDialog, ImportOutcome, ImportPage, optional_line, usable_mode};
use super::import_theme::{ThemeRow, Themed};
use eframe::egui::{self, Color32, RichText};
use sheaf_core::template::Template;
use sheaf_io::ImportKind;
use sheaf_io::batch::ImportMode;
use std::io::Read;
use std::path::{Path, PathBuf};

/// Lines of the file shown on the Header Lines and Column Names pages.
const RAW_LINES: usize = 14;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WizardPage {
    Source,
    HeaderLines,
    VariableExtraction,
    FileNameOptions,
    DataColumns,
    DataSelection,
    SaveFilters,
}

impl WizardPage {
    const ALL: [WizardPage; 7] = [
        WizardPage::Source,
        WizardPage::HeaderLines,
        WizardPage::VariableExtraction,
        WizardPage::FileNameOptions,
        WizardPage::DataColumns,
        WizardPage::DataSelection,
        WizardPage::SaveFilters,
    ];

    fn title(self) -> &'static str {
        match self {
            WizardPage::Source => "Source",
            WizardPage::HeaderLines => "Header Lines",
            WizardPage::VariableExtraction => "Variable Extraction",
            WizardPage::FileNameOptions => "File Name Options",
            WizardPage::DataColumns => "Data Columns",
            WizardPage::DataSelection => "Data Selection",
            WizardPage::SaveFilters => "Save Filters",
        }
    }

    fn index(self) -> usize {
        Self::ALL.iter().position(|p| *p == self).unwrap_or(0)
    }
}

/// The first lines of a file as text, for the pages that choose header lines.
struct RawLines {
    file: PathBuf,
    lines: Vec<String>,
}

pub struct ImportWizard {
    dlg: ImportDialog,
    page: WizardPage,
    /// The text of the File box on the Source page.
    path_text: String,
    /// The text of the File box is not an existing file (the files chosen before are kept).
    path_error: bool,
    /// "List filters applicable to both Data Type and file name" (only the state is kept).
    filters_both: bool,
    /// File Name Options: the workbook's long name is the file name.
    name_workbook: bool,
    /// File Name Options: the sheet is named after the file.
    name_sheet: bool,
    raw: Option<RawLines>,
}

impl ImportWizard {
    /// A wizard on its Source page, with the files already chosen when there are any.
    pub fn new(files: Vec<PathBuf>) -> Self {
        let path_text = files.first().map(|f| f.display().to_string()).unwrap_or_default();
        let mut dlg = ImportDialog::new(files);
        dlg.themes = ThemeRow::new("import_wizard");
        ImportWizard {
            dlg,
            page: WizardPage::Source,
            path_text,
            path_error: false,
            filters_both: true,
            name_workbook: true,
            name_sheet: false,
            raw: None,
        }
    }

    /// Points the wizard at the theme folder; it starts from `<last used>` when there is one.
    pub fn attach_themes(&mut self, dir: Option<PathBuf>) {
        self.dlg.attach_themes(dir);
    }

    /// The failure of the last Finish to record `<last used>`, once.
    pub fn take_theme_warning(&mut self) -> Option<String> {
        self.dlg.take_theme_warning()
    }

    /// The entry shown in the Dialog Theme list (for tests).
    #[cfg(test)]
    pub fn theme_entry(&self) -> &str {
        self.dlg.theme_entry()
    }

    /// The import window the wizard holds (for tests).
    #[cfg(test)]
    pub fn dialog(&self) -> &ImportDialog {
        &self.dlg
    }

    /// The page the wizard is on (for tests).
    #[cfg(test)]
    pub fn page(&self) -> WizardPage {
        self.page
    }

    pub fn show_page(&mut self, page: WizardPage) {
        if page != self.page {
            self.dlg.themes.close_prompt();
        }
        self.page = page;
    }

    /// A file is chosen and the File box does not show a path that is not a file.
    fn has_file(&self) -> bool {
        !self.dlg.files.is_empty() && !self.path_error
    }

    fn set_files(&mut self, files: Vec<PathBuf>) {
        self.path_text = files.first().map(|f| f.display().to_string()).unwrap_or_default();
        self.path_error = false;
        self.dlg.files = files;
        self.dlg.current = 0;
        self.dlg.preview = None;
        self.raw = None;
    }

    /// The first lines of the current file, read once per file.
    fn raw_lines(&mut self) -> &[String] {
        let file = self.dlg.files.get(self.dlg.current).cloned();
        let fresh = matches!((&self.raw, &file), (Some(r), Some(f)) if r.file == *f);
        if !fresh {
            self.raw = file.map(|f| RawLines { lines: read_lines(&f, RAW_LINES), file: f });
        }
        self.raw.as_ref().map(|r| r.lines.as_slice()).unwrap_or(&[])
    }

    /// Reads every file with the chosen options and applies the File Name Options.
    fn finish(&mut self) -> ImportOutcome {
        self.dlg.confirm_themes();
        let mut items = Vec::new();
        let mut errors = Vec::new();
        for (i, path) in self.dlg.files.iter().enumerate() {
            match self.dlg.read(i) {
                Ok(mut imported) => {
                    let stem = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
                    if !self.name_workbook {
                        imported.workbook.long_name = String::new();
                    }
                    if self.name_sheet
                        && !stem.is_empty()
                        && let Some(sheet) = imported.workbook.sheets.first_mut()
                    {
                        sheet.name = stem;
                    }
                    items.push(imported);
                }
                Err(e) => errors.push(e),
            }
        }
        ImportOutcome::Done(self.dlg.request(items, errors))
    }

    pub fn show(&mut self, ctx: &egui::Context, target: Option<&str>) -> ImportOutcome {
        self.dlg.target = target.map(str::to_string);
        if self.has_file() {
            self.dlg.current = self.dlg.current.min(self.dlg.files.len() - 1);
            if self.dlg.preview.is_none() {
                self.dlg.preview = Some(self.dlg.read(self.dlg.current));
            }
        }
        let before = (
            self.dlg.current,
            self.dlg.kind,
            self.dlg.chi.clone(),
            self.dlg.ascii.clone(),
            self.dlg.partial_except_first,
        );

        let page_before = self.page;
        let avail = ctx.available_rect();
        let size = egui::vec2(620.0, 595.0);
        let pos = (avail.center() - size / 2.0).max(avail.min);
        let mut open = true;
        let mut outcome = ImportOutcome::Open;
        egui::Window::new(format!("Import Wizard - {}", self.page.title()))
            .id(egui::Id::new("import_wizard"))
            .open(&mut open)
            .default_pos(pos)
            .fixed_size(size)
            .collapsible(false)
            .show(ctx, |ui| {
                let body = (ui.available_height() - 40.0).max(200.0);
                egui::ScrollArea::vertical()
                    .id_salt("import_wizard_page")
                    .max_height(body)
                    .auto_shrink([false, false])
                    .show(ui, |ui| match self.page {
                        WizardPage::Source => self.source_page(ui),
                        WizardPage::HeaderLines => self.header_lines_page(ui),
                        WizardPage::VariableExtraction => variable_extraction_page(ui),
                        WizardPage::FileNameOptions => self.file_name_page(ui),
                        WizardPage::DataColumns => self.data_columns_page(ui),
                        WizardPage::DataSelection => self.data_selection_page(ui),
                        WizardPage::SaveFilters => self.save_filters_page(ui),
                    });
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        outcome = ImportOutcome::Cancelled;
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let i = self.page.index();
                        let ready = self.has_file();
                        if ui.add_enabled(ready, egui::Button::new("Finish")).clicked() {
                            outcome = self.finish();
                        }
                        let can_next = ready && i + 1 < WizardPage::ALL.len();
                        if ui.add_enabled(can_next, egui::Button::new("Next >>")).clicked() {
                            self.page = WizardPage::ALL[i + 1];
                        }
                        if ui.add_enabled(ready && i > 0, egui::Button::new("<< Back")).clicked() {
                            self.page = WizardPage::ALL[i - 1];
                        }
                    });
                });
            });

        if before
            != (
                self.dlg.current,
                self.dlg.kind,
                self.dlg.chi.clone(),
                self.dlg.ascii.clone(),
                self.dlg.partial_except_first,
            )
        {
            self.dlg.preview = None;
            ctx.request_repaint();
        }
        // The Save As window belongs to the Source page, the only one that draws it.
        if self.page != page_before {
            self.dlg.themes.close_prompt();
        }
        if !open {
            outcome = ImportOutcome::Cancelled;
        }
        outcome
    }

    // ----- pages ----------------------------------------------------------------------------

    fn source_page(&mut self, ui: &mut egui::Ui) {
        if let Some(command) = self.dlg.themes.ui(ui) {
            self.dlg.run_theme_command(command);
        }
        group_box(ui, "Data Type", |ui| {
            ui.horizontal(|ui| {
                let _ = ui.radio(true, "ASCII");
                ui.add_enabled(false, egui::RadioButton::new(false, "Binary"));
                ui.add_enabled(false, egui::RadioButton::new(false, "User Defined"));
            });
        });
        group_box(ui, "Data Source", |ui| {
            ui.horizontal(|ui| {
                let _ = ui.radio(true, "File");
                let width = (ui.available_width() - 34.0).max(120.0);
                let response = ui.add(egui::TextEdit::singleline(&mut self.path_text).desired_width(width));
                if response.changed() {
                    let typed = PathBuf::from(self.path_text.trim().trim_matches('"'));
                    if typed.is_file() {
                        if self.dlg.files.first() != Some(&typed) {
                            let text = self.path_text.clone();
                            self.set_files(vec![typed]);
                            self.path_text = text;
                        }
                        self.path_error = false;
                    } else if self.path_text.trim().is_empty() {
                        self.set_files(Vec::new());
                    } else {
                        self.path_error = true;
                    }
                }
                if ui.button("...").on_hover_text("Choose the files").clicked()
                    && let Some(files) = rfd::FileDialog::new().set_title("Import data files").pick_files()
                    && !files.is_empty()
                {
                    self.set_files(files);
                }
            });
            if self.dlg.files.len() > 1 {
                ui.weak(format!("{} files are imported with these options", self.dlg.files.len()));
            }
            ui.add_enabled(false, egui::RadioButton::new(false, "Clipboard"));
        });
        group_box(ui, "Import Filter", |ui| {
            ui.checkbox(&mut self.filters_both, "List filters applicable to both Data Type and file name");
            ui.horizontal(|ui| {
                ui.label("Import Filters for current Data Type");
                let name = |k: ImportKind| match k {
                    ImportKind::Auto => "<None>",
                    ImportKind::Chi => "CH Instruments",
                    ImportKind::Ascii => "Delimited Text",
                };
                egui::ComboBox::from_id_salt("wizard_filter").selected_text(name(self.dlg.kind)).width(260.0).show_ui(
                    ui,
                    |ui| {
                        for k in [ImportKind::Auto, ImportKind::Chi, ImportKind::Ascii] {
                            ui.selectable_value(&mut self.dlg.kind, k, name(k));
                        }
                    },
                );
            });
            ui.horizontal(|ui| {
                ui.label("Description");
                let text = match self.dlg.kind {
                    ImportKind::Auto => "The format is found from the file",
                    ImportKind::Chi => "Text and binary files of the CH Instruments potentiostat",
                    ImportKind::Ascii => "Text with a delimiter between the columns",
                };
                read_only_box(ui, text, ui.available_width());
            });
        });
        group_box(ui, "Target Window", |ui| {
            ui.horizontal(|ui| {
                let _ = ui.radio(true, "Worksheet");
                ui.add_enabled(false, egui::RadioButton::new(false, "Matrix"));
                ui.add_enabled(
                    false,
                    egui::RadioButton::new(false, "None (User Defined filter needs to create window)"),
                );
            });
            ui.horizontal(|ui| {
                ui.label("Import Mode");
                egui::ComboBox::from_id_salt("wizard_import_mode")
                    .selected_text(usable_mode(self.dlg.mode, self.dlg.target.is_some()).label())
                    .width(200.0)
                    .show_ui(ui, |ui| {
                        for m in ImportMode::ALL {
                            let usable = m == ImportMode::NewBook || self.dlg.target.is_some();
                            ui.add_enabled_ui(usable, |ui| {
                                ui.selectable_value(&mut self.dlg.mode, m, m.label());
                            });
                        }
                    });
            });
            match (&self.dlg.target, self.dlg.mode) {
                (Some(t), m) if m != ImportMode::NewBook => {
                    ui.weak(format!("Active sheet: {t}"));
                }
                (None, _) => {
                    ui.weak("Open or select a workbook to add to its sheets.");
                }
                _ => {}
            }
        });
        group_box(ui, "Graph", |ui| {
            ui.checkbox(&mut self.dlg.plot, "Make a graph of each file");
            ui.add_enabled_ui(self.dlg.plot, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Template");
                    let suggested = match &self.dlg.preview {
                        Some(Ok(p)) => format!("Suggested ({})", p.suggested_template.label()),
                        _ => "Suggested".to_string(),
                    };
                    let text = self.dlg.template.map(|t| t.label().to_string()).unwrap_or_else(|| suggested.clone());
                    egui::ComboBox::from_id_salt("wizard_template").selected_text(text).show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.dlg.template, None, suggested);
                        for t in Template::ALL {
                            ui.selectable_value(&mut self.dlg.template, Some(t), t.label());
                        }
                    });
                });
            });
        });
        self.file_list(ui);
        if self.path_error {
            super::frame::hint(ui, "This path is not a file.");
        } else if self.dlg.files.is_empty() {
            super::frame::hint(ui, "Choose a file to continue.");
        }
    }

    /// The files that are imported with these options: click one to preview it, right-click to
    /// remove it, "Add files..." to add more.
    fn file_list(&mut self, ui: &mut egui::Ui) {
        if self.dlg.files.is_empty() {
            return;
        }
        group_box(ui, "Files", |ui| {
            ui.horizontal_wrapped(|ui| {
                let mut remove = None;
                for (i, f) in self.dlg.files.iter().enumerate() {
                    let name = f.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                    let response =
                        ui.selectable_label(i == self.dlg.current, name).on_hover_text(f.display().to_string());
                    if response.clicked() {
                        self.dlg.current = i;
                    }
                    response.context_menu(|ui| {
                        if ui.button("Remove from the list").clicked() {
                            remove = Some(i);
                            ui.close();
                        }
                    });
                }
                if let Some(i) = remove {
                    let mut files = self.dlg.files.clone();
                    files.remove(i);
                    let keep = self.dlg.current.min(files.len().saturating_sub(1));
                    self.set_files(files);
                    self.dlg.current = keep;
                }
                if ui.small_button("Add files...").clicked()
                    && let Some(more) = rfd::FileDialog::new().set_title("Add data files").pick_files()
                {
                    self.dlg.files.extend(more);
                }
            });
        });
    }

    /// The header entries the importer found in the file shown in the preview.
    fn file_info(&mut self, ui: &mut egui::Ui) {
        if let Some(Ok(p)) = &self.dlg.preview
            && !p.meta.is_empty()
        {
            group_box(ui, "File Info and Parameters", |ui| {
                ui.weak(format!("{} entries in the header of the file shown in the preview", p.meta.len()));
                for (k, v) in &p.meta {
                    ui.horizontal(|ui| {
                        ui.weak(k);
                        ui.label(v);
                    });
                }
            });
        }
    }

    /// The lines of the file with numbers; those before the data are shaded.
    fn raw_view(&mut self, ui: &mut egui::Ui, header_lines: Option<usize>) {
        let lines: Vec<String> = self.raw_lines().to_vec();
        ui.label(RichText::new("File").strong());
        egui::Frame::new().fill(Color32::WHITE).stroke(egui::Stroke::new(1.0_f32, crate::theme::BORDER)).show(
            ui,
            |ui| {
                ui.set_width(ui.available_width());
                for (i, line) in lines.iter().enumerate() {
                    let n = i + 1;
                    let shade = header_lines.is_some_and(|h| n <= h);
                    let text = RichText::new(format!("{n:>3}  {line}")).monospace();
                    let text = if shade { text.background_color(Color32::from_rgb(255, 250, 205)) } else { text };
                    ui.add(egui::Label::new(text).truncate());
                }
                if lines.is_empty() {
                    ui.weak("Nothing to show.");
                }
            },
        );
    }

    /// Delimited-text pages need a text file; a CH Instruments file has its own header.
    fn needs_ascii(&mut self, ui: &mut egui::Ui) -> bool {
        if self.dlg.shown_format() == "ascii" {
            return true;
        }
        ui.add_enabled_ui(false, |ui| ui.label("These options apply to delimited text files."));
        false
    }

    fn header_lines_page(&mut self, ui: &mut egui::Ui) {
        self.header_lines_body(ui);
        self.file_info(ui);
    }

    fn header_lines_body(&mut self, ui: &mut egui::Ui) {
        if self.needs_ascii(ui) {
            let detected = self.dlg.detected();
            let found = detected.as_ref().map(|d| d.header_lines);
            // Origin's Header Lines page also says which header line gives which column label.
            group_box(ui, "Header Lines", |ui| {
                egui::Grid::new("wizard_header_lines").num_columns(2).spacing([12.0, 5.0]).show(ui, |ui| {
                    ui.label("Number of main header lines");
                    optional_line(ui, &mut self.dlg.ascii.header_lines, found, "none");
                    ui.end_row();
                    let o = &mut self.dlg.ascii;
                    ui.label("Long Names");
                    optional_line(ui, &mut o.long_name_line, detected.as_ref().map(|d| d.long_name_line), "none");
                    ui.end_row();
                    ui.label("Units");
                    optional_line(ui, &mut o.units_line, detected.as_ref().map(|d| d.units_line), "none");
                    ui.end_row();
                    ui.label("Comments");
                    optional_line(ui, &mut o.comments_line, detected.as_ref().map(|d| d.comments_line), "none");
                    ui.end_row();
                    ui.label("Short Names");
                    let mut short = (o.short_name_line > 0).then_some(o.short_name_line);
                    optional_line(ui, &mut short, None, "<None>");
                    o.short_name_line = short.unwrap_or(0);
                    ui.end_row();
                });
            });
            let shown = self.dlg.ascii.header_lines.or(found);
            self.raw_view(ui, shown);
        } else {
            self.raw_view(ui, None);
        }
    }

    /// Origin's Data Selection page: which rows to import (Partial Import).
    fn data_selection_page(&mut self, ui: &mut egui::Ui) {
        if self.dlg.shown_format() == "chi" {
            ui.weak("CH Instruments files are imported whole.");
            return;
        }
        group_box(ui, "Partial Import", |ui| {
            self.dlg.page = ImportPage::PartialImport;
            self.dlg.ascii_page(ui);
        });
        ui.add_space(4.0);
        ui.allocate_ui(egui::vec2(ui.available_width(), 170.0), |ui| self.dlg.preview_ui(ui));
    }

    fn file_name_page(&mut self, ui: &mut egui::Ui) {
        group_box(ui, "File Name Options", |ui| {
            ui.checkbox(&mut self.name_workbook, "Use the file name as the long name of the workbook");
            ui.checkbox(&mut self.name_sheet, "Use the file name as the name of the sheet");
        });
        if let Some(file) = self.dlg.files.get(self.dlg.current) {
            let stem = file.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
            let name = file.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
            ui.add_space(6.0);
            ui.label(format!("File: {name}"));
            ui.label(format!("Workbook long name: {}", if self.name_workbook { stem.as_str() } else { "(empty)" }));
            if self.name_sheet {
                ui.label(format!("Sheet name: {stem}"));
            }
        }
    }

    fn data_columns_page(&mut self, ui: &mut egui::Ui) {
        if self.dlg.shown_format() == "chi" {
            group_box(ui, "CH Instruments", |ui| {
                let o = &mut self.dlg.chi;
                ui.checkbox(&mut o.split_scans, "Split cyclic voltammetry into scans");
                ui.checkbox(
                    &mut o.split_segments,
                    "Add a sheet with each charge and discharge segment (chronopotentiometry)",
                );
                ui.checkbox(&mut o.add_milliamp, "Add the current in mA");
                ui.checkbox(&mut o.add_charge, "Add the charge to current-vs-time files");
                ui.checkbox(&mut o.invert_current, "Invert the sign of the current");
                ui.checkbox(&mut o.exact_time, "Recompute rounded times from the sampling interval");
            });
        } else {
            for (title, page) in [
                ("Delimiter", ImportPage::Options),
                ("Column Designation", ImportPage::ColumnDesignation),
                ("Numbers and Dates", ImportPage::Miscellaneous),
            ] {
                group_box(ui, title, |ui| {
                    self.dlg.page = page;
                    self.dlg.ascii_page(ui);
                });
            }
        }
        ui.add_space(4.0);
        ui.allocate_ui(egui::vec2(ui.available_width(), 170.0), |ui| self.dlg.preview_ui(ui));
    }

    fn save_filters_page(&mut self, ui: &mut egui::Ui) {
        group_box(ui, "Save Filters", |ui| {
            ui.add_enabled_ui(false, |ui| {
                let mut save = false;
                ui.checkbox(&mut save, "Save the settings as an import filter");
                ui.horizontal(|ui| {
                    ui.label("Filter file name");
                    let mut name = String::new();
                    ui.add(egui::TextEdit::singleline(&mut name).desired_width(240.0));
                });
                ui.horizontal(|ui| {
                    ui.label("Description");
                    let mut text = String::new();
                    ui.add(egui::TextEdit::singleline(&mut text).desired_width(240.0));
                });
            });
        });
        ui.weak("Saved import filters are not available in this version. Press Finish to import.");
    }
}

/// Origin's Variable Extraction page (variables from file names and headers), which Sheaf does not
/// offer: shown greyed.
fn variable_extraction_page(ui: &mut egui::Ui) {
    group_box(ui, "Variable Extraction", |ui| {
        ui.add_enabled_ui(false, |ui| {
            let mut off = false;
            ui.checkbox(&mut off, "Extract variables from file names and file headers");
            ui.checkbox(&mut off, "Add variables to page info");
            ui.checkbox(&mut off, "Create User Parameter rows for each variable");
        });
    });
    ui.weak("Not available in this version: header lines can become parameter rows on the Header Lines page.");
}

/// The first `count` lines of a text file (the first 64 KB are read at most).
fn read_lines(path: &Path, count: usize) -> Vec<String> {
    let mut bytes = Vec::new();
    if std::fs::File::open(path).and_then(|f| f.take(64 * 1024).read_to_end(&mut bytes)).is_err() {
        return Vec::new();
    }
    match sheaf_io::decode_text(&bytes, path) {
        Ok(text) => text.lines().take(count).map(str::to_string).collect(),
        Err(_) => Vec::new(),
    }
}
