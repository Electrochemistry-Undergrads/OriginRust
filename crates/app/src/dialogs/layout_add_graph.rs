//! Layout > Add Graph... and Add Worksheet... (T237): choose a graph, or a worksheet, of the
//! project; OK puts its picture on the page in the next free cell, as one undo step.

use eframe::egui;
use sheaf_core::layout_page::LayoutContent;
use sheaf_core::{Document, WindowId};

pub enum Outcome {
    Open,
    Closed,
    Done,
}

pub struct LayoutAddGraph {
    pub layout: WindowId,
    /// Worksheets (Add Worksheet...) rather than graphs.
    pub sheets: bool,
    pub choices: Vec<(LayoutContent, String)>,
    pub pick: usize,
}

/// Adds the picture of `content` (a graph or a sheet) to `layout` as one undo step.
pub fn add_item(doc: &mut Document, layout: WindowId, content: LayoutContent) -> Result<(), String> {
    let aspect = sheaf_plot::layout_item_aspect(doc.project(), &content).ok_or("the window is gone")?;
    let label = match content {
        LayoutContent::Sheet { .. } => "Add worksheet to layout",
        _ => "Add graph to layout",
    };
    doc.edit(label, |p| {
        p.layout_mut(layout).ok_or("the layout page is gone")?.add(content, aspect);
        Ok::<_, String>(())
    })
}

impl LayoutAddGraph {
    /// The window for `layout`, listing the project's graphs (or, with `sheets`, every sheet of every
    /// workbook as `[Book1]Sheet1`); `None` when there is none.
    pub fn new(doc: &Document, layout: WindowId, sheets: bool) -> Option<Self> {
        let p = doc.project();
        let choices: Vec<(LayoutContent, String)> = if sheets {
            p.workbooks()
                .flat_map(|b| {
                    b.sheets.iter().enumerate().map(move |(k, s)| {
                        (LayoutContent::Sheet { book: b.id, sheet: k }, format!("[{}]{}", b.name, s.name))
                    })
                })
                .collect()
        } else {
            p.graphs().map(|g| (LayoutContent::Graph { graph: g.id }, g.title().to_string())).collect()
        };
        (!choices.is_empty()).then_some(LayoutAddGraph { layout, sheets, choices, pick: 0 })
    }

    pub fn show(&mut self, ctx: &egui::Context, doc: &mut Document) -> Outcome {
        if doc.project().layout(self.layout).is_none() || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            return Outcome::Closed;
        }
        let mut open = true;
        let (mut ok, mut cancel) = (false, false);
        let (title, what) = if self.sheets { ("Add Worksheet", "Worksheet") } else { ("Add Graph", "Graph") };
        egui::Window::new(title)
            .id(egui::Id::new("layout_add_graph"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(what);
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
        if ok && let Some((content, _)) = self.choices.get(self.pick) {
            return match add_item(doc, self.layout, content.clone()) {
                Ok(()) => Outcome::Done,
                Err(_) => Outcome::Closed,
            };
        }
        Outcome::Open
    }
}
