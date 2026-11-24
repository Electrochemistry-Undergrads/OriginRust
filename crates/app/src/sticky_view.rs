//! Sticky Notes (T421): the coloured strip a note shows at the top of its graph window or workbook (a sheet's
//! note while that sheet is shown). Open, it has the colour swatches, Collapse and Delete Note above the text;
//! collapsed, it is a one-line indicator with the note's first line. Rules in `sheaf_core::sticky`.

use crate::actions::{Action, StickyChange};
use eframe::egui::{self, Color32, RichText};
use sheaf_core::Document;
use sheaf_core::sticky::{NoteColor, NoteTarget};

fn color32(c: NoteColor) -> Color32 {
    let c = c.rgb();
    Color32::from_rgb(c.r, c.g, c.b)
}

/// The undo token that makes a run of typing in one note one undo step.
pub fn typing_token(target: NoteTarget) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    ("sticky-note", target).hash(&mut h);
    h.finish() | 1
}

/// Draws `target`'s note, if it has one, as a strip across `ui`; `label` names what it is pinned to.
pub fn strip(ui: &mut egui::Ui, doc: &Document, target: NoteTarget, label: &str, actions: &mut Vec<Action>) {
    let Some(note) = doc.project().sticky_note(target) else { return };
    egui::Frame::new().fill(color32(note.color)).inner_margin(egui::Margin::same(4)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.visuals_mut().override_text_color = Some(Color32::BLACK);
        ui.horizontal(|ui| {
            if ui.small_button(if note.collapsed { "Expand" } else { "Collapse" }).clicked() {
                actions.push(Action::EditStickyNote { target, change: StickyChange::Collapsed(!note.collapsed) });
            }
            ui.label(RichText::new(label).strong());
            if note.collapsed {
                ui.label(note.first_line());
                return;
            }
            for c in NoteColor::ALL {
                let swatch = egui::Button::new("")
                    .fill(color32(c))
                    .stroke(egui::Stroke::new(if c == note.color { 2.0_f32 } else { 1.0 }, Color32::from_gray(90)))
                    .min_size(egui::vec2(14.0, 14.0));
                let r = ui.add(swatch).on_hover_text(c.name());
                r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, c.name()));
                if r.clicked() && c != note.color {
                    actions.push(Action::EditStickyNote { target, change: StickyChange::Color(c) });
                }
            }
            if ui.small_button("Delete Note").clicked() {
                actions.push(Action::RemoveStickyNote { target });
            }
        });
        if !note.collapsed {
            let mut text = note.text.clone();
            let field = egui::TextEdit::multiline(&mut text)
                .id_salt(("sticky-note", target))
                .hint_text("Type a note")
                .desired_rows(2)
                .desired_width(f32::INFINITY)
                .frame(false);
            if ui.add(field).changed() {
                actions.push(Action::EditStickyNote { target, change: StickyChange::Text(text) });
            }
        }
    });
}
