//! Choose an existing Y-error column for one plot. Origin 2016's Error Bars window instead
//! generates errors (graphing spec 14.13a); this chooser uses the existing plot-column model.

use eframe::egui;
use sheaf_core::graph::Plot;
use sheaf_core::{ColumnId, Document, Project, WindowId};

pub enum Outcome {
    Open,
    Closed,
    Added,
}

struct Target {
    label: String,
    plot: Plot,
}

pub struct ErrorBarsDialog {
    graph: WindowId,
    layer: usize,
    targets: Vec<Target>,
    target: usize,
    column: Option<ColumnId>,
}

impl ErrorBarsDialog {
    pub fn new(project: &Project, graph: WindowId, layer: usize, selected: Option<usize>) -> Option<Self> {
        let plots = &project.graph(graph)?.layers.get(layer)?.plots;
        if plots.is_empty() {
            return None;
        }
        let targets = plots
            .iter()
            .enumerate()
            .map(|(i, plot)| Target {
                label: format!("{}: {}", i + 1, column_name(project, plot.y)),
                plot: plot.clone(),
            })
            .collect();
        let target = selected.filter(|i| *i < plots.len()).unwrap_or(0);
        let mut dialog = Self { graph, layer, targets, target, column: None };
        dialog.choose_default(project);
        Some(dialog)
    }

    fn choose_default(&mut self, project: &Project) {
        let plot = &self.targets[self.target].plot;
        self.column = plot.y_error.or_else(|| {
            let (_, sheet, _) = project.locate(plot.y)?;
            sheet.y_error_for(sheet.column_index(plot.y)?).map(|c| c.id)
        });
        if !self.column.is_some_and(|c| project.column(c).is_some_and(|c| c.data.is_numeric())) {
            self.column = None;
        }
    }

    fn target_valid(&self, project: &Project) -> bool {
        project
            .graph(self.graph)
            .and_then(|g| g.layers.get(self.layer))
            .and_then(|l| l.plots.get(self.target))
            .is_some_and(|p| {
                *p == self.targets[self.target].plot && project.column(p.y).is_some_and(|c| c.data.is_numeric())
            })
    }

    pub fn show(&mut self, ctx: &egui::Context, doc: &mut Document) -> Outcome {
        if doc.project().graph(self.graph).is_none() || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            return Outcome::Closed;
        }
        let mut open = true;
        let mut outcome = Outcome::Open;
        egui::Window::new("Error Bars")
            .id(egui::Id::new("add_error_bars"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                let before = self.target;
                ui.horizontal(|ui| {
                    let label = ui.label("Add Error Bars to");
                    egui::ComboBox::from_id_salt("error_bar_target")
                        .selected_text(&self.targets[self.target].label)
                        .show_ui(ui, |ui| {
                            for (i, target) in self.targets.iter().enumerate() {
                                ui.selectable_value(&mut self.target, i, &target.label);
                            }
                        })
                        .response
                        .labelled_by(label.id);
                });
                if before != self.target {
                    self.choose_default(doc.project());
                }
                ui.separator();
                ui.label("Source of Errors");
                let sources = numeric_columns(doc.project());
                let chosen = self.column.and_then(|c| sources.iter().find(|(id, _)| *id == c));
                ui.horizontal(|ui| {
                    let label = ui.label("Y Error Column");
                    egui::ComboBox::from_id_salt("error_bar_column")
                        .selected_text(chosen.map_or("Select a column", |(_, name)| name.as_str()))
                        .show_ui(ui, |ui| {
                            for (id, name) in &sources {
                                ui.selectable_value(&mut self.column, Some(*id), name);
                            }
                        })
                        .response
                        .labelled_by(label.id);
                });
                if sources.is_empty() {
                    ui.label("No numeric columns are available.");
                }
                let valid = self.target_valid(doc.project());
                if !valid {
                    ui.colored_label(ui.visuals().error_fg_color, "The plot changed. Reopen Add Error Bars.");
                }
                if self.targets[self.target].plot.y_error.is_some() {
                    ui.label("OK replaces the Y error column of this plot.");
                }
                ui.horizontal(|ui| {
                    if ui.add_enabled(valid && chosen.is_some(), egui::Button::new("OK")).clicked() {
                        self.apply(doc);
                        outcome = Outcome::Added;
                    }
                    if ui.button("Cancel").clicked() {
                        outcome = Outcome::Closed;
                    }
                });
            });
        if !open {
            return Outcome::Closed;
        }
        outcome
    }

    fn apply(&self, doc: &mut Document) {
        // The dialog can stay open while a column or plot changes elsewhere. Recheck before
        // touching the model rather than assigning an old column or plot index.
        if !self.target_valid(doc.project())
            || !self.column.is_some_and(|c| doc.project().column(c).is_some_and(|c| c.data.is_numeric()))
        {
            return;
        }
        doc.apply("Add error bars", |project| {
            let plot = &mut project.graph_mut(self.graph).unwrap().layers[self.layer].plots[self.target];
            plot.y_error = self.column;
        });
    }
}

fn column_name(project: &Project, column: ColumnId) -> String {
    let Some((book, sheet, c)) = project.locate(column) else { return "Missing column".into() };
    let long = if c.long_name.is_empty() { String::new() } else { format!(" ({})", c.long_name) };
    format!("[{}]{}!{}{long}", book.name, sheet.name, c.short_name)
}

fn numeric_columns(project: &Project) -> Vec<(ColumnId, String)> {
    let mut sources = Vec::new();
    for book in project.workbooks() {
        for sheet in &book.sheets {
            for c in &sheet.columns {
                if c.data.is_numeric() {
                    let long = if c.long_name.is_empty() { String::new() } else { format!(" ({})", c.long_name) };
                    sources.push((c.id, format!("[{}]{}!{}{long}", book.name, sheet.name, c.short_name)));
                }
            }
        }
    }
    sources
}
