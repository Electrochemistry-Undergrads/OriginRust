//! Analysis > Fitting > Simulate Curve: the window that makes the curve of a library function for typed
//! parameter values. The numbers and the new sheet come from `sheaf_core::analysis::Tool::SimulateCurve`;
//! this file is the controls, the live preview and the one undo step. The layout is Sheaf's own until the
//! window of Origin is measured (`docs/spec/fitting.md`, section 13).

use super::fit::{Series, draw_xy};
use eframe::egui::{self, DragValue};
use sheaf_core::analysis::{Request, RowRange, SimulateCurveSpec, SimulateX, Tool};
use sheaf_core::{ColumnData, ColumnId, Designation, Document, Project, WindowId};
use sheaf_fit::library::{self, FunctionInfo};
use sheaf_fit::simulate::{self as sim, Spacing};

/// What the window did in this frame.
pub enum Outcome {
    Open,
    Closed,
    /// The curve was made: the workbook, the graph if one was asked for, and the status line.
    Done {
        book: WindowId,
        graph: Option<WindowId>,
        message: String,
    },
}

/// Where the X values come from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum XSource {
    Range,
    Column,
}

/// The points of the live preview curve; a long run is drawn with fewer points.
const PREVIEW_POINTS: usize = 400;

/// What the window keeps between frames.
pub struct SimulateDialog {
    functions: Vec<FunctionInfo>,
    function: String,
    /// One value per parameter of the function, in the order of the function.
    values: Vec<f64>,
    source: XSource,
    from: f64,
    to: f64,
    points: usize,
    log: bool,
    /// Numeric columns of the workbook that was active when the window opened.
    columns: Vec<(ColumnId, String)>,
    x_column: Option<ColumnId>,
    /// The workbooks that can receive the sheet.
    books: Vec<(WindowId, String)>,
    /// `None` makes a new workbook.
    output: Option<WindowId>,
    plot: bool,
    problem: Option<String>,
    /// The preview for the settings it was made for.
    preview: Option<(PreviewKey, Result<Vec<(f64, f64)>, String>)>,
}

#[derive(Debug, Clone, PartialEq)]
struct PreviewKey {
    spec: SimulateCurveSpec,
    revision: u64,
}

impl SimulateDialog {
    /// Opens on the Gauss function with the defaults of Origin's window (fitting.md 13.1): its X range,
    /// 200 points, output to a new workbook and a graph of the curve. `book` is the active workbook, whose
    /// numeric columns can give the X values (and which the Output list offers).
    pub fn new(project: &Project, book: Option<WindowId>) -> Self {
        let defaults = SimulateCurveSpec::default();
        let functions = library::list();
        let (from, to, points) = match defaults.x {
            SimulateX::Range { from, to, points, .. } => (from, to, points),
            SimulateX::OfColumn(_) => (-1.0, 5.0, 200),
        };
        let columns = book
            .and_then(|id| project.workbook(id))
            .and_then(|b| b.sheets.get(b.active_sheet))
            .map(|s| {
                s.columns
                    .iter()
                    .filter(|c| c.data.is_numeric())
                    .map(|c| (c.id, format!("{}: {}", c.short_name, c.axis_title())))
                    .collect()
            })
            .unwrap_or_default();
        let books = project.workbooks().map(|b| (b.id, b.name.clone())).collect();
        let mut d = SimulateDialog {
            functions,
            function: defaults.function,
            values: Vec::new(),
            source: XSource::Range,
            from,
            to,
            points,
            log: false,
            columns,
            x_column: None,
            books,
            // Origin writes the curve to a new workbook and draws it (fitting.md 13.1).
            output: None,
            plot: true,
            problem: None,
            preview: None,
        };
        d.x_column = d.columns.first().map(|c| c.0);
        d.reset_values();
        d
    }

    /// The starting values of the chosen function.
    fn reset_values(&mut self) {
        self.values = library::get(&self.function).map(|f| sim::initial_params(&f)).unwrap_or_default();
    }

    fn categories(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for f in &self.functions {
            if !out.contains(&f.category) {
                out.push(f.category.clone());
            }
        }
        out
    }

    fn category(&self) -> String {
        self.functions.iter().find(|f| f.name == self.function).map(|f| f.category.clone()).unwrap_or_default()
    }

    /// Chooses a function by name, ignoring case; an unknown name is left out.
    pub fn choose_function_named(&mut self, name: &str) {
        if let Some(f) = self.functions.iter().find(|f| f.name.eq_ignore_ascii_case(name)) {
            let name = f.name.clone();
            self.choose_function(&name);
        }
    }

    fn choose_function(&mut self, name: &str) {
        if self.function != name {
            self.function = name.to_string();
            self.reset_values();
            // As Origin's window: each function comes with its own X range.
            if let Some(f) = library::get(&self.function) {
                (self.from, self.to, self.points) = sim::default_range(&f);
            }
            self.problem = None;
        }
    }

    /// The tool's settings as the window stands.
    fn spec(&self) -> SimulateCurveSpec {
        let names = self.functions.iter().find(|f| f.name == self.function).map(|f| f.params.as_slice());
        let params = names.unwrap_or_default().iter().cloned().zip(self.values.iter().copied()).collect();
        let x = match (self.source, self.x_column) {
            (XSource::Column, Some(id)) => SimulateX::OfColumn(id),
            _ => SimulateX::Range {
                from: self.from,
                to: self.to,
                points: self.points,
                spacing: if self.log { Spacing::Log } else { Spacing::Linear },
            },
        };
        SimulateCurveSpec { function: self.function.clone(), params, x, plot: self.plot }
    }

    /// The preview curve for these settings, or why there is none.
    fn compute_preview(&self, project: &Project) -> Result<Vec<(f64, f64)>, String> {
        let function = library::get(&self.function).ok_or_else(|| format!("there is no function {}", self.function))?;
        let spec = self.spec();
        let params = sim::resolve_params(&function, &spec.params).map_err(|e| e.to_string())?;
        let xs = match &spec.x {
            SimulateX::Range { from, to, points, spacing } => {
                // A count outside the allowed range goes to `x_values` as it is, for its message.
                let n = if (2..=sim::MAX_POINTS).contains(points) { (*points).min(PREVIEW_POINTS) } else { *points };
                sim::x_values(*from, *to, n, *spacing).map_err(|e| e.to_string())?
            }
            SimulateX::OfColumn(id) => {
                let v = project.column(*id).and_then(|c| c.numeric()).ok_or("The X column is not numeric")?;
                v.iter().copied().take(2000).collect()
            }
        };
        let ys = sim::simulate(&function, &params, &xs).map_err(|e| e.to_string())?;
        Ok(xs.into_iter().zip(ys).filter(|(x, y)| x.is_finite() && y.is_finite()).collect())
    }

    fn refresh_preview(&mut self, doc: &Document) {
        let key = PreviewKey { spec: self.spec(), revision: doc.revision() };
        if self.preview.as_ref().is_some_and(|(k, _)| *k == key) {
            return;
        }
        let curve = self.compute_preview(doc.project());
        self.preview = Some((key, curve));
        // A message from an earlier OK click is about settings that have changed since.
        self.problem = None;
    }

    /// Whether the settings make a curve; the preview says why not when they do not.
    fn can_make(&self) -> bool {
        matches!(self.preview, Some((_, Ok(_))))
    }

    /// Makes the sheet (and the graph) as one undo step.
    fn run(&self, doc: &mut Document) -> Result<Outcome, String> {
        let spec = self.spec();
        let function = self.function.clone();
        let output = self.output;
        doc.edit("Simulate curve", |p| {
            // The workbook that gets the sheet, and a column to name it in the request.
            let (book, created) = match (&spec.x, output) {
                (SimulateX::OfColumn(id), _) => (p.locate(*id).ok_or("The X column is gone")?.0.id, false),
                (_, Some(book)) if p.workbook(book).is_some() => (book, false),
                _ => {
                    let id = p.add_workbook(&format!("Simulate {function}"));
                    (id, true)
                }
            };
            let (column, placeholder) = match &spec.x {
                SimulateX::OfColumn(id) => (*id, false),
                _ => {
                    let b = p.workbook(book).ok_or("The workbook is gone")?;
                    let sheet = b.active_sheet.min(b.sheets.len().saturating_sub(1));
                    match b.sheets.get(sheet).and_then(|s| s.columns.first()).map(|c| c.id) {
                        Some(id) => (id, false),
                        None => (
                            p.add_column(book, sheet, Designation::Y, ColumnData::numeric(Vec::new()))
                                .map_err(|e| e.to_string())?,
                            true,
                        ),
                    }
                }
            };
            let request = Request { columns: vec![column], rows: RowRange::All, tool: Tool::SimulateCurve(spec) };
            let report = p.analyze(&request).map_err(|e| e.to_string())?;
            let mut new_sheet = report.new_sheet;
            if placeholder && let Some(b) = p.workbook_mut(book) {
                if created {
                    b.sheets.remove(0);
                    new_sheet = new_sheet.map(|s| s.saturating_sub(1));
                } else if let Some(s) = b.sheets.get_mut(report.sheet) {
                    s.columns.retain(|c| c.id != column);
                }
            }
            if let (Some(index), Some(b)) = (new_sheet, p.workbook_mut(book)) {
                b.active_sheet = index;
            }
            Ok::<_, String>(Outcome::Done { book, graph: report.new_graph, message: report.message })
        })
    }

    /// Draws the window. OK makes the curve and closes it; it is off while the preview shows a problem,
    /// and a problem found when making the curve stays in the window.
    pub fn show(&mut self, ctx: &egui::Context, doc: &mut Document) -> Outcome {
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            return Outcome::Closed;
        }
        self.refresh_preview(doc);
        let mut open = true;
        let mut outcome = Outcome::Open;
        egui::Window::new("Simulate Curve")
            .id(egui::Id::new("simulate_curve"))
            .open(&mut open)
            .collapsible(false)
            .default_width(480.0)
            .show(ctx, |ui| {
                self.function_ui(ui);
                ui.separator();
                self.params_ui(ui);
                ui.separator();
                self.x_ui(ui);
                ui.separator();
                self.output_ui(ui);
                ui.separator();
                self.preview_ui(ui);
                if let Some(e) = &self.problem {
                    ui.colored_label(ui.visuals().error_fg_color, e);
                }
                ui.horizontal(|ui| {
                    if ui.add_enabled(self.can_make(), egui::Button::new("OK")).clicked() {
                        match self.run(doc) {
                            Ok(done) => outcome = done,
                            Err(e) => self.problem = Some(e),
                        }
                    }
                    if ui.button("Cancel").clicked() {
                        outcome = Outcome::Closed;
                    }
                });
            });
        if open { outcome } else { Outcome::Closed }
    }

    fn function_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let category = self.category();
            let label = ui.label("Category");
            egui::ComboBox::from_id_salt("simulate_category")
                .selected_text(&category)
                .width(170.0)
                .height(420.0)
                .show_ui(ui, |ui| {
                    for c in self.categories() {
                        if ui.selectable_label(c == category, &c).clicked() && c != category {
                            let first = self.functions.iter().find(|f| f.category == c).map(|f| f.name.clone());
                            if let Some(name) = first {
                                self.choose_function(&name);
                            }
                        }
                    }
                })
                .response
                .labelled_by(label.id);
            ui.add_space(8.0);
            let label = ui.label("Function");
            let mut chosen = None;
            egui::ComboBox::from_id_salt("simulate_function")
                .selected_text(&self.function)
                .width(170.0)
                .height(420.0)
                .show_ui(ui, |ui| {
                    for f in self.functions.iter().filter(|f| f.category == category) {
                        if ui.selectable_label(f.name == self.function, &f.name).clicked() {
                            chosen = Some(f.name.clone());
                        }
                    }
                })
                .response
                .labelled_by(label.id);
            if let Some(name) = chosen {
                self.choose_function(&name);
            }
        });
        let formula = self.functions.iter().find(|f| f.name == self.function).map(|f| match &f.formula {
            Some(text) => format!("y = {text}"),
            None => "(computed in code)".to_string(),
        });
        ui.add(egui::Label::new(egui::RichText::new(formula.unwrap_or_default()).monospace()).wrap());
    }

    fn params_ui(&mut self, ui: &mut egui::Ui) {
        let names: Vec<String> =
            self.functions.iter().find(|f| f.name == self.function).map(|f| f.params.clone()).unwrap_or_default();
        egui::ScrollArea::vertical().id_salt("simulate_params").max_height(180.0).auto_shrink([true, true]).show(
            ui,
            |ui| {
                ui.spacing_mut().interact_size.x = 110.0;
                egui::Grid::new("simulate_param_grid").num_columns(2).spacing([14.0, 4.0]).show(ui, |ui| {
                    for (name, value) in names.iter().zip(self.values.iter_mut()) {
                        let label = ui.label(name);
                        let speed = (value.abs() * 0.01).max(0.001);
                        ui.add(DragValue::new(value).speed(speed).max_decimals(12)).labelled_by(label.id);
                        ui.end_row();
                    }
                });
            },
        );
        if ui.button("Initial values").on_hover_text("Put the starting values of the function back").clicked() {
            self.reset_values();
        }
    }

    fn x_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.radio_value(&mut self.source, XSource::Range, "X range");
            ui.add_enabled_ui(!self.columns.is_empty(), |ui| {
                ui.radio_value(&mut self.source, XSource::Column, "X of a column");
            });
        });
        match self.source {
            XSource::Range => {
                ui.spacing_mut().interact_size.x = 110.0;
                egui::Grid::new("simulate_x").num_columns(2).spacing([14.0, 4.0]).show(ui, |ui| {
                    let label = ui.label("From");
                    ui.add(DragValue::new(&mut self.from).speed(0.1).max_decimals(12)).labelled_by(label.id);
                    ui.end_row();
                    let label = ui.label("To");
                    ui.add(DragValue::new(&mut self.to).speed(0.1).max_decimals(12)).labelled_by(label.id);
                    ui.end_row();
                    let label = ui.label("Points");
                    ui.add(DragValue::new(&mut self.points).speed(1.0)).labelled_by(label.id);
                    ui.end_row();
                });
                ui.checkbox(&mut self.log, "Logarithmic spacing");
            }
            XSource::Column => {
                let shown = self.columns.iter().find(|c| Some(c.0) == self.x_column).map(|c| c.1.clone());
                ui.horizontal(|ui| {
                    let label = ui.label("Column");
                    egui::ComboBox::from_id_salt("simulate_x_column")
                        .selected_text(shown.unwrap_or_default())
                        .show_ui(ui, |ui| {
                            for (id, name) in &self.columns {
                                ui.selectable_value(&mut self.x_column, Some(*id), name);
                            }
                        })
                        .response
                        .labelled_by(label.id);
                });
                ui.weak("The curve is made at every value of the column, row by row.");
            }
        }
    }

    fn output_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let label = ui.label("Output");
            let own_book = self.source == XSource::Column;
            let shown = match (own_book, self.output) {
                (true, _) => "Workbook of the X column".to_string(),
                (false, None) => "New workbook".to_string(),
                (false, Some(id)) => self
                    .books
                    .iter()
                    .find(|b| b.0 == id)
                    .map_or("New workbook".to_string(), |b| format!("New sheet in {}", b.1)),
            };
            ui.add_enabled_ui(!own_book, |ui| {
                egui::ComboBox::from_id_salt("simulate_output")
                    .selected_text(shown)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.output, None, "New workbook");
                        for (id, name) in &self.books {
                            ui.selectable_value(&mut self.output, Some(*id), format!("New sheet in {name}"));
                        }
                    })
                    .response
                    .labelled_by(label.id);
            });
            ui.checkbox(&mut self.plot, "Plot the curve");
        });
    }

    fn preview_ui(&mut self, ui: &mut egui::Ui) {
        match self.preview.as_ref().map(|(_, p)| p) {
            Some(Ok(curve)) => {
                let red = egui::Color32::from_rgb(190, 30, 30);
                draw_xy(ui, egui::vec2(ui.available_width(), 120.0), &[(curve.as_slice(), Series::Line, red)], false);
            }
            Some(Err(e)) => {
                ui.colored_label(ui.visuals().error_fg_color, e);
            }
            None => {}
        }
    }

    // ----- test hooks ---------------------------------------------------------------------------

    #[cfg(test)]
    pub(crate) fn preview_for_test(&self) -> Option<Result<usize, String>> {
        self.preview.as_ref().map(|(_, p)| p.as_ref().map(Vec::len).map_err(Clone::clone))
    }

    #[cfg(test)]
    pub(crate) fn problem_for_test(&self) -> Option<String> {
        self.problem.clone()
    }

    #[cfg(test)]
    pub(crate) fn set_for_test(&mut self, function: &str, from: f64, to: f64, points: usize, log: bool) {
        self.choose_function(function);
        (self.from, self.to, self.points, self.log) = (from, to, points, log);
    }

    #[cfg(test)]
    pub(crate) fn use_column_for_test(&mut self, id: ColumnId) {
        self.source = XSource::Column;
        self.x_column = Some(id);
    }

    #[cfg(test)]
    pub(crate) fn values_for_test(&self) -> Vec<f64> {
        self.values.clone()
    }
}
