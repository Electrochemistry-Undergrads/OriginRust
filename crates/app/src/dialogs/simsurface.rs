//! Analysis > Fitting > Simulate Surface: the window that makes XYZ data of a surface fit function on a grid. The
//! numbers and the new sheet come from `sheaf_core::analysis::Tool::SimulateSurface`; this file is the controls and
//! the one undo step. The controls and their defaults are Origin's (`docs/spec/fitting.md`, section 16.1: the 22
//! functions with their own parameter values and ranges, 20 by 20 points, Noise Level (%)); Recalculate and the
//! preview pane are not built, and the output goes to a new workbook or a new sheet (Sheaf's choice, 16.3).

use eframe::egui::{self, DragValue};
use sheaf_core::analysis::{Request, RowRange, SimulateSurfaceSpec, Tool};
use sheaf_core::{ColumnData, Designation, Document, Project, WindowId};
use sheaf_fit::simsurface as sim;

/// What the window did in this frame.
pub enum Outcome {
    Open,
    Closed,
    /// The surface was made: the workbook and the status line.
    Done {
        book: WindowId,
        message: String,
    },
}

/// What the window keeps between frames.
pub struct SimulateSurfaceDialog {
    function: String,
    /// One value per parameter of the function, in the order of the function.
    values: Vec<f64>,
    x: (f64, f64, usize),
    y: (f64, f64, usize),
    noise: f64,
    seed: u64,
    /// The workbooks that can receive the sheet.
    books: Vec<(WindowId, String)>,
    /// `None` makes a new workbook (Origin's `<new>`).
    output: Option<WindowId>,
    problem: Option<String>,
}

impl SimulateSurfaceDialog {
    /// Opens on Gauss2D with Origin's window values; `book` is the active workbook, which the Output list offers.
    pub fn new(project: &Project, _book: Option<WindowId>) -> Self {
        let d = SimulateSurfaceSpec::default();
        let mut w = SimulateSurfaceDialog {
            function: d.function,
            values: Vec::new(),
            x: d.x,
            y: d.y,
            noise: d.noise,
            seed: d.seed,
            books: project.workbooks().map(|b| (b.id, b.name.clone())).collect(),
            output: None,
            problem: None,
        };
        w.reset_values();
        w
    }

    fn reset_values(&mut self) {
        self.values = sim::sample(&self.function).map(|s| s.params.to_vec()).unwrap_or_default();
    }

    /// Chooses a function by name (Origin's or the library's, any case); an unknown name is left out.
    pub fn choose_function_named(&mut self, name: &str) {
        if let Some(s) = sim::sample(name) {
            self.choose_function(s.origin_name);
        }
    }

    /// As Origin's window: each function comes with its own values, ranges and noise level.
    fn choose_function(&mut self, name: &str) {
        if self.function != name
            && let Some(spec) = SimulateSurfaceSpec::for_function(name)
        {
            self.function = spec.function;
            self.x = spec.x;
            self.y = spec.y;
            self.noise = spec.noise;
            self.reset_values();
            self.problem = None;
        }
    }

    fn param_names(&self) -> Vec<String> {
        sim::function(&self.function)
            .map(|(_, f)| f.params.iter().map(|p| p.name.clone()).collect())
            .unwrap_or_default()
    }

    /// The tool's settings as the window stands.
    fn spec(&self) -> SimulateSurfaceSpec {
        SimulateSurfaceSpec {
            function: self.function.clone(),
            params: self.param_names().into_iter().zip(self.values.iter().copied()).collect(),
            x: self.x,
            y: self.y,
            noise: self.noise,
            seed: self.seed,
        }
    }

    /// Makes the sheet as one undo step: a new workbook, or a new sheet in the chosen one.
    fn run(&self, doc: &mut Document) -> Result<Outcome, String> {
        let spec = self.spec();
        let output = self.output;
        let function = self.function.clone();
        doc.edit("Simulate surface", |p| {
            let (book, created) = match output {
                Some(book) if p.workbook(book).is_some() => (book, false),
                _ => (p.add_workbook(&format!("Simulate {function}")), true),
            };
            let b = p.workbook(book).ok_or("The workbook is gone")?;
            let sheet = b.active_sheet.min(b.sheets.len().saturating_sub(1));
            let (column, placeholder) = match b.sheets.get(sheet).and_then(|s| s.columns.first()).map(|c| c.id) {
                Some(id) => (id, false),
                None => (
                    p.add_column(book, sheet, Designation::Y, ColumnData::numeric(Vec::new()))
                        .map_err(|e| e.to_string())?,
                    true,
                ),
            };
            let request = Request { columns: vec![column], rows: RowRange::All, tool: Tool::SimulateSurface(spec) };
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
            Ok::<_, String>(Outcome::Done { book, message: report.message })
        })
    }

    /// Why the settings make no surface, if they do not.
    fn check(&self) -> Option<String> {
        let (x, y) = (self.x, self.y);
        sim::axis(x.0, x.1, x.2, "X").and_then(|_| sim::axis(y.0, y.1, y.2, "Y")).err().map(|e| e.to_string()).or_else(
            || {
                (x.2.saturating_mul(y.2) > sim::MAX_POINTS)
                    .then(|| format!("{} by {} points are too many; the most is {} in all", x.2, y.2, sim::MAX_POINTS))
            },
        )
    }

    /// Draws the window. OK makes the surface and closes it; a problem found when making it stays in the window.
    pub fn show(&mut self, ctx: &egui::Context, doc: &mut Document) -> Outcome {
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            return Outcome::Closed;
        }
        let mut open = true;
        let mut outcome = Outcome::Open;
        let check = self.check();
        egui::Window::new("Simulate Surface")
            .id(egui::Id::new("simulate_surface"))
            .open(&mut open)
            .collapsible(false)
            .default_width(420.0)
            .show(ctx, |ui| {
                ui.weak("Generate surface data with specified surface function");
                self.function_ui(ui);
                ui.separator();
                self.params_ui(ui);
                ui.separator();
                self.grid_ui(ui);
                ui.separator();
                self.output_ui(ui);
                if let Some(e) = check.as_ref().or(self.problem.as_ref()) {
                    ui.colored_label(ui.visuals().error_fg_color, e);
                }
                ui.horizontal(|ui| {
                    if ui.add_enabled(check.is_none(), egui::Button::new("OK")).clicked() {
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
            // Origin's Category box is an edit box with the one category.
            let label = ui.label("Category");
            let mut category = "Surface Fitting".to_string();
            ui.add_enabled(false, egui::TextEdit::singleline(&mut category).desired_width(150.0)).labelled_by(label.id);
            ui.add_space(8.0);
            let label = ui.label("Function");
            let mut chosen = None;
            egui::ComboBox::from_id_salt("simsurface_function")
                .selected_text(&self.function)
                .width(150.0)
                .height(420.0)
                .show_ui(ui, |ui| {
                    for name in sim::names() {
                        if ui.selectable_label(name == self.function, name).clicked() {
                            chosen = Some(name);
                        }
                    }
                })
                .response
                .labelled_by(label.id);
            if let Some(name) = chosen {
                self.choose_function(name);
            }
        });
        let formula = sim::sample(&self.function).and_then(|s| sheaf_fit::surface::formula(s.library_name));
        ui.add(egui::Label::new(egui::RichText::new(format!("z = {}", formula.unwrap_or(""))).monospace()).wrap());
    }

    fn params_ui(&mut self, ui: &mut egui::Ui) {
        let names = self.param_names();
        egui::ScrollArea::vertical().id_salt("simsurface_params").max_height(200.0).auto_shrink([true, true]).show(
            ui,
            |ui| {
                ui.spacing_mut().interact_size.x = 110.0;
                egui::Grid::new("simsurface_param_grid").num_columns(2).spacing([14.0, 4.0]).show(ui, |ui| {
                    for (name, value) in names.iter().zip(self.values.iter_mut()) {
                        let label = ui.label(name);
                        let speed = (value.abs() * 0.01).max(0.001);
                        ui.add(DragValue::new(value).speed(speed).max_decimals(12)).labelled_by(label.id);
                        ui.end_row();
                    }
                });
            },
        );
        if ui.button("Initial values").on_hover_text("Put the values Origin's window shows back").clicked() {
            self.reset_values();
        }
    }

    fn grid_ui(&mut self, ui: &mut egui::Ui) {
        ui.spacing_mut().interact_size.x = 110.0;
        egui::Grid::new("simsurface_grid").num_columns(2).spacing([14.0, 4.0]).show(ui, |ui| {
            let row = |ui: &mut egui::Ui, name: &str, value: &mut f64| {
                let label = ui.label(name);
                ui.add(DragValue::new(value).speed(0.1).max_decimals(12)).labelled_by(label.id);
                ui.end_row();
            };
            row(ui, "X Minimum", &mut self.x.0);
            row(ui, "X Maximum", &mut self.x.1);
            let label = ui.label("X Points");
            ui.add(DragValue::new(&mut self.x.2).speed(1.0)).labelled_by(label.id);
            ui.end_row();
            row(ui, "Y Minimum", &mut self.y.0);
            row(ui, "Y Maximum", &mut self.y.1);
            let label = ui.label("Y Points");
            ui.add(DragValue::new(&mut self.y.2).speed(1.0)).labelled_by(label.id);
            ui.end_row();
            let label = ui.label("Noise Level (%)");
            ui.add(DragValue::new(&mut self.noise).speed(0.5).range(0.0..=f64::MAX)).labelled_by(label.id);
            ui.end_row();
        });
        ui.weak("Noise is added to Z as a percentage of its mean; Sheaf's generator, not Origin's sequence.");
    }

    fn output_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let label = ui.label("Result Data");
            let shown = match self.output {
                None => "New workbook".to_string(),
                Some(id) => self
                    .books
                    .iter()
                    .find(|b| b.0 == id)
                    .map_or("New workbook".to_string(), |b| format!("New sheet in {}", b.1)),
            };
            egui::ComboBox::from_id_salt("simsurface_output")
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
    }

    // ----- test hooks ---------------------------------------------------------------------------

    #[cfg(test)]
    pub(crate) fn values_for_test(&self) -> Vec<f64> {
        self.values.clone()
    }

    #[cfg(test)]
    pub(crate) fn function_for_test(&self) -> String {
        self.function.clone()
    }

    #[cfg(test)]
    pub(crate) fn set_grid_for_test(&mut self, x: (f64, f64, usize), y: (f64, f64, usize), noise: f64) {
        (self.x, self.y, self.noise) = (x, y, noise);
    }
}
