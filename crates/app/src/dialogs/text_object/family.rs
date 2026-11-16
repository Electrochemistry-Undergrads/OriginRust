//! Installed family names that the existing full-name markup syntax can represent.
use eframe::egui;
use std::sync::OnceLock;

pub(super) fn valid(name: &str) -> bool {
    !name.is_empty()
        && name.trim() == name
        && name.chars().count() <= 64
        && !name.chars().any(|c| c.is_control() || matches!(c, '(' | ')' | '\\'))
}

fn families() -> &'static [String] {
    static NAMES: OnceLock<Vec<String>> = OnceLock::new();
    NAMES.get_or_init(|| {
        let mut names: Vec<_> = sheaf_plot::SystemFonts::shared()
            .database()
            .faces()
            .flat_map(|face| face.families.iter().map(|(name, _)| name.clone()))
            .filter(|name| valid(name))
            .collect();
        names.sort();
        names.dedup();
        names
    })
}

#[derive(Default)]
pub(super) struct Picker {
    search: String,
}

impl Picker {
    pub fn show(&mut self, ui: &mut egui::Ui) -> Option<&'static str> {
        ui.label("Font family");
        let search =
            ui.add(egui::TextEdit::singleline(&mut self.search).desired_width(180.0).hint_text("Search fonts"));
        search.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, "Font search"));
        let filter = self.search.to_lowercase();
        let mut chosen = None;
        let mut found = false;
        egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
            for name in families().iter().filter(|name| name.to_lowercase().contains(&filter)) {
                found = true;
                if ui.button(format!("Text font: {name}")).clicked() {
                    chosen = Some(name.as_str());
                }
            }
        });
        if !found {
            ui.label("No matching installed fonts");
        }
        if chosen.is_some() {
            self.search.clear();
        }
        chosen
    }
}

#[cfg(test)]
mod tests {
    use super::super::editor::{self, Format};
    use super::*;
    use egui::text::{CCursor, CCursorRange};

    fn range(start: usize, end: usize) -> CCursorRange {
        CCursorRange::two(CCursor::new(start), CCursor::new(end))
    }

    #[test]
    fn family_names_obey_the_existing_full_name_grammar_and_bound() {
        for name in ["Arial", "Times New Roman", "Courier New", "Symbol", "字体"] {
            assert!(valid(name));
        }
        assert!(valid(&"a".repeat(64)));
        for name in ["", " Arial", "Arial ", "a(b", "a)b", "a\\b", "a\nb", &"a".repeat(65)] {
            assert!(!valid(name), "{name:?}");
        }
    }

    #[test]
    fn setting_a_family_replaces_nested_families_but_retains_other_styles_and_literals() {
        let mut text = r"\b(\c14(\f:Arial(a\f:Times New Roman(b))))\v(\f:Arial(literal))".to_string();
        let count = text.chars().count();
        let selection = editor::apply(&mut text, range(0, count), Format::Family("Courier New"));
        assert_eq!(text, r"\f:Courier New(\b(\c14(ab))\v(\f:Arial(literal)))");
        let before = text.clone();
        editor::apply(&mut text, selection, Format::Family("Courier New"));
        assert_eq!(text, before);
    }

    #[test]
    fn font_switches_start_and_end_the_existing_symbol_greek_mapping() {
        let mut text = "m".to_string();
        let selection = editor::apply(&mut text, range(0, 1), Format::Family("Symbol"));
        assert_eq!(sheaf_plot::markup::plain(&text), "μ");
        editor::apply(&mut text, selection, Format::Family("Arial"));
        assert_eq!(text, r"\f:Arial(m)");
        assert_eq!(sheaf_plot::markup::plain(&text), "m");
        let mut text = r"\g(m)".to_string();
        editor::apply(&mut text, range(3, 4), Format::Family("Arial"));
        assert_eq!(text, r"\g(\f:Arial(m))");
        assert_eq!(sheaf_plot::markup::plain(&text), "m");
    }

    #[test]
    fn invalid_family_names_and_unsafe_raw_fragments_are_inert() {
        for (source, start, end, name) in [
            ("sample", 0, 6, "Bad(Name"),
            (r"\f:Arial(x)", 3, 8, "Courier New"),
            (r"\v(literal)", 3, 10, "Courier New"),
            (r"\b(unfinished", 0, 13, "Courier New"),
        ] {
            let mut text = source.to_string();
            editor::apply(&mut text, range(start, end), Format::Family(name));
            assert_eq!(text, source);
        }
    }
}
