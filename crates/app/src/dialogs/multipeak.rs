//! Analysis > Peaks and Baseline > Multiple Peak Fit (Origin's `nlfitpeaks`): choose the peak
//! function for the active data plot, then pick the peak centres on the graph with the Data Reader
//! (double-click or Enter, as Origin's Get Points dialog), then Fit: the peaks are fitted together
//! with a constant offset `y0`, the results go to a sheet as the Peak Analyzer's do, and each fitted
//! peak and their sum are drawn on the graph. One undo step.

use super::frame::{DialogButton, button_row};
use crate::reader;
use eframe::egui;
use sheaf_core::analysis::{
    AnalysisError, ChainBaseline, ChainOptions, FitBaseline, PeakChainSpec, PeakFitSpec, PeakShape, Report, Request,
    RowRange, Tool,
};
use sheaf_core::color::Color;
use sheaf_core::graph::{LineStyle, PlotKind};

/// The colour of the fitted curve: the first curve colour of the fit tools (`fitting.rs`).
const FIT_RED: Color = Color::rgb(220, 30, 30);
/// The colours of the single peaks: the other curve colours of the fit tools.
const PEAK_COLORS: [Color; 3] = [Color::rgb(30, 80, 220), Color::rgb(0, 150, 70), Color::rgb(235, 140, 0)];
use sheaf_core::{ColumnId, Document, Project, WindowId};

/// What the window asks of the application after a frame.
pub enum MultiPeakOutcome {
    Open,
    /// Cancelled or closed: the Data Reader goes off again.
    Closed,
    /// The peak function is chosen: show the graph and turn its Data Reader on.
    StartPicking,
    /// The fit ran.
    Done(Box<Report>),
}

pub struct MultiPeakDialog {
    pub graph: WindowId,
    pub layer: usize,
    pub plot: usize,
    y: ColumnId,
    x: Option<ColumnId>,
    label: String,
    shape: PeakShape,
    /// False while the first window (input and peak function) is open; true in Get Points.
    pub picking: bool,
    /// The picked centres (X, Y), in picking order.
    pub centers: Vec<(f64, f64)>,
    problem: Option<String>,
    /// Where the Get Points window was drawn, so a double-click on it does not pick.
    pub window_rect: Option<egui::Rect>,
}

impl MultiPeakDialog {
    /// The window for the plot of `y` in `graph`, preferring `layer`.
    pub fn new(project: &Project, graph: WindowId, y: ColumnId, layer: usize) -> Option<Self> {
        let g = project.graph(graph)?;
        let found = |l: usize| g.layers.get(l)?.plots.iter().position(|p| p.y == y).map(|i| (l, i));
        let (layer, plot) = found(layer).or_else(|| (0..g.layers.len()).find_map(found))?;
        let x = g.layers[layer].plots[plot].x;
        let col = project.column(y)?;
        Some(MultiPeakDialog {
            graph,
            layer,
            plot,
            y,
            x,
            label: format!("{} of {}", col.display_name(), g.title()),
            shape: PeakShape::Gauss,
            picking: false,
            centers: Vec::new(),
            problem: None,
            window_rect: None,
        })
    }

    /// Picks the Data Reader's point as a centre when it is a point of this plot. True when added.
    pub fn pick(&mut self, project: &Project, p: reader::Point) -> bool {
        if !self.picking || p.layer != self.layer || p.plot != self.plot {
            return false;
        }
        let Some((x, y)) = crate::gadget::plot_xy(project, self.graph, p.layer, p.plot) else { return false };
        let (Some(&cx), Some(&cy)) = (x.get(p.row), y.get(p.row)) else { return false };
        if !cx.is_finite() || self.centers.iter().any(|(c, _)| *c == cx) {
            return false;
        }
        self.centers.push((cx, cy));
        true
    }

    fn request(&self) -> Request {
        Request {
            columns: vec![self.y],
            rows: RowRange::All,
            tool: Tool::PeakChain(PeakChainSpec {
                chain: ChainOptions { baseline: ChainBaseline::None, ..ChainOptions::default() },
                integrate: false,
                fit: Some(PeakFitSpec { peak_curves: true, ..PeakFitSpec::of(self.shape, FitBaseline::Constant) }),
                centers: self.centers.iter().map(|c| c.0).collect(),
            }),
        }
    }

    /// Runs the fit and draws the fitted curve on the graph, as one undo step.
    fn fit(&self, doc: &mut Document) -> Result<Report, String> {
        let request = self.request();
        let (graph, layer, x) = (self.graph, self.layer, self.x);
        doc.edit("Multiple peak fit", |p| {
            let report = p.analyze(&request)?;
            let named = |prefix: &str| -> Vec<ColumnId> {
                report
                    .new_columns
                    .iter()
                    .copied()
                    .filter(|c| p.column(*c).is_some_and(|c| c.long_name.starts_with(prefix)))
                    .collect()
            };
            // Each peak first, then the sum on top in red, as the curves of the fit tools (ADR 0006).
            let curves: Vec<(ColumnId, Color, f64)> = named("Peak ")
                .into_iter()
                .filter(|c| !named("Peak fit of").contains(c))
                .enumerate()
                .map(|(k, c)| (c, PEAK_COLORS[k % PEAK_COLORS.len()], 1.0))
                .chain(named("Peak fit of").into_iter().map(|c| (c, FIT_RED, 1.5)))
                .collect();
            for (column, color, min_width) in curves {
                p.add_plot_xy(graph, layer, x, column, None).map_err(|e| AnalysisError::BadOption(e.to_string()))?;
                if let Some(l) = p.graph_mut(graph).and_then(|g| g.layers.get_mut(layer)) {
                    let width = l.default_line.width_pt.max(min_width);
                    if let Some(plot) = l.plots.last_mut() {
                        plot.kind = PlotKind::Line;
                        plot.line = LineStyle::solid(color, width);
                        plot.symbol.color = color;
                        plot.auto_color = false;
                    }
                }
            }
            Ok::<_, AnalysisError>(report)
        })
        .map_err(|e| e.to_string())
    }

    pub fn show(&mut self, ctx: &egui::Context, doc: &mut Document) -> MultiPeakOutcome {
        if doc.project().graph(self.graph).is_none() || doc.project().column(self.y).is_none() {
            return MultiPeakOutcome::Closed;
        }
        let mut open = true;
        let mut outcome = MultiPeakOutcome::Open;
        if !self.picking {
            let mut pressed = None;
            egui::Window::new("Multiple Peak Fit")
                .id(egui::Id::new("multi_peak_setup"))
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    egui::Grid::new("multi_peak_grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
                        ui.label("Input");
                        ui.label(&self.label);
                        ui.end_row();
                        ui.label("Peak Function");
                        egui::ComboBox::from_id_salt("multi_peak_shape").selected_text(self.shape.name()).show_ui(
                            ui,
                            |ui| {
                                for s in [PeakShape::Gauss, PeakShape::Lorentz, PeakShape::Voigt] {
                                    ui.selectable_value(&mut self.shape, s, s.name());
                                }
                            },
                        );
                        ui.end_row();
                    });
                    ui.add_enabled(false, egui::Checkbox::new(&mut false, "Use Peak Marker"));
                    ui.weak("OK shows the graph: pick the peak centres there.");
                    ui.add_space(6.0);
                    pressed = button_row(ui, false);
                });
            match pressed {
                Some(DialogButton::Ok) => {
                    self.picking = true;
                    outcome = MultiPeakOutcome::StartPicking;
                }
                Some(DialogButton::Cancel) => return MultiPeakOutcome::Closed,
                _ => {}
            }
        } else {
            let mut fit = false;
            let mut cancel = false;
            let response = egui::Window::new("Get Points")
                .id(egui::Id::new("multi_peak_points"))
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label("Click a data point, then double-click it or press Enter to pick a peak centre.");
                    ui.label(format!("Peak centres picked: {}", self.centers.len()));
                    if !self.centers.is_empty() {
                        egui::Grid::new("multi_peak_list").striped(true).num_columns(3).show(ui, |ui| {
                            ui.strong("Peak");
                            ui.strong("X");
                            ui.strong("Y");
                            ui.end_row();
                            for (i, (x, y)) in self.centers.iter().enumerate() {
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
                        if ui.add_enabled(!self.centers.is_empty(), egui::Button::new("Remove Last")).clicked() {
                            self.centers.pop();
                        }
                        if ui.add_enabled(!self.centers.is_empty(), egui::Button::new("Fit")).clicked() {
                            fit = true;
                        }
                        ui.add_enabled(false, egui::Button::new("Open NLFit"));
                        if ui.button("Cancel").clicked() {
                            cancel = true;
                        }
                    });
                });
            self.window_rect = response.map(|r| r.response.rect);
            if cancel {
                return MultiPeakOutcome::Closed;
            }
            if fit {
                match self.fit(doc) {
                    Ok(report) => return MultiPeakOutcome::Done(Box::new(report)),
                    Err(e) => self.problem = Some(e),
                }
            }
        }
        if open { outcome } else { MultiPeakOutcome::Closed }
    }
}
