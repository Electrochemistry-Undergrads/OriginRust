//! Worksheet > Conditional Formatting > Conditional Format Manager: the conditional formats of the
//! active worksheet, top rule first (the upper rule wins where ranges overlap). A format can be
//! renamed (double-click its name), its rule edited in the window that made it (Edit Rule), moved
//! up or down, or removed. Each change is applied at once as one undo step.

use super::cond_format::{range_text, rule_text};
use eframe::egui;
use sheaf_core::{Document, WindowId};

/// What the window asks of the application after a frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ManagerOutcome {
    Open,
    Closed,
    /// Open the rule window of the format at `index` of sheet `sheet`.
    EditRule {
        sheet: usize,
        index: usize,
    },
}

pub struct CondManagerDialog {
    pub book: WindowId,
    sheet: usize,
    selected: Option<usize>,
    /// The format being renamed, the name typed so far, and whether its box has had the focus.
    renaming: Option<(usize, String, bool)>,
}

impl CondManagerDialog {
    pub fn new(doc: &Document, book: WindowId) -> Option<Self> {
        let b = doc.project().workbook(book)?;
        let sheet = b.active_sheet.min(b.sheets.len().checked_sub(1)?);
        let selected = (!b.sheets[sheet].cond_formats.is_empty()).then_some(0);
        Some(CondManagerDialog { book, sheet, selected, renaming: None })
    }

    /// Draws the window.
    pub fn show(&mut self, ctx: &egui::Context, doc: &mut Document) -> ManagerOutcome {
        let Some(sheet) = doc.project().workbook(self.book).and_then(|b| b.sheets.get(self.sheet)) else {
            return ManagerOutcome::Closed;
        };
        // What the list shows, read again every frame so undo and other edits show at once.
        let rows: Vec<(String, String, String)> = sheet
            .cond_formats
            .iter()
            .map(|r| (r.name.clone(), range_text(sheet, &r.range), rule_text(&r.rule)))
            .collect();
        let count = rows.len();
        self.selected = self.selected.filter(|i| *i < count).or((count > 0).then_some(0));
        if self.renaming.as_ref().is_some_and(|(i, _, _)| *i >= count) {
            self.renaming = None;
        }
        let book = self.book;
        let sheet_index = self.sheet;
        let mut open = true;
        let mut outcome = ManagerOutcome::Open;
        let mut rename: Option<(usize, String)> = None;
        let mut swap: Option<(usize, usize)> = None;
        let mut remove: Option<usize> = None;
        egui::Window::new("Conditional Format Manager")
            .id(egui::Id::new("cond_manager"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                if count == 0 {
                    ui.label("This worksheet has no conditional formats.");
                }
                egui::Grid::new("cond_manager_list").striped(true).num_columns(3).show(ui, |ui| {
                    ui.strong("Name");
                    ui.strong("Range");
                    ui.strong("Rule");
                    ui.end_row();
                    for (i, (name, range, rule)) in rows.iter().enumerate() {
                        match &mut self.renaming {
                            Some((r, text, focused)) if *r == i => {
                                let edit = ui.add(egui::TextEdit::singleline(text).desired_width(140.0));
                                // Focus once: asking every frame would keep it through Enter.
                                if !*focused {
                                    edit.request_focus();
                                    *focused = true;
                                }
                                if edit.lost_focus() {
                                    let typed = text.trim().to_string();
                                    if !ui.input(|inp| inp.key_pressed(egui::Key::Escape))
                                        && !typed.is_empty()
                                        && typed != *name
                                    {
                                        rename = Some((i, typed));
                                    }
                                    self.renaming = None;
                                }
                            }
                            _ => {
                                let row = ui.selectable_label(self.selected == Some(i), name);
                                if row.clicked() {
                                    self.selected = Some(i);
                                }
                                if row.double_clicked() {
                                    self.selected = Some(i);
                                    self.renaming = Some((i, name.clone(), false));
                                }
                            }
                        }
                        ui.label(range);
                        ui.label(rule);
                        ui.end_row();
                    }
                });
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    let sel = self.selected;
                    if ui.add_enabled(sel.is_some(), egui::Button::new("Edit Rule")).clicked()
                        && let Some(i) = sel
                    {
                        outcome = ManagerOutcome::EditRule { sheet: sheet_index, index: i };
                    }
                    if ui.add_enabled(sel.is_some_and(|i| i > 0), egui::Button::new("Move Up")).clicked()
                        && let Some(i) = sel
                    {
                        swap = Some((i, i - 1));
                    }
                    if ui.add_enabled(sel.is_some_and(|i| i + 1 < count), egui::Button::new("Move Down")).clicked()
                        && let Some(i) = sel
                    {
                        swap = Some((i, i + 1));
                    }
                    if ui.add_enabled(sel.is_some(), egui::Button::new("Remove")).clicked() {
                        remove = sel;
                    }
                    ui.add_space(12.0);
                    if ui.button("Close").clicked() {
                        outcome = ManagerOutcome::Closed;
                    }
                });
                ui.weak("The upper format wins where ranges overlap. Double-click a name to rename it.");
            });
        if let Some((i, name)) = rename {
            doc.apply("Rename conditional format", |p| {
                if let Some(r) = formats_of(p, book, sheet_index).and_then(|f| f.get_mut(i)) {
                    r.name = name;
                }
            });
        }
        if let Some((a, b)) = swap {
            let label = if b < a { "Move conditional format up" } else { "Move conditional format down" };
            doc.apply(label, |p| {
                if let Some(f) = formats_of(p, book, sheet_index)
                    && a.max(b) < f.len()
                {
                    f.swap(a, b);
                }
            });
            self.selected = Some(b);
        }
        if let Some(i) = remove {
            doc.apply("Remove conditional format", |p| {
                if let Some(f) = formats_of(p, book, sheet_index)
                    && i < f.len()
                {
                    f.remove(i);
                }
            });
            self.selected = (i + 1 < count).then_some(i).or(i.checked_sub(1));
        }
        if !open { ManagerOutcome::Closed } else { outcome }
    }
}

/// The rule list of one sheet of a workbook.
fn formats_of(
    p: &mut sheaf_core::Project,
    book: WindowId,
    sheet: usize,
) -> Option<&mut Vec<sheaf_core::condformat::CondFormat>> {
    p.workbook_mut(book).and_then(|b| b.sheets.get_mut(sheet)).map(|s| &mut s.cond_formats)
}
