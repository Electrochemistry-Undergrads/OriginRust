//! The format clipboard of the application and the Apply Formats window (brief T72, graphing spec
//! section 24). Copy Format puts a [`FormatClip`] here; Paste Format and Paste Format To apply it
//! to a graph or an axis as one undo step named "Apply Theme" (Origin's own word).
//!
//! The clipboard is inside the program: Origin does not use the system clipboard for formats.
//! Origin keeps it across a restart; this one does not (left open).

use eframe::egui;
use sheaf_core::format_clip::{self, AxisPart, FormatClip, GraphPart, Group};
use sheaf_core::{Document, WindowId};
use std::cell::RefCell;

/// The undo step of every paste of a format (Origin's text, graphing spec 24).
pub const APPLY_UNDO: &str = "Apply Theme";

thread_local! {
    static CLIP: RefCell<Option<FormatClip>> = const { RefCell::new(None) };
}

/// What the clipboard holds, which decides which paste items the menus offer.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ClipKind {
    Graph,
    Axis,
}

pub fn kind() -> Option<ClipKind> {
    CLIP.with(|c| {
        c.borrow().as_ref().map(|c| match c {
            FormatClip::Graph(_) => ClipKind::Graph,
            FormatClip::Axis(_) => ClipKind::Axis,
        })
    })
}

fn held() -> Option<FormatClip> {
    CLIP.with(|c| c.borrow().clone())
}

/// Copy Format of a whole graph.
pub fn copy_graph(doc: &Document, graph: WindowId, part: GraphPart) -> bool {
    let Some(g) = doc.project().graph(graph) else { return false };
    let clip = format_clip::copy_graph(g, part);
    CLIP.with(|c| *c.borrow_mut() = Some(clip));
    true
}

/// Copy Format of one axis.
pub fn copy_axis(doc: &Document, graph: WindowId, layer: usize, y: bool, part: AxisPart) -> bool {
    let Some(clip) = doc.project().graph(graph).and_then(|g| format_clip::copy_axis(g, layer, y, part)) else {
        return false;
    };
    CLIP.with(|c| *c.borrow_mut() = Some(clip));
    true
}

/// Applies `change` to a copy of the graph first and records an undo step only when it changes
/// something.
fn apply(doc: &mut Document, graph: WindowId, change: impl Fn(&mut sheaf_core::Graph) -> bool) -> bool {
    let Some(mut probe) = doc.project().graph(graph).cloned() else { return false };
    let before = probe.clone();
    if !change(&mut probe) || probe == before {
        return false;
    }
    doc.apply(APPLY_UNDO, |p| {
        if let Some(g) = p.graph_mut(graph) {
            change(g);
        }
    });
    true
}

/// Paste Format of a graph format onto a graph. False when nothing changed (empty clipboard, an
/// axis format, or nothing the format holds differs).
pub fn paste_graph(doc: &mut Document, graph: WindowId, parts: &[Group]) -> bool {
    let Some(clip) = held() else { return false };
    apply(doc, graph, |g| format_clip::paste_graph(g, &clip, parts))
}

/// Paste Format To of an axis format onto one axis.
pub fn paste_axis(doc: &mut Document, graph: WindowId, layer: usize, y: bool, part: AxisPart) -> bool {
    let Some(clip) = held() else { return false };
    apply(doc, graph, |g| format_clip::paste_axis(g, layer, y, &clip, part))
}

/// The label of a group in the Apply Formats window.
fn group_label(g: Group) -> &'static str {
    match g {
        Group::Dimensions => "Dimensions",
        Group::Scales => "Scales",
        Group::Colors => "Colors",
        Group::Fonts => "Fonts",
        Group::Background => "Background",
        Group::Thickness => "Thickness",
        Group::SymbolLineFill => "Symbol/Line/Fill",
        Group::Text => "Text",
    }
}

/// Edit > Paste Format...: the modeless Apply Formats window. It shows what the clipboard holds
/// with a check box per group, an Apply Scope and, for an axis format, which axis gets it.
/// Its Apply, Undo Apply and Close buttons follow Origin's; the contents of Origin's window for a
/// graph format were not measured (spec 24).
pub struct ApplyFormats {
    graph: WindowId,
    layer: usize,
    checks: Vec<(Group, bool)>,
    axis_scales: bool,
    axis_styles: bool,
    y_axis: bool,
    applied: bool,
}

impl ApplyFormats {
    pub fn new(graph: WindowId, layer: usize) -> Self {
        let mut w = ApplyFormats {
            graph,
            layer,
            checks: Vec::new(),
            axis_scales: true,
            axis_styles: true,
            y_axis: false,
            applied: false,
        };
        w.sync_checks();
        w
    }

    /// Rebuilds the group check boxes when the clipboard holds another graph format than the one
    /// they were made for (the window is modeless: Copy Format may run while it is open).
    fn sync_checks(&mut self) {
        let groups: Vec<Group> = match held() {
            Some(FormatClip::Graph(f)) => f.groups().to_vec(),
            _ => Vec::new(),
        };
        if self.checks.iter().map(|(g, _)| *g).ne(groups.iter().copied()) {
            self.checks = groups.into_iter().map(|g| (g, true)).collect();
        }
    }

    /// Draws the window. Returns `false` when it was closed.
    pub fn show(&mut self, ctx: &egui::Context, doc: &mut Document) -> bool {
        self.sync_checks();
        let mut open = true;
        let mut close = false;
        egui::Window::new("Apply Formats").id(egui::Id::new("apply_formats")).open(&mut open).resizable(false).show(
            ctx,
            |ui| {
                ui.horizontal(|ui| {
                    ui.label("Apply From");
                    ui.label("Clipboard");
                });
                ui.separator();
                match held() {
                    None => {
                        ui.label("The clipboard holds no format.");
                    }
                    Some(FormatClip::Graph(_)) => {
                        ui.label("Formats to Apply");
                        for (g, on) in &mut self.checks {
                            ui.checkbox(on, group_label(*g));
                        }
                        ui.label("Apply Scope: Active Graph");
                    }
                    Some(FormatClip::Axis(f)) => {
                        ui.label("Formats to Apply");
                        ui.add_enabled(f.scales, egui::Checkbox::new(&mut self.axis_scales, "Scales"));
                        ui.add_enabled(f.styles, egui::Checkbox::new(&mut self.axis_styles, "Styles"));
                        ui.label("Apply Scope: Active Layer");
                        ui.horizontal(|ui| {
                            ui.label("Apply To");
                            ui.radio_value(&mut self.y_axis, false, "Bottom Axes");
                            ui.radio_value(&mut self.y_axis, true, "Left Axes");
                        });
                    }
                }
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    let can_undo = self.applied && doc.undo_label() == Some(APPLY_UNDO);
                    if ui.add_enabled(can_undo, egui::Button::new("Undo Apply")).clicked() {
                        doc.undo();
                        self.applied = false;
                    }
                    if ui.add_enabled(kind().is_some(), egui::Button::new("Apply")).clicked() {
                        self.apply(doc);
                    }
                    if ui.button("Close").clicked() {
                        close = true;
                    }
                });
            },
        );
        open && !close
    }

    fn apply(&mut self, doc: &mut Document) {
        let changed = match kind() {
            Some(ClipKind::Graph) => {
                let parts: Vec<Group> = self.checks.iter().filter(|(_, on)| *on).map(|(g, _)| *g).collect();
                paste_graph(doc, self.graph, &parts)
            }
            Some(ClipKind::Axis) => {
                let part = match (self.axis_scales, self.axis_styles) {
                    (true, true) => AxisPart::All,
                    (true, false) => AxisPart::Scales,
                    (false, true) => AxisPart::Styles,
                    (false, false) => return,
                };
                paste_axis(doc, self.graph, self.layer, self.y_axis, part)
            }
            None => false,
        };
        self.applied |= changed;
    }
}
