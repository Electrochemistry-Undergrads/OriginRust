//! Graph > Graph Slicer... (T406): pick a category column of the plots' source sheet and tick the values to show
//! (`Project::set_graph_slicer`); Remove Slicer shows every row again. Rules in `sheaf_core::slicer`.

use eframe::egui;
use sheaf_core::{ColumnId, Project, WindowId};

pub enum SlicerOutcome {
    Open,
    Closed,
    Apply { graph: WindowId, column: ColumnId, values: Vec<String> },
    Remove { graph: WindowId },
}

pub struct GraphSlicerDialog {
    pub graph: WindowId,
    /// The columns of the source sheet: id and name.
    pub columns: Vec<(ColumnId, String)>,
    pub column: usize,
    /// The chosen column's values and whether each is shown.
    pub values: Vec<(String, bool)>,
}

impl GraphSlicerDialog {
    /// The window for `graph`: the columns of the sheet its first plot comes from, starting on the graph's
    /// slicer when it has one, else on the first text column. `None` when the graph plots no worksheet column.
    pub fn new(project: &Project, graph: WindowId) -> Option<Self> {
        let g = project.graph(graph)?;
        let y = g.layers.iter().flat_map(|l| l.plots.iter()).map(|p| p.y).next()?;
        let (_, sheet, _) = project.locate(y)?;
        let columns: Vec<(ColumnId, String)> =
            sheet.columns.iter().map(|c| (c.id, c.display_name().to_string())).collect();
        let current = g.slicer.as_ref();
        let column = current
            .and_then(|s| columns.iter().position(|c| c.0 == s.column))
            .or_else(|| sheet.columns.iter().position(|c| !c.data.is_numeric()))
            .unwrap_or(0);
        let mut d = GraphSlicerDialog { graph, columns, column, values: Vec::new() };
        d.load(project);
        if let Some(s) = current.filter(|s| d.columns.get(d.column).is_some_and(|c| c.0 == s.column)) {
            for (v, on) in &mut d.values {
                *on = s.values.contains(v);
            }
        }
        Some(d)
    }

    /// Reads the chosen column's values, all ticked.
    pub fn load(&mut self, project: &Project) {
        self.values = self
            .columns
            .get(self.column)
            .map(|c| project.slicer_values(c.0).into_iter().map(|v| (v, true)).collect())
            .unwrap_or_default();
    }

    pub fn show(&mut self, ctx: &egui::Context, project: &Project) -> SlicerOutcome {
        let mut open = true;
        let mut outcome = SlicerOutcome::Open;
        let mut reload = false;
        egui::Window::new("Graph Slicer")
            .id(egui::Id::new("graph_slicer"))
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Slice by");
                    let shown = self.columns.get(self.column).map_or("", |c| c.1.as_str()).to_string();
                    egui::ComboBox::from_id_salt("graph_slicer_column").selected_text(shown).show_ui(ui, |ui| {
                        for (k, (_, name)) in self.columns.iter().enumerate() {
                            if ui.selectable_label(k == self.column, name).clicked() && k != self.column {
                                self.column = k;
                                reload = true;
                            }
                        }
                    });
                });
                egui::ScrollArea::vertical().max_height(240.0).show(ui, |ui| {
                    for (v, on) in &mut self.values {
                        ui.checkbox(on, v.as_str());
                    }
                });
                ui.horizontal(|ui| {
                    if ui.button("Select All").clicked() {
                        self.values.iter_mut().for_each(|v| v.1 = true);
                    }
                    if ui.button("Clear All").clicked() {
                        self.values.iter_mut().for_each(|v| v.1 = false);
                    }
                });
                ui.separator();
                ui.horizontal(|ui| {
                    let any = self.values.iter().any(|v| v.1);
                    if ui.add_enabled(any, egui::Button::new("OK")).clicked()
                        && let Some(c) = self.columns.get(self.column)
                    {
                        outcome = SlicerOutcome::Apply {
                            graph: self.graph,
                            column: c.0,
                            values: self.values.iter().filter(|v| v.1).map(|v| v.0.clone()).collect(),
                        };
                    }
                    if ui.button("Remove Slicer").clicked() {
                        outcome = SlicerOutcome::Remove { graph: self.graph };
                    }
                    if ui.button("Cancel").clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        outcome = SlicerOutcome::Closed;
                    }
                });
            });
        if reload {
            self.load(project);
        }
        if !open { SlicerOutcome::Closed } else { outcome }
    }
}
