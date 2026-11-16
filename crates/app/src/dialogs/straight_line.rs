//! Graph > Add Straight Line... (T67): a vertical line at an X value or a horizontal one at a Y
//! value, with a label that shows the value. The same window edits a line (double click on it).
//!
//! A new line is not in the project until OK: Cancel adds nothing. OK is one undo step. The ends of
//! a new line are the other axis' range at that moment, as Origin fixes them.

use super::graph_props::color_button;
use eframe::egui;
use sheaf_core::graph::Dash;
use sheaf_core::straight_line::{StraightLine, StraightLineLabel};
use sheaf_core::{Color, Document, WindowId};

/// The names of Origin's eight line styles, in the order of [`Dash::ALL`].
const STYLE_NAMES: [&str; 8] =
    ["Solid", "Dash", "Dot", "Dash Dot", "Dash Dot Dot", "Short Dash", "Short Dot", "Short Dash Dot"];

fn style_name(d: Dash) -> &'static str {
    Dash::ALL.iter().position(|x| *x == d).map_or("Solid", |i| STYLE_NAMES[i])
}

pub struct StraightLineDialog {
    graph: WindowId,
    layer: usize,
    /// The line being edited with what it was when the window opened (to tell when Delete, Undo or
    /// Redo put another one at its index); `None` while it is new.
    existing: Option<(usize, StraightLine)>,
    /// The layer's X and Y ranges now, for the ends of a new line.
    ranges: ((f64, f64), (f64, f64)),
    vertical: bool,
    value: String,
    color: Color,
    dash: Dash,
    format: String,
    location: StraightLineLabel,
    selectable: bool,
    movable: bool,
    name: String,
    /// The name the window suggested, so that a change of type can suggest the other kind.
    auto_name: String,
    problem: Option<String>,
    /// The value box takes the keyboard when the window opens.
    focus: bool,
}

impl StraightLineDialog {
    fn ranges(doc: &Document, graph: WindowId, layer: usize) -> ((f64, f64), (f64, f64)) {
        match sheaf_plot::layout_geometry(doc.project(), graph) {
            Ok(g) if layer < g.len() => ((g[layer].x.min, g[layer].x.max), (g[layer].y.min, g[layer].y.max)),
            _ => ((0.0, 1.0), (0.0, 1.0)),
        }
    }

    /// For a new line in `layer`: vertical at 0, Origin's defaults, the next free name.
    pub fn new_line(doc: &Document, graph: WindowId, layer: usize) -> Option<Self> {
        let l = doc.project().graph(graph)?.layers.get(layer)?;
        let d = StraightLine::new(true, 0.0);
        Some(StraightLineDialog {
            graph,
            layer,
            existing: None,
            ranges: Self::ranges(doc, graph, layer),
            vertical: d.vertical,
            value: "0".to_string(),
            color: d.color,
            dash: d.dash,
            format: String::new(),
            location: d.location,
            selectable: d.selectable,
            movable: d.movable,
            name: StraightLine::next_name(&l.straight_lines, true),
            auto_name: StraightLine::next_name(&l.straight_lines, true),
            problem: None,
            focus: true,
        })
    }

    /// For line number `line` of `layer`.
    pub fn edit(doc: &Document, graph: WindowId, layer: usize, line: usize) -> Option<Self> {
        let s = doc.project().graph(graph)?.layers.get(layer)?.straight_lines.get(line)?.clone();
        Some(StraightLineDialog {
            graph,
            layer,
            ranges: Self::ranges(doc, graph, layer),
            vertical: s.vertical,
            value: format!("{}", s.value),
            color: s.color,
            dash: s.dash,
            format: s.format.clone().unwrap_or_default(),
            location: s.location,
            selectable: s.selectable,
            movable: s.movable,
            name: s.name.clone(),
            auto_name: String::new(),
            existing: Some((line, s)),
            problem: None,
            focus: true,
        })
    }

    /// Draws the window. Returns `false` when it was closed or when its line is gone.
    pub fn show(&mut self, ctx: &egui::Context, doc: &mut Document) -> bool {
        let lines = doc.project().graph(self.graph).and_then(|g| g.layers.get(self.layer)).map(|l| &l.straight_lines);
        let Some(lines) = lines else { return false };
        if let Some((k, snapshot)) = &self.existing
            && lines.get(*k) != Some(snapshot)
        {
            return false;
        }
        let next = [StraightLine::next_name(lines, true), StraightLine::next_name(lines, false)];
        let mut open = true;
        let mut close = false;
        // Editing an existing line is not adding one (Sheaf's title; Origin's line properties are its
        // object Properties window).
        let title = if self.existing.is_some() { "Straight Line" } else { "Add Straight Line" };
        egui::Window::new(title)
            .id(egui::Id::new("straight_line"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                egui::Grid::new("straight_line_grid").num_columns(2).spacing([14.0, 6.0]).show(ui, |ui| {
                    ui.label("Type");
                    ui.horizontal(|ui| {
                        ui.radio_value(&mut self.vertical, true, "Vertical");
                        ui.radio_value(&mut self.vertical, false, "Horizontal");
                    });
                    ui.end_row();
                    let kind = usize::from(!self.vertical);
                    if self.name == self.auto_name && self.name != next[kind] {
                        self.name = next[kind].clone();
                        self.auto_name = self.name.clone();
                    }
                    ui.label("At Value");
                    let response = ui.add(egui::TextEdit::singleline(&mut self.value).desired_width(120.0));
                    if self.focus {
                        response.request_focus();
                        self.focus = false;
                    }
                    ui.end_row();
                    ui.label("Color");
                    color_button(ui, &mut self.color);
                    ui.end_row();
                    ui.label("Style");
                    egui::ComboBox::from_id_salt("straight_line_style").selected_text(style_name(self.dash)).show_ui(
                        ui,
                        |ui| {
                            for (d, name) in Dash::ALL.into_iter().zip(STYLE_NAMES) {
                                ui.selectable_value(&mut self.dash, d, name);
                            }
                        },
                    );
                    ui.end_row();
                    ui.label("Label Display Format");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.format)
                            .desired_width(120.0)
                            .hint_text("auto, or .2 for two decimals"),
                    );
                    ui.end_row();
                    ui.label("Label Location");
                    egui::ComboBox::from_id_salt("straight_line_location").selected_text(self.location.name()).show_ui(
                        ui,
                        |ui| {
                            for l in StraightLineLabel::ALL {
                                ui.selectable_value(&mut self.location, l, l.name());
                            }
                        },
                    );
                    ui.end_row();
                    ui.label("");
                    ui.checkbox(&mut self.selectable, "Selectable");
                    ui.end_row();
                    ui.label("");
                    // Dragging a line is not built (it waits for the drawn objects' drag code).
                    ui.add_enabled(false, egui::Checkbox::new(&mut self.movable, "Moveable"))
                        .on_disabled_hover_text("Dragging a straight line is not built yet");
                    ui.end_row();
                    ui.label("Name");
                    ui.add(egui::TextEdit::singleline(&mut self.name).desired_width(120.0));
                    ui.end_row();
                });
                if let Some(p) = &self.problem {
                    ui.colored_label(egui::Color32::from_rgb(170, 30, 30), p);
                }
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.button("OK").clicked() && self.apply(doc) {
                        close = true;
                    }
                    if ui.button("Cancel").clicked() {
                        close = true;
                    }
                });
            });
        open && !close
    }

    /// Adds or replaces the line. Returns `false` when the value is not a number.
    fn apply(&mut self, doc: &mut Document) -> bool {
        let value = match self.value.trim().parse::<f64>() {
            Ok(v) if v.is_finite() => v,
            _ => {
                self.problem = Some("At Value must be a number.".to_string());
                return false;
            }
        };
        let format = self.format.trim();
        let (graph, layer) = (self.graph, self.layer);
        let base =
            self.existing.as_ref().map(|(_, s)| s.clone()).unwrap_or_else(|| StraightLine::new(self.vertical, value));
        let mut line = StraightLine {
            vertical: self.vertical,
            value,
            color: self.color,
            dash: self.dash,
            format: (!format.is_empty()).then(|| format.to_string()),
            location: self.location,
            selectable: self.selectable,
            movable: self.movable,
            name: self.name.trim().to_string(),
            ..base
        };
        // The ends are the other axis' range, fixed now; an edit that keeps the type keeps its ends.
        if self.existing.as_ref().is_none_or(|(_, s)| s.vertical != self.vertical) {
            line.span = Some(if self.vertical { self.ranges.1 } else { self.ranges.0 });
        }
        let existing = self.existing.as_ref().map(|(k, _)| *k);
        if line.name.is_empty() {
            let others = doc.project().graph(graph).and_then(|g| g.layers.get(layer)).map(|l| &l.straight_lines[..]);
            line.name = StraightLine::next_name(others.unwrap_or(&[]), self.vertical);
        }
        doc.apply(if existing.is_some() { "Edit straight line" } else { "Add straight line" }, |p| {
            if let Some(l) = p.graph_mut(graph).and_then(|g| g.layers.get_mut(layer)) {
                match existing {
                    Some(k) if k < l.straight_lines.len() => l.straight_lines[k] = line,
                    Some(_) => {}
                    None => l.straight_lines.push(line),
                }
            }
        });
        true
    }
}
