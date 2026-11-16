//! Worksheet Properties: Data fonts, below manual cell/range formats (T59).
use super::frame::{DialogButton, ok_cancel_apply};
use eframe::egui;
use sheaf_core::sheet_fonts::WorksheetFont;
use sheaf_core::{Document, WindowId};

pub struct WorksheetFontsDialog {
    book: WindowId,
    sheet: usize,
    generation: u64,
    revision: u64,
    base: WorksheetFont,
    shown: WorksheetFont,
    fonts: Vec<String>,
    font_search: String,
}

impl WorksheetFontsDialog {
    pub fn new(doc: &Document, book: WindowId) -> Option<Self> {
        let b = doc.project().workbook(book)?;
        let sheet = b.active_sheet.min(b.sheets.len().checked_sub(1)?);
        let base = b.sheets[sheet].font_defaults.data.clone().unwrap_or_default();
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
        let font = self.shown.clone();
        doc.apply("Worksheet font", |p| {
            p.workbook_mut(self.book).unwrap().sheets[self.sheet].set_data_font(font);
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
                    egui::ComboBox::from_id_salt("worksheet_font_category").selected_text("Data").show_ui(ui, |ui| {
                        let _ = ui
                            .selectable_label(true, "Data")
                            .on_hover_text("All current and future data cells in this sheet");
                        ui.add_enabled_ui(false, |ui| {
                            for name in [
                                "Standard",
                                "Column Labels",
                                "Column Header",
                                "Row Header",
                                "Long Name",
                                "Units",
                                "Comments",
                                "F(x)=",
                            ] {
                                let _ = ui.selectable_label(false, name).on_hover_text("Not available in Sheaf yet");
                            }
                        });
                    });
                    ui.end_row();
                    ui.label("Font");
                    let family = if self.shown.family.is_empty() { "Automatic" } else { &self.shown.family };
                    egui::ComboBox::from_id_salt("worksheet_default_family")
                        .selected_text(family)
                        .width(180.0)
                        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                        .show_ui(ui, |ui| {
                            ui.add(egui::TextEdit::singleline(&mut self.font_search).hint_text("Find font"))
                                .widget_info(|| {
                                    egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, "Worksheet font search")
                                });
                            let search = self.font_search.trim().to_lowercase();
                            if ui.selectable_value(&mut self.shown.family, String::new(), "Automatic").clicked() {
                                ui.close();
                            }
                            for name in self.fonts.iter().filter(|name| name.to_lowercase().contains(&search)) {
                                if ui.selectable_value(&mut self.shown.family, name.clone(), name).clicked() {
                                    ui.close();
                                }
                            }
                        });
                    ui.end_row();
                    ui.label("Size (pt)");
                    ui.add(egui::DragValue::new(&mut self.shown.size_pt).range(4.0..=96.0).speed(0.25)).widget_info(
                        || egui::WidgetInfo::labeled(egui::WidgetType::DragValue, true, "Worksheet font size"),
                    );
                    ui.end_row();
                });
                ui.add_space(6.0);
                ui.label("Manual cell fonts and sizes take precedence.");
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
