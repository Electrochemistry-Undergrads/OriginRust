//! The Fit window (Analysis > Fit): fit a straight line, a polynomial, a built-in function, a
//! typed formula or an Origin `.FDF` function to one column.
//!
//! The window shows the column, the model with its formula, one row per parameter (starting
//! value, fixed value, bounds), the weighting and the rows. OK runs the fit as one undoable step;
//! the results go where `docs/adr/0006-fit-results.md` says, and if no graph shows the column yet
//! a new graph with the data and the fitted curve is made.

use eframe::egui::{self, DragValue, RichText};
use sheaf_core::analysis::RowRange;
use sheaf_core::fitting::{
    FitModel, FitOutcome, FitRequest, FitRequestError, FitWeighting, FunctionInfo, ParamSetting, builtin_functions,
    model_parameters,
};
use sheaf_core::{ColumnId, Designation, Document, Project, WindowId};
use std::path::PathBuf;

/// The model families of the window; also the entries of the Analysis menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FitKind {
    Linear,
    Polynomial,
    BuiltIn,
    Formula,
    Fdf,
}

impl FitKind {
    /// Menu entries of the Analysis menu that open this window.
    pub const MENU: [(FitKind, &'static str); 3] = [
        (FitKind::Linear, "Fit Linear..."),
        (FitKind::Polynomial, "Fit Polynomial..."),
        (FitKind::BuiltIn, "Nonlinear Curve Fit..."),
    ];

    fn label(self) -> &'static str {
        match self {
            FitKind::Linear => "Straight line",
            FitKind::Polynomial => "Polynomial",
            FitKind::BuiltIn => "Built-in function",
            FitKind::Formula => "My own formula",
            FitKind::Fdf => "Origin .FDF file",
        }
    }

    /// The name used with `--show fit:NAME`.
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "linear" => FitKind::Linear,
            "poly" | "polynomial" => FitKind::Polynomial,
            "nonlinear" | "builtin" => FitKind::BuiltIn,
            "formula" => FitKind::Formula,
            "fdf" => FitKind::Fdf,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RowMode {
    All,
    Rows,
    XRange,
}

/// One row of the parameter table.
#[derive(Debug, Clone, PartialEq)]
struct ParamRow {
    name: String,
    /// A starting value was typed (otherwise the function's own guess is used).
    set_initial: bool,
    initial: f64,
    fixed: bool,
    has_lower: bool,
    lower: f64,
    has_upper: bool,
    upper: f64,
}

impl ParamRow {
    fn new(name: &str) -> Self {
        ParamRow {
            name: name.to_string(),
            set_initial: false,
            initial: 1.0,
            fixed: false,
            has_lower: false,
            lower: 0.0,
            has_upper: false,
            upper: 1.0,
        }
    }

    fn setting(&self) -> Option<ParamSetting> {
        let s = ParamSetting {
            name: self.name.clone(),
            initial: (self.set_initial || self.fixed).then_some(self.initial),
            fixed: self.fixed,
            lower: self.has_lower.then_some(self.lower),
            upper: self.has_upper.then_some(self.upper),
        };
        (s.initial.is_some() || s.fixed || s.lower.is_some() || s.upper.is_some()).then_some(s)
    }
}

pub enum FitDialogOutcome {
    Open,
    Closed,
    /// The fit ran; the window closes. The graph is the one made for it, if any.
    Done(Box<FitOutcome>, Option<WindowId>),
}

pub struct FitDialog {
    book: WindowId,
    /// The numeric Y columns of the sheet: id and "B: Current" style label.
    choices: Vec<(ColumnId, String)>,
    y: ColumnId,
    kind: FitKind,
    order: usize,
    functions: Vec<FunctionInfo>,
    function: String,
    search: String,
    formula: String,
    formula_params: String,
    fdf_path: Option<PathBuf>,
    fdf_text: Option<String>,
    params: Vec<ParamRow>,
    /// Why the parameter list could not be built, if it could not.
    model_problem: Option<String>,
    weighting: FitWeighting,
    has_error_column: bool,
    confidence: f64,
    row_mode: RowMode,
    rows_in_sheet: usize,
    from: usize,
    to: usize,
    x_from: f64,
    x_to: f64,
    problem: Option<String>,
}

impl FitDialog {
    /// A Fit window for column `y` (or, when `None`, the first numeric Y column of the workbook's
    /// active sheet). `None` when there is nothing to fit.
    pub fn new(project: &Project, book: WindowId, y: Option<ColumnId>, kind: FitKind) -> Option<Self> {
        let b = project.workbook(book)?;
        let sheet = match y {
            Some(id) => b.sheets.iter().find(|s| s.column(id).is_some())?,
            None => b.sheets.get(b.active_sheet.min(b.sheets.len().checked_sub(1)?))?,
        };
        let choices: Vec<(ColumnId, String)> = sheet
            .columns
            .iter()
            .filter(|c| c.designation == Designation::Y && c.data.is_numeric())
            .map(|c| (c.id, format!("{}: {}", c.short_name, c.axis_title())))
            .collect();
        let y = match y {
            Some(id) if choices.iter().any(|(c, _)| *c == id) => id,
            Some(_) => return None,
            None => choices.first()?.0,
        };
        let rows_in_sheet = sheet.row_count().max(1);
        let functions = builtin_functions();
        let function = functions.iter().find(|f| f.name == "Gauss").or(functions.first()).map(|f| f.name.clone())?;
        let mut d = FitDialog {
            book,
            choices,
            y,
            kind,
            order: 2,
            functions,
            function,
            search: String::new(),
            formula: "y0 + A*exp(-x/t)".to_string(),
            formula_params: "y0, A, t".to_string(),
            fdf_path: None,
            fdf_text: None,
            params: Vec::new(),
            model_problem: None,
            weighting: FitWeighting::None,
            has_error_column: false,
            confidence: 95.0,
            row_mode: RowMode::All,
            rows_in_sheet,
            from: 1,
            to: rows_in_sheet,
            x_from: 0.0,
            x_to: 1.0,
            problem: None,
        };
        d.column_changed(project);
        d.refresh_params();
        Some(d)
    }

    /// Updates what depends on the chosen column: the X range and whether it has error bars.
    fn column_changed(&mut self, project: &Project) {
        let Some((_, sheet, _)) = project.locate(self.y) else {
            return;
        };
        let index = sheet.column_index(self.y).expect("located");
        self.has_error_column = sheet.y_error_for(index).is_some();
        if !self.has_error_column && self.weighting == FitWeighting::Instrumental {
            self.weighting = FitWeighting::None;
        }
        let x = sheet.x_for(index).and_then(|c| c.numeric());
        let (lo, hi) = match x {
            Some(v) => v
                .iter()
                .filter(|v| v.is_finite())
                .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), v| (a.min(*v), b.max(*v))),
            None => (1.0, sheet.row_count() as f64),
        };
        if lo <= hi {
            (self.x_from, self.x_to) = (lo, hi);
        }
    }

    fn model(&self) -> Option<FitModel> {
        Some(match self.kind {
            FitKind::Linear => FitModel::Linear,
            FitKind::Polynomial => FitModel::Polynomial { order: self.order },
            FitKind::BuiltIn => FitModel::BuiltIn(self.function.clone()),
            FitKind::Formula => FitModel::Formula {
                formula: self.formula.clone(),
                params: self
                    .formula_params
                    .split([',', ' ', ';'])
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(String::from)
                    .collect(),
            },
            FitKind::Fdf => FitModel::Fdf(self.fdf_text.clone()?),
        })
    }

    /// Rebuilds the parameter table for the current model, keeping what was typed for names
    /// that stay.
    fn refresh_params(&mut self) {
        let names = match self.model() {
            None => {
                self.model_problem = Some("Choose an .FDF file.".to_string());
                Vec::new()
            }
            Some(m) => match model_parameters(&m) {
                Ok(n) => {
                    self.model_problem = None;
                    n
                }
                Err(e) => {
                    self.model_problem = Some(e.to_string());
                    Vec::new()
                }
            },
        };
        let old = std::mem::take(&mut self.params);
        self.params = names
            .iter()
            .map(|n| old.iter().find(|p| &p.name == n).cloned().unwrap_or_else(|| ParamRow::new(n)))
            .collect();
    }

    fn request(&self) -> Option<FitRequest> {
        let mut r = FitRequest::new(self.y, self.model()?);
        let closed_form = matches!(self.kind, FitKind::Linear | FitKind::Polynomial);
        r.params = self
            .params
            .iter()
            // A straight line or polynomial has only fixed values; start values and bounds typed
            // for another model with the same parameter names do not apply.
            .filter(|p| !closed_form || p.fixed)
            .map(|p| {
                if closed_form {
                    ParamRow { set_initial: false, has_lower: false, has_upper: false, ..p.clone() }
                } else {
                    p.clone()
                }
            })
            .filter_map(|p| p.setting())
            .collect();
        r.weighting = self.weighting;
        r.confidence_level = self.confidence / 100.0;
        r.rows = match self.row_mode {
            RowMode::All => RowRange::All,
            RowMode::Rows => RowRange::Rows(self.from, self.to),
            RowMode::XRange => RowRange::XBetween(self.x_from, self.x_to),
        };
        Some(r)
    }

    fn model_ui(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label("Model");
            egui::ComboBox::from_id_salt("fit_kind").selected_text(self.kind.label()).show_ui(ui, |ui| {
                for k in [FitKind::Linear, FitKind::Polynomial, FitKind::BuiltIn, FitKind::Formula, FitKind::Fdf] {
                    changed |= ui.selectable_value(&mut self.kind, k, k.label()).changed();
                }
            });
        });
        match self.kind {
            FitKind::Linear => {
                ui.weak("y = Intercept + Slope·x");
            }
            FitKind::Polynomial => {
                ui.horizontal(|ui| {
                    ui.label("Order");
                    changed |= ui.add(DragValue::new(&mut self.order).range(1..=9)).changed();
                    ui.weak("y = Intercept + B1·x + B2·x² + ...");
                });
            }
            FitKind::BuiltIn => {
                ui.horizontal(|ui| {
                    ui.label("Function");
                    let selected = self.function.clone();
                    egui::ComboBox::from_id_salt("fit_function")
                        .selected_text(&selected)
                        .width(200.0)
                        .height(420.0)
                        .show_ui(ui, |ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut self.search).hint_text("search").desired_width(180.0),
                            );
                            let needle = self.search.to_ascii_lowercase();
                            let mut category = "";
                            for f in &self.functions {
                                let hit = needle.is_empty()
                                    || f.name.to_ascii_lowercase().contains(&needle)
                                    || f.category.to_ascii_lowercase().contains(&needle);
                                if !hit {
                                    continue;
                                }
                                if f.category != category {
                                    category = &f.category;
                                    ui.label(RichText::new(category).weak().small());
                                }
                                if ui.selectable_label(f.name == selected, &f.name).clicked() {
                                    self.function = f.name.clone();
                                    changed = true;
                                }
                            }
                        });
                });
                if let Some(f) = self.functions.iter().find(|f| f.name == self.function) {
                    let text =
                        f.formula.as_deref().map_or_else(|| "(computed in code)".to_string(), |s| format!("y = {s}"));
                    ui.add(egui::Label::new(RichText::new(text).monospace().small()).wrap());
                }
            }
            FitKind::Formula => {
                ui.horizontal(|ui| {
                    ui.label("y =");
                    changed |= ui
                        .add(
                            egui::TextEdit::singleline(&mut self.formula)
                                .font(egui::TextStyle::Monospace)
                                .desired_width(330.0),
                        )
                        .changed();
                });
                ui.horizontal(|ui| {
                    ui.label("Parameters");
                    changed |= ui
                        .add(
                            egui::TextEdit::singleline(&mut self.formula_params)
                                .desired_width(250.0)
                                .hint_text("y0, A, t"),
                        )
                        .changed();
                });
                ui.weak(
                    "The variable is x. Functions: exp, ln, log (base 10), sqrt, sin, cos, erf, ... and ^ for powers.",
                );
            }
            FitKind::Fdf => {
                ui.horizontal(|ui| {
                    if ui.button("Choose file...").clicked()
                        && let Some(path) =
                            rfd::FileDialog::new().add_filter("Origin fit function", &["fdf", "FDF"]).pick_file()
                    {
                        // Origin writes these files in the Windows code page or as UTF-16; the
                        // shared reader handles both, as in `sheaf fit --fdf`.
                        match sheaf_io::read_text_file(&path) {
                            Ok(text) => {
                                self.fdf_text = Some(text);
                                self.fdf_path = Some(path);
                                self.problem = None;
                            }
                            Err(e) => self.problem = Some(e.to_string()),
                        }
                        changed = true;
                    }
                    match &self.fdf_path {
                        Some(p) => {
                            ui.label(p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default())
                        }
                        None => ui.weak("no file chosen"),
                    };
                });
            }
        }
        changed
    }

    fn params_ui(&mut self, ui: &mut egui::Ui) {
        if let Some(p) = &self.model_problem {
            ui.colored_label(egui::Color32::from_rgb(170, 30, 30), p);
            return;
        }
        if self.params.is_empty() {
            return;
        }
        // A linear fit can fix the intercept and the slope; a polynomial only the intercept.
        let closed_form = matches!(self.kind, FitKind::Linear | FitKind::Polynomial);
        egui::Grid::new("fit_params").num_columns(5).spacing([10.0, 4.0]).striped(true).show(ui, |ui| {
            ui.strong("Parameter");
            ui.strong(if closed_form { "" } else { "Start value" });
            ui.strong("Fixed");
            ui.strong(if closed_form { "" } else { "Lower bound" });
            ui.strong(if closed_form { "" } else { "Upper bound" });
            ui.end_row();
            for p in self.params.iter_mut() {
                let can_fix =
                    !closed_form || p.name == "Intercept" || (self.kind == FitKind::Linear && p.name == "Slope");
                ui.label(&p.name);
                if closed_form {
                    ui.label("");
                } else {
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut p.set_initial, "")
                            .on_hover_text("Off: the function's own guess from the data");
                        if p.set_initial || p.fixed {
                            ui.add(DragValue::new(&mut p.initial).speed(0.01).max_decimals(10));
                        } else {
                            ui.weak("guess");
                        }
                    });
                }
                ui.add_enabled_ui(can_fix, |ui| {
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut p.fixed, "");
                        if closed_form && p.fixed {
                            ui.add(DragValue::new(&mut p.initial).speed(0.01).max_decimals(10));
                        }
                    });
                });
                if closed_form {
                    ui.label("");
                    ui.label("");
                } else {
                    for (on, value) in [(&mut p.has_lower, &mut p.lower), (&mut p.has_upper, &mut p.upper)] {
                        ui.horizontal(|ui| {
                            ui.checkbox(on, "");
                            if *on {
                                ui.add(DragValue::new(value).speed(0.01).max_decimals(10));
                            } else {
                                ui.weak("none");
                            }
                        });
                    }
                }
                ui.end_row();
            }
        });
    }

    /// Draws the window and runs the fit when OK is pressed.
    pub fn show(&mut self, ctx: &egui::Context, doc: &mut Document) -> FitDialogOutcome {
        if doc.project().column(self.y).is_none() || doc.project().workbook(self.book).is_none() {
            return FitDialogOutcome::Closed;
        }
        let mut open = true;
        let mut run = false;
        let mut cancel = false;
        let mut column_changed = false;
        egui::Window::new("Fit")
            .id(egui::Id::new("fit"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(520.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Column");
                    let label =
                        self.choices.iter().find(|(c, _)| *c == self.y).map(|(_, l)| l.clone()).unwrap_or_default();
                    egui::ComboBox::from_id_salt("fit_column").selected_text(label).width(300.0).show_ui(ui, |ui| {
                        for (id, l) in &self.choices {
                            column_changed |= ui.selectable_value(&mut self.y, *id, l).changed();
                        }
                    });
                });
                ui.separator();
                if self.model_ui(ui) {
                    self.refresh_params();
                }
                ui.add_space(4.0);
                self.params_ui(ui);
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("Weighting");
                    ui.radio_value(&mut self.weighting, FitWeighting::None, "None");
                    ui.add_enabled_ui(self.has_error_column, |ui| {
                        ui.radio_value(&mut self.weighting, FitWeighting::Instrumental, "Error bars (1/err²)")
                            .on_disabled_hover_text("The column has no Y-error column next to it.");
                    });
                    ui.radio_value(&mut self.weighting, FitWeighting::Statistical, STATISTICAL_LABEL);
                });
                ui.horizontal(|ui| {
                    ui.label("Confidence level");
                    ui.add(DragValue::new(&mut self.confidence).range(50.0..=99.99).speed(0.1).suffix(" %"));
                });
                ui.horizontal(|ui| {
                    ui.label("Rows");
                    ui.radio_value(&mut self.row_mode, RowMode::All, "All");
                    ui.radio_value(&mut self.row_mode, RowMode::Rows, "From row");
                    ui.add_enabled_ui(self.row_mode == RowMode::Rows, |ui| {
                        ui.add(DragValue::new(&mut self.from).range(1..=self.rows_in_sheet));
                        ui.label("to");
                        ui.add(DragValue::new(&mut self.to).range(1..=self.rows_in_sheet));
                    });
                });
                ui.horizontal(|ui| {
                    ui.label("    ");
                    ui.radio_value(&mut self.row_mode, RowMode::XRange, "X from")
                        .on_hover_text("The first unbroken run of rows whose X value lies in the interval.");
                    ui.add_enabled_ui(self.row_mode == RowMode::XRange, |ui| {
                        ui.add(DragValue::new(&mut self.x_from).speed(0.01));
                        ui.label("to");
                        ui.add(DragValue::new(&mut self.x_to).speed(0.01));
                    });
                });
                ui.weak(
                    "Parameters and statistics go to new sheets; fitted values and residuals go next to the column.",
                );
                if let Some(p) = &self.problem {
                    ui.colored_label(egui::Color32::from_rgb(170, 30, 30), p);
                }
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    run = ui.add_enabled(self.model_problem.is_none(), egui::Button::new("OK")).clicked();
                    cancel = ui.button("Cancel").clicked();
                });
            });
        if column_changed {
            self.column_changed(doc.project());
        }

        if run && let Some(request) = self.request() {
            let y = self.y;
            let result = doc.edit("Fit", |p| -> Result<(FitOutcome, Option<WindowId>), FitRequestError> {
                let outcome = p.fit(&request)?;
                let graph = if outcome.graphs.is_empty() {
                    Some(p.plot_fit(&outcome, y).map_err(|e| FitRequestError::BadOption(e.to_string()))?)
                } else {
                    None
                };
                Ok((outcome, graph))
            });
            match result {
                Ok((outcome, graph)) => return FitDialogOutcome::Done(Box::new(outcome), graph),
                Err(e) => self.problem = Some(e.to_string()),
            }
        }
        if open && !cancel { FitDialogOutcome::Open } else { FitDialogOutcome::Closed }
    }
}

/// Label of the statistical weighting radio button: the weight is 1/|y|, so
/// it works for negative currents too.
const STATISTICAL_LABEL: &str = "1/|y|";

#[cfg(test)]
mod tests {
    use super::*;
    use sheaf_core::ColumnData;

    fn project() -> (Project, WindowId, ColumnId) {
        let mut p = Project::new();
        let book = p.add_workbook("data");
        p.add_column(book, 0, Designation::X, ColumnData::numeric((0..20).map(f64::from).collect())).unwrap();
        let y = p
            .add_column(
                book,
                0,
                Designation::Y,
                ColumnData::numeric((0..20).map(|i| 1.0 + 2.0 * f64::from(i)).collect()),
            )
            .unwrap();
        (p, book, y)
    }

    #[test]
    fn statistical_weighting_label_names_the_absolute_value() {
        assert!(STATISTICAL_LABEL.contains("|y|"));
    }

    #[test]
    fn parameter_table_follows_the_model_and_keeps_typed_values() {
        let (p, book, y) = project();
        let mut d = FitDialog::new(&p, book, Some(y), FitKind::Linear).unwrap();
        let names = |d: &FitDialog| d.params.iter().map(|r| r.name.clone()).collect::<Vec<_>>();
        assert_eq!(names(&d), ["Intercept", "Slope"]);
        d.params[0].fixed = true;
        d.params[0].initial = 0.5;
        d.kind = FitKind::Polynomial;
        d.order = 3;
        d.refresh_params();
        assert_eq!(names(&d), ["Intercept", "B1", "B2", "B3"]);
        assert!(d.params[0].fixed && d.params[0].initial == 0.5, "the intercept setting stays");
        d.kind = FitKind::BuiltIn;
        d.function = "ExpDec1".into();
        d.refresh_params();
        assert_eq!(names(&d), ["y0", "A1", "t1"]);
        d.kind = FitKind::Formula;
        d.formula = "a + b*x".into();
        d.formula_params = "a, b".into();
        d.refresh_params();
        assert_eq!(names(&d), ["a", "b"]);
        d.kind = FitKind::Fdf;
        d.refresh_params();
        assert!(d.model_problem.is_some(), "no file chosen yet");
    }

    #[test]
    fn the_request_carries_only_the_settings_that_were_made() {
        let (p, book, y) = project();
        let mut d = FitDialog::new(&p, book, None, FitKind::BuiltIn).unwrap();
        assert_eq!(d.y, y, "without a column the first Y column is taken");
        d.function = "Gauss".into();
        d.refresh_params();
        let w = d.params.iter_mut().find(|r| r.name == "w").unwrap();
        w.has_lower = true;
        w.lower = 0.1;
        let r = d.request().unwrap();
        assert_eq!(r.params.len(), 1);
        assert_eq!(r.params[0].lower, Some(0.1));
        assert!(r.params[0].initial.is_none());
        assert_eq!(r.confidence_level, 0.95);

        // Switching to a straight line keeps only fixed values.
        let (p, book, y) = project();
        let mut d = FitDialog::new(&p, book, Some(y), FitKind::Formula).unwrap();
        d.formula = "Intercept + Slope*x".into();
        d.formula_params = "Intercept, Slope".into();
        d.refresh_params();
        d.params[0].set_initial = true;
        d.params[0].initial = 7.0;
        d.params[1].has_lower = true;
        d.kind = FitKind::Linear;
        d.refresh_params();
        assert!(d.request().unwrap().params.is_empty());
        d.params[1].fixed = true;
        d.params[1].initial = 2.0;
        let r = d.request().unwrap();
        assert_eq!(r.params.len(), 1);
        assert_eq!((r.params[0].initial, r.params[0].fixed, r.params[0].lower), (Some(2.0), true, None));
    }
}
