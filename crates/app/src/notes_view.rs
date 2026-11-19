//! A Notes window (T154): one text box over the whole window. The text being typed is kept here and
//! written to the project as one undo step ("Edit notes") when the box loses the keyboard.

use crate::actions::Action;
use eframe::egui;
use sheaf_core::{Document, WindowId};

/// What a Notes window keeps between frames.
#[derive(Default)]
pub struct NotesUi {
    /// While the text is edited: the project's text when the editing began, and the text typed.
    editing: Option<(String, String)>,
}

pub fn show(ui: &mut egui::Ui, doc: &Document, id: WindowId, state: &mut NotesUi, actions: &mut Vec<Action>) {
    let Some(notes) = doc.project().notes_window(id) else { return };
    // An undo or redo that changed the text meanwhile wins over what was typed before it.
    if state.editing.as_ref().is_some_and(|(base, _)| *base != notes.text) {
        state.editing = None;
    }
    let (base, text) = state.editing.get_or_insert_with(|| (notes.text.clone(), notes.text.clone()));
    let edit = egui::TextEdit::multiline(text)
        .id_salt(("notes_window", id.0))
        .desired_width(f32::INFINITY)
        .font(egui::TextStyle::Monospace);
    let r = egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| ui.add_sized(ui.available_size(), edit))
        .inner;
    if !r.has_focus() {
        if *text != *base {
            actions.push(Action::SetNotesText { id, text: text.clone() });
        }
        state.editing = None;
    }
}
