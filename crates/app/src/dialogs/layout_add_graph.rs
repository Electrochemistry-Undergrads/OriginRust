//! Layout > Add Graph... (T237): choose a graph of the project; OK puts its picture on the page in the
//! next free cell, as one undo step.

use eframe::egui;
use sheaf_core::{Document, WindowId};

pub enum Outcome {
    Open,
    Closed,
    Done,
}

pub struct LayoutAddGraph {
    pub layout: WindowId,
    pub choices: Vec<(WindowId, String)>,
    pub pick: usize,
}

/// Adds the picture of `graph` to `layout` as one undo step.
pub fn add_graph(doc: &mut Document, layout: WindowId, graph: WindowId) -> Result<(), String> {
    let g = doc.project().graph(graph).ok_or("the graph is gone")?;
    let aspect = g.page.width_pt / g.page.height_pt.max(1.0);
    doc.edit("Add graph to layout", |p| {
        p.layout_mut(layout).ok_or("the layout page is gone")?.add_graph(graph, aspect);
        Ok::<_, String>(())
    })
}

impl LayoutAddGraph {
    /// The window for `layout`, listing the project's graphs; `None` when there is none.
    pub fn new(doc: &Document, layout: WindowId) -> Option<Self> {
        let choices: Vec<(WindowId, String)> = doc.project().graphs().map(|g| (g.id, g.title().to_string())).collect();
        (!choices.is_empty()).then_some(LayoutAddGraph { layout, choices, pick: 0 })
    }

    pub fn show(&mut self, ctx: &egui::Context, doc: &mut Document) -> Outcome {
        if doc.project().layout(self.layout).is_none() || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            return Outcome::Closed;
        }
        let mut open = true;
        let (mut ok, mut cancel) = (false, false);
        egui::Window::new("Add Graph")
            .id(egui::Id::new("layout_add_graph"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Graph");
                    let shown = self.choices.get(self.pick).map(|c| c.1.clone()).unwrap_or_default();
                    egui::ComboBox::from_id_salt("layout_add_graph_pick").selected_text(shown).show_ui(ui, |ui| {
                        for (k, (_, name)) in self.choices.iter().enumerate() {
                            ui.selectable_value(&mut self.pick, k, name);
                        }
                    });
                });
                ui.separator();
                ui.horizontal(|ui| {
                    ok = ui.button("OK").clicked();
                    cancel = ui.button("Cancel").clicked();
                });
            });
        if !open || cancel {
            return Outcome::Closed;
        }
        if ok && let Some((graph, _)) = self.choices.get(self.pick) {
            return match add_graph(doc, self.layout, *graph) {
                Ok(()) => Outcome::Done,
                Err(_) => Outcome::Closed,
            };
        }
        Outcome::Open
    }
}
