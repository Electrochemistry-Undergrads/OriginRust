//! Graph > Graph Slicer... (T406): pick a category column of the plots' source sheet and tick the values to show
//! (`Project::set_graph_slicer`); Remove Slicer shows every row again. Scope chooses the whole page or one layer and
//! Extract applies the slicer and copies the rows it shows to a new workbook (T444). Rules in `sheaf_core::slicer`.

use eframe::egui;
use sheaf_core::{ColumnId, Project, WindowId};

pub enum SlicerOutcome {
    Open,
    Closed,
    Apply {
        graph: WindowId,
        /// The slicer columns with their values shown, the first the slicer's own (T487).
        columns: Vec<(ColumnId, Vec<String>)>,
        layer: Option<usize>,
        plot: Option<usize>,
        extract: bool,
    },
    Remove {
        graph: WindowId,
    },
}

pub struct GraphSlicerDialog {
    pub graph: WindowId,
    /// The columns of the source sheet: id and name.
    pub columns: Vec<(ColumnId, String)>,
    pub column: usize,
    /// The chosen column's values and whether each is shown.
    pub values: Vec<(String, bool)>,
    /// The graph's number of layers, and the scope: one layer, or `None` for the page (T444).
    pub layers: usize,
    pub layer: Option<usize>,
    /// The plots of each layer by name, and the plot scope within the chosen layer (T467).
    pub plots: Vec<Vec<String>>,
    pub plot: Option<usize>,
    /// Further slicer columns (T487): the column's index and its values with whether each is shown.
    pub more: Vec<(usize, Vec<(String, bool)>)>,
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
        let layer = current.and_then(|s| s.layer);
        let plot = current.and_then(|s| s.plot);
        let plots = g.layers.iter().map(|l| l.plots.iter().map(|p| project.plot_label(p)).collect()).collect();
        let mut d = GraphSlicerDialog {
            graph,
            columns,
            column,
            values: Vec::new(),
            layers: g.layers.len(),
            layer,
            plots,
            plot,
            more: Vec::new(),
        };
        d.load(project);
        if let Some(s) = current.filter(|s| d.columns.get(d.column).is_some_and(|c| c.0 == s.column)) {
            for (v, on) in &mut d.values {
                *on = s.values.contains(v);
            }
            // T487: the further columns of the slicer
            for m in &s.more {
                if let Some(k) = d.columns.iter().position(|c| c.0 == m.column) {
                    let values = project
                        .slicer_values(m.column)
                        .into_iter()
                        .map(|v| (v.clone(), m.values.contains(&v)))
                        .collect();
                    d.more.push((k, values));
                }
            }
        }
        Some(d)
    }

    /// The columns and values the dialog applies: its own column, then the further ones (T487).
    pub fn chosen(&self) -> Vec<(ColumnId, Vec<String>)> {
        let ticked = |v: &[(String, bool)]| v.iter().filter(|v| v.1).map(|v| v.0.clone()).collect::<Vec<_>>();
        let mut out: Vec<(ColumnId, Vec<String>)> =
            self.columns.get(self.column).map(|c| (c.0, ticked(&self.values))).into_iter().collect();
        out.extend(self.more.iter().filter_map(|(k, v)| self.columns.get(*k).map(|c| (c.0, ticked(v)))));
        out
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
                if self.layers > 1 || self.plots.first().is_some_and(|p| p.len() > 1) {
                    ui.horizontal(|ui| {
                        ui.label("Scope");
                        let shown = self.layer.map_or("Page".to_string(), |l| format!("Layer {}", l + 1));
                        egui::ComboBox::from_id_salt("graph_slicer_scope").selected_text(shown).show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.layer, None, "Page");
                            for l in 0..self.layers {
                                ui.selectable_value(&mut self.layer, Some(l), format!("Layer {}", l + 1));
                            }
                        });
                        // Plot scope (T467): one plot of the chosen layer.
                        if let Some(names) = self.layer.and_then(|l| self.plots.get(l)) {
                            let shown =
                                self.plot.and_then(|p| names.get(p)).map_or("All plots".to_string(), Clone::clone);
                            egui::ComboBox::from_id_salt("graph_slicer_plot").selected_text(shown).show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.plot, None, "All plots");
                                for (k, n) in names.iter().enumerate() {
                                    ui.selectable_value(&mut self.plot, Some(k), n);
                                }
                            });
                        } else {
                            self.plot = None;
                        }
                    });
                }
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
                // T487: further slicer columns of the sheet, each with its own ticks
                let mut drop = None;
                for (n, (k, values)) in self.more.iter_mut().enumerate() {
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label("And by");
                        let shown = self.columns.get(*k).map_or("", |c| c.1.as_str()).to_string();
                        egui::ComboBox::from_id_salt(("graph_slicer_more", n)).selected_text(shown).show_ui(ui, |ui| {
                            for (j, (id, name)) in self.columns.iter().enumerate() {
                                if ui.selectable_label(j == *k, name).clicked() && j != *k {
                                    *k = j;
                                    *values = project.slicer_values(*id).into_iter().map(|v| (v, true)).collect();
                                }
                            }
                        });
                        if ui.small_button("Remove").clicked() {
                            drop = Some(n);
                        }
                    });
                    egui::ScrollArea::vertical().id_salt(("graph_slicer_more_values", n)).max_height(120.0).show(
                        ui,
                        |ui| {
                            for (v, on) in values.iter_mut() {
                                ui.checkbox(on, v.as_str());
                            }
                        },
                    );
                }
                if let Some(n) = drop {
                    self.more.remove(n);
                }
                if ui.button("Add Slicer Column").clicked() {
                    // the next column not yet used
                    let used: Vec<usize> = std::iter::once(self.column).chain(self.more.iter().map(|m| m.0)).collect();
                    if let Some(k) = (0..self.columns.len()).find(|k| !used.contains(k)) {
                        let values = project.slicer_values(self.columns[k].0).into_iter().map(|v| (v, true)).collect();
                        self.more.push((k, values));
                    }
                }
                ui.separator();
                ui.horizontal(|ui| {
                    let any = self.values.iter().any(|v| v.1) && self.more.iter().all(|m| m.1.iter().any(|v| v.1));
                    for (label, extract) in [("OK", false), ("Extract", true)] {
                        let button = ui.add_enabled(any, egui::Button::new(label));
                        let button = if extract {
                            button.on_hover_text("Apply and copy the rows shown to a new workbook")
                        } else {
                            button
                        };
                        if button.clicked() && self.columns.get(self.column).is_some() {
                            outcome = SlicerOutcome::Apply {
                                graph: self.graph,
                                columns: self.chosen(),
                                layer: self.layer,
                                plot: self.plot.filter(|_| self.layer.is_some()),
                                extract,
                            };
                        }
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
