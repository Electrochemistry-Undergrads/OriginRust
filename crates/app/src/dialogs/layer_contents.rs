//! Graph > Layer Contents... (F12) as Origin's own window "Layer Contents: Add, Remove, Group, Order
//! Plots - Layer1" (measured, graphing spec 18.2): the worksheet columns at the left, the plots of the
//! layer at the right, a plot type and the arrows between them, Group, Ungroup, Up, Down, Rescale on
//! Apply, and Layer Properties..., Plot Setup... (greyed), Apply, Close and Cancel. The edits are made
//! to the graph at once and form one undo step; Cancel takes them back.

use eframe::egui::{self, RichText};
use sheaf_core::graph::*;
use sheaf_core::{ColumnId, Document, Project, WindowId};

const LABEL: &str = "Change layer contents";

/// One numeric Y column the left list offers.
struct Available {
    y: ColumnId,
    x: Option<ColumnId>,
    y_error: Option<ColumnId>,
    book: String,
    sheet: String,
    long_name: String,
    short_name: String,
}

/// The kinds the plot type button offers (Origin's list starts with these).
const KINDS: [(PlotKind, &str); 4] = [
    (PlotKind::Line, "Line"),
    (PlotKind::Scatter, "Scatter"),
    (PlotKind::LineSymbol, "Line + Symbol"),
    (PlotKind::Column, "Column"),
];

fn kind_name(k: PlotKind) -> String {
    KINDS.iter().find(|(x, _)| *x == k).map_or_else(|| format!("{k:?}"), |(_, n)| (*n).to_string())
}

/// What the app should do after a frame of the window.
pub enum Outcome {
    Open,
    Closed,
    /// Layer Properties...: open Plot Details on this layer.
    LayerProperties(WindowId, usize),
    /// Plot Setup...: Origin closes Layer Contents and opens Plot Setup on the layer.
    PlotSetup(WindowId, usize),
}

pub struct LayerContentsDialog {
    pub graph: WindowId,
    pub layer: usize,
    token: u64,
    baseline: Option<Graph>,
    touched: bool,
    left: Vec<usize>,
    right: Vec<usize>,
    kind: PlotKind,
    rescale: bool,
}

impl LayerContentsDialog {
    pub fn new(graph: WindowId, layer: usize, token: u64) -> Self {
        LayerContentsDialog {
            graph,
            layer,
            token,
            baseline: None,
            touched: false,
            left: Vec::new(),
            right: Vec::new(),
            kind: PlotKind::Line,
            rescale: false,
        }
    }

    fn available(project: &Project) -> Vec<Available> {
        let mut out = Vec::new();
        for book in project.workbooks() {
            for sheet in &book.sheets {
                for i in sheet.y_columns() {
                    let c = &sheet.columns[i];
                    if !c.data.is_numeric() {
                        continue;
                    }
                    let x = sheet.x_for(i).map(|c| c.id);
                    let y_error = sheet.y_error_for(i).map(|c| c.id);
                    out.push(Available {
                        y: c.id,
                        x,
                        y_error,
                        book: book.name.clone(),
                        sheet: sheet.name.clone(),
                        long_name: c.long_name.clone(),
                        short_name: format!("{}(Y)", c.short_name),
                    });
                }
            }
        }
        out
    }

    /// Draws the window; the app keeps it while this returns [`Outcome::Open`].
    pub fn show(&mut self, ctx: &egui::Context, doc: &mut Document) -> Outcome {
        let Some(original) = doc.project().graph(self.graph).cloned() else {
            return Outcome::Closed;
        };
        if original.layers.is_empty() {
            return Outcome::Closed;
        }
        self.layer = self.layer.min(original.layers.len() - 1);
        if self.baseline.is_none() {
            self.baseline = Some(original.clone());
        }
        let available = Self::available(doc.project());
        let short = |id: ColumnId| doc.project().column(id).map(|c| c.short_name.clone()).unwrap_or_default();
        let long = |id: ColumnId| doc.project().column(id).map(|c| c.long_name.clone()).unwrap_or_default();
        let mut graph = original.clone();
        let li = self.layer;
        let mut outcome = Outcome::Open;
        let mut cancel = false;
        let mut open = true;
        let title = format!("Layer Contents: Add, Remove, Group, Order Plots - Layer{}", li + 1);
        egui::Window::new(title)
            .id(egui::Id::new("layer_contents"))
            .open(&mut open)
            .default_size([700.0, 320.0])
            .collapsible(false)
            .show(ctx, |ui| {
                ui.horizontal_top(|ui| {
                    // Left: the columns that can be plotted.
                    ui.vertical(|ui| {
                        egui::ComboBox::from_id_salt("lc_source").selected_text("Worksheets in Project").show_ui(
                            ui,
                            |ui| {
                                let _ = ui.selectable_label(true, "Worksheets in Project");
                            },
                        );
                        egui::ScrollArea::vertical().id_salt("lc_left").max_height(200.0).show(ui, |ui| {
                            egui::Grid::new("lc_left_grid").striped(true).num_columns(4).show(ui, |ui| {
                                for h in ["Book Name", "Sheet", "Long Name", "Short Name"] {
                                    ui.label(RichText::new(h).strong());
                                }
                                ui.end_row();
                                for (i, a) in available.iter().enumerate() {
                                    ui.label(&a.book);
                                    ui.label(&a.sheet);
                                    ui.label(&a.long_name);
                                    let sel = self.left.contains(&i);
                                    if ui.selectable_label(sel, &a.short_name).clicked() {
                                        toggle(&mut self.left, i, ui.input(|s| s.modifiers.command));
                                    }
                                    ui.end_row();
                                }
                            });
                        });
                    });
                    // Middle: the plot type and the arrows.
                    ui.vertical(|ui| {
                        ui.add_space(30.0);
                        egui::ComboBox::from_id_salt("lc_kind").selected_text(kind_name(self.kind)).show_ui(ui, |ui| {
                            for (k, n) in KINDS {
                                ui.selectable_value(&mut self.kind, k, n);
                            }
                        });
                        if ui.button("=>").on_hover_text("Add the chosen columns to the layer").clicked() {
                            let layer = &mut graph.layers[li];
                            for &i in &self.left {
                                let a = &available[i];
                                if layer.plots.iter().any(|p| p.y == a.y) {
                                    continue;
                                }
                                let mut plot = new_plot(layer, a.x, a.y, a.y_error);
                                plot.kind = self.kind;
                                layer.plots.push(plot);
                            }
                            layer.recolor_automatic();
                            self.left.clear();
                        }
                        if ui.button("<=").on_hover_text("Remove the chosen plots from the layer").clicked() {
                            let layer = &mut graph.layers[li];
                            let mut keep = 0;
                            layer.plots.retain(|_| {
                                keep += 1;
                                !self.right.contains(&(keep - 1))
                            });
                            self.right.clear();
                        }
                    });
                    // Right: the plots of the layer.
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            let _ = ui.button(format!("Layer ({})", li + 1));
                            let count = graph.layers[li].plots.len();
                            let chosen = self.right.len();
                            if ui.add_enabled(chosen >= 2, egui::Button::new("Group")).clicked() {
                                let layer = &mut graph.layers[li];
                                let id = layer.plots.iter().filter_map(|p| p.group).max().map_or(1, |g| g + 1);
                                for &k in &self.right {
                                    layer.plots[k].group = Some(id);
                                }
                            }
                            let grouped = self.right.iter().any(|&k| graph.layers[li].plots[k].group.is_some());
                            if ui.add_enabled(grouped, egui::Button::new("Ungroup")).clicked() {
                                for &k in &self.right {
                                    graph.layers[li].plots[k].group = None;
                                }
                            }
                            let one = (self.right.len() == 1).then(|| self.right[0]);
                            if ui.add_enabled(one.is_some_and(|k| k > 0), egui::Button::new("Up")).clicked() {
                                let k = one.unwrap_or(0);
                                graph.layers[li].plots.swap(k, k - 1);
                                self.right = vec![k - 1];
                            }
                            if ui.add_enabled(one.is_some_and(|k| k + 1 < count), egui::Button::new("Down")).clicked() {
                                let k = one.unwrap_or(0);
                                graph.layers[li].plots.swap(k, k + 1);
                                self.right = vec![k + 1];
                            }
                            ui.checkbox(&mut self.rescale, "Rescale on Apply");
                        });
                        egui::ScrollArea::vertical().id_salt("lc_right").max_height(200.0).show(ui, |ui| {
                            egui::Grid::new("lc_right_grid").striped(true).num_columns(5).show(ui, |ui| {
                                for h in ["", "Short Name", "Long Name", "Legend", "Plot Type"] {
                                    ui.label(RichText::new(h).strong());
                                }
                                ui.end_row();
                                for (k, p) in graph.layers[li].plots.iter().enumerate() {
                                    ui.label(p.group.map(|g| format!("g{g}")).unwrap_or_default());
                                    let sel = self.right.contains(&k);
                                    if ui.selectable_label(sel, format!("{}(Y)", short(p.y))).clicked() {
                                        toggle(&mut self.right, k, ui.input(|s| s.modifiers.command));
                                    }
                                    ui.label(long(p.y));
                                    ui.label(doc.project().plot_label(p));
                                    ui.label(kind_name(p.kind));
                                    ui.end_row();
                                }
                            });
                        });
                    });
                });
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Layer Properties...").clicked() {
                        outcome = Outcome::LayerProperties(self.graph, li);
                    }
                    if ui.button("Plot Setup...").clicked() {
                        outcome = Outcome::PlotSetup(self.graph, li);
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Cancel").clicked() {
                            outcome = Outcome::Closed;
                            cancel = true;
                        }
                        if ui.button("Close").clicked() {
                            outcome = Outcome::Closed;
                        }
                        let dirty = self.baseline.as_ref().is_some_and(|b| *b != graph);
                        if ui.add_enabled(dirty || self.rescale, egui::Button::new("Apply")).clicked() {
                            if self.rescale {
                                graph.layers[li].x_axis.min = None;
                                graph.layers[li].x_axis.max = None;
                                graph.layers[li].y_axis.min = None;
                                graph.layers[li].y_axis.max = None;
                            }
                            self.baseline = None;
                            self.token = self.token.wrapping_add(1 << 40);
                            self.touched = false;
                        }
                    });
                });
            });
        if cancel {
            self.cancel(doc);
            return Outcome::Closed;
        }
        if graph != original {
            let id = self.graph;
            let edited = graph;
            doc.apply_merging(LABEL, self.token, move |p| {
                if let Some(g) = p.graph_mut(id) {
                    *g = edited;
                }
            });
            self.touched = true;
        }
        if !open {
            return Outcome::Closed;
        }
        outcome
    }

    /// Takes the graph back to how it was at the opening (or the last Apply).
    fn cancel(&mut self, doc: &mut Document) {
        let Some(base) = self.baseline.take() else {
            return;
        };
        if !self.touched {
            return;
        }
        if doc.undo_label() == Some(LABEL) {
            doc.undo();
        }
        if doc.project().graph(self.graph) != Some(&base) {
            let id = self.graph;
            doc.apply("Restore layer contents", move |p| {
                if let Some(g) = p.graph_mut(id) {
                    *g = base;
                }
            });
        }
    }
}

/// Adds `i` to the choice, or with Ctrl toggles it among the others; without Ctrl it is the only one.
fn toggle(chosen: &mut Vec<usize>, i: usize, ctrl: bool) {
    if ctrl {
        if let Some(k) = chosen.iter().position(|&c| c == i) {
            chosen.remove(k);
        } else {
            chosen.push(i);
            chosen.sort_unstable();
        }
    } else {
        *chosen = vec![i];
    }
}

/// A plot of `y` with the layer's defaults.
fn new_plot(layer: &Layer, x: Option<ColumnId>, y: ColumnId, y_error: Option<ColumnId>) -> Plot {
    Plot {
        x,
        y,
        y_error,
        rows: None,
        kind: layer.default_kind,
        line: layer.default_line.clone(),
        symbol: layer.default_symbol.clone(),
        label: None,
        visible: true,
        auto_color: true,
        group: None,
        x_error: None,
        y_base: None,
        error: ErrorStyle::default(),
        hist: Default::default(),
        ridge: layer.inherited_ridge(),
        hexbin: layer.inherited_hexbin(),
        parallel: layer.inherited_parallel(),
        treemap: layer.inherited_treemap(),
        size_from: None,
        color_from: None,
        decor: Default::default(),
    }
}
