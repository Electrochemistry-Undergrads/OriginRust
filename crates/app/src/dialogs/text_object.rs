//! Shared selection editor for free labels, existing axis titles and legend content.
//! Text, font and visibility edits stay in a draft until OK or Apply. A new empty label
//! adds nothing; Cancel discards the unapplied draft. Each changed apply is one undo step.
//! Preview uses the published graph renderer, including raw metadata and legend notation.

use eframe::egui::{self, DragValue};
use egui::text::{CCursor, CCursorRange};
use sheaf_core::graph::{Font, TextLabel};
use sheaf_core::{Document, WindowId};
use sheaf_plot::scene::ObjectKind;

mod editor;
mod family;
mod paired;
mod palette;
mod preview;
use editor::Format;

pub struct TextDialog {
    graph: WindowId,
    layer: usize,
    /// The label being edited; `None` while it is new and not yet added.
    label: Option<usize>,
    object: Option<ObjectKind>,
    automatic: bool,
    visible: bool,
    generation: u64,
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
    colors: palette::Picker,
    families: family::Picker,
    scripts: paired::Picker,
}

impl TextDialog {
    /// For a new label in `layer`, at `at` (fractions of the layer frame).
    pub fn new_label(doc: &Document, graph: WindowId, layer: usize, at: (f64, f64)) -> Option<Self> {
        let font = doc.project().graph(graph)?.layers.get(layer)?.new_label_font();
        Some(TextDialog {
            graph,
            layer,
            label: None,
            object: None,
            automatic: false,
            visible: true,
            generation: doc.generation(),
            at,
            size: font.size_pt,
            colors: palette::Picker::new(font.color),
            families: family::Picker::default(),
            scripts: paired::Picker::default(),
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
            object: None,
            automatic: false,
            visible: true,
            generation: doc.generation(),
            at: (l.x, l.y),
            font: l.font.clone(),
            colors: palette::Picker::new(l.font.color),
            families: family::Picker::default(),
            scripts: paired::Picker::default(),
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

    /// The text and base font of an existing axis title.
    pub fn axis_title(doc: &Document, graph: WindowId, layer: usize, vertical: bool) -> Option<Self> {
        Self::for_object(doc, graph, layer, if vertical { ObjectKind::YTitle } else { ObjectKind::XTitle })
    }

    /// The existing legend content, including its plot-symbol and name substitutions.
    pub fn legend(doc: &Document, graph: WindowId, layer: usize) -> Option<Self> {
        Self::for_object(doc, graph, layer, ObjectKind::Legend)
    }

    fn for_object(doc: &Document, graph: WindowId, layer: usize, object: ObjectKind) -> Option<Self> {
        let l = doc.project().graph(graph)?.layers.get(layer)?;
        let (text, font, automatic, visible) = match object {
            ObjectKind::XTitle | ObjectKind::YTitle => {
                let vertical = object == ObjectKind::YTitle;
                let axis = if vertical { &l.y_axis } else { &l.x_axis };
                (
                    sheaf_plot::axis_title(doc.project(), graph, layer, vertical).ok()?,
                    axis.title_font.clone(),
                    axis.title.is_none(),
                    axis.show_title,
                )
            }
            ObjectKind::Legend => (
                sheaf_plot::legend_text(doc.project(), graph, layer).ok()?,
                l.legend.font.clone(),
                l.legend.text.is_none(),
                l.legend.visible,
            ),
            _ => return None,
        };
        let mut dialog = Self::new_label(doc, graph, layer, (0.5, 0.5))?;
        dialog.object = Some(object);
        dialog.automatic = automatic;
        dialog.visible = visible;
        dialog.selection = CCursorRange::one(CCursor::new(text.chars().count()));
        dialog.text = text;
        dialog.size = font.size_pt;
        dialog.bold = font.bold;
        dialog.italic = font.italic;
        dialog.font = font;
        dialog.colors = palette::Picker::new(dialog.font.color);
        Some(dialog)
    }

    fn automatic_text(&self, doc: &Document) -> Option<String> {
        match self.object? {
            ObjectKind::Legend => sheaf_plot::legend_automatic_text(doc.project(), self.graph, self.layer).ok(),
            ObjectKind::XTitle | ObjectKind::YTitle => {
                let vertical = self.object == Some(ObjectKind::YTitle);
                let mut project = doc.project().clone();
                let l = project.graph_mut(self.graph)?.layers.get_mut(self.layer)?;
                if vertical {
                    l.y_axis.title = None
                } else {
                    l.x_axis.title = None
                }
                sheaf_plot::axis_title(&project, self.graph, self.layer, vertical).ok()
            }
            _ => None,
        }
    }

    fn is_new_label(&self) -> bool {
        self.object.is_none() && self.label.is_none()
    }

    /// Draws the window. Returns `false` when it was closed.
    pub fn show(&mut self, ctx: &egui::Context, doc: &mut Document) -> bool {
        let exists = doc
            .project()
            .graph(self.graph)
            .and_then(|g| g.layers.get(self.layer))
            .is_some_and(|l| self.label.is_none_or(|k| k < l.labels.len()));
        if !exists || doc.generation() != self.generation {
            return false;
        }
        let mut open = true;
        let mut close = false;
        let caption = match self.object {
            Some(ObjectKind::XTitle) => "X Axis Title",
            Some(ObjectKind::YTitle) => "Y Axis Title",
            Some(ObjectKind::Legend) => "Legend Text",
            _ => "Text",
        };
        let caption = if self.object.is_some() && doc.project().graph(self.graph).is_some_and(|g| g.layers.len() > 1) {
            format!("{caption} - Layer {}", self.layer + 1)
        } else {
            caption.to_string()
        };
        egui::Window::new(caption)
            .id(egui::Id::new("text_object"))
            .default_pos(ctx.content_rect().center() - egui::vec2(200.0, 180.0))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                if self.object.is_some() {
                    ui.horizontal(|ui| {
                        if ui.checkbox(&mut self.automatic, "Automatic text").changed()
                            && self.automatic
                            && let Some(text) = self.automatic_text(doc)
                        {
                            self.text = text;
                            self.selection = CCursorRange::one(CCursor::new(self.text.chars().count()));
                            self.focus = true;
                        }
                        ui.checkbox(&mut self.visible, "Show on graph");
                    });
                }
                let edit_id = ui.make_persistent_id("text_object_content");
                let mut command = None;
                let mut scripts = None;
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
                    egui::containers::menu::MenuButton::new("Text color")
                        .config(
                            egui::containers::menu::MenuConfig::new()
                                .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside),
                        )
                        .ui(ui, |ui| {
                            if let Some(color) = self.colors.show(ui) {
                                command = Some(Format::Color(color));
                                ui.close();
                            }
                        })
                        .0
                        .on_hover_text("Color selected text, or insert colored text at the caret");
                    egui::containers::menu::MenuButton::new("Text font")
                        .config(
                            egui::containers::menu::MenuConfig::new()
                                .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside),
                        )
                        .ui(ui, |ui| {
                            if let Some(name) = self.families.show(ui) {
                                command = Some(Format::Family(name));
                                ui.close();
                            }
                        })
                        .0
                        .on_hover_text("Set the font of selected text, or insert text in that font at the caret");
                    let response = egui::containers::menu::MenuButton::new("Paired scripts")
                        .config(
                            egui::containers::menu::MenuConfig::new()
                                .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside),
                        )
                        .ui(ui, |ui| {
                            if let Some(pair) = self.scripts.show(ui) {
                                scripts = Some(pair);
                                ui.close();
                            }
                        })
                        .0;
                    if response.clicked() {
                        self.scripts.start(&self.text, self.selection, self.object == Some(ObjectKind::Legend));
                    }
                });
                let formatting = command.is_some() || scripts.is_some();
                if formatting {
                    let before = self.text.clone();
                    if let Some((sub, sup)) = scripts {
                        self.selection = paired::insert(
                            &mut self.text,
                            self.selection,
                            self.object == Some(ObjectKind::Legend),
                            &sub,
                            &sup,
                        );
                    } else if let Some(format) = command {
                        self.selection = if self.object == Some(ObjectKind::Legend) {
                            editor::apply_legend(&mut self.text, self.selection, format)
                        } else {
                            editor::apply(&mut self.text, self.selection, format)
                        };
                    }
                    if self.text != before {
                        self.automatic = false;
                    }
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
                if output.response.changed() {
                    self.automatic = false;
                }
                if let Some(selection) = output.cursor_range {
                    self.selection = selection;
                }
                if self.focus || formatting {
                    output.response.request_focus();
                    self.focus = false;
                }
                ui.add_space(4.0);
                egui::Grid::new("text_object_grid").num_columns(2).spacing([14.0, 6.0]).show(ui, |ui| {
                    ui.label("Size");
                    ui.add(DragValue::new(&mut self.size).range(1.0..=1000.0).speed(0.2).suffix(" pt"));
                    ui.end_row();
                    ui.label("Style");
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut self.bold, "Bold");
                        ui.checkbox(&mut self.italic, "Italic");
                    });
                    ui.end_row();
                    if self.object.is_none() {
                        ui.label("Rotation");
                        ui.add(DragValue::new(&mut self.rotation).range(-360.0..=360.0).speed(1.0).suffix(" \u{b0}"));
                        ui.end_row();
                    }
                });
                let mut preview_font = self.font.clone();
                preview_font.size_pt = self.size;
                preview_font.bold = self.bold;
                preview_font.italic = self.italic;
                if let Some(object) = self.object {
                    self.preview.show_object(
                        ui,
                        &self.text,
                        preview_font,
                        preview::Context { doc, graph: self.graph, layer: self.layer, object },
                    );
                } else {
                    self.preview.show(ui, &self.text, preview_font, self.rotation);
                }
                if let Some(p) = &self.problem {
                    ui.colored_label(egui::Color32::from_rgb(170, 30, 30), p);
                }
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.button("OK").clicked() {
                        if self.is_empty() && self.is_new_label() {
                            close = true; // an empty new label cancels
                        } else if self.apply(doc) {
                            close = true;
                        }
                    }
                    if ui.button("Cancel").clicked() {
                        close = true;
                    }
                    if ui.button("Apply").clicked() && !(self.is_empty() && self.is_new_label()) {
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
        if self.object.is_none() && self.is_empty() {
            self.problem = Some("Type some text, or press Delete on the label to remove it.".to_string());
            return false;
        }
        self.problem = None;
        let mut font = self.font.clone();
        font.size_pt = self.size;
        font.bold = self.bold;
        font.italic = self.italic;
        let (graph, layer, text, rotation) = (self.graph, self.layer, self.text.clone(), self.rotation);
        if let Some(object) = self.object {
            let (automatic, visible) = (self.automatic, self.visible);
            let label = if object == ObjectKind::Legend { "Edit legend text" } else { "Edit axis title" };
            doc.apply(label, |p| {
                if let Some(l) = p.graph_mut(graph).and_then(|g| g.layers.get_mut(layer)) {
                    if object == ObjectKind::Legend {
                        l.legend.text = (!automatic).then_some(text);
                        l.legend.font = font;
                        l.legend.visible = visible;
                    } else {
                        let axis = if object == ObjectKind::YTitle { &mut l.y_axis } else { &mut l.x_axis };
                        axis.title = (!automatic).then_some(text);
                        axis.title_font = font;
                        axis.show_title = visible;
                    }
                }
            });
            return true;
        }
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
