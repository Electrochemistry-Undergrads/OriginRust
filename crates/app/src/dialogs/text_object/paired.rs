//! Insert one shared script column without changing the renderer or the object's base font.
use super::editor::{self, Format};
use eframe::egui::{
    self,
    text::{CCursor, CCursorRange},
};

fn selection_error(text: &str, cursor: CCursorRange, legend: bool) -> Option<&'static str> {
    let chars: Vec<_> = text.chars().collect();
    let range = cursor.as_sorted_char_range();
    let selected = &chars[range.start.min(chars.len())..range.end.min(chars.len())];
    if selected.iter().any(|c| matches!(c, '\n' | '\r')) {
        return Some("Select text on one line");
    }
    if legend && selected.windows(3).any(|s| s[0] == '\\' && matches!(s[1], 'l' | 'L') && s[2] == '(') {
        return Some("Select text within one legend entry");
    }
    let mut probe = text.to_string();
    editor::apply(&mut probe, cursor, Format::Underline);
    (probe == text).then_some("Select complete text or formatting groups")
}

/// Literal top-level commas must not become the separator between the two branches.
fn branch(text: &str) -> Option<String> {
    if text.contains(['\n', '\r']) {
        return None;
    }
    let body = editor::safe_body(&text.chars().collect::<Vec<_>>())?;
    let mut depth = 0;
    let mut out = String::new();
    for ch in body.chars() {
        match ch {
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => {
                out.push_str(r"\v(,)");
                continue;
            }
            _ => {}
        }
        out.push(ch);
    }
    Some(out)
}

pub(super) fn insert(text: &mut String, cursor: CCursorRange, legend: bool, sub: &str, sup: &str) -> CCursorRange {
    if selection_error(text, cursor, legend).is_some() || (sub.is_empty() && sup.is_empty()) {
        return cursor;
    }
    let (Some(sub), Some(sup)) = (branch(sub), branch(sup)) else { return cursor };
    let replacement = format!("\\=({sub},{sup})");
    let mut chars: Vec<_> = text.chars().collect();
    let selected = cursor.as_sorted_char_range();
    let range = selected.start.min(chars.len())..selected.end.min(chars.len());
    let caret = range.start + replacement.chars().count();
    chars.splice(range, replacement.chars());
    *text = chars.into_iter().collect();
    CCursorRange::one(CCursor::new(caret))
}

#[derive(Default)]
pub(super) struct Picker {
    sub: String,
    sup: String,
    problem: Option<&'static str>,
}

impl Picker {
    pub fn start(&mut self, text: &str, cursor: CCursorRange, legend: bool) {
        let selected = cursor.as_sorted_char_range();
        self.sub = text.chars().skip(selected.start).take(selected.end - selected.start).collect();
        self.sup.clear();
        self.problem = selection_error(text, cursor, legend);
    }

    pub fn show(&mut self, ui: &mut egui::Ui) -> Option<(String, String)> {
        ui.label("Align two scripts in one column");
        for (label, value) in [("Subscript text", &mut self.sub), ("Superscript text", &mut self.sup)] {
            ui.label(label);
            let response = ui.add(egui::TextEdit::singleline(value).desired_width(200.0));
            response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, label));
        }
        let valid = branch(&self.sub).is_some() && branch(&self.sup).is_some();
        if let Some(problem) = self.problem {
            ui.label(problem);
        } else if !valid {
            ui.label("Complete the formatting in each script");
        }
        let enabled = self.problem.is_none() && valid && (!self.sub.is_empty() || !self.sup.is_empty());
        if ui.add_enabled(enabled, egui::Button::new("Insert scripts")).clicked() {
            return Some((self.sub.clone(), self.sup.clone()));
        }
        if ui.button("Cancel scripts").clicked() {
            ui.close();
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn range(start: usize, end: usize) -> CCursorRange {
        CCursorRange::two(CCursor::new(start), CCursor::new(end))
    }

    #[test]
    fn commas_and_unmatched_literal_parentheses_preserve_unicode_and_outside_text() {
        let mut text = "μPZ".to_string();
        let cursor = insert(&mut text, range(1, 2), false, "a,b(", r"\v(a,b)");
        assert_eq!(text, r"μ\=(a\v(,)b\x(0028),\v(a,b))Z");
        assert_eq!(sheaf_plot::markup::plain(&text), "μa,b(a,bZ");
        assert_eq!(cursor.primary.index, text.chars().count() - 1);
        assert_eq!(cursor.primary.index, cursor.secondary.index);
    }

    #[test]
    fn nested_pairs_styles_and_one_empty_branch_keep_existing_rendering_rules() {
        let mut text = "P".to_string();
        insert(&mut text, range(0, 1), false, r"\f:Arial(\b(\=(1,2)))", r"\c14(\i(3))");
        let runs = sheaf_plot::markup::parse(&text);
        assert!(
            runs.iter()
                .filter(|r| r.text == "1" || r.text == "2")
                .all(|r| r.bold && r.family.as_deref() == Some("Arial") && (r.scale - 0.36).abs() < 1e-9)
        );
        assert!(runs.iter().any(|r| r.text == "3" && r.italic && r.color == Some(sheaf_core::Color::rgb(0, 0, 160))));
        let mut text = "P".to_string();
        insert(&mut text, range(1, 1), false, "", "2");
        assert_eq!(text, r"P\=(,2)");
        assert_eq!(sheaf_plot::markup::plain(&text), "P2");
    }

    #[test]
    fn malformed_inputs_empty_pairs_and_unsafe_fragments_are_inert() {
        for (source, start, end, sub, sup) in [
            ("P", 0, 1, "", ""),
            ("P", 0, 1, r"\=(1", "2"),
            ("P", 0, 1, r"\b(unfinished", "2"),
            ("P", 0, 1, "1\n2", "3"),
            (r"\f:Arial(x)", 3, 8, "1", "2"),
            (r"\v(literal)", 3, 10, "1", "2"),
        ] {
            let mut text = source.to_string();
            insert(&mut text, range(start, end), false, sub, sup);
            assert_eq!(text, source);
        }
    }

    #[test]
    fn legend_samples_and_rows_are_protected_but_entry_text_remains_editable() {
        let source = r"\l(1) %(1) cm2";
        let mut text = source.to_string();
        insert(&mut text, range(0, source.chars().count()), true, "1", "2");
        assert_eq!(text, source);
        let end = text.chars().count();
        insert(&mut text, range(end - 1, end), true, "2", "3");
        assert_eq!(text, r"\l(1) %(1) cm\=(2,3)");
        let mut text = "first\nsecond".to_string();
        insert(&mut text, range(0, 12), true, "1", "2");
        assert_eq!(text, "first\nsecond");
    }
}
