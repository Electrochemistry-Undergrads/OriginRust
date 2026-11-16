//! Format Cells: the window that sets the font, size, bold/italic/underline, text and fill colour
//! and horizontal alignment of the selected data cells, or of a selected label cell (T59).
//! Sheaf's manual-style layout is its own. Origin 2016's extra text-control and numeric fields
//! were measured separately; they are not implemented here. See docs/spec/gui-look.md (T59).
//!
//! The window starts from the look of the first selected cell and applies only what the user
//! changed, so a mixed selection keeps what it had. OK and Apply are one undo step each; Cancel
//! and the close box change nothing. "Clear Formats" removes the formats of the selection.

use super::frame::{DialogButton, ok_cancel_apply};
use super::graph_props::color_button;
use eframe::egui;
use sheaf_core::cellformat::{CellFormat, FormatTarget, HAlign, ResolvedStyle};
use sheaf_core::color::Color;
use sheaf_core::{Document, WindowId};

/// The font names offered when no installed font list is available (and the first ones of it).
const COMMON_FONTS: [&str; 8] =
    ["Arial", "Times New Roman", "Courier New", "Verdana", "Calibri", "Consolas", "Segoe UI", "Tahoma"];

/// What the controls hold. `None` colours are "automatic" (the sheet's own colours).
#[derive(Clone, Debug, PartialEq)]
struct Edit {
    /// Empty is the default font.
    font: String,
    /// 0 is the default size.
    size: f32,
    bold: bool,
    italic: bool,
    underline: bool,
    text: Option<Color>,
    fill: Option<Color>,
    align: HAlign,
}

impl Edit {
    fn of(style: &ResolvedStyle<'_>) -> Edit {
        Edit {
            font: style.font.unwrap_or_default().to_string(),
            size: style.size_pt.unwrap_or(0.0),
            bold: style.bold,
            italic: style.italic,
            underline: style.underline,
            text: style.text_color,
            fill: style.fill,
            align: style.h_align.unwrap_or(HAlign::Auto),
        }
    }

    /// The properties that differ from `from`, as a format to apply.
    fn changes_from(&self, from: &Edit) -> CellFormat {
        CellFormat {
            bold: (self.bold != from.bold).then_some(self.bold),
            italic: (self.italic != from.italic).then_some(self.italic),
            underline: (self.underline != from.underline).then_some(self.underline),
            font: (self.font != from.font).then(|| self.font.clone()),
            size_pt: (self.size != from.size).then_some(self.size),
            text_color: (self.text != from.text).then(|| self.text.unwrap_or(Color::TRANSPARENT)),
            fill: (self.fill != from.fill).then(|| self.fill.unwrap_or(Color::TRANSPARENT)),
            h_align: (self.align != from.align).then_some(self.align),
        }
    }
}

pub struct CellFormatDialog {
    pub book: WindowId,
    sheet: usize,
    target: FormatTarget,
    shown: Edit,
    /// What the sheet had when the window opened or Apply was last pressed.
    base: Edit,
    fonts: Vec<String>,
}

/// The installed font families, sorted, or a short list when none can be read.
fn installed_fonts() -> Vec<String> {
    let db = sheaf_plot::SystemFonts::shared().database();
    let mut names: Vec<String> = db.faces().filter_map(|f| f.families.first().map(|(n, _)| n.clone())).collect();
    names.sort_by_key(|n| n.to_lowercase());
    names.dedup();
    if names.is_empty() {
        names = COMMON_FONTS.iter().map(|s| s.to_string()).collect();
    }
    names
}

impl CellFormatDialog {
    /// The window for `target`, starting from the look of the first cell of it.
    pub fn new(doc: &Document, book: WindowId, target: FormatTarget) -> Option<Self> {
        let b = doc.project().workbook(book)?;
        let sheet = b.active_sheet.min(b.sheets.len().checked_sub(1)?);
        let ws = &b.sheets[sheet];
        let first = *target.columns().first()?;
        let ix = ws.format_index();
        let style = match &target {
            FormatTarget::Cells { rows, .. } => ix.resolve(ws, first, rows.map_or(0, |r| r.0)),
            FormatTarget::LabelRows { rows, .. } => ix.resolve_label(ws, first, rows.first()?),
        };
        let shown = Edit::of(&style);
        Some(CellFormatDialog { book, sheet, target, base: shown.clone(), shown, fonts: installed_fonts() })
    }

    fn apply(&mut self, doc: &mut Document) {
        let change = self.shown.changes_from(&self.base);
        if change.is_empty() {
            return;
        }
        let (book, sheet, target) = (self.book, self.sheet, self.target.clone());
        doc.apply("Format cells", |p| {
            if let Some(s) = p.workbook_mut(book).and_then(|b| b.sheets.get_mut(sheet)) {
                s.apply_cell_format(target, change);
            }
        });
        self.base = self.shown.clone();
    }

    fn clear(&self, doc: &mut Document) {
        let (book, sheet, target) = (self.book, self.sheet, self.target.clone());
        let has =
            doc.project().workbook(book).and_then(|b| b.sheets.get(sheet)).is_some_and(|s| !s.cell_formats.is_empty());
        if has {
            doc.apply("Clear formats", |p| {
                if let Some(s) = p.workbook_mut(book).and_then(|b| b.sheets.get_mut(sheet)) {
                    s.clear_cell_formats(&target);
                }
            });
        }
    }

    /// Draws the window; false when it closed.
    pub fn show(&mut self, ctx: &egui::Context, doc: &mut Document) -> bool {
        if doc.project().workbook(self.book).and_then(|b| b.sheets.get(self.sheet)).is_none() {
            return false;
        }
        let mut open = true;
        let mut pressed = None;
        let mut clear = false;
        egui::Window::new("Format Cells")
            .id(egui::Id::new("cell_format"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                egui::Grid::new("cell_format_grid").num_columns(2).spacing([8.0, 6.0]).show(ui, |ui| {
                    ui.label("Font");
                    let shown = if self.shown.font.is_empty() { "Default" } else { self.shown.font.as_str() };
                    egui::ComboBox::from_id_salt("cell_format_font").selected_text(shown).width(180.0).show_ui(
                        ui,
                        |ui| {
                            ui.selectable_value(&mut self.shown.font, String::new(), "Default");
                            for name in &self.fonts {
                                ui.selectable_value(&mut self.shown.font, name.clone(), name);
                            }
                        },
                    );
                    ui.end_row();
                    ui.label("Size");
                    ui.add(
                        egui::DragValue::new(&mut self.shown.size)
                            .range(0.0..=96.0)
                            .speed(0.25)
                            .custom_formatter(|v, _| if v < 0.05 { "Default".into() } else { format!("{v:.1} pt") })
                            .custom_parser(|s| {
                                let t = s.trim();
                                if t.eq_ignore_ascii_case("default") {
                                    Some(0.0)
                                } else {
                                    t.trim_end_matches("pt").trim().parse::<f64>().ok()
                                }
                            }),
                    );
                    ui.end_row();
                    ui.label("Style");
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut self.shown.bold, "Bold");
                        ui.checkbox(&mut self.shown.italic, "Italic");
                        ui.checkbox(&mut self.shown.underline, "Underline");
                    });
                    ui.end_row();
                    ui.label("Text Color");
                    optional_color(ui, &mut self.shown.text, "Use color", Color::BLACK);
                    ui.end_row();
                    ui.label("Fill Color");
                    optional_color(ui, &mut self.shown.fill, "Use fill", Color::rgb(255, 255, 153));
                    ui.end_row();
                    ui.label("Horizontal");
                    ui.horizontal(|ui| {
                        ui.radio_value(&mut self.shown.align, HAlign::Auto, "Default");
                        ui.radio_value(&mut self.shown.align, HAlign::Left, "Left");
                        ui.radio_value(&mut self.shown.align, HAlign::Center, "Center");
                        ui.radio_value(&mut self.shown.align, HAlign::Right, "Right");
                    });
                    ui.end_row();
                });
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    clear = ui.button("Clear Formats").clicked();
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        pressed = ok_cancel_apply(ui, self.shown != self.base);
                    });
                });
            });
        if clear {
            self.clear(doc);
            return false;
        }
        match pressed {
            Some(DialogButton::Ok) => {
                self.apply(doc);
                false
            }
            Some(DialogButton::Apply) => {
                self.apply(doc);
                open
            }
            Some(DialogButton::Cancel) => false,
            None => open,
        }
    }
}

/// A checkbox that switches a colour on (with the colour button) or off (automatic).
fn optional_color(ui: &mut egui::Ui, value: &mut Option<Color>, label: &str, default: Color) {
    ui.horizontal(|ui| {
        let mut on = value.is_some();
        if ui.checkbox(&mut on, label).changed() {
            *value = on.then_some(default);
        }
        if let Some(c) = value {
            color_button(ui, c);
        }
    });
}
