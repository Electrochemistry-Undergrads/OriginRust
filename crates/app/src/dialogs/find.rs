//! Edit > Find in Sheets... (Ctrl+F) and Replace... (Ctrl+H). Numeric mode compares stored values;
//! String mode compares displayed text. Searches traverse columns from the cursor and wrap,
//! optionally upward. Replacement edits are applied by the caller in one undo step.

use eframe::egui;
use sheaf_core::{Column, WindowId, Worksheet};

#[derive(Clone, Copy, PartialEq, Eq)]
enum DataType {
    Numeric,
    String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Condition {
    Equal,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    NotEqual,
}

impl Condition {
    const ALL: [Self; 6] =
        [Self::Equal, Self::Less, Self::LessEqual, Self::Greater, Self::GreaterEqual, Self::NotEqual];

    fn label(self) -> &'static str {
        match self {
            Self::Equal => "=",
            Self::Less => "<",
            Self::LessEqual => "<=",
            Self::Greater => ">",
            Self::GreaterEqual => ">=",
            Self::NotEqual => "!=",
        }
    }

    fn allows_absolute(self) -> bool {
        matches!(self, Self::Equal | Self::LessEqual | Self::GreaterEqual)
    }
}

/// The window that asks what to find.
pub struct FindDialog {
    pub book: WindowId,
    what: String,
    with: String,
    replacing: bool,
    data_type: DataType,
    condition: Condition,
    tolerance: String,
    absolute: bool,
    keep_sign: bool,
    set_missing: bool,
    match_case: bool,
    whole_cell: bool,
    wildcards: bool,
    search_up: bool,
    /// What the last search found or not.
    note: Option<String>,
    /// True until the first frame, to put the keyboard in the text field.
    first_frame: bool,
}

/// What the window did in a frame.
pub enum FindOutcome {
    Open,
    Closed,
    /// Find the next cell after the cursor; the caller searches with [`FindDialog::search`].
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
            with: String::new(),
            replacing: false,
            data_type: DataType::Numeric,
            condition: Condition::Equal,
            tolerance: "1e-8".into(),
            absolute: false,
            keep_sign: false,
            set_missing: false,
            match_case: false,
            whole_cell: false,
            wildcards: false,
            search_up: false,
            note: None,
            first_frame: true,
        }
    }

    /// The Replace window (Edit > Replace...).
    pub fn replace(book: WindowId) -> Self {
        FindDialog { replacing: true, ..FindDialog::new(book) }
    }

    /// Seeds the query from the selected cell when the dialog opens.
    pub fn initial_text(mut self, text: String) -> Self {
        self.what = text;
        self
    }

    pub fn show(&mut self, ctx: &egui::Context) -> FindOutcome {
        let mut open = true;
        let mut outcome = FindOutcome::Open;
        let popup_was_open = egui::Popup::is_any_open(ctx);
        egui::Window::new("Find and Replace").open(&mut open).collapsible(false).resizable(false).show(ctx, |ui| {
            ui.horizontal(|ui| {
                for (replacing, label) in [(false, "Find"), (true, "Replace")] {
                    let response = ui.selectable_value(&mut self.replacing, replacing, label);
                    let _ = ctx.accesskit_node_builder(response.id, |node| {
                        node.set_role(egui::accesskit::Role::Tab);
                        node.set_selected(self.replacing == replacing);
                    });
                }
            });
            ui.horizontal(|ui| {
                ui.label("Data Type:");
                ui.radio_value(&mut self.data_type, DataType::Numeric, "Numeric");
                ui.radio_value(&mut self.data_type, DataType::String, "String");
            });
            ui.horizontal(|ui| {
                if self.data_type == DataType::Numeric {
                    ui.vertical(|ui| {
                        let label = ui.label("Condition:");
                        egui::ComboBox::from_id_salt("find_condition")
                            .selected_text(self.condition.label())
                            .width(45.0)
                            .show_ui(ui, |ui| {
                                for condition in Condition::ALL {
                                    ui.selectable_value(&mut self.condition, condition, condition.label());
                                }
                            })
                            .response
                            .labelled_by(label.id);
                    });
                }
                ui.vertical(|ui| {
                    let label = ui.label("Find what (Enter -- for missing value):");
                    let field =
                        ui.add(egui::TextEdit::singleline(&mut self.what).desired_width(220.0)).labelled_by(label.id);
                    if self.first_frame {
                        field.request_focus();
                        self.first_frame = false;
                    }
                });
            });
            if self.replacing {
                let label = ui.label("Replace with:");
                ui.add(egui::TextEdit::singleline(&mut self.with).desired_width(280.0)).labelled_by(label.id);
            }
            ui.label("Look in:");
            ui.add_enabled_ui(false, |ui| {
                egui::ComboBox::from_id_salt("find_scope")
                    .selected_text("Active Worksheet")
                    .width(280.0)
                    .show_ui(ui, |_ui| {});
            });
            egui::CollapsingHeader::new("Options").default_open(true).show(ui, |ui| {
                // Sheaf has no individual cell links. Formula columns remain protected by the caller.
                ui.add_enabled(false, egui::Checkbox::new(&mut true, "Skip linked cells"));
                if self.data_type == DataType::String {
                    ui.checkbox(&mut self.match_case, "Match case");
                    ui.add_enabled(
                        !self.wildcards,
                        egui::Checkbox::new(&mut self.whole_cell, "Match whole cell contents"),
                    );
                    ui.checkbox(&mut self.wildcards, "Use wildcards");
                    ui.add_enabled(false, egui::Checkbox::new(&mut false, "Include label rows"));
                } else {
                    ui.add_enabled(
                        self.condition.allows_absolute(),
                        egui::Checkbox::new(&mut self.absolute, "Use absolute value"),
                    );
                    if self.replacing {
                        ui.checkbox(&mut self.keep_sign, "Keep sign if condition true");
                        ui.checkbox(&mut self.set_missing, "Set as missing value if condition false");
                    }
                }
                ui.checkbox(&mut self.search_up, "Search up");
                if self.data_type == DataType::Numeric {
                    ui.horizontal(|ui| {
                        let label = ui.label("Tolerance:");
                        ui.add(egui::TextEdit::singleline(&mut self.tolerance).desired_width(80.0))
                            .labelled_by(label.id);
                    });
                }
            });
            if let Some(n) = &self.note {
                ui.weak(n);
            }
            let valid = !self.what.is_empty() && (self.data_type == DataType::String || self.numeric_query().is_ok());
            let can_replace = valid && (self.data_type == DataType::String || self.numeric_replacement().is_ok());
            let enter = valid && !popup_was_open && ui.input(|i| i.key_pressed(egui::Key::Enter));
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Close").clicked() {
                        outcome = FindOutcome::Closed;
                    }
                    if self.replacing {
                        if ui.add_enabled(can_replace, egui::Button::new("Replace All")).clicked() {
                            outcome = FindOutcome::ReplaceAll;
                        }
                        if ui.add_enabled(can_replace, egui::Button::new("Replace")).clicked() {
                            outcome = FindOutcome::Replace;
                        }
                    }
                    if ui.add_enabled(valid, egui::Button::new("Find Next")).clicked() || enter {
                        outcome = FindOutcome::Next;
                    }
                });
            });
        });
        if !open || (!popup_was_open && ctx.input(|i| i.key_pressed(egui::Key::Escape))) {
            return FindOutcome::Closed;
        }
        if matches!(outcome, FindOutcome::Next | FindOutcome::Replace | FindOutcome::ReplaceAll) {
            let error = if self.what.is_empty() {
                Some("Type the value to find".to_string())
            } else if self.data_type == DataType::Numeric {
                self.numeric_query().err().or_else(|| {
                    matches!(outcome, FindOutcome::Replace | FindOutcome::ReplaceAll)
                        .then(|| self.numeric_replacement().err())
                        .flatten()
                })
            } else {
                None
            };
            if let Some(error) = error {
                self.note = Some(error);
                return FindOutcome::Open;
            }
        }
        outcome
    }

    /// The next cell after `from` (column position, row) that matches, and a note for the window.
    pub fn search(&mut self, sheet: &Worksheet, from: Option<(usize, usize)>) -> Option<(usize, usize)> {
        if self.what.is_empty() {
            self.note = Some("Type the value to find".to_string());
            return None;
        }
        if self.data_type == DataType::Numeric
            && let Err(error) = self.numeric_query()
        {
            self.note = Some(error);
            return None;
        }
        let hit = if self.data_type == DataType::String && !self.wildcards && !self.search_up {
            find_next(sheet, &self.what, self.match_case, self.whole_cell, from)
        } else {
            find_with(sheet, from, self.search_up, |column, row| self.matches(column, row))
        };
        self.note = match hit {
            Some((c, r)) => Some(format!("Found in column {}, row {}", sheet.columns[c].short_name, r + 1)),
            None => Some(format!("\"{}\" was not found", self.what)),
        };
        hit
    }

    /// The replacement for a cell, including clearing Numeric nonmatches when requested.
    pub fn replaced(&self, sheet: &Worksheet, cell: (usize, usize)) -> Option<String> {
        if !self.replacing {
            return None;
        }
        let column = sheet.columns.get(cell.0)?;
        if self.data_type == DataType::Numeric {
            let value = *column.numeric()?.get(cell.1)?;
            if column.data.text_cell(cell.1).is_some() {
                return None;
            }
            self.numeric_query().ok()?;
            let replacement = self.numeric_replacement().ok()?;
            if self.matches(column, cell.1) {
                let replacement = if self.keep_sign && !replacement.is_nan() && !value.is_nan() {
                    replacement.abs().copysign(value)
                } else {
                    replacement
                };
                return Some(if replacement.is_nan() { "--".into() } else { replacement.to_string() });
            }
            return self.set_missing.then(|| "--".into());
        }
        let shown = column.shown_text(cell.1);
        if self.wildcards {
            return wildcard_match(&shown, &self.what, self.match_case).then(|| self.with.clone());
        }
        replace_in(&shown, &self.what, &self.with, self.match_case, self.whole_cell)
    }

    fn numeric_query(&self) -> Result<(f64, f64), String> {
        let value = parse_number(&self.what)?;
        let tolerance = self
            .tolerance
            .trim()
            .parse::<f64>()
            .ok()
            .filter(|t| t.is_finite() && *t >= 0.0)
            .ok_or_else(|| "Tolerance must be a finite nonnegative number".to_string())?;
        Ok((value, tolerance))
    }

    fn numeric_replacement(&self) -> Result<f64, String> {
        if self.with.trim().is_empty() { Ok(f64::NAN) } else { parse_number(&self.with) }
    }

    fn matches(&self, column: &Column, row: usize) -> bool {
        if self.data_type == DataType::String {
            let shown = column.shown_text(row);
            if self.wildcards {
                return wildcard_match(&shown, &self.what, self.match_case);
            }
            let fold = |text: &str| if self.match_case { text.to_string() } else { text.to_lowercase() };
            return if self.whole_cell {
                fold(&shown) == fold(&self.what)
            } else {
                !shown.is_empty() && fold(&shown).contains(&fold(&self.what))
            };
        }
        let Ok((target, tolerance)) = self.numeric_query() else { return false };
        let Some(&value) = column.numeric().and_then(|values| values.get(row)) else { return false };
        if column.data.text_cell(row).is_some() {
            return false;
        }
        if target.is_nan() || value.is_nan() {
            return match self.condition {
                Condition::Equal | Condition::LessEqual | Condition::GreaterEqual => target.is_nan() && value.is_nan(),
                Condition::NotEqual => target.is_nan() != value.is_nan(),
                Condition::Less | Condition::Greater => false,
            };
        }
        let value = if self.absolute && self.condition.allows_absolute() { value.abs() } else { value };
        let equal = value == target || (value - target).abs() / (value.abs() + target.abs()) < tolerance;
        match self.condition {
            Condition::Equal => equal,
            Condition::Less => value < target,
            Condition::LessEqual => value < target || equal,
            Condition::Greater => value > target,
            Condition::GreaterEqual => value > target || equal,
            // Origin's != uses a strict comparison, even when = considers the pair equal
            // within tolerance (conditions/condition-5 in the 2016 UI probe).
            Condition::NotEqual => value != target,
        }
    }

    /// Every planned worksheet replacement: (column position, row, new text).
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
        self.with = with.to_string();
        self.replacing = true;
    }
}

fn parse_number(text: &str) -> Result<f64, String> {
    if text.trim() == "--" {
        return Ok(f64::NAN);
    }
    text.trim()
        .parse::<f64>()
        .ok()
        .filter(|number| number.is_finite())
        .ok_or_else(|| format!("\"{text}\" is not a number (use -- for missing)"))
}

/// Wildcards match the entire cell, independently of the whole-cell checkbox.
fn wildcard_match(text: &str, pattern: &str, match_case: bool) -> bool {
    let fold = |text: &str| if match_case { text.to_string() } else { text.to_lowercase() };
    let text: Vec<char> = fold(text).chars().collect();
    let pattern: Vec<char> = fold(pattern).chars().collect();
    let (mut i, mut j, mut star, mut retry) = (0, 0, None, 0);
    while i < text.len() {
        if j < pattern.len() && (pattern[j] == '?' || pattern[j] == text[i]) {
            i += 1;
            j += 1;
        } else if j < pattern.len() && pattern[j] == '*' {
            star = Some(j);
            j += 1;
            retry = i;
        } else if let Some(at) = star {
            retry += 1;
            i = retry;
            j = at + 1;
        } else {
            return false;
        }
    }
    while j < pattern.len() && pattern[j] == '*' {
        j += 1;
    }
    j == pattern.len()
}

fn find_with(
    sheet: &Worksheet,
    from: Option<(usize, usize)>,
    up: bool,
    matches: impl Fn(&Column, usize) -> bool,
) -> Option<(usize, usize)> {
    let cells: Vec<(usize, usize)> =
        sheet.columns.iter().enumerate().flat_map(|(c, column)| (0..column.len()).map(move |r| (c, r))).collect();
    let count = cells.len();
    if count == 0 {
        return None;
    }
    let start = from
        .and_then(|cell| cells.iter().position(|&at| at == cell))
        .map_or(if up { count - 1 } else { 0 }, |i| if up { (i + count - 1) % count } else { (i + 1) % count });
    (0..count)
        .map(|i| cells[if up { (start + count - i) % count } else { (start + i) % count }])
        .find(|&(c, r)| matches(&sheet.columns[c], r))
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

    fn numeric_fixture(values: &[f64]) -> (WindowId, Worksheet) {
        let mut project = Project::default();
        let book = project.add_workbook("");
        project.add_column(book, 0, Designation::Y, ColumnData::numeric(values.to_vec())).unwrap();
        (book, project.workbook(book).unwrap().sheet().clone())
    }

    #[test]
    fn numeric_mode_uses_stored_values_and_the_measured_relative_tolerance() {
        let (book, sheet) = numeric_fixture(&[100.000001, 100.0, 90.0]);
        let mut dialog = FindDialog::replace(book).initial_text("100".into());
        dialog.with = "7".into();
        assert_eq!(dialog.search(&sheet, None), Some((0, 0)));
        assert_eq!(dialog.replace_all(&sheet), [(0, 0, "7".into()), (0, 1, "7".into())]);
        dialog.tolerance = "0".into();
        assert_eq!(dialog.search(&sheet, None), Some((0, 1)), "the stored 100.000001 differs from its display");
        dialog.data_type = DataType::String;
        dialog.whole_cell = true;
        assert_eq!(dialog.search(&sheet, None), Some((0, 0)), "String searches the display");
    }

    #[test]
    fn numeric_replace_matches_origin_sign_conditional_and_missing_probes() {
        let (book, sheet) = numeric_fixture(&[-2.0, 2.0, -3.0, 0.0]);
        let mut dialog = FindDialog::replace(book).initial_text("2".into());
        dialog.with = "7".into();
        dialog.absolute = true;
        dialog.keep_sign = true;
        assert_eq!(dialog.replace_all(&sheet), [(0, 0, "-7".into()), (0, 1, "7".into())]);

        let (_, sheet) = numeric_fixture(&[-2.0, 2.0, 3.0, f64::NAN]);
        dialog.absolute = false;
        dialog.keep_sign = false;
        dialog.set_missing = true;
        dialog.condition = Condition::Greater;
        assert_eq!(
            dialog.replace_all(&sheet),
            [(0, 0, "--".into()), (0, 1, "--".into()), (0, 2, "7".into()), (0, 3, "--".into())]
        );
        dialog.condition = Condition::Equal;
        dialog.what = "--".into();
        assert_eq!(dialog.search(&sheet, None), Some((0, 3)));
        dialog.what = "2".into();
        dialog.with.clear();
        assert_eq!(dialog.replaced(&sheet, (0, 1)).as_deref(), Some("--"));
    }

    #[test]
    fn wildcards_match_whole_cells_and_characters_without_expanding_replacement_text() {
        assert!(wildcard_match("abXXc", "a*c", false));
        assert!(!wildcard_match("xabc", "a*c", false));
        assert!(wildcard_match("abc", "a*c", false));
        assert!(wildcard_match("a猫c", "a?c", false));
        assert!(wildcard_match("Alpha", "a*", false));
        assert!(!wildcard_match("Alpha", "a*", true));
        assert!(wildcard_match("", "*", false));
    }

    #[test]
    fn numeric_conditions_match_the_origin_tolerance_boundary_and_missing_matrix() {
        let (book, sheet) = numeric_fixture(&[1.99999999, 2.0, 2.00000001, f64::NAN]);
        let mut dialog = FindDialog::new(book).initial_text("2".into());
        for (condition, expected) in [
            (Condition::Equal, [true, true, true, false]),
            (Condition::Less, [true, false, false, false]),
            (Condition::LessEqual, [true, true, true, false]),
            (Condition::Greater, [false, false, true, false]),
            (Condition::GreaterEqual, [true, true, true, false]),
            (Condition::NotEqual, [true, false, true, true]),
        ] {
            dialog.condition = condition;
            let actual = [0, 1, 2, 3].map(|row| dialog.matches(&sheet.columns[0], row));
            assert_eq!(actual, expected, "{}", condition.label());
        }
    }
}
