//! Graph > Plot Setup... as Origin's own window "Plot Setup: Configure Data Plots in Layer" (measured,
//! graphing spec 18.3): a Plot Type list, the columns of one sheet with X, Y and yEr ticks (L greyed),
//! Add and Replace, and the Plot List of the layer (Range, Show, Plot Type, Legend) with Rescale;
//! Preview (greyed), OK, Cancel and Apply. The edits are made to the graph at once and form one undo
//! step; Cancel takes them back.

use eframe::egui::{self, RichText};
use sheaf_core::column::Designation;
use sheaf_core::graph::*;
use sheaf_core::{ColumnId, Document, Project, WindowId};

const LABEL: &str = "Plot setup";

/// Origin's Plot Type list: the first four entries are offered; Bubble, Color Mapped and Bubble +
/// Color Mapped (which need a third column) are shown greyed.
const KINDS: [(PlotKind, &str); 4] = [
    (PlotKind::Line, "Line"),
    (PlotKind::Scatter, "Scatter"),
    (PlotKind::LineSymbol, "Line + Symbol"),
    (PlotKind::Column, "Column / Bar"),
];
const GREY_KINDS: [&str; 3] = ["Bubble", "Color Mapped", "Bubble + Color Mapped"];

fn kind_name(k: PlotKind) -> String {
    KINDS.iter().find(|(x, _)| *x == k).map_or_else(|| format!("{k:?}"), |(_, n)| (*n).to_string())
}

/// One column of the chosen sheet and its ticks.
#[derive(Clone)]
struct Row {
    id: ColumnId,
    short: String,
    long: String,
    comments: String,
    x: bool,
    y: bool,
    err: bool,
}

pub enum Outcome {
    Open,
    Closed,
}

pub struct PlotSetupDialog {
    pub graph: WindowId,
    pub layer: usize,
    token: u64,
    baseline: Option<Graph>,
    touched: bool,
    kind: PlotKind,
    /// The chosen sheet: (book, sheet index).
    sheet: Option<(WindowId, usize)>,
    rows: Vec<Row>,
    chosen: Option<usize>,
    rescale: bool,
}

impl PlotSetupDialog {
    pub fn new(graph: WindowId, layer: usize, token: u64, project: &Project) -> Self {
        // The sheet of the layer's first plot, else the first sheet of the project.
        let first_plot = project.graph(graph).and_then(|g| g.layers.get(layer)).and_then(|l| l.plots.first());
        let sheet = first_plot
            .and_then(|p| project.locate(p.y))
            .and_then(|(b, s, _)| b.sheets.iter().position(|x| std::ptr::eq(x, s)).map(|i| (b.id, i)))
            .or_else(|| project.workbooks().next().map(|b| (b.id, 0)));
        let mut d = PlotSetupDialog {
            graph,
            layer,
            token,
            baseline: None,
            touched: false,
            kind: first_plot.map_or(PlotKind::Line, |p| p.kind),
            sheet,
            rows: Vec::new(),
            chosen: None,
            rescale: false,
        };
        d.load_rows(project);
        d
    }

    fn load_rows(&mut self, project: &Project) {
        self.rows.clear();
        let Some((book, i)) = self.sheet else { return };
        let Some(ws) = project.workbook(book).and_then(|b| b.sheets.get(i)) else { return };
        for c in &ws.columns {
            self.rows.push(Row {
                id: c.id,
                short: c.short_name.clone(),
                long: c.long_name.clone(),
                comments: c.comments.lines().next().unwrap_or_default().to_string(),
                x: false,
                y: false,
                err: false,
            });
        }
        // Origin ticks the designated X column for the plots to come.
        if let Some(r) = ws.columns.iter().position(|c| c.designation == Designation::X) {
            self.rows[r].x = true;
        }
    }

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
        let li = self.layer;
        let mut graph = original.clone();
        let sheets: Vec<((WindowId, usize), String)> = doc
            .project()
            .workbooks()
            .flat_map(|b| {
                // Sheets marked Exclude from Plotting dialogs are not offered (T184).
                b.sheets
                    .iter()
                    .enumerate()
                    .filter(|(_, s)| !s.exclude_from_plotting)
                    .map(move |(i, s)| ((b.id, i), format!("[{}]{}", b.name, s.name)))
            })
            .collect();
        let names: Vec<String> = original.layers[li].plots.iter().map(|p| doc.project().plot_data_name(p)).collect();
        let legends: Vec<String> = original.layers[li].plots.iter().map(|p| doc.project().plot_label(p)).collect();
        let ranges: Vec<String> = original.layers[li].plots.iter().map(|p| range_text(doc.project(), p)).collect();
        let (mut outcome, mut cancel, mut ok, mut open, mut reload) = (Outcome::Open, false, false, true, false);
        egui::Window::new("Plot Setup: Configure Data Plots in Layer")
            .id(egui::Id::new("plot_setup"))
            .open(&mut open)
            .default_size([720.0, 460.0])
            .collapsible(false)
            .show(ctx, |ui| {
                ui.horizontal_top(|ui| {
                    ui.vertical(|ui| {
                        ui.label("Plot Type:");
                        for (k, n) in KINDS {
                            ui.selectable_value(&mut self.kind, k, n);
                        }
                        for n in GREY_KINDS {
                            ui.add_enabled(false, egui::Button::selectable(false, n));
                        }
                    });
                    ui.vertical(|ui| {
                        let current =
                            sheets.iter().find(|(s, _)| Some(*s) == self.sheet).map_or("", |(_, n)| n.as_str());
                        egui::ComboBox::from_id_salt("ps_sheet").selected_text(current).width(260.0).show_ui(
                            ui,
                            |ui| {
                                for (s, n) in &sheets {
                                    if ui.selectable_label(Some(*s) == self.sheet, n).clicked() {
                                        self.sheet = Some(*s);
                                        reload = true;
                                    }
                                }
                            },
                        );
                        egui::ScrollArea::vertical().id_salt("ps_cols").max_height(150.0).show(ui, |ui| {
                            egui::Grid::new("ps_cols_grid").striped(true).num_columns(7).show(ui, |ui| {
                                for h in ["X", "Y", "yEr", "L", "Column", "Long Name", "Comments"] {
                                    ui.label(RichText::new(h).strong());
                                }
                                ui.end_row();
                                for k in 0..self.rows.len() {
                                    let mut x = self.rows[k].x;
                                    if ui.checkbox(&mut x, "").changed() {
                                        // One X at a time.
                                        for r in &mut self.rows {
                                            r.x = false;
                                        }
                                        self.rows[k].x = x;
                                    }
                                    ui.checkbox(&mut self.rows[k].y, "");
                                    let mut err = self.rows[k].err;
                                    if ui.checkbox(&mut err, "").changed() {
                                        for r in &mut self.rows {
                                            r.err = false;
                                        }
                                        self.rows[k].err = err;
                                    }
                                    ui.add_enabled(false, egui::Checkbox::new(&mut false, ""));
                                    let r = &self.rows[k];
                                    ui.label(&r.short);
                                    ui.label(&r.long);
                                    ui.label(&r.comments);
                                    ui.end_row();
                                }
                            });
                        });
                        ui.horizontal(|ui| {
                            let x = self.rows.iter().find(|r| r.x).map(|r| r.id);
                            let err = self.rows.iter().find(|r| r.err).map(|r| r.id);
                            let ys: Vec<ColumnId> = self.rows.iter().filter(|r| r.y).map(|r| r.id).collect();
                            let can_replace = self.chosen.is_some() && ys.len() == 1;
                            if ui.add_enabled(can_replace, egui::Button::new("Replace")).clicked()
                                && let Some(p) = self.chosen.and_then(|k| graph.layers[li].plots.get_mut(k))
                            {
                                p.x = x;
                                p.y = ys[0];
                                p.y_error = err;
                                p.kind = self.kind;
                            }
                            if ui.add_enabled(!ys.is_empty(), egui::Button::new("Add")).clicked() {
                                let layer = &mut graph.layers[li];
                                for y in ys {
                                    let mut p = new_plot(layer, x, y, err);
                                    p.kind = self.kind;
                                    layer.plots.push(p);
                                }
                                layer.recolor_automatic();
                            }
                        });
                    });
                });
                ui.separator();
                ui.label("Plot List: the plots of the layer; Show hides or shows one.");
                egui::ScrollArea::vertical().id_salt("ps_list").max_height(160.0).show(ui, |ui| {
                    egui::Grid::new("ps_list_grid").striped(true).num_columns(5).show(ui, |ui| {
                        for h in ["Plot", "Range", "Show", "Plot Type", "Legend"] {
                            ui.label(RichText::new(h).strong());
                        }
                        ui.end_row();
                        ui.label(format!("Layer {}", li + 1));
                        ui.checkbox(&mut self.rescale, "Rescale");
                        ui.end_row();
                        let n = graph.layers[li].plots.len().min(names.len());
                        for k in 0..n {
                            if ui.selectable_label(self.chosen == Some(k), format!("    {}", names[k])).clicked() {
                                self.chosen = Some(k);
                            }
                            ui.label(&ranges[k]);
                            ui.checkbox(&mut graph.layers[li].plots[k].visible, "");
                            ui.label(kind_name(graph.layers[li].plots[k].kind));
                            ui.label(&legends[k]);
                            ui.end_row();
                        }
                    });
                });
                ui.separator();
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let dirty = self.baseline.as_ref().is_some_and(|b| *b != graph);
                    if ui.add_enabled(dirty || self.rescale, egui::Button::new("Apply")).clicked() {
                        rescale(&mut graph.layers[li], self.rescale);
                        self.baseline = None;
                        self.token = self.token.wrapping_add(1 << 40);
                        self.touched = false;
                    }
                    if ui.button("Cancel").clicked() {
                        cancel = true;
                    }
                    if ui.button("OK").clicked() {
                        rescale(&mut graph.layers[li], self.rescale);
                        ok = true;
                    }
                    ui.add_enabled(false, egui::Button::new("Preview"));
                });
            });
        if reload {
            self.load_rows(doc.project());
        }
        if cancel || !open && !ok {
            if cancel {
                self.cancel(doc);
            }
            return Outcome::Closed;
        }
        if graph != original {
            let id = self.graph;
            doc.apply_merging(LABEL, self.token, move |p| {
                if let Some(g) = p.graph_mut(id) {
                    *g = graph;
                }
            });
            self.touched = true;
        }
        if ok {
            outcome = Outcome::Closed;
        }
        outcome
    }

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
            doc.apply("Restore plot setup", move |p| {
                if let Some(g) = p.graph_mut(id) {
                    *g = base;
                }
            });
        }
    }
}

/// Rescale: the axes of the layer follow the data again.
fn rescale(layer: &mut Layer, on: bool) {
    if on {
        layer.x_axis.min = None;
        layer.x_axis.max = None;
        layer.y_axis.min = None;
        layer.y_axis.max = None;
    }
}

/// Origin's Range column: the rows and the X and Y ranges of the data, `[1*:11*] 0 < X < 10 , 5 < Y < 5`.
fn range_text(project: &Project, p: &Plot) -> String {
    let span = |id: Option<ColumnId>| -> Option<(f64, f64)> {
        let v = project.numeric(id?)?;
        let f: Vec<f64> = v.iter().copied().filter(|a| a.is_finite()).collect();
        (!f.is_empty()).then(|| {
            (f.iter().copied().fold(f64::INFINITY, f64::min), f.iter().copied().fold(f64::NEG_INFINITY, f64::max))
        })
    };
    let rows = match p.rows {
        Some((a, b)) => format!("[{}:{}]", a + 1, b),
        None => format!("[1*:{}*]", project.column(p.y).map_or(0, |c| c.len())),
    };
    let mut out = rows;
    if let Some((a, b)) = span(p.x) {
        out.push_str(&format!(" {a} < X < {b}"));
    }
    if let Some((a, b)) = span(Some(p.y)) {
        out.push_str(&format!(" , {a} < Y < {b}"));
    }
    out
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
        indexed: None,
        stat: None,
        sunburst: layer.inherited_sunburst(),
        sankey: Default::default(),
        size_from: None,
        color_from: None,
        decor: Default::default(),
    }
}
