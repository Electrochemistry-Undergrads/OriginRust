//! Worksheet Properties: category defaults below manual cell/range formats (T59).
use super::frame::{DialogButton, ok_cancel_apply};
use eframe::egui;
use sheaf_core::cellformat::LabelKind;
use sheaf_core::sheet_fonts::{ColumnLabelFont, WorksheetFont};
use sheaf_core::{Document, WindowId};

const DATA: usize = 0;
const BULK: usize = 4;
const COLUMN_HEADER: usize = 5;
const ROW_HEADER: usize = 6;
const CATEGORIES: [&str; 7] =
    ["Data", "Long Name", "Units", "Comments", "Column Labels", "Column Header", "Row Header"];

pub struct WorksheetFontsDialog {
    book: WindowId,
    sheet: usize,
    generation: u64,
    revision: u64,
    base: [ColumnLabelFont; CATEGORIES.len()],
    shown: [ColumnLabelFont; CATEGORIES.len()],
    category: usize,
    fonts: Vec<String>,
    font_search: String,
}

impl WorksheetFontsDialog {
    pub fn new(doc: &Document, book: WindowId) -> Option<Self> {
        let b = doc.project().workbook(book)?;
        let sheet = b.active_sheet.min(b.sheets.len().checked_sub(1)?);
        let defaults = &b.sheets[sheet].font_defaults;
        let data = defaults.data.clone().unwrap_or_default();
        let base = [
            ColumnLabelFont { family: data.family, size_pt: Some(data.size_pt) },
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

    fn apply(&mut self, doc: &mut Document) {
        if self.shown == self.base || !self.current(doc) {
            return;
        }
        doc.apply("Worksheet font", |p| {
            let sheet = &mut p.workbook_mut(self.book).unwrap().sheets[self.sheet];
            if self.shown[DATA] != self.base[DATA] {
                let data = &self.shown[DATA];
                sheet
                    .set_data_font(WorksheetFont { family: data.family.clone(), size_pt: data.size_pt.unwrap_or(9.0) });
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
            for (i, kind) in [LabelKind::LongName, LabelKind::Units, LabelKind::Comments].iter().enumerate() {
                if self.shown[i + 1].size_pt != self.base[i + 1].size_pt {
                    sheet.set_label_font_size_override(kind, self.shown[i + 1].size_pt);
                }
                if self.shown[i + 1].family != self.base[i + 1].family {
                    sheet.set_label_font_family(kind, &self.shown[i + 1].family);
                }
            }
        });
        self.base = self.shown.clone();
        self.revision = doc.revision();
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
                ui.label("Format");
                egui::Grid::new("worksheet_fonts_grid").num_columns(2).spacing([12.0, 8.0]).show(ui, |ui| {
                    ui.label("Apply To");
                    egui::ComboBox::from_id_salt("worksheet_font_category")
                        .selected_text(CATEGORIES[self.category])
                        .show_ui(ui, |ui| {
                            for (i, name) in CATEGORIES.iter().enumerate() {
                                ui.selectable_value(&mut self.category, i, *name);
                            }
                            ui.add_enabled_ui(false, |ui| {
                                for name in ["Standard", "F(x)="] {
                                    let _ =
                                        ui.selectable_label(false, name).on_hover_text("Not available in Sheaf yet");
                                }
                            });
                        });
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
                                ui.add(egui::TextEdit::singleline(&mut self.font_search).hint_text("Find font"))
                                    .widget_info(|| {
                                        egui::WidgetInfo::labeled(
                                            egui::WidgetType::TextEdit,
                                            true,
                                            "Worksheet font search",
                                        )
                                    });
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
                    let value = egui::DragValue::new(&mut size).speed(0.25);
                    let value = if self.category == DATA {
                        value.range(4.0..=96.0)
                    } else {
                        value
                            .range(0.0..=96.0)
                            .custom_formatter(|v, _| if v == 0.0 { "Default".into() } else { format!("{v:.1}") })
                            .custom_parser(|s| {
                                let s = s.trim();
                                if s.eq_ignore_ascii_case("default") { Some(0.0) } else { s.parse::<f64>().ok() }
                            })
                    };
                    let response = ui.add(value);
                    response.widget_info(|| {
                        egui::WidgetInfo::labeled(egui::WidgetType::DragValue, true, "Worksheet font size")
                    });
                    if response.changed() {
                        shown.size_pt = (size > 0.0).then_some(size.max(4.0));
                    }
                    ui.end_row();
                });
                ui.add_space(6.0);
                ui.label("Manual cell formats take precedence over category defaults.");
                ui.allocate_ui_with_layout(
                    egui::vec2(340.0, ui.spacing().interact_size.y),
                    egui::Layout::right_to_left(egui::Align::Center),
                    |ui| {
                        pressed = ok_cancel_apply(ui, self.shown != self.base);
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
