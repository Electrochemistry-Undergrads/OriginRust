//! Analysis > Mathematics > Integrate Ranges on a graph (T353): each peak's two limits are picked on the graph
//! with the Data Reader (a double-click on a data point of the active plot, or Enter, as Multiple Peak Fit's Get
//! Points window); every two picks make one range. Integrate runs Integrate Ranges (`Tool::IntegrateRanges`) over
//! the rows of the ranges, with or without the end-point baseline of each range, as one undo step. From a
//! workbook the typed window of `analysis.rs` is used.

use crate::reader;
use eframe::egui;
use sheaf_core::analysis::{IntegrateRangesSpec, Report, Request, RowRange, Tool};
use sheaf_core::{ColumnId, Document, Project, WindowId};

/// What the window asks of the application after a frame.
pub enum RangePickOutcome {
    Open,
    /// Cancelled or closed: the Data Reader goes off again.
    Closed,
    /// The ranges were integrated.
    Done(Box<Report>),
}

pub struct RangePickDialog {
    pub graph: WindowId,
    pub layer: usize,
    pub plot: usize,
    y: ColumnId,
    label: String,
    /// The picked limits: (row, X), in picking order; each two make a range.
    pub limits: Vec<(usize, f64)>,
    /// Subtract the straight line through each range's ends first.
    pub end_point_baseline: bool,
    problem: Option<String>,
    /// Where the window was drawn, so a double-click on it does not pick.
    pub window_rect: Option<egui::Rect>,
}

impl RangePickDialog {
    /// The tool for the plot of `y` in `graph`, preferring `layer`.
    pub fn new(project: &Project, graph: WindowId, y: ColumnId, layer: usize) -> Option<Self> {
        let g = project.graph(graph)?;
        let found = |l: usize| g.layers.get(l)?.plots.iter().position(|p| p.y == y).map(|i| (l, i));
        let (layer, plot) = found(layer).or_else(|| (0..g.layers.len()).find_map(found))?;
        let col = project.column(y)?;
        Some(RangePickDialog {
            graph,
            layer,
            plot,
            y,
            label: format!("{} of {}", col.display_name(), g.title()),
            limits: Vec::new(),
            end_point_baseline: false,
            problem: None,
            window_rect: None,
        })
    }

    /// Picks the Data Reader's point as a limit when it is a point of this plot. True when added.
    pub fn pick(&mut self, project: &Project, p: reader::Point) -> bool {
        if p.layer != self.layer || p.plot != self.plot {
            return false;
        }
        let Some((x, y)) = crate::gadget::plot_xy(project, self.graph, p.layer, p.plot) else { return false };
        let (Some(&px), Some(&py)) = (x.get(p.row), y.get(p.row)) else { return false };
        // The second limit of a range must be another row than the first.
        let open = self.limits.len() % 2 == 1;
        if !(px.is_finite() && py.is_finite()) || (open && self.limits.last().is_some_and(|l| l.0 == p.row)) {
            return false;
        }
        self.limits.push((p.row, px));
        true
    }

    /// The finished ranges as rows (0-based, inclusive, lower first).
    pub fn ranges(&self) -> Vec<(usize, usize)> {
        self.limits.chunks_exact(2).map(|c| (c[0].0.min(c[1].0), c[0].0.max(c[1].0))).collect()
    }

    fn request(&self) -> Request {
        Request {
            columns: vec![self.y],
            rows: RowRange::All,
            tool: Tool::IntegrateRanges(IntegrateRangesSpec {
                ranges: self.ranges().into_iter().map(|(a, b)| RowRange::Rows(a + 1, b + 1)).collect(),
                end_point_baseline: self.end_point_baseline,
            }),
        }
    }

    pub fn show(&mut self, ctx: &egui::Context, doc: &mut Document) -> RangePickOutcome {
        if doc.project().graph(self.graph).is_none() || doc.project().column(self.y).is_none() {
            return RangePickOutcome::Closed;
        }
        let mut open = true;
        let mut run = false;
        let mut cancel = false;
        let response = egui::Window::new("Integrate Ranges")
            .id(egui::Id::new("integrate_ranges_pick"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label(format!("Input: {}", self.label));
                ui.label(
                    "Click a data point, then double-click it or press Enter to pick a limit; two limits make a range.",
                );
                let ranges = self.ranges();
                ui.label(format!(
                    "Ranges: {}{}",
                    ranges.len(),
                    if self.limits.len() % 2 == 1 { " (one limit of the next picked)" } else { "" }
                ));
                if !ranges.is_empty() {
                    egui::Grid::new("range_pick_list").striped(true).num_columns(3).show(ui, |ui| {
                        ui.strong("Range");
                        ui.strong("From row");
                        ui.strong("To row");
                        ui.end_row();
                        for (k, (a, b)) in ranges.iter().enumerate() {
                            ui.label(format!("{}", k + 1));
                            ui.label(format!("{}", a + 1));
                            ui.label(format!("{}", b + 1));
                            ui.end_row();
                        }
                    });
                }
                ui.checkbox(&mut self.end_point_baseline, "Subtract the line through each range's ends");
                if let Some(p) = &self.problem {
                    ui.colored_label(egui::Color32::from_rgb(170, 30, 30), p);
                }
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.add_enabled(!self.limits.is_empty(), egui::Button::new("Remove Last")).clicked() {
                        self.limits.pop();
                    }
                    if ui.add_enabled(!ranges.is_empty(), egui::Button::new("Integrate")).clicked() {
                        run = true;
                    }
                    if ui.button("Cancel").clicked() {
                        cancel = true;
                    }
                });
            });
        self.window_rect = response.map(|r| r.response.rect);
        if cancel || !open {
            return RangePickOutcome::Closed;
        }
        if run {
            let request = self.request();
            match doc.edit("Integrate ranges", |p| p.analyze(&request)) {
                Ok(report) => return RangePickOutcome::Done(Box::new(report)),
                Err(e) => self.problem = Some(e.to_string()),
            }
        }
        RangePickOutcome::Open
    }
}
