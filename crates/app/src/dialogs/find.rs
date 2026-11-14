//! Edit > Find in Sheets... (Ctrl+F) in a workbook: finds a text in the cells of the active sheet
//! and moves the cell cursor to the next cell that holds it. Cells are compared by the text the
//! worksheet shows, searched column by column from the cell after the cursor, wrapping around.
//! Edit > Replace... (Ctrl+H) is the same window with a replacement text: the matched part of a
//! cell's shown text is replaced and the cell is set from the result, as if it had been typed.

use eframe::egui;
use sheaf_core::{WindowId, Worksheet};

/// The window that asks what to find.
pub struct FindDialog {
    pub book: WindowId,
    what: String,
    /// The replacement text, when the window is Replace.
    with: Option<String>,
    match_case: bool,
    whole_cell: bool,
    /// What the last search found or not.
    note: Option<String>,
    /// True until the first frame, to put the keyboard in the text field.
    first_frame: bool,
}

/// What the window did in a frame.
pub enum FindOutcome {
    Open,
    Closed,
    /// Find the next cell after the cursor; the caller searches with [`find_next`].
    Next,
    /// Replace in the cell at the cursor if it matches, then find the next one.
    Replace,
    /// Replace in every matching cell of the sheet.
    ReplaceAll,
}

impl FindDialog {
    pub fn new(book: WindowId) -> Self {
        FindDialog {
            book,
            what: String::new(),
            with: None,
            match_case: false,
            whole_cell: false,
            note: None,
            first_frame: true,
        }
    }

    /// The Replace window (Edit > Replace...).
    pub fn replace(book: WindowId) -> Self {
        FindDialog { with: Some(String::new()), ..FindDialog::new(book) }
    }

    pub fn show(&mut self, ctx: &egui::Context) -> FindOutcome {
        let mut open = true;
        let mut outcome = FindOutcome::Open;
        let title = if self.with.is_some() { "Replace" } else { "Find in Sheets" };
        egui::Window::new(title).open(&mut open).collapsible(false).resizable(false).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Find what");
                let field = ui.add(egui::TextEdit::singleline(&mut self.what).desired_width(180.0));
                if self.first_frame {
                    field.request_focus();
                    self.first_frame = false;
                }
            });
            if let Some(with) = &mut self.with {
                ui.horizontal(|ui| {
                    ui.label("Replace with");
                    ui.add(egui::TextEdit::singleline(with).desired_width(180.0));
                });
            }
            ui.checkbox(&mut self.match_case, "Match case");
            ui.checkbox(&mut self.whole_cell, "Match whole cell only");
            if let Some(n) = &self.note {
                ui.weak(n);
            }
            let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
            ui.horizontal(|ui| {
                if ui.button("Find Next").clicked() || enter {
                    outcome = FindOutcome::Next;
                }
                if self.with.is_some() {
                    if ui.button("Replace").clicked() {
                        outcome = FindOutcome::Replace;
                    }
                    if ui.button("Replace All").clicked() {
                        outcome = FindOutcome::ReplaceAll;
                    }
                }
                if ui.button("Close").clicked() {
                    outcome = FindOutcome::Closed;
                }
            });
        });
        if !open || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            return FindOutcome::Closed;
        }
        outcome
    }

    /// The next cell after `from` (column position, row) that matches, and a note for the window.
    pub fn search(&mut self, sheet: &Worksheet, from: Option<(usize, usize)>) -> Option<(usize, usize)> {
        if self.what.is_empty() {
            self.note = Some("Type the text to find".to_string());
            return None;
        }
        let hit = find_next(sheet, &self.what, self.match_case, self.whole_cell, from);
        self.note = match hit {
            Some((c, r)) => Some(format!("Found in column {}, row {}", sheet.columns[c].short_name, r + 1)),
            None => Some(format!("\"{}\" was not found", self.what)),
        };
        hit
    }

    /// The new text of cell (column position, row) when it matches, else `None`.
    pub fn replaced(&self, sheet: &Worksheet, cell: (usize, usize)) -> Option<String> {
        let with = self.with.as_deref()?;
        let shown = sheet.columns.get(cell.0)?.shown_text(cell.1);
        replace_in(&shown, &self.what, with, self.match_case, self.whole_cell)
    }

    /// Every cell of the sheet that matches, with its new text: (column position, row, text).
    pub fn replace_all(&mut self, sheet: &Worksheet) -> Vec<(usize, usize, String)> {
        let out: Vec<(usize, usize, String)> = sheet
            .columns
            .iter()
            .enumerate()
            .flat_map(|(c, col)| (0..col.len()).map(move |r| (c, r)))
            .filter_map(|(c, r)| self.replaced(sheet, (c, r)).map(|t| (c, r, t)))
            .collect();
        self.note = Some(format!("Replaced in {} cell(s)", out.len()));
        out
    }

    #[cfg(test)]
    pub fn set_text(&mut self, what: &str) {
        self.what = what.to_string();
    }

    #[cfg(test)]
    pub fn set_replacement(&mut self, with: &str) {
        self.with = Some(with.to_string());
    }
}

/// `text` with every occurrence of `what` replaced by `with` (the whole text when `whole_cell`),
/// or `None` when `what` does not occur.
pub fn replace_in(text: &str, what: &str, with: &str, match_case: bool, whole_cell: bool) -> Option<String> {
    if what.is_empty() || text.is_empty() {
        return None;
    }
    let fold = |s: &str| if match_case { s.to_string() } else { s.to_lowercase() };
    if whole_cell {
        return (fold(text) == fold(what)).then(|| with.to_string());
    }
    if match_case {
        return text.contains(what).then(|| text.replace(what, with));
    }
    // Without case: find the matches in the lower-case text and keep the rest of the original. When
    // lower-casing changes the byte length (some non-ASCII letters), replace in the lower case.
    let (low, needle) = (text.to_lowercase(), what.to_lowercase());
    if !low.contains(&needle) {
        return None;
    }
    if low.len() != text.len() {
        return Some(low.replace(&needle, with));
    }
    let mut out = String::new();
    let mut at = 0;
    while let Some(i) = low[at..].find(&needle) {
        out.push_str(&text[at..at + i]);
        out.push_str(with);
        at += i + needle.len();
    }
    out.push_str(&text[at..]);
    Some(out)
}

/// The first cell after `from` (column position, row; both from zero), going down each column and
/// then on to the next column, wrapping around to the start, whose shown text contains `what` (or
/// equals it with `whole_cell`). `None` when no cell matches.
pub fn find_next(
    sheet: &Worksheet,
    what: &str,
    match_case: bool,
    whole_cell: bool,
    from: Option<(usize, usize)>,
) -> Option<(usize, usize)> {
    let fold = |s: &str| if match_case { s.to_string() } else { s.to_lowercase() };
    let needle = fold(what);
    let cells: Vec<(usize, usize)> =
        sheet.columns.iter().enumerate().flat_map(|(c, col)| (0..col.len()).map(move |r| (c, r))).collect();
    if cells.is_empty() {
        return None;
    }
    let start = from.and_then(|f| cells.iter().position(|&x| x == f)).map_or(0, |i| i + 1);
    (0..cells.len()).map(|k| cells[(start + k) % cells.len()]).find(|&(c, r)| {
        let text = fold(&sheet.columns[c].shown_text(r));
        if whole_cell { text == needle } else { !text.is_empty() && text.contains(&needle) }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sheaf_core::{ColumnData, Designation, Project};

    #[test]
    fn find_goes_down_columns_from_the_cursor_and_wraps_around() {
        let mut p = Project::default();
        let b = p.add_workbook("");
        p.add_column(b, 0, Designation::X, ColumnData::numeric(vec![1.0, 2.5, 3.0])).unwrap();
        p.add_column(b, 0, Designation::Y, ColumnData::text(vec!["LMO".into(), "Ni-LMO".into(), "x".into()])).unwrap();
        let sheet = &p.workbook(b).unwrap().sheets[0];
        assert_eq!(find_next(sheet, "lmo", false, false, None), Some((1, 0)));
        assert_eq!(find_next(sheet, "lmo", false, false, Some((1, 0))), Some((1, 1)));
        assert_eq!(find_next(sheet, "lmo", false, false, Some((1, 1))), Some((1, 0)), "wraps around");
        assert_eq!(find_next(sheet, "lmo", true, false, None), None, "case matters");
        assert_eq!(find_next(sheet, "LMO", true, true, None), Some((1, 0)), "whole cell");
        assert_eq!(find_next(sheet, "2.5", false, true, None), Some((0, 1)), "numbers by their shown text");
        assert_eq!(find_next(sheet, "zzz", false, false, None), None);
    }

    #[test]
    fn replace_keeps_the_rest_of_the_text_and_follows_case_and_whole_cell() {
        assert_eq!(replace_in("Ni-LMO", "lmo", "LNMO", false, false).as_deref(), Some("Ni-LNMO"));
        assert_eq!(replace_in("Ni-LMO", "lmo", "LNMO", true, false), None, "case matters");
        assert_eq!(replace_in("LMO", "lmo", "x", false, true).as_deref(), Some("x"));
        assert_eq!(replace_in("Ni-LMO", "LMO", "x", true, true), None, "not the whole cell");
        assert_eq!(replace_in("2.5", ".5", ".75", true, false).as_deref(), Some("2.75"));
    }
}
