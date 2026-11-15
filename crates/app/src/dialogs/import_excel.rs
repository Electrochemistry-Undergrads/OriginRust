//! The Excel import window (File > Import > Excel): which sheets of a workbook to read, the header
//! rows, where the data goes, and a preview of the cells of one sheet. The header-row, partial
//! import and import-mode controls are the ones of the ASCII import window (`import.rs`); the
//! sheet list and the preview are specific to Excel. Origin's own window is not measured
//! (`docs/spec/import-excel.md`), so the layout follows the ASCII window.

use super::import::{
    HeaderLines, ImportOutcome, ImportRequest, ModeControls, header_line_rows, mode_and_graph_controls, selection_rows,
    usable_mode,
};
use super::import_theme::{ThemeRow, Themed};
use eframe::egui::{self, Color32, RichText};
use egui_extras::{Column as TableColumn, TableBuilder};
use sheaf_core::template::Template;
use sheaf_io::batch::{ImportMode, PlaceOptions};
use sheaf_io::chi::ChiOptions;
use sheaf_io::excel::{ExcelOptions, SheetInfo, SheetKind, SheetPreview, SheetRef};
use sheaf_io::themes::{self, ExcelSettings};
use sheaf_io::{ImportKind, Imported, IoError};
use std::path::{Path, PathBuf};

/// Rows of the cell preview.
const PREVIEW_ROWS: usize = 40;
/// Columns of the cell preview (a sheet can have thousands).
const PREVIEW_COLUMNS: usize = 40;

/// The nodes of the tree at the left.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ExcelPage {
    Sheets,
    ImportMode,
    HeaderLines,
    ColumnDesignation,
    PartialImport,
    Miscellaneous,
}

impl ExcelPage {
    const ALL: [ExcelPage; 6] = [
        ExcelPage::Sheets,
        ExcelPage::ImportMode,
        ExcelPage::HeaderLines,
        ExcelPage::ColumnDesignation,
        ExcelPage::PartialImport,
        ExcelPage::Miscellaneous,
    ];

    fn label(self) -> &'static str {
        match self {
            ExcelPage::Sheets => "Sheets",
            ExcelPage::ImportMode => "Import Mode",
            ExcelPage::HeaderLines => "Header Rows",
            ExcelPage::ColumnDesignation => "Column Designation",
            ExcelPage::PartialImport => "Partial Import",
            ExcelPage::Miscellaneous => "Miscellaneous",
        }
    }
}

/// One sheet of a file and whether it is imported.
struct Choice {
    info: SheetInfo,
    selected: bool,
}

impl Choice {
    fn is_worksheet(&self) -> bool {
        self.info.kind == SheetKind::Worksheet
    }
}

/// A file of the list; its sheets are read when it first needs them.
struct FileEntry {
    path: PathBuf,
    sheets: Option<Result<Vec<Choice>, String>>,
    /// The sheet shown in the preview (an index into `sheets`).
    shown: usize,
}

impl FileEntry {
    fn new(path: PathBuf) -> Self {
        FileEntry { path, sheets: None, shown: 0 }
    }

    /// Lists the sheets once. Every worksheet is chosen, hidden ones too (Origin's `impExcel`
    /// imports hidden sheets, import-excel.md 2); chart sheets are not.
    fn load(&mut self) -> &mut Result<Vec<Choice>, String> {
        let path = &self.path;
        let shown = &mut self.shown;
        self.sheets.get_or_insert_with(|| {
            let list = sheaf_io::excel::list_sheets(path).map_err(|e| e.to_string())?;
            let choices: Vec<Choice> = list
                .into_iter()
                .map(|info| {
                    let selected = info.kind == SheetKind::Worksheet;
                    Choice { info, selected }
                })
                .collect();
            *shown = choices
                .iter()
                .position(|c| c.selected)
                .or_else(|| choices.iter().position(Choice::is_worksheet))
                .unwrap_or(0);
            Ok(choices)
        })
    }
}

/// The preview of one sheet, and what it was computed for so that it is not repeated every frame.
struct PreviewCache {
    key: (PathBuf, usize, ExcelOptions),
    result: Result<SheetPreview, String>,
}

pub struct ExcelImportDialog {
    files: Vec<FileEntry>,
    /// The file shown in the preview.
    current: usize,
    pub opts: ExcelOptions,
    partial_except_first: bool,
    /// Make a graph of the first sheet of each file.
    plot: bool,
    template: Option<Template>,
    mode: ImportMode,
    /// Name of the active sheet (book and sheet), when there is one to put data into.
    target: Option<String>,
    page: ExcelPage,
    preview: Option<PreviewCache>,
    /// The Dialog Theme row (T46).
    themes: ThemeRow,
    /// A failure to record `<last used>` at OK, for the application to log.
    theme_warning: Option<String>,
    #[cfg(test)]
    preview_runs: usize,
}

impl ExcelImportDialog {
    pub fn new(files: Vec<PathBuf>) -> Self {
        ExcelImportDialog {
            files: files.into_iter().map(FileEntry::new).collect(),
            current: 0,
            opts: ExcelOptions::default(),
            partial_except_first: false,
            // Origin's Excel import does not graph (not measured); Sheaf leaves the choice to the user.
            plot: false,
            template: None,
            mode: ImportMode::NewBook,
            target: None,
            page: ExcelPage::Sheets,
            preview: None,
            themes: ThemeRow::new("import_excel"),
            theme_warning: None,
            #[cfg(test)]
            preview_runs: 0,
        }
    }

    /// The controls as a theme stores them (the ticked sheets, the file list and the target are not
    /// part of it).
    fn settings(&self) -> ExcelSettings {
        ExcelSettings {
            options: self.opts.clone(),
            partial_except_first: self.partial_except_first,
            mode: self.mode,
            plot: self.plot,
        }
    }

    /// The failure of the last OK to record `<last used>`, once.
    pub fn take_theme_warning(&mut self) -> Option<String> {
        self.theme_warning.take()
    }

    /// The entry shown in the Dialog Theme list (for tests).
    #[cfg(test)]
    pub fn theme_entry(&self) -> &str {
        self.themes.selected()
    }

    /// The id of the Save As window (for tests).
    #[cfg(test)]
    pub fn theme_prompt_id(&self) -> egui::Id {
        self.themes.prompt_id()
    }

    /// The saved names in the Dialog Theme list (for tests).
    #[cfg(test)]
    pub fn theme_names(&self) -> &[String] {
        self.themes.names()
    }

    /// Changes the options as the controls would (for tests).
    #[cfg(test)]
    pub fn edit_options(&mut self, change: impl FnOnce(&mut ExcelOptions)) {
        change(&mut self.opts);
    }

    /// The options and choices that a theme holds (for tests).
    #[cfg(test)]
    pub fn theme_settings(&self) -> ExcelSettings {
        self.settings()
    }

    /// OK: imports the files and remembers the settings as `<last used>`.
    fn confirm(&mut self) -> ImportOutcome {
        self.theme_warning = self.record_last_used();
        self.run()
    }

    /// Starts on a node of the tree (`--show import-excel-page:header`).
    pub fn show_page(&mut self, name: &str) {
        self.page = match name {
            "mode" => ExcelPage::ImportMode,
            "header" => ExcelPage::HeaderLines,
            "columns" => ExcelPage::ColumnDesignation,
            "partial" => ExcelPage::PartialImport,
            "misc" => ExcelPage::Miscellaneous,
            _ => ExcelPage::Sheets,
        };
    }

    /// How many times a preview was read (for tests).
    #[cfg(test)]
    pub fn preview_runs(&self) -> usize {
        self.preview_runs
    }

    /// Chooses the import mode (for tests; the window has a list).
    #[cfg(test)]
    pub fn set_mode(&mut self, mode: ImportMode) {
        self.mode = mode;
    }

    /// The names of the sheets chosen for a file (for tests).
    #[cfg(test)]
    pub fn selected_sheets(&mut self, file: usize) -> Vec<String> {
        match self.files[file].load() {
            Ok(list) => list.iter().filter(|c| c.selected).map(|c| c.info.name.clone()).collect(),
            Err(_) => Vec::new(),
        }
    }

    /// The options of the file at `index` without the sheet choice: the partial import is
    /// switched off for the first file when "All files except first" is on.
    fn scoped(&self, index: usize) -> ExcelOptions {
        if index == 0 && self.partial_except_first { self.opts.without_partial_import() } else { self.opts.clone() }
    }

    /// The options for one file: the sheets that are ticked, and the partial import unless it
    /// is switched off for the first file.
    fn options_for(&mut self, index: usize) -> Result<ExcelOptions, String> {
        let scoped = self.scoped(index);
        let file = &mut self.files[index];
        let name = file.path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let chosen = match file.load() {
            Ok(list) => list.iter().filter(|c| c.selected).map(|c| SheetRef::Index(c.info.index)).collect::<Vec<_>>(),
            Err(e) => return Err(format!("{name}: {e}")),
        };
        if chosen.is_empty() {
            return Err(format!("{name}: no sheet is ticked"));
        }
        Ok(ExcelOptions { sheets: chosen, ..scoped })
    }

    fn read(&mut self, index: usize) -> Result<Imported, String> {
        let opts = self.options_for(index)?;
        let path = self.files[index].path.clone();
        sheaf_io::import_file_with_excel(
            &path,
            ImportKind::Auto,
            &ChiOptions::default(),
            &sheaf_io::ascii::AsciiOptions::default(),
            &opts,
        )
        .map_err(|e| e.to_string())
    }

    pub fn show(&mut self, ctx: &egui::Context, target: Option<&str>) -> ImportOutcome {
        self.target = target.map(str::to_string);
        if self.files.is_empty() {
            return ImportOutcome::Cancelled;
        }
        self.current = self.current.min(self.files.len() - 1);
        self.refresh_preview();

        let mut open = true;
        let mut outcome = ImportOutcome::Open;
        egui::Window::new("Import Excel")
            .id(egui::Id::new("import_excel_dialog"))
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_size([780.0, 640.0])
            .show(ctx, |ui| {
                egui::TopBottomPanel::bottom("import_excel_buttons").show_inside(ui, |ui| {
                    ui.add_space(6.0);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Cancel").clicked() {
                            outcome = ImportOutcome::Cancelled;
                        }
                        if ui.button("OK").clicked() {
                            outcome = self.confirm();
                        }
                        if self.files.len() > 1 {
                            ui.weak(format!("{} files are imported with these options", self.files.len()));
                        }
                    });
                });
                egui::CentralPanel::default().show_inside(ui, |ui| {
                    if let Some(command) = self.themes.ui(ui) {
                        self.run_theme_command(command);
                    }
                    self.files_ui(ui);
                    ui.separator();
                    ui.allocate_ui_with_layout(
                        egui::vec2(ui.available_width(), 215.0),
                        egui::Layout::left_to_right(egui::Align::Min),
                        |ui| {
                            ui.vertical(|ui| {
                                ui.set_width(150.0);
                                self.tree_ui(ui);
                            });
                            ui.separator();
                            ui.vertical(|ui| {
                                egui::ScrollArea::vertical()
                                    .id_salt("import_excel_page")
                                    .auto_shrink([false, false])
                                    .show(ui, |ui| self.page_ui(ui));
                            });
                        },
                    );
                    ui.separator();
                    self.preview_ui(ui);
                });
            });
        if !open {
            outcome = ImportOutcome::Cancelled;
        }
        outcome
    }

    fn run(&mut self) -> ImportOutcome {
        let mut items = Vec::new();
        let mut errors = Vec::new();
        for i in 0..self.files.len() {
            match self.read(i) {
                Ok(imported) => items.push(imported),
                Err(e) => errors.push(e),
            }
        }
        ImportOutcome::Done(self.request(items, errors))
    }

    pub(super) fn request(&self, items: Vec<Imported>, errors: Vec<String>) -> ImportRequest {
        ImportRequest {
            items,
            errors,
            plot: self.plot,
            template: self.template,
            mode: usable_mode(self.mode, self.target.is_some()),
            // The sheets keep the names they have in Excel.
            place: PlaceOptions { rename_sheet: true },
        }
    }

    /// Reads the cells of the shown sheet again when the file, the sheet or the options changed.
    fn refresh_preview(&mut self) {
        let opts = self.scoped(self.current);
        let file = &mut self.files[self.current];
        if file.load().is_err() {
            self.preview = None;
            return;
        }
        let Some(Ok(list)) = &file.sheets else { return };
        let Some(choice) = list.get(file.shown).filter(|c| c.is_worksheet()) else {
            self.preview = None;
            return;
        };
        let key = (file.path.clone(), choice.info.index, opts);
        if matches!(&self.preview, Some(p) if p.key == key) {
            return;
        }
        #[cfg(test)]
        {
            self.preview_runs += 1;
        }
        let result = sheaf_io::excel::preview_sheet(&key.0, &SheetRef::Index(key.1), &key.2, PREVIEW_ROWS)
            .map_err(|e| e.to_string());
        self.preview = Some(PreviewCache { key, result });
    }

    fn files_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.label("Files:");
            let mut remove = None;
            for (i, f) in self.files.iter().enumerate() {
                let name = f.path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                let response = ui.selectable_label(i == self.current, name).on_hover_text(f.path.display().to_string());
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
                && let Some(more) = rfd::FileDialog::new()
                    .set_title("Add Excel workbooks")
                    .add_filter("Excel workbooks", &sheaf_io::excel::EXTENSIONS)
                    .pick_files()
            {
                self.files.extend(more.into_iter().map(FileEntry::new));
            }
        });
    }

    fn tree_ui(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("Import Options").strong());
        for page in ExcelPage::ALL {
            ui.horizontal(|ui| {
                ui.add_space(16.0);
                if ui.selectable_label(self.page == page, page.label()).clicked() {
                    self.page = page;
                }
            });
        }
    }

    fn page_ui(&mut self, ui: &mut egui::Ui) {
        match self.page {
            ExcelPage::Sheets => self.sheets_page(ui),
            ExcelPage::ImportMode => {
                mode_and_graph_controls(
                    ui,
                    ModeControls {
                        mode: &mut self.mode,
                        target: self.target.as_deref(),
                        plot: &mut self.plot,
                        template: &mut self.template,
                        suggested: format!("Suggested ({})", Template::Line.label()),
                    },
                );
            }
            ExcelPage::HeaderLines => self.header_page(ui),
            ExcelPage::ColumnDesignation => {
                egui::Grid::new("excel_designation").num_columns(2).spacing([12.0, 5.0]).show(ui, |ui| {
                    ui.label("Columns");
                    ui.checkbox(&mut self.opts.first_column_x, "The first column is X");
                    ui.end_row();
                });
            }
            ExcelPage::PartialImport => {
                egui::Grid::new("excel_partial").num_columns(2).spacing([12.0, 5.0]).show(ui, |ui| {
                    if self.files.len() > 1 {
                        ui.label("Apply selection");
                        ui.checkbox(&mut self.partial_except_first, "All files except first");
                        ui.end_row();
                    }
                    let o = &mut self.opts;
                    selection_rows(ui, &mut o.skip_rows, &mut o.rows, &mut o.columns);
                });
            }
            ExcelPage::Miscellaneous => {
                egui::Grid::new("excel_misc").num_columns(2).spacing([12.0, 5.0]).show(ui, |ui| {
                    ui.label("Dates and times");
                    let mut as_text = self.opts.dates == sheaf_io::ascii::DateImport::AsText;
                    if ui.checkbox(&mut as_text, "Keep as text").changed() {
                        self.opts.dates = if as_text {
                            sheaf_io::ascii::DateImport::AsText
                        } else {
                            sheaf_io::ascii::DateImport::Detect
                        };
                    }
                    ui.end_row();
                    ui.label("Naming");
                    ui.checkbox(&mut self.opts.rename_book, "Rename Book with the file name");
                    ui.end_row();
                });
                ui.add_space(6.0);
                ui.weak("Booleans are read as 1 and 0, error cells as missing values, formulas as their stored value.");
            }
        }
    }

    /// The sheets of the current file, each with a check box. Hidden sheets are marked and off.
    fn sheets_page(&mut self, ui: &mut egui::Ui) {
        let file = &mut self.files[self.current];
        match file.load() {
            Err(e) => {
                ui.colored_label(Color32::from_rgb(170, 30, 30), format!("This file cannot be read: {e}"));
            }
            Ok(list) => {
                ui.horizontal(|ui| {
                    if ui.small_button("Select all").clicked() {
                        for c in list.iter_mut().filter(|c| c.is_worksheet()) {
                            c.selected = true;
                        }
                    }
                    if ui.small_button("Clear").clicked() {
                        for c in list.iter_mut() {
                            c.selected = false;
                        }
                    }
                    let ticked = list.iter().filter(|c| c.selected).count();
                    ui.weak(format!("{ticked} of {} sheets", list.len()));
                });
                for c in list.iter_mut() {
                    let mut text = c.info.name.clone();
                    if c.info.hidden {
                        text.push_str(" (hidden)");
                    }
                    match c.info.kind {
                        SheetKind::Worksheet => {}
                        SheetKind::Chart => text.push_str(" (chart sheet, no cells)"),
                        SheetKind::Other => text.push_str(" (not a worksheet)"),
                    }
                    let usable = c.is_worksheet();
                    ui.add_enabled(usable, egui::Checkbox::new(&mut c.selected, text));
                }
            }
        }
    }

    fn header_page(&mut self, ui: &mut egui::Ui) {
        let detected = match &self.preview {
            Some(PreviewCache { result: Ok(p), .. }) => {
                p.layout.as_ref().map(|l| [l.header_lines, l.long_name_line, l.units_line, l.comments_line])
            }
            _ => None,
        };
        let o = &mut self.opts;
        egui::Grid::new("excel_header_lines").num_columns(2).spacing([12.0, 5.0]).show(ui, |ui| {
            header_line_rows(
                ui,
                HeaderLines {
                    header_lines: &mut o.header_lines,
                    long_name_line: &mut o.long_name_line,
                    units_line: &mut o.units_line,
                    comments_line: &mut o.comments_line,
                    short_name_line: &mut o.short_name_line,
                },
                detected,
            );
        });
        ui.add_space(4.0);
        ui.weak("Line numbers are the row numbers of the sheet in Excel.");
    }

    /// The cells of one sheet as Excel has them, with what each row becomes.
    fn preview_ui(&mut self, ui: &mut egui::Ui) {
        if let Some(Err(e)) = &self.files[self.current].sheets {
            ui.colored_label(Color32::from_rgb(170, 30, 30), format!("This file cannot be read: {e}"));
            return;
        }
        // The sheet to look at.
        let file = &mut self.files[self.current];
        if let Some(Ok(list)) = &file.sheets {
            let current = list.get(file.shown).map(|c| c.info.name.clone()).unwrap_or_default();
            let names: Vec<(usize, String)> = list
                .iter()
                .enumerate()
                .filter(|(_, c)| c.is_worksheet())
                .map(|(i, c)| (i, c.info.name.clone()))
                .collect();
            let shown = &mut file.shown;
            ui.horizontal(|ui| {
                ui.label("Preview of sheet");
                egui::ComboBox::from_id_salt("excel_preview_sheet").selected_text(current).show_ui(ui, |ui| {
                    for (i, name) in names {
                        ui.selectable_value(shown, i, name);
                    }
                });
            });
        }
        let preview = match &self.preview {
            Some(PreviewCache { result: Ok(p), .. }) => p,
            Some(PreviewCache { result: Err(e), .. }) => {
                ui.colored_label(Color32::from_rgb(170, 30, 30), e);
                return;
            }
            None => return,
        };
        let Some(layout) = &preview.layout else {
            ui.label("The sheet has no data.");
            return;
        };
        ui.weak(format!(
            "Imports {} columns and {} rows from column {}; rows are numbered as in Excel",
            layout.columns,
            layout.rows,
            column_letters(layout.first_column)
        ));
        let shown_columns = preview.rows.iter().map(Vec::len).max().unwrap_or(0).min(PREVIEW_COLUMNS);
        egui::ScrollArea::horizontal().id_salt("import_excel_preview_scroll").auto_shrink([false, false]).show(
            ui,
            |ui| {
                let mut table =
                    TableBuilder::new(ui).id_salt("import_excel_preview").striped(true).min_scrolled_height(0.0);
                table = table.column(TableColumn::auto().at_least(70.0));
                for _ in 0..shown_columns {
                    table = table.column(TableColumn::initial(96.0).at_least(50.0).clip(true));
                }
                table
                    .header(20.0, |mut header| {
                        header.col(|ui| {
                            ui.weak("Row");
                        });
                        for j in 0..shown_columns {
                            header.col(|ui| {
                                ui.label(RichText::new(column_letters(preview.first_column + j)).strong());
                            });
                        }
                    })
                    .body(|body| {
                        body.rows(18.0, preview.rows.len(), |mut row| {
                            let i = row.index();
                            let number = preview.first_row + i;
                            let tag = row_tag(layout, number);
                            let label_row = !tag.is_empty() && tag != "Data";
                            row.col(|ui| {
                                ui.label(RichText::new(format!("{number} {tag}")).weak());
                            });
                            for j in 0..shown_columns {
                                let text = preview.rows[i].get(j).cloned().unwrap_or_default();
                                row.col(|ui| {
                                    let text =
                                        if label_row { RichText::new(text).strong() } else { RichText::new(text) };
                                    ui.add(egui::Label::new(text).truncate());
                                });
                            }
                        });
                    });
            },
        );
    }
}

/// What the importer makes of the row `number` (an Excel row number).
fn row_tag(layout: &sheaf_io::excel::SheetLayout, number: usize) -> &'static str {
    if number == layout.long_name_line {
        "Name"
    } else if number == layout.units_line {
        "Units"
    } else if number == layout.comments_line {
        "Comment"
    } else if number <= layout.header_lines {
        "Header"
    } else {
        ""
    }
}

/// Excel's column letters: 1 is A, 27 is AA.
pub(super) fn column_letters(mut n: usize) -> String {
    let mut out = Vec::new();
    while n > 0 {
        n -= 1;
        out.push(b'A' + (n % 26) as u8);
        n /= 26;
    }
    out.reverse();
    String::from_utf8(out).unwrap_or_default()
}

/// The workbooks among `paths` (by name and first bytes: a text file named `.xls` is not one), and
/// the other files.
pub fn split_excel(paths: Vec<PathBuf>) -> (Vec<PathBuf>, Vec<PathBuf>) {
    paths.into_iter().partition(|p| sheaf_io::is_excel_workbook(Path::new(p)))
}

impl Themed for ExcelImportDialog {
    fn theme_row(&mut self) -> &mut ThemeRow {
        &mut self.themes
    }

    fn theme_row_ref(&self) -> &ThemeRow {
        &self.themes
    }

    fn list_names(&self, dir: &Path) -> Vec<String> {
        themes::names::<ExcelSettings>(dir)
    }

    fn apply_entry(&mut self, dir: &Path, entry: &str) -> Result<(), themes::ThemeError> {
        let s = themes::resolve::<ExcelSettings>(dir, entry)?;
        // The ticked sheets belong to the files, not to the theme.
        let sheets = std::mem::take(&mut self.opts.sheets);
        self.opts = ExcelOptions { sheets, ..s.options };
        self.partial_except_first = s.partial_except_first;
        self.mode = s.mode;
        self.plot = s.plot;
        Ok(())
    }

    fn store_as(&self, dir: &Path, name: &str) -> Result<String, themes::ThemeError> {
        themes::save_theme(dir, name, &self.settings())
    }

    fn remove_theme(&self, dir: &Path, name: &str) -> Result<(), IoError> {
        themes::delete_theme::<ExcelSettings>(dir, name).map(|_| ())
    }

    fn record_theme(&self, dir: &Path) -> Result<(), IoError> {
        themes::record_last_used(dir, &self.settings())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn column_letters_follow_excel() {
        assert_eq!(column_letters(1), "A");
        assert_eq!(column_letters(26), "Z");
        assert_eq!(column_letters(27), "AA");
        assert_eq!(column_letters(702), "ZZ");
        assert_eq!(column_letters(703), "AAA");
        assert_eq!(column_letters(16384), "XFD");
    }

    #[test]
    fn excel_files_are_told_apart_from_data_files_and_from_text_named_like_them() {
        let dir = std::env::temp_dir().join(format!("sheaf-split-excel-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let file = |name: &str, bytes: &[u8]| {
            let p = dir.join(name);
            std::fs::write(&p, bytes).unwrap();
            p
        };
        let zip = file("a.xlsx", b"PK\x03\x04rest");
        let ole = file("c.XLS", &[0xD0, 0xCF, 0x11, 0xE0, 1, 2]);
        let macro_book = file("d.xlsm", b"PK\x03\x04");
        let csv = file("b.csv", b"x,y\n1,2\n");
        let text = file("e.xls", b"x\ty\n1\t2\n3\t4\n");
        let (excel, other) = split_excel(vec![zip.clone(), csv.clone(), ole.clone(), macro_book.clone(), text.clone()]);
        assert_eq!(excel, vec![zip, ole, macro_book]);
        assert_eq!(other, vec![csv, text]);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
