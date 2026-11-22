//! Analysis > Data Manipulation > Subtract Straight Line on a graph (T348): as Origin's `subtract_line`, an
//! interactive tool on the graph. The Data Reader is on; a double-click on a data point of the active plot (or
//! Enter) picks it, as Multiple Peak Fit's Get Points window does. With two points, Subtract writes the curve minus
//! the straight line through them as a new Y column behind the data (`Tool::SubtractLine` with both points given)
//! and draws it on the graph, as one undo step. From a workbook the typed window of `manip_options.rs` is used.

use crate::reader;
use eframe::egui;
use sheaf_core::analysis::{AnalysisError, LinePoint, Report, Request, RowRange, SubtractLineSpec, Tool};
use sheaf_core::graph::PlotKind;
use sheaf_core::{ColumnId, Document, Project, WindowId};

/// What the window asks of the application after a frame.
pub enum SubtractPickOutcome {
    Open,
    /// Cancelled or closed: the Data Reader goes off again.
    Closed,
    /// The line was subtracted.
    Done(Box<Report>),
}

pub struct SubtractPickDialog {
    pub graph: WindowId,
    pub layer: usize,
    pub plot: usize,
    y: ColumnId,
    x: Option<ColumnId>,
    label: String,
    /// The picked points (X, Y), at most two.
    pub points: Vec<(f64, f64)>,
    problem: Option<String>,
    /// Where the window was drawn, so a double-click on it does not pick.
    pub window_rect: Option<egui::Rect>,
}

impl SubtractPickDialog {
    /// The tool for the plot of `y` in `graph`, preferring `layer`.
    pub fn new(project: &Project, graph: WindowId, y: ColumnId, layer: usize) -> Option<Self> {
        let g = project.graph(graph)?;
        let found = |l: usize| g.layers.get(l)?.plots.iter().position(|p| p.y == y).map(|i| (l, i));
        let (layer, plot) = found(layer).or_else(|| (0..g.layers.len()).find_map(found))?;
        let col = project.column(y)?;
        Some(SubtractPickDialog {
            graph,
            layer,
            plot,
            y,
            x: g.layers[layer].plots[plot].x,
            label: format!("{} of {}", col.display_name(), g.title()),
            points: Vec::new(),
            problem: None,
            window_rect: None,
        })
    }

    /// Picks the Data Reader's point when it is a point of this plot and two are not picked yet. True when added.
    pub fn pick(&mut self, project: &Project, p: reader::Point) -> bool {
        if self.points.len() >= 2 || p.layer != self.layer || p.plot != self.plot {
            return false;
        }
        let Some((x, y)) = crate::gadget::plot_xy(project, self.graph, p.layer, p.plot) else { return false };
        let (Some(&px), Some(&py)) = (x.get(p.row), y.get(p.row)) else { return false };
        if !(px.is_finite() && py.is_finite()) || self.points.iter().any(|(q, _)| *q == px) {
            return false;
        }
        self.points.push((px, py));
        true
    }

    fn request(&self) -> Request {
        let point = |k: usize| LinePoint { x: self.points[k].0, y: Some(self.points[k].1) };
        Request {
            columns: vec![self.y],
            rows: RowRange::All,
            tool: Tool::SubtractLine(SubtractLineSpec { p1: point(0), p2: point(1) }),
        }
    }

    /// Subtracts the line and draws the result on the graph, as one undo step.
    fn subtract(&self, doc: &mut Document) -> Result<Report, String> {
        let request = self.request();
        let (graph, layer, x) = (self.graph, self.layer, self.x);
        doc.edit("Subtract straight line", |p| {
            let report = p.analyze(&request)?;
            for &column in &report.new_columns {
                p.add_plot_xy(graph, layer, x, column, None).map_err(|e| AnalysisError::BadOption(e.to_string()))?;
                if let Some(plot) =
                    p.graph_mut(graph).and_then(|g| g.layers.get_mut(layer)).and_then(|l| l.plots.last_mut())
                {
                    plot.kind = PlotKind::Line;
                }
            }
            Ok::<_, AnalysisError>(report)
        })
        .map_err(|e| e.to_string())
    }

    pub fn show(&mut self, ctx: &egui::Context, doc: &mut Document) -> SubtractPickOutcome {
        if doc.project().graph(self.graph).is_none() || doc.project().column(self.y).is_none() {
            return SubtractPickOutcome::Closed;
        }
        let mut open = true;
        let mut run = false;
        let mut cancel = false;
        let response = egui::Window::new("Subtract Straight Line")
            .id(egui::Id::new("subtract_line_points"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label(format!("Input: {}", self.label));
                ui.label("Click a data point, then double-click it or press Enter to pick an end of the line.");
                ui.label(format!("Points picked: {} of 2", self.points.len()));
                if !self.points.is_empty() {
                    egui::Grid::new("subtract_line_list").striped(true).num_columns(3).show(ui, |ui| {
                        ui.strong("Point");
                        ui.strong("X");
                        ui.strong("Y");
                        ui.end_row();
                        for (i, (x, y)) in self.points.iter().enumerate() {
                            ui.label(format!("{}", i + 1));
                            ui.label(format!("{x}"));
                            ui.label(format!("{y}"));
                            ui.end_row();
                        }
                    });
                }
                if let Some(p) = &self.problem {
                    ui.colored_label(egui::Color32::from_rgb(170, 30, 30), p);
                }
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.add_enabled(!self.points.is_empty(), egui::Button::new("Remove Last")).clicked() {
                        self.points.pop();
                    }
                    if ui.add_enabled(self.points.len() == 2, egui::Button::new("Subtract")).clicked() {
                        run = true;
                    }
                    if ui.button("Cancel").clicked() {
                        cancel = true;
                    }
                });
            });
        self.window_rect = response.map(|r| r.response.rect);
        if cancel || !open {
            return SubtractPickOutcome::Closed;
        }
        if run {
            match self.subtract(doc) {
                Ok(report) => return SubtractPickOutcome::Done(Box::new(report)),
                Err(e) => self.problem = Some(e),
            }
        }
        SubtractPickOutcome::Open
    }
}
