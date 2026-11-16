//! The window of a free text label: its text (several lines, with Origin's escapes such as
//! `\b(..)` for bold, `\g(a)` for Greek and `\+(..)` for a superscript), font size, bold, italic
//! and rotation. It opens for a new label (Add Text, the Text tool) and for an existing one
//! (double click, Properties).
//!
//! A new label is not in the project until OK or Apply: Cancel, or OK with no text, adds nothing.
//! Each OK or Apply is one undo step.

use eframe::egui::{self, DragValue};
use egui::text::{CCursor, CCursorRange};
use sheaf_core::graph::{Font, TextLabel};
use sheaf_core::{Document, WindowId};

mod editor;
mod preview;
use editor::Format;

pub struct TextDialog {
    graph: WindowId,
    layer: usize,
    /// The label being edited; `None` while it is new and not yet added.
    label: Option<usize>,
    /// Where a new label goes (fractions of the layer frame).
    at: (f64, f64),
    /// The font of the label (family and colour are kept; the dialog sets size, bold, italic).
    font: Font,
    text: String,
    size: f64,
    bold: bool,
    italic: bool,
    rotation: f64,
    problem: Option<String>,
    /// The text box takes the keyboard when the window opens.
    focus: bool,
    selection: CCursorRange,
    preview: preview::Preview,
}

impl TextDialog {
    /// For a new label in `layer`, at `at` (fractions of the layer frame).
    pub fn new_label(doc: &Document, graph: WindowId, layer: usize, at: (f64, f64)) -> Option<Self> {
        let font = doc.project().graph(graph)?.layers.get(layer)?.new_label_font();
        Some(TextDialog {
            graph,
            layer,
            label: None,
            at,
            size: font.size_pt,
            bold: font.bold,
            italic: font.italic,
            font,
            text: String::new(),
            rotation: 0.0,
            problem: None,
            focus: true,
            selection: CCursorRange::default(),
            preview: preview::Preview::default(),
        })
    }

    /// For label number `label` of `layer`.
    pub fn edit(doc: &Document, graph: WindowId, layer: usize, label: usize) -> Option<Self> {
        let l = doc.project().graph(graph)?.layers.get(layer)?.labels.get(label)?;
        Some(TextDialog {
            graph,
            layer,
            label: Some(label),
            at: (l.x, l.y),
            font: l.font.clone(),
            text: l.text.clone(),
            size: l.font.size_pt,
            bold: l.font.bold,
            italic: l.font.italic,
            rotation: l.rotation_deg,
            problem: None,
            focus: true,
            selection: CCursorRange::one(CCursor::new(l.text.chars().count())),
            preview: preview::Preview::default(),
        })
    }

    /// Draws the window. Returns `false` when it was closed.
    pub fn show(&mut self, ctx: &egui::Context, doc: &mut Document) -> bool {
        let exists = doc
            .project()
            .graph(self.graph)
            .and_then(|g| g.layers.get(self.layer))
            .is_some_and(|l| self.label.is_none_or(|k| k < l.labels.len()));
        if !exists {
            return false;
        }
        let mut open = true;
        let mut close = false;
        egui::Window::new("Text")
            .id(egui::Id::new("text_object"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                let edit_id = ui.make_persistent_id("text_object_content");
                let mut command = None;
                if ctx.memory(|m| m.has_focus(edit_id)) {
                    for (key, shift, format) in [
                        (egui::Key::B, false, Format::Bold),
                        (egui::Key::I, false, Format::Italic),
                        (egui::Key::U, false, Format::Underline),
                        (egui::Key::G, false, Format::Greek),
                        (egui::Key::Equals, true, Format::Superscript),
                        (egui::Key::Equals, false, Format::Subscript),
                    ] {
                        let modifiers = egui::Modifiers { ctrl: true, shift, ..Default::default() };
                        if ui.input_mut(|i| i.consume_key(modifiers, key)) {
                            command = Some(format);
                        }
                    }
                }
                ui.horizontal(|ui| {
                    for format in Format::ALL {
                        let button = ui.button(format.icon()).on_hover_text(format.tip());
                        button.widget_info(|| {
                            egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), format.label())
                        });
                        if button.clicked() {
                            command = Some(format);
                        }
                    }
                });
                if let Some(format) = command {
                    self.selection = editor::apply(&mut self.text, self.selection, format);
                    let mut state = egui::text_edit::TextEditState::load(ctx, edit_id).unwrap_or_default();
                    state.cursor.set_char_range(Some(self.selection));
                    state.store(ctx, edit_id);
                    // Keyboard activation of a format button must not insert into the text.
                    ui.input_mut(|i| {
                        i.consume_key(egui::Modifiers::NONE, egui::Key::Enter);
                        i.consume_key(egui::Modifiers::NONE, egui::Key::Space);
                    });
                    ctx.memory_mut(|m| m.request_focus(edit_id));
                }
                if self.focus {
                    let mut state = egui::text_edit::TextEditState::load(ctx, edit_id).unwrap_or_default();
                    state.cursor.set_char_range(Some(self.selection));
                    state.store(ctx, edit_id);
                }
                let edit = egui::TextEdit::multiline(&mut self.text)
                    .id(edit_id)
                    .desired_rows(3)
                    .desired_width(370.0)
                    .hint_text(r"Text, with Origin's escapes such as \b(bold) and \g(a)");
                let output = edit.show(ui);
                if let Some(selection) = output.cursor_range {
                    self.selection = selection;
                }
                if self.focus || command.is_some() {
                    output.response.request_focus();
                    self.focus = false;
                }
                ui.add_space(4.0);
                egui::Grid::new("text_object_grid").num_columns(2).spacing([14.0, 6.0]).show(ui, |ui| {
                    ui.label("Size");
                    ui.add(DragValue::new(&mut self.size).range(1.0..=400.0).speed(0.2).suffix(" pt"));
                    ui.end_row();
                    ui.label("Style");
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut self.bold, "Bold");
                        ui.checkbox(&mut self.italic, "Italic");
                    });
                    ui.end_row();
                    ui.label("Rotation");
                    ui.add(DragValue::new(&mut self.rotation).range(-360.0..=360.0).speed(1.0).suffix(" \u{b0}"));
                    ui.end_row();
                });
                let mut preview_font = self.font.clone();
                preview_font.size_pt = self.size;
                preview_font.bold = self.bold;
                preview_font.italic = self.italic;
                self.preview.show(ui, &self.text, preview_font, self.rotation);
                if let Some(p) = &self.problem {
                    ui.colored_label(egui::Color32::from_rgb(170, 30, 30), p);
                }
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.button("OK").clicked() {
                        if self.is_empty() && self.label.is_none() {
                            close = true; // an empty new label cancels
                        } else if self.apply(doc) {
                            close = true;
                        }
                    }
                    if ui.button("Cancel").clicked() {
                        close = true;
                    }
                    if ui.button("Apply").clicked() && !(self.is_empty() && self.label.is_none()) {
                        self.apply(doc);
                    }
                });
            });
        open && !close
    }

    fn is_empty(&self) -> bool {
        sheaf_plot::markup::plain(&self.text).trim().is_empty()
    }

    /// Writes the fields to the label (adding it when it is new). Returns `false` when they are
    /// not usable.
    fn apply(&mut self, doc: &mut Document) -> bool {
        if self.is_empty() {
            self.problem = Some("Type some text, or press Delete on the label to remove it.".to_string());
            return false;
        }
        self.problem = None;
        let mut font = self.font.clone();
        font.size_pt = self.size;
        font.bold = self.bold;
        font.italic = self.italic;
        let (graph, layer, text, rotation) = (self.graph, self.layer, self.text.clone(), self.rotation);
        match self.label {
            None => {
                let mut label = TextLabel::new(text, self.at.0, self.at.1, font);
                label.rotation_deg = rotation;
                let mut index = 0;
                doc.apply("Add text", |p| {
                    if let Some(l) = p.graph_mut(graph).and_then(|g| g.layers.get_mut(layer)) {
                        l.labels.push(label);
                        index = l.labels.len() - 1;
                    }
                });
                // From now on the window changes this label.
                self.label = Some(index);
            }
            Some(k) => {
                doc.apply("Edit text", |p| {
                    if let Some(l) =
                        p.graph_mut(graph).and_then(|g| g.layers.get_mut(layer)).and_then(|l| l.labels.get_mut(k))
                    {
                        l.text = text;
                        l.font = font;
                        l.rotation_deg = rotation;
                    }
                });
            }
        }
        true
    }
}
