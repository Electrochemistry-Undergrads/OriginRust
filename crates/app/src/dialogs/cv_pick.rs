//! Analysis > Mathematics > CV Charge on a graph (T349): the feet of a voltammetric peak are picked on the graph
//! with the Data Reader (a double-click on a data point of the active plot, or Enter, as Multiple Peak Fit's Get
//! Points window), then CV Charge (`Tool::CvCharge`) runs over the rows from one foot to the other with the
//! end-point baseline, so the straight line through the two feet is the peak's sloped baseline. One undo step.
//! From a workbook the typed window of `analysis.rs` is used.

use crate::reader;
use eframe::egui;
use sheaf_core::analysis::{CvChargeSpec, Report, Request, RowRange, Tool};
use sheaf_core::{ColumnId, Document, Project, WindowId};

/// What the window asks of the application after a frame.
pub enum CvPickOutcome {
    Open,
    /// Cancelled or closed: the Data Reader goes off again.
    Closed,
    /// The charge was computed.
    Done(Box<Report>),
}

pub struct CvPickDialog {
    pub graph: WindowId,
    pub layer: usize,
    pub plot: usize,
    y: ColumnId,
    label: String,
    /// Potential units per second.
    pub scan_rate: f64,
    /// The picked feet: (row, potential, current), at most two.
    pub feet: Vec<(usize, f64, f64)>,
    problem: Option<String>,
    /// Where the window was drawn, so a double-click on it does not pick.
    pub window_rect: Option<egui::Rect>,
}

impl CvPickDialog {
    /// The tool for the plot of `y` in `graph`, preferring `layer`, starting at `scan_rate`.
    pub fn new(project: &Project, graph: WindowId, y: ColumnId, layer: usize, scan_rate: f64) -> Option<Self> {
        let g = project.graph(graph)?;
        let found = |l: usize| g.layers.get(l)?.plots.iter().position(|p| p.y == y).map(|i| (l, i));
        let (layer, plot) = found(layer).or_else(|| (0..g.layers.len()).find_map(found))?;
        let col = project.column(y)?;
        Some(CvPickDialog {
            graph,
            layer,
            plot,
            y,
            label: format!("{} of {}", col.display_name(), g.title()),
            scan_rate,
            feet: Vec::new(),
            problem: None,
            window_rect: None,
        })
    }

    /// Picks the Data Reader's point as a foot when it is a point of this plot and two are not picked yet.
    pub fn pick(&mut self, project: &Project, p: reader::Point) -> bool {
        if self.feet.len() >= 2 || p.layer != self.layer || p.plot != self.plot {
            return false;
        }
        let Some((x, y)) = crate::gadget::plot_xy(project, self.graph, p.layer, p.plot) else { return false };
        let (Some(&e), Some(&i)) = (x.get(p.row), y.get(p.row)) else { return false };
        if !(e.is_finite() && i.is_finite()) || self.feet.iter().any(|f| f.0 == p.row) {
            return false;
        }
        self.feet.push((p.row, e, i));
        true
    }

    /// The request: CV Charge over the rows from one foot to the other (1-based, inclusive).
    pub fn request(&self) -> Request {
        let (a, b) = (self.feet[0].0.min(self.feet[1].0), self.feet[0].0.max(self.feet[1].0));
        Request {
            columns: vec![self.y],
            rows: RowRange::Rows(a + 1, b + 1),
            tool: Tool::CvCharge(CvChargeSpec { scan_rate: self.scan_rate, end_point_baseline: true }),
        }
    }

    pub fn show(&mut self, ctx: &egui::Context, doc: &mut Document) -> CvPickOutcome {
        if doc.project().graph(self.graph).is_none() || doc.project().column(self.y).is_none() {
            return CvPickOutcome::Closed;
        }
        let mut open = true;
        let mut run = false;
        let mut cancel = false;
        let response = egui::Window::new("CV Charge")
            .id(egui::Id::new("cv_charge_feet"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label(format!("Input: {}", self.label));
                ui.label("Click a data point, then double-click it or press Enter to pick a foot of the peak.");
                ui.label(format!("Feet picked: {} of 2", self.feet.len()));
                if !self.feet.is_empty() {
                    egui::Grid::new("cv_feet_list").striped(true).num_columns(4).show(ui, |ui| {
                        ui.strong("Foot");
                        ui.strong("Row");
                        ui.strong("E");
                        ui.strong("I");
                        ui.end_row();
                        for (k, (row, e, i)) in self.feet.iter().enumerate() {
                            ui.label(format!("{}", k + 1));
                            ui.label(format!("{}", row + 1));
                            ui.label(format!("{e}"));
                            ui.label(format!("{i}"));
                            ui.end_row();
                        }
                    });
                }
                ui.horizontal(|ui| {
                    ui.label("Scan rate (potential units per second)");
                    ui.add(egui::DragValue::new(&mut self.scan_rate).speed(0.001).range(1e-9..=1e9));
                });
                ui.weak("The charge between the feet, above the straight line through them.");
                if let Some(p) = &self.problem {
                    ui.colored_label(egui::Color32::from_rgb(170, 30, 30), p);
                }
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.add_enabled(!self.feet.is_empty(), egui::Button::new("Remove Last")).clicked() {
                        self.feet.pop();
                    }
                    if ui.add_enabled(self.feet.len() == 2, egui::Button::new("Calculate")).clicked() {
                        run = true;
                    }
                    if ui.button("Cancel").clicked() {
                        cancel = true;
                    }
                });
            });
        self.window_rect = response.map(|r| r.response.rect);
        if cancel || !open {
            return CvPickOutcome::Closed;
        }
        if run {
            let request = self.request();
            match doc.edit("CV charge", |p| p.analyze(&request)) {
                Ok(report) => return CvPickOutcome::Done(Box::new(report)),
                Err(e) => self.problem = Some(e.to_string()),
            }
        }
        CvPickOutcome::Open
    }
}
