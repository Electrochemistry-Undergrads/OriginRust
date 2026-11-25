//! Worksheet Properties: formatting defaults and display choices (T59).
use super::cell_format::optional_color;
use super::frame::{DialogButton, ok_cancel_apply};
use eframe::egui;
use sheaf_core::cellformat::{CategoryColors, LabelColors, LabelKind};
use sheaf_core::color::Color;
use sheaf_core::sheet_fonts::ColumnLabelFont;
use sheaf_core::sheet_view::SheetView;
use sheaf_core::{Document, Project, WindowId, Worksheet};

const DATA: usize = 0;
const LONG_NAME: usize = 1;
const UNITS: usize = 2;
const COMMENTS: usize = 3;
const BULK: usize = 4;
const COLUMN_HEADER: usize = 5;
const ROW_HEADER: usize = 6;
const STANDARD: usize = 7;
const FORMULA: usize = 8;
const CATEGORIES: [&str; 9] =
    ["Data", "Long Name", "Units", "Comments", "Column Labels", "Column Header", "Row Header", "Standard", "F(x)="];

const MAX_SIZE_ROWS: usize = 1_048_576;

#[derive(Clone, Copy, PartialEq, Eq)]
enum PropertiesTab {
    Format,
    View,
    Size,
    Miscellaneous,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct SheetSize {
    rows: usize,
    columns: usize,
}

impl SheetSize {
    fn new(sheet: &Worksheet) -> Self {
        Self { rows: crate::grid::shown_rows(sheet), columns: sheet.columns.len() }
    }
}

fn measurement(ui: &mut egui::Ui, name: &str, percent: &mut Option<u16>, automatic: u16, minimum: u16) {
    let label = ui.label(name);
    let mut value = percent.unwrap_or(automatic);
    let limit = 10_000.max(value);
    let response = ui.add(egui::DragValue::new(&mut value).range(minimum..=limit).suffix("%")).labelled_by(label.id);
    if response.changed() {
        *percent = Some(value);
    }
    ui.end_row();
}

pub struct WorksheetFontsDialog {
    book: WindowId,
    sheet: usize,
    generation: u64,
    revision: u64,
    base: [ColumnLabelFont; CATEGORIES.len()],
    shown: [ColumnLabelFont; CATEGORIES.len()],
    base_colors: [CategoryColors; CATEGORIES.len()],
    shown_colors: [CategoryColors; CATEGORIES.len()],
    base_view: SheetView,
    shown_view: SheetView,
    tab: PropertiesTab,
    base_size: SheetSize,
    shown_size: SheetSize,
    viewport_rows: usize,
    category: usize,
    fonts: Vec<String>,
    font_search: String,
}

impl WorksheetFontsDialog {
    pub fn new(doc: &Document, book: WindowId, grid: Option<&crate::grid::BookUi>) -> Option<Self> {
        let b = doc.project().workbook(book)?;
        let sheet = b.active_sheet.min(b.sheets.len().checked_sub(1)?);
        let worksheet = &b.sheets[sheet];
        let size = SheetSize::new(worksheet);
        let defaults = &worksheet.font_defaults;
        let base = [
            defaults.data.clone().unwrap_or_default(),
            ColumnLabelFont {
                family: defaults.long_name_family.clone().unwrap_or_default(),
                size_pt: defaults.long_name_size_pt,
            },
            ColumnLabelFont {
                family: defaults.units_family.clone().unwrap_or_default(),
                size_pt: defaults.units_size_pt,
            },
            ColumnLabelFont {
                family: defaults.comments_family.clone().unwrap_or_default(),
                size_pt: defaults.comments_size_pt,
            },
            defaults.column_labels.clone(),
            defaults.column_header.clone(),
            defaults.row_header.clone(),
            defaults.standard.clone(),
            ColumnLabelFont {
                family: defaults.formula_family.clone().unwrap_or_default(),
                size_pt: defaults.formula_size_pt,
            },
        ];
        let labels = worksheet.label_colors;
        let colors = [
            worksheet.data_colors,
            labels.long_name,
            labels.units,
            labels.comments,
            worksheet.column_label_colors,
            labels.column_header,
            labels.row_header,
            worksheet.standard_colors,
            labels.formula,
        ];
        let mut fonts: Vec<_> = sheaf_plot::SystemFonts::shared()
            .database()
            .faces()
            .filter_map(|f| f.families.first().map(|(n, _)| n.clone()))
            .collect();
        fonts.sort_by_key(|n| n.to_lowercase());
        fonts.dedup();
        Some(Self {
            book,
            sheet,
            generation: doc.generation(),
            revision: doc.revision(),
            shown: base.clone(),
            base,
            base_colors: colors,
            shown_colors: colors,
            base_view: worksheet.view,
            shown_view: worksheet.view,
            tab: PropertiesTab::Format,
            base_size: size,
            shown_size: size,
            viewport_rows: grid.map_or(1, |grid| grid.viewport_min_rows(doc.generation(), sheet)),
            category: DATA,
            fonts,
            font_search: String::new(),
        })
    }

    fn current(&self, doc: &Document) -> bool {
        self.generation == doc.generation()
            && self.revision == doc.revision()
            && doc
                .project()
                .workbook(self.book)
                .is_some_and(|b| b.active_sheet == self.sheet && b.sheets.get(self.sheet).is_some())
    }

    fn changed(&self) -> bool {
        self.shown != self.base
            || self.shown_colors != self.base_colors
            || self.shown_view != self.base_view
            || self.shown_size != self.base_size
    }

    fn rows_locked(&self, sheet: &Worksheet) -> bool {
        self.shown_size.rows < self.base_size.rows
            && sheet
                .columns
                .iter()
                .take(self.shown_size.columns)
                .any(|column| column.is_locked() && column.len() > self.shown_size.rows)
    }

    fn resize(&self, project: &mut Project) {
        let sheet = &project.workbook(self.book).unwrap().sheets[self.sheet];
        let removed: Vec<_> = sheet.columns.iter().skip(self.shown_size.columns).map(|column| column.id).collect();
        crate::app::delete_columns(project, &removed);
        let added = self.shown_size.columns.saturating_sub(self.base_size.columns);
        if added > 0 {
            project
                .insert_columns(self.book, self.sheet, self.base_size.columns, added)
                .expect("worksheet still exists");
        }
        let shrinking_rows = self.shown_size.rows < self.base_size.rows;
        if shrinking_rows {
            let sheet = &mut project.workbook_mut(self.book).unwrap().sheets[self.sheet];
            let columns: Vec<_> = (0..sheet.columns.len()).collect();
            sheet.delete_rows(&columns, self.shown_size.rows..self.base_size.rows);
        }
        if shrinking_rows || !removed.is_empty() {
            let _ = project.reapply_row_filter(self.book, self.sheet);
        }
    }

    fn apply(&mut self, doc: &mut Document) {
        if !self.changed() || !self.current(doc) {
            return;
        }
        let headers = self.shown_view.header_measurements;
        let previous_headers = self.base_view.header_measurements;
        let cells = self.shown_view.cell_measurements;
        let previous_cells = self.base_view.cell_measurements;
        let automatic_rows = self.shown_view.automatic_rows();
        let auto_add_rows = self.shown_view.auto_add_rows;
        let mut view = self.shown_view;
        view.auto_add_rows = self.base_view.auto_add_rows;
        view.header_measurements = previous_headers;
        view.cell_measurements = previous_cells;
        let mut layout = view;
        layout.show_missing_as_blank = self.base_view.show_missing_as_blank;
        let label = match (
            self.shown != self.base,
            self.shown_colors != self.base_colors,
            view != self.base_view,
            self.shown_size != self.base_size
                || headers != previous_headers
                || cells != previous_cells
                || auto_add_rows != self.base_view.auto_add_rows,
        ) {
            (true, false, false, false) => "Worksheet font",
            (false, true, false, false) => "Worksheet colors",
            (false, false, true, false) if layout == self.base_view => "Worksheet missing display",
            (false, false, true, false) => "Worksheet view",
            (false, false, false, true) => "Worksheet size",
            _ => "Worksheet formatting",
        };
        doc.apply(label, |p| {
            if self.shown_size != self.base_size {
                self.resize(p);
            }
            let sheet = &mut p.workbook_mut(self.book).unwrap().sheets[self.sheet];
            sheet.view = view;
            sheet.view.auto_add_rows = auto_add_rows;
            sheet.view.header_measurements.top_gap_pt = headers.top_gap_pt;
            if self.shown_size.rows != self.base_size.rows {
                // Origin clears a zero-row sheet and leaves two blank rows.
                let rows = if self.shown_size.rows == 0 { 2 } else { self.shown_size.rows };
                sheet.view.allocated_rows = Some(if automatic_rows { rows.max(self.viewport_rows) } else { rows });
            } else if automatic_rows && !self.base_view.automatic_rows() && self.shown_size.rows < self.viewport_rows {
                sheet.view.allocated_rows = Some(self.viewport_rows);
            } else if (auto_add_rows == Some(false) && sheet.view.allocated_rows.is_none())
                || (self.shown_size.columns < self.base_size.columns
                    && crate::grid::shown_rows(sheet) < self.shown_size.rows)
            {
                sheet.view.allocated_rows = Some(self.shown_size.rows);
            }
            if automatic_rows && sheet.view.allocated_rows.is_some() {
                sheet.view.auto_add_rows = Some(true);
            }
            sheet.standard_colors = self.shown_colors[STANDARD];
            sheet.data_colors = self.shown_colors[DATA];
            sheet.column_label_colors = self.shown_colors[BULK];
            sheet.label_colors = LabelColors {
                long_name: self.shown_colors[LONG_NAME],
                units: self.shown_colors[UNITS],
                comments: self.shown_colors[COMMENTS],
                formula: self.shown_colors[FORMULA],
                column_header: self.shown_colors[COLUMN_HEADER],
                row_header: self.shown_colors[ROW_HEADER],
            };
            if self.shown[DATA] != self.base[DATA] {
                sheet.set_data_font_override(self.shown[DATA].clone());
            }
            if self.shown[BULK] != self.base[BULK] {
                sheet.set_column_labels_font(self.shown[BULK].clone());
            }
            if self.shown[COLUMN_HEADER] != self.base[COLUMN_HEADER] {
                sheet.set_column_header_font(self.shown[COLUMN_HEADER].clone());
            }
            if self.shown[ROW_HEADER] != self.base[ROW_HEADER] {
                sheet.set_row_header_font(self.shown[ROW_HEADER].clone());
            }
            if self.shown[STANDARD] != self.base[STANDARD] {
                sheet.set_standard_font(self.shown[STANDARD].clone());
            }
            // Explicit measurements use the displayed units even when applying a font change.
            if headers.column_height != previous_headers.column_height {
                sheet.view.header_measurements.column_height = headers.column_height;
            }
            if headers.row_width != previous_headers.row_width {
                sheet.view.header_measurements.row_width = headers.row_width;
            }
            if cells.row_height != previous_cells.row_height {
                sheet.view.cell_measurements.row_height = cells.row_height;
                sheet.view.fitted_cells.clear_row_height();
            }
            if cells.column_width != previous_cells.column_width {
                sheet.view.cell_measurements.column_width = cells.column_width;
                sheet.view.fitted_cells.clear_column_width();
                for column in &mut sheet.columns {
                    column.width = None;
                }
            }
            for (i, kind) in [
                (LONG_NAME, LabelKind::LongName),
                (UNITS, LabelKind::Units),
                (COMMENTS, LabelKind::Comments),
                (FORMULA, LabelKind::Formula),
            ] {
                if self.shown[i].size_pt != self.base[i].size_pt {
                    sheet.set_label_font_size_override(&kind, self.shown[i].size_pt);
                }
                if self.shown[i].family != self.base[i].family {
                    sheet.set_label_font_family(&kind, &self.shown[i].family);
                }
            }
        });
        self.base = self.shown.clone();
        self.base_colors = self.shown_colors;
        let sheet = &doc.project().workbook(self.book).unwrap().sheets[self.sheet];
        self.base_view = sheet.view;
        self.shown_view = sheet.view;
        self.base_size = SheetSize::new(sheet);
        self.shown_size = self.base_size;
        self.revision = doc.revision();
    }

    fn show_view(&mut self, ui: &mut egui::Ui) {
        let view = &mut self.shown_view;
        ui.checkbox(&mut view.row_labels, "Row Labels for Column Label Rows");
        ui.separator();
        ui.label("Headers");
        ui.checkbox(&mut view.column_header, "Column Header");
        ui.checkbox(&mut view.row_header, "Row Header");
        ui.separator();
        ui.label("Grid Lines");
        ui.checkbox(&mut view.column_grid, "Column Grid");
        ui.checkbox(&mut view.row_grid, "Row Grid");
        ui.checkbox(&mut view.page_breaks, "Page Break Preview Lines");
    }

    fn show_miscellaneous(&mut self, ui: &mut egui::Ui) {
        let mut automatic = self.shown_view.automatic_rows();
        if ui.checkbox(&mut automatic, "Auto Add Rows").changed() {
            self.shown_view.auto_add_rows = if automatic == self.base_view.automatic_rows() {
                self.base_view.auto_add_rows
            } else {
                Some(automatic)
            };
        }
        ui.checkbox(&mut self.shown_view.fit_sheet_to_window, "Fit Sheet to Window");
    }

    fn show_size(&mut self, ui: &mut egui::Ui, sheet: &Worksheet) {
        let row_limit = MAX_SIZE_ROWS.max(self.base_size.rows);
        let column_limit = self.base_size.columns.saturating_add(crate::app::MAX_ADDED_COLUMNS);
        egui::Grid::new("worksheet_size_grid").num_columns(2).show(ui, |ui| {
            let rows = ui.label("Row Number");
            ui.add(egui::DragValue::new(&mut self.shown_size.rows).range(0..=row_limit)).labelled_by(rows.id);
            ui.end_row();
            let columns = ui.label("Column Number");
            ui.add(egui::DragValue::new(&mut self.shown_size.columns).range(0..=column_limit)).labelled_by(columns.id);
            ui.end_row();
        });
        if self.shown_size.rows < sheet.row_count() || self.shown_size.columns < self.base_size.columns {
            ui.label("Applying this size removes trailing rows or columns.");
        }

        ui.separator();
        ui.label("Worksheet Measurement");
        let headers = &mut self.shown_view.header_measurements;
        let cells = &mut self.shown_view.cell_measurements;
        let standard = (100.0 * sheet.font_defaults.size_scale()).round() as u16;
        egui::Grid::new("worksheet_measurement_grid").num_columns(2).show(ui, |ui| {
            let height = crate::worksheet_layout::automatic_column_header_height(ui.ctx(), sheet);
            let width = crate::grid::automatic_row_header_width(sheet);
            measurement(
                ui,
                "Column Header Height",
                &mut headers.column_height,
                (100.0 * height / crate::grid::ROW_HEIGHT).round() as u16,
                0,
            );
            measurement(
                ui,
                "Row Header Width",
                &mut headers.row_width,
                (100.0 * width / crate::grid::COLUMN_WIDTH).round() as u16,
                0,
            );
            let gap = ui.label("Gap From Top");
            ui.add(egui::DragValue::new(&mut headers.top_gap_pt).range(0..=u8::MAX).suffix(" pt")).labelled_by(gap.id);
            ui.end_row();
            measurement(ui, "All Rows Height", &mut cells.row_height, standard, 1);
            measurement(ui, "All Columns Width", &mut cells.column_width, standard, 1);
        });
    }

    fn show_format(&mut self, ui: &mut egui::Ui) {
        egui::Grid::new("worksheet_fonts_grid").num_columns(2).spacing([12.0, 8.0]).show(ui, |ui| {
            ui.label("Apply To");
            egui::ComboBox::from_id_salt("worksheet_font_category").selected_text(CATEGORIES[self.category]).show_ui(
                ui,
                |ui| {
                    for (i, name) in CATEGORIES.iter().enumerate() {
                        ui.selectable_value(&mut self.category, i, *name);
                    }
                },
            );
            ui.end_row();
            ui.label("Font");
            let shown = &mut self.shown[self.category];
            let family = if shown.family.is_empty() { "Automatic".to_owned() } else { shown.family.clone() };
            ui.scope(|ui| {
                egui::ComboBox::from_id_salt("worksheet_default_family")
                    .selected_text(family)
                    .width(180.0)
                    .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                    .show_ui(ui, |ui| {
                        ui.add(egui::TextEdit::singleline(&mut self.font_search).hint_text("Find font")).widget_info(
                            || egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, "Worksheet font search"),
                        );
                        let search = self.font_search.trim().to_lowercase();
                        if ui.selectable_value(&mut shown.family, String::new(), "Automatic").clicked() {
                            ui.close();
                        }
                        for name in self.fonts.iter().filter(|name| name.to_lowercase().contains(&search)) {
                            if ui.selectable_value(&mut shown.family, name.clone(), name).clicked() {
                                ui.close();
                            }
                        }
                    });
            });
            ui.end_row();
            ui.label("Size (pt)");
            let mut size = shown.size_pt.unwrap_or(0.0);
            let value = egui::DragValue::new(&mut size)
                .speed(0.25)
                .range(0.0..=96.0)
                .custom_formatter(|v, _| if v == 0.0 { "Default".into() } else { format!("{v:.1}") })
                .custom_parser(|s| {
                    let s = s.trim();
                    if s.eq_ignore_ascii_case("default") { Some(0.0) } else { s.parse::<f64>().ok() }
                });
            let response = ui.add(value);
            response
                .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::DragValue, true, "Worksheet font size"));
            if response.changed() {
                shown.size_pt = (size > 0.0).then_some(size.max(4.0));
            }
            ui.end_row();
            let category = CATEGORIES[self.category];
            let colors = &mut self.shown_colors[self.category];
            ui.label("Text Color");
            optional_color(ui, &mut colors.text_color, "Use color", Color::BLACK, &format!("{category} text color"));
            ui.end_row();
            ui.label("Fill Color");
            optional_color(
                ui,
                &mut colors.fill,
                "Use fill",
                Color::rgb(255, 255, 153),
                &format!("{category} fill color"),
            );
            ui.end_row();
            if self.category == DATA {
                let mut blank = self.shown_view.show_missing_as_blank.unwrap_or(false);
                let check = egui::Checkbox::new(&mut blank, "Show Missing as Blank")
                    .indeterminate(self.shown_view.show_missing_as_blank.is_none());
                if ui.add(check).changed() {
                    self.shown_view.show_missing_as_blank = Some(blank);
                }
                ui.end_row();
            }
        });
        ui.add_space(6.0);
        ui.label("Manual cell formats take precedence over category defaults.");
    }

    pub fn show(&mut self, ctx: &egui::Context, doc: &mut Document) -> bool {
        if !self.current(doc) {
            return false;
        }
        let sheet = &doc.project().workbook(self.book).unwrap().sheets[self.sheet];
        let has_grid = !sheet.is_notes() && sheet.graph_sheet.is_none();
        let mut open = true;
        let mut pressed = None;
        let mut rows_locked = false;
        egui::Window::new("Worksheet Properties")
            .id(egui::Id::new("worksheet_fonts"))
            .open(&mut open)
            .auto_sized()
            .min_width(340.0)
            .max_width(340.0)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.tab, PropertiesTab::Format, "Format");
                    ui.selectable_value(&mut self.tab, PropertiesTab::View, "View");
                    if has_grid {
                        ui.selectable_value(&mut self.tab, PropertiesTab::Size, "Size");
                        ui.selectable_value(&mut self.tab, PropertiesTab::Miscellaneous, "Miscellaneous");
                    }
                });
                ui.separator();
                match self.tab {
                    PropertiesTab::Format => self.show_format(ui),
                    PropertiesTab::View => self.show_view(ui),
                    PropertiesTab::Size => self.show_size(ui, sheet),
                    PropertiesTab::Miscellaneous => self.show_miscellaneous(ui),
                }
                rows_locked = self.rows_locked(sheet);
                if rows_locked {
                    ui.label("A locked column extends beyond the requested row count.");
                }
                ui.add_space(6.0);
                ui.allocate_ui_with_layout(
                    egui::vec2(340.0, ui.spacing().interact_size.y),
                    egui::Layout::right_to_left(egui::Align::Center),
                    |ui| {
                        pressed = ok_cancel_apply(ui, self.changed() && !rows_locked);
                    },
                );
            });
        if !open || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            return false;
        }
        match pressed {
            Some(DialogButton::Ok) if rows_locked => true,
            Some(DialogButton::Ok) => {
                self.apply(doc);
                false
            }
            Some(DialogButton::Apply) => {
                self.apply(doc);
                true
            }
            Some(DialogButton::Cancel) => false,
            None => open,
        }
    }
}
