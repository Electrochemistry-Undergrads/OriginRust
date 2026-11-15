//! Worksheet > Conditional Formatting > Highlight Min Max and Highlight Top Bottom: the windows
//! that add a rule to the active sheet for the selected cells (Origin's `wcolorbycr` and
//! `wcolortb`). The rule is evaluated live by the grid (see `sheaf_core::condformat`).

use super::frame::{DialogButton, button_row};
use super::graph_props::color_button;
use eframe::egui;
use sheaf_core::color::Color;
use sheaf_core::condformat::{self, Colours, CondFormat, CondRange, Direction, Rule};
use sheaf_core::{Document, WindowId};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CondKind {
    MinMax,
    TopBottom,
}

/// The four modes of Highlight Top Bottom.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    TopItems,
    TopPercent,
    BottomItems,
    BottomPercent,
}

impl Mode {
    const ALL: [Mode; 4] = [Mode::TopItems, Mode::TopPercent, Mode::BottomItems, Mode::BottomPercent];

    fn label(self) -> &'static str {
        match self {
            Mode::TopItems => "Top N Items",
            Mode::TopPercent => "Top N%",
            Mode::BottomItems => "Bottom N Items",
            Mode::BottomPercent => "Bottom N%",
        }
    }

    fn parts(self) -> (bool, bool) {
        (matches!(self, Mode::TopItems | Mode::TopPercent), matches!(self, Mode::TopPercent | Mode::BottomPercent))
    }
}

/// One class of cells: whether its fill and its font colour are set, and the colours.
#[derive(Clone, Copy)]
struct ClassEdit {
    use_fill: bool,
    fill: Color,
    use_font: bool,
    font: Color,
}

impl ClassEdit {
    fn new(fill: Color, font: Color) -> Self {
        ClassEdit { use_fill: true, fill, use_font: true, font }
    }

    fn colours(&self) -> Colours {
        Colours { fill: self.use_fill.then_some(self.fill), font: self.use_font.then_some(self.font) }
    }

    fn show(&mut self, ui: &mut egui::Ui, title: &str) {
        ui.label(title);
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.use_fill, "Fill");
            ui.add_enabled_ui(self.use_fill, |ui| color_button(ui, &mut self.fill));
            ui.add_space(12.0);
            ui.checkbox(&mut self.use_font, "Font");
            ui.add_enabled_ui(self.use_font, |ui| color_button(ui, &mut self.font));
        });
    }
}

pub struct CondFormatDialog {
    pub book: WindowId,
    sheet: usize,
    kind: CondKind,
    range: CondRange,
    name: String,
    dir: Direction,
    max: ClassEdit,
    min: ClassEdit,
    mode: Mode,
    n: f64,
    hit: ClassEdit,
}

impl CondFormatDialog {
    pub fn new(doc: &Document, book: WindowId, kind: CondKind, range: CondRange) -> Option<Self> {
        let b = doc.project().workbook(book)?;
        let sheet = b.active_sheet.min(b.sheets.len().checked_sub(1)?);
        let name = condformat::default_name(&b.sheets[sheet], &range);
        Some(CondFormatDialog {
            book,
            sheet,
            kind,
            range,
            name,
            dir: Direction::RowWise,
            max: ClassEdit::new(condformat::MAX_FILL, condformat::MAX_FONT),
            min: ClassEdit::new(condformat::MIN_FILL, condformat::MIN_FONT),
            mode: Mode::TopItems,
            n: condformat::DEFAULT_N,
            hit: ClassEdit::new(condformat::TOP_FILL, condformat::TOP_FONT),
        })
    }

    fn rule(&self) -> Rule {
        match self.kind {
            CondKind::MinMax => Rule::MinMax { dir: self.dir, max: self.max.colours(), min: self.min.colours() },
            CondKind::TopBottom => {
                let (top, percent) = self.mode.parts();
                Rule::TopBottom { top, n: self.n, percent, colours: self.hit.colours() }
            }
        }
    }

    /// Draws the window; false when it closed.
    pub fn show(&mut self, ctx: &egui::Context, doc: &mut Document) -> bool {
        if doc.project().workbook(self.book).and_then(|b| b.sheets.get(self.sheet)).is_none() {
            return false;
        }
        let title = match self.kind {
            CondKind::MinMax => "Highlight Min Max",
            CondKind::TopBottom => "Highlight Top Bottom",
        };
        let mut open = true;
        let mut pressed = None;
        egui::Window::new(title)
            .id(egui::Id::new("cond_format"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Name");
                    ui.add(egui::TextEdit::singleline(&mut self.name).desired_width(160.0));
                });
                ui.add_space(4.0);
                match self.kind {
                    CondKind::MinMax => {
                        ui.horizontal(|ui| {
                            ui.label("Direction");
                            ui.radio_value(&mut self.dir, Direction::RowWise, "Row-wise");
                            ui.radio_value(&mut self.dir, Direction::ColumnWise, "Column-wise");
                        });
                        ui.add_space(4.0);
                        self.max.show(ui, "Maximum");
                        self.min.show(ui, "Minimum");
                    }
                    CondKind::TopBottom => {
                        ui.horizontal(|ui| {
                            ui.label("Mode");
                            let before = self.mode;
                            egui::ComboBox::from_id_salt("cond_mode").selected_text(self.mode.label()).show_ui(
                                ui,
                                |ui| {
                                    for m in Mode::ALL {
                                        ui.selectable_value(&mut self.mode, m, m.label());
                                    }
                                },
                            );
                            // The default colours follow top or bottom until they are changed.
                            if self.mode != before && self.mode.parts().0 != before.parts().0 {
                                let top = self.mode.parts().0;
                                let default = if top {
                                    ClassEdit::new(condformat::TOP_FILL, condformat::TOP_FONT)
                                } else {
                                    ClassEdit::new(condformat::BOTTOM_FILL, condformat::BOTTOM_FONT)
                                };
                                let was_default = {
                                    let c = if before.parts().0 {
                                        (condformat::TOP_FILL, condformat::TOP_FONT)
                                    } else {
                                        (condformat::BOTTOM_FILL, condformat::BOTTOM_FONT)
                                    };
                                    self.hit.fill == c.0 && self.hit.font == c.1
                                };
                                if was_default {
                                    self.hit = default;
                                }
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label("N");
                            ui.add(egui::DragValue::new(&mut self.n).range(0.0..=1.0e9).speed(0.1));
                            ui.weak(if self.mode.parts().1 { "percent of the numbers" } else { "items" });
                        });
                        ui.add_space(4.0);
                        self.hit.show(ui, "Highlighted cells");
                        ui.weak("Each column of the selection is ranked on its own.");
                    }
                }
                ui.add_space(6.0);
                pressed = button_row(ui, false);
            });
        match pressed {
            Some(DialogButton::Ok) => {
                let (book, sheet) = (self.book, self.sheet);
                let range = self.range.clone();
                let rule = self.rule();
                let name = self.name.trim().to_string();
                let label = match self.kind {
                    CondKind::MinMax => "Highlight min max",
                    CondKind::TopBottom => "Highlight top bottom",
                };
                doc.apply(label, |p| {
                    if let Some(s) = p.workbook_mut(book).and_then(|b| b.sheets.get_mut(sheet)) {
                        let mut rule = CondFormat::new(s, range, rule);
                        if !name.is_empty() {
                            rule.name = name;
                        }
                        // The newest rule goes on top, so it wins where rules overlap.
                        s.cond_formats.insert(0, rule);
                    }
                });
                return false;
            }
            Some(DialogButton::Cancel) => return false,
            _ => {}
        }
        open
    }
}
