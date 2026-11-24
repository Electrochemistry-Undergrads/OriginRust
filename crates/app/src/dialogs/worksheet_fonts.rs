//! Worksheet Properties: formatting defaults and display choices (T59).
use super::cell_format::optional_color;
use super::frame::{DialogButton, ok_cancel_apply};
use eframe::egui;
use sheaf_core::cellformat::{CategoryColors, LabelColors, LabelKind};
use sheaf_core::color::Color;
use sheaf_core::sheet_fonts::ColumnLabelFont;
use sheaf_core::sheet_view::SheetView;
use sheaf_core::{Document, WindowId};

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
    view_tab: bool,
    category: usize,
    fonts: Vec<String>,
    font_search: String,
}

impl WorksheetFontsDialog {
    pub fn new(doc: &Document, book: WindowId) -> Option<Self> {
        let b = doc.project().workbook(book)?;
        let sheet = b.active_sheet.min(b.sheets.len().checked_sub(1)?);
        let worksheet = &b.sheets[sheet];
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
            view_tab: false,
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
        self.shown != self.base || self.shown_colors != self.base_colors || self.shown_view != self.base_view
    }

    fn apply(&mut self, doc: &mut Document) {
        if !self.changed() || !self.current(doc) {
            return;
        }
        let mut layout = self.shown_view;
        layout.show_missing_as_blank = self.base_view.show_missing_as_blank;
        let label =
            match (self.shown != self.base, self.shown_colors != self.base_colors, self.shown_view != self.base_view) {
                (true, false, false) => "Worksheet font",
                (false, true, false) => "Worksheet colors",
                (false, false, true) if layout == self.base_view => "Worksheet missing display",
                (false, false, true) => "Worksheet view",
                _ => "Worksheet formatting",
            };
        doc.apply(label, |p| {
            let sheet = &mut p.workbook_mut(self.book).unwrap().sheets[self.sheet];
            sheet.view = self.shown_view;
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
        self.base_view = self.shown_view;
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
        let mut open = true;
        let mut pressed = None;
        egui::Window::new("Worksheet Properties")
            .id(egui::Id::new("worksheet_fonts"))
            .open(&mut open)
            .auto_sized()
            .min_width(340.0)
            .max_width(340.0)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.view_tab, false, "Format");
                    ui.selectable_value(&mut self.view_tab, true, "View");
                });
                ui.separator();
                if self.view_tab {
                    self.show_view(ui);
                } else {
                    self.show_format(ui);
                }
                ui.add_space(6.0);
                ui.allocate_ui_with_layout(
                    egui::vec2(340.0, ui.spacing().interact_size.y),
                    egui::Layout::right_to_left(egui::Align::Center),
                    |ui| {
                        pressed = ok_cancel_apply(ui, self.changed());
                    },
                );
            });
        if !open || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            return false;
        }
        match pressed {
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
