//! The Fit window (Analysis > Fit): fit a straight line, a polynomial, a built-in function, a
//! typed formula or an Origin `.FDF` function to one column.
//!
//! The window follows Origin's NLFit (`docs/spec/gui-look.md`, `docs/spec/fitting.md`
//! section 9): tabs Settings / Code / Parameters / Bounds, with the pages Function Selection, Data
//! Selection, Fitted Curves, Find X/Y, Advanced and Output at the left of Settings, and a lower
//! panel (Fit Curve, Residual, Formula, Messages) that can be folded away. 1 Iteration and Fit
//! until converged work on the table of the Parameters tab and keep the window open; Fit runs
//! what is left and writes the results, Done writes them at the values of the table without
//! iterating, Cancel writes nothing. Writing is one undoable step; the results go where
//! `docs/adr/0006-fit-results.md` says, and if no graph shows the column yet a new graph with the
//! data and the fitted curve is made.

use eframe::egui::{self, DragValue, RichText};
use sheaf_core::analysis::RowRange;
use sheaf_core::fitting::{
    BandChoice, FitModel, FitOutcome, FitRequest, FitRequestError, FitWeighting, FunctionInfo, ParamSetting,
    builtin_functions, model_parameters,
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
    bands: BandChoice,
    /// Confidence level of the bands, in percent.
    band_level: f64,
    max_iterations: usize,
    /// The tolerance is 10^-digits (Origin's default 1e-9 is 9 digits).
    tolerance_digits: u32,
    row_mode: RowMode,
    rows_in_sheet: usize,
    from: usize,
    to: usize,
    x_from: f64,
    x_to: f64,
    problem: Option<String>,
    /// The tab along the top and the page of the Settings list that are shown.
    tab: FitTab,
    page: FitPage,
    /// Whether the lower panel is shown, and its tab.
    lower_open: bool,
    lower: LowerTab,
    /// Number of points of the fitted curve, and its X range ("Use Input Data Range" or custom).
    curve_points: usize,
    curve_custom: bool,
    curve_from: f64,
    curve_to: f64,
    /// Fitted Curves page: add the curve to the source graph, and rebuild its legend.
    plot_on_graph: bool,
    update_legend: bool,
    /// How the last trial fit (Fit until converged, 1 Iteration) went.
    message: Option<String>,
    /// Every message of the session, oldest first (the Messages tab).
    log: Vec<String>,
    /// The table at the moment a trial converged; while the table still equals it, the fit is
    /// converged and Done reports it as such.
    converged_at: Option<Vec<ParamRow>>,
    /// What the lower panel draws, with the request it was made for.
    preview: Option<(FitRequest, Preview)>,
    /// Iterations made by the Fit and 1 Iteration buttons since the function was chosen.
    iterations: usize,
    /// The parameter table and iteration count before each Fit / 1 Iteration press, newest
    /// last, for Revert.
    history: Vec<(Vec<ParamRow>, usize)>,
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
            None => sheet
                .columns
                .iter()
                .find(|c| c.designation == Designation::Y && c.data.is_numeric() && !c.fit_result)
                .map(|c| c.id)
                .or_else(|| choices.first().map(|c| c.0))?,
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
            formula: "y0 + A*exp(-x/t)".to_string(),
            formula_params: "y0, A, t".to_string(),
            fdf_path: None,
            fdf_text: None,
            params: Vec::new(),
            model_problem: None,
            weighting: FitWeighting::None,
            has_error_column: false,
            confidence: 95.0,
            bands: BandChoice::None,
            band_level: 95.0,
            max_iterations: 400,
            tolerance_digits: 9,
            row_mode: RowMode::All,
            rows_in_sheet,
            from: 1,
            to: rows_in_sheet,
            x_from: 0.0,
            x_to: 1.0,
            problem: None,
            tab: FitTab::Settings,
            page: FitPage::FunctionSelection,
            lower_open: true,
            lower: LowerTab::FitCurve,
            curve_points: 1000,
            curve_custom: false,
            curve_from: 0.0,
            curve_to: 1.0,
            plot_on_graph: true,
            update_legend: false,
            message: None,
            log: Vec::new(),
            converged_at: None,
            preview: None,

            iterations: 0,
            history: Vec::new(),
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
            if !self.curve_custom {
                (self.curve_from, self.curve_to) = (lo, hi);
            }
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

    /// Whether the model is fitted by iterating (everything but a line and a polynomial).
    fn iterative(&self) -> bool {
        !matches!(self.kind, FitKind::Linear | FitKind::Polynomial)
    }

    /// Runs the fit on a copy of the project, from the values now in the Parameters table, for
    /// at most `max_iterations` iterations (the dialog's own limit when `None`), and puts the
    /// resulting values back into the table. Revert goes back to the table as it was.
    fn say(&mut self, text: String) {
        self.log.push(text.clone());
        self.message = Some(text);
    }

    fn trial(&mut self, project: &Project, max_iterations: Option<usize>) {
        let Some(mut request) = self.request() else { return };
        let one = max_iterations.is_some();
        if let Some(n) = max_iterations {
            request.max_iterations = n;
        }
        self.problem = None;
        self.message = None;
        let mut copy = project.clone();
        let outcome = match copy.fit(&request) {
            Ok(o) => o,
            Err(e) => {
                self.problem = Some(e.to_string());
                return;
            }
        };
        let report = &outcome.report;
        if !self.iterative() {
            self.say(outcome.message);
            return;
        }
        self.history.push((self.params.clone(), self.iterations));
        for q in &report.params {
            if let Some(row) = self.params.iter_mut().find(|r| r.name == q.name)
                && !row.fixed
                && q.value.is_finite()
            {
                row.set_initial = true;
                row.initial = q.value;
            }
        }
        let made = report.stats.iterations.unwrap_or(0);
        self.iterations += made;
        let chi = sheaf_core::format::format_general(report.stats.reduced_chi_sqr, 6);
        let converged = report.status == sheaf_core::fitting::FitStatus::Converged;
        let text = match (converged, one) {
            (true, _) => format!(
                "The fit converged: {} iteration(s) in all, reduced chi-square {chi}. The Parameters page shows the values.",
                self.iterations
            ),
            (false, true) => format!(
                "Iteration {}: reduced chi-square {chi}; not converged yet. Press 1 Iteration or Fit until converged to go on.",
                self.iterations
            ),
            (false, false) => format!(
                "The fit did not converge: it stopped after {made} iteration(s), the limit set on the Advanced page; reduced chi-square {chi}. Fit until converged goes on from these values."
            ),
        };
        self.say(text);
        self.converged_at = converged.then(|| self.params.clone());
    }

    fn converged(&self) -> bool {
        self.converged_at.as_ref() == Some(&self.params)
    }

    /// Test hooks: the built-in function in use, the table, the status line.
    #[cfg(test)]
    pub(crate) fn choose_for_test(&mut self, name: &str) {
        self.function = name.to_string();
        self.refresh_params();
    }

    #[cfg(test)]
    pub(crate) fn rows_for_test(&self) -> Vec<(String, bool, f64)> {
        self.params.iter().map(|p| (p.name.clone(), p.set_initial, p.initial)).collect()
    }

    #[cfg(test)]
    pub(crate) fn message_for_test(&self) -> String {
        self.message.clone().unwrap_or_default()
    }

    /// Puts the Parameters table back as it was before the last Fit or 1 Iteration.
    fn revert(&mut self) {
        if let Some((rows, iterations)) = self.history.pop() {
            self.params = rows;
            self.iterations = iterations;
            self.problem = None;
            self.say("The parameter values are back to what they were before the last iteration.".into());
        }
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
        r.bands = self.bands;
        r.band_level = self.band_level / 100.0;
        r.max_iterations = self.max_iterations;
        r.curve_points = self.curve_points;
        r.curve_range = self.curve_custom.then_some((self.curve_from, self.curve_to));
        r.add_to_graphs = self.plot_on_graph;
        r.update_legend = self.update_legend;
        r.tolerance = 10f64.powi(-(self.tolerance_digits as i32));
        r.rows = match self.row_mode {
            RowMode::All => RowRange::All,
            RowMode::Rows => RowRange::Rows(self.from, self.to),
            RowMode::XRange => RowRange::XBetween(self.x_from, self.x_to),
        };
        Some(r)
    }

    /// The categories of the Category drop-down: the two closed-form fits, Origin's function
    /// categories, then the two ways to bring in a function of one's own.
    fn categories(&self) -> Vec<String> {
        let mut out = vec![CAT_LINEAR.to_string(), CAT_POLY.to_string()];
        for f in &self.functions {
            if !out.contains(&f.category) {
                out.push(f.category.clone());
            }
        }
        out.push(CAT_FORMULA.to_string());
        out.push(CAT_FDF.to_string());
        out
    }

    /// The category the current model belongs to.
    fn category(&self) -> String {
        match self.kind {
            FitKind::Linear => CAT_LINEAR.to_string(),
            FitKind::Polynomial => CAT_POLY.to_string(),
            FitKind::Formula => CAT_FORMULA.to_string(),
            FitKind::Fdf => CAT_FDF.to_string(),
            FitKind::BuiltIn => {
                self.functions.iter().find(|f| f.name == self.function).map(|f| f.category.clone()).unwrap_or_default()
            }
        }
    }

    /// Chooses a category; a built-in category starts on its first function.
    fn set_category(&mut self, category: &str) {
        match category {
            CAT_LINEAR => self.kind = FitKind::Linear,
            CAT_POLY => self.kind = FitKind::Polynomial,
            CAT_FORMULA => self.kind = FitKind::Formula,
            CAT_FDF => self.kind = FitKind::Fdf,
            _ => {
                if let Some(f) = self.functions.iter().find(|f| f.category == category) {
                    self.kind = FitKind::BuiltIn;
                    self.function = f.name.clone();
                }
            }
        }
    }

    /// The name of the function in the Function drop-down and in the title.
    fn function_label(&self) -> String {
        match self.kind {
            FitKind::BuiltIn => self.function.clone(),
            FitKind::Fdf => self
                .fdf_path
                .as_ref()
                .and_then(|p| p.file_stem())
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "(no file)".to_string()),
            k => k.label().to_string(),
        }
    }

    /// The formula as text: what the Description and the Code page show.
    fn formula_text(&self) -> String {
        match self.kind {
            FitKind::Linear => "y = Intercept + Slope*x".to_string(),
            FitKind::Polynomial => "y = Intercept + B1*x + B2*x^2 + ...".to_string(),
            FitKind::BuiltIn => match self.functions.iter().find(|f| f.name == self.function) {
                Some(f) => {
                    f.formula.as_deref().map_or_else(|| "(computed in code)".to_string(), |s| format!("y = {s}"))
                }
                None => String::new(),
            },
            FitKind::Formula => format!("y = {}", self.formula),
            FitKind::Fdf => self.fdf_text.as_deref().map(|_| "(from the .FDF file)".to_string()).unwrap_or_default(),
        }
    }

    /// The top row: Category and Function drop-downs, and the formula. True when the model
    /// changed.
    fn top_ui(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label("Category");
            let current = self.category();
            egui::ComboBox::from_id_salt("fit_category").selected_text(&current).width(190.0).height(420.0).show_ui(
                ui,
                |ui| {
                    for c in self.categories() {
                        if ui.selectable_label(c == current, &c).clicked() && c != current {
                            self.set_category(&c);
                            changed = true;
                        }
                    }
                },
            );
            ui.add_space(12.0);
            ui.label("Function");
            let label = self.function_label();
            match self.kind {
                FitKind::BuiltIn => {
                    let category = self.category();
                    egui::ComboBox::from_id_salt("fit_function")
                        .selected_text(&label)
                        .width(190.0)
                        .height(420.0)
                        .show_ui(ui, |ui| {
                            for f in self.functions.iter().filter(|f| f.category == category) {
                                if ui.selectable_label(f.name == self.function, &f.name).clicked() {
                                    self.function = f.name.clone();
                                    changed = true;
                                }
                            }
                        });
                }
                _ => {
                    // The only function of its category: shown, not chosen.
                    ui.add_enabled_ui(false, |ui| {
                        egui::ComboBox::from_id_salt("fit_function_fixed")
                            .selected_text(&label)
                            .width(190.0)
                            .show_ui(ui, |_| {});
                    });
                }
            }
        });
        changed
    }

    /// Function Selection: what belongs to the chosen function (order, file) and its description.
    fn function_selection_ui(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = self.top_ui(ui);
        match self.kind {
            FitKind::Polynomial => {
                ui.horizontal(|ui| {
                    ui.label("Order");
                    changed |= ui.add(DragValue::new(&mut self.order).range(1..=9)).changed();
                });
            }
            FitKind::Fdf => {
                ui.horizontal(|ui| {
                    ui.label("File Name(.FDF)");
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
            FitKind::Formula => {
                ui.weak("Write the formula on the Code page.");
            }
            _ => {}
        }
        ui.add_enabled_ui(false, |ui| {
            ui.horizontal(|ui| {
                ui.label("Iteration Algorithm");
                egui::ComboBox::from_id_salt("fit_algorithm").selected_text("Levenberg Marquardt").show_ui(ui, |_| {});
            });
        });
        ui.horizontal_top(|ui| {
            ui.label("Description");
            ui.add(egui::Label::new(RichText::new(self.formula_text()).monospace().small()).wrap());
        });
        changed
    }

    /// Code: the formula. A formula of one's own is typed here; for the others it is read-only.
    fn code_ui(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;
        if self.kind == FitKind::Formula {
            ui.horizontal(|ui| {
                ui.label("y =");
                changed |= ui
                    .add(
                        egui::TextEdit::singleline(&mut self.formula)
                            .font(egui::TextStyle::Monospace)
                            .desired_width(380.0),
                    )
                    .changed();
            });
            ui.horizontal(|ui| {
                ui.label("Parameters");
                changed |= ui
                    .add(
                        egui::TextEdit::singleline(&mut self.formula_params).desired_width(250.0).hint_text("y0, A, t"),
                    )
                    .changed();
            });
            ui.weak("The variable is x. Functions: exp, ln, log (base 10), sqrt, sin, cos, erf, ... and ^ for powers.");
        } else {
            ui.add(egui::Label::new(RichText::new(self.formula_text()).monospace()).wrap());
            ui.weak("Choose the category \"User Defined\" to type a formula of your own.");
        }
        changed
    }

    fn data_selection_ui(&mut self, ui: &mut egui::Ui) -> bool {
        let mut column_changed = false;
        ui.horizontal(|ui| {
            ui.label("Weights");
            ui.radio_value(&mut self.weighting, FitWeighting::None, "None");
            ui.add_enabled_ui(self.has_error_column, |ui| {
                ui.radio_value(&mut self.weighting, FitWeighting::Instrumental, "Error bars (1/err²)")
                    .on_disabled_hover_text("The column has no Y-error column next to it.");
            });
            ui.radio_value(&mut self.weighting, FitWeighting::Statistical, STATISTICAL_LABEL);
        });
        ui.add_space(4.0);
        ui.strong("Input Data");
        ui.horizontal(|ui| {
            ui.label("Column");
            let label = self.choices.iter().find(|(c, _)| *c == self.y).map(|(_, l)| l.clone()).unwrap_or_default();
            egui::ComboBox::from_id_salt("fit_column").selected_text(label).width(300.0).show_ui(ui, |ui| {
                for (id, l) in &self.choices {
                    column_changed |= ui.selectable_value(&mut self.y, *id, l).changed();
                }
            });
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
        column_changed
    }

    fn fitted_curves_ui(&mut self, ui: &mut egui::Ui) {
        ui.strong("Fitted Curves Plot");
        ui.horizontal(|ui| {
            ui.label("Bands");
            ui.radio_value(&mut self.bands, BandChoice::None, "No bands");
            ui.radio_value(&mut self.bands, BandChoice::Confidence, "Confidence bands");
            ui.radio_value(&mut self.bands, BandChoice::Prediction, "Prediction bands");
            ui.radio_value(&mut self.bands, BandChoice::Both, "Confidence and prediction");
        });
        ui.add_enabled_ui(self.bands != BandChoice::None, |ui| {
            ui.horizontal(|ui| {
                ui.label("Band confidence level");
                ui.add(DragValue::new(&mut self.band_level).range(50.0..=99.99).speed(0.1).suffix(" %"));
            });
        });
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label("Number of Points");
            ui.add(DragValue::new(&mut self.curve_points).range(2..=100_000).speed(10.0));
        });
        ui.horizontal(|ui| {
            ui.label("X Data Type");
            if ui.radio(!self.curve_custom, "Use Input Data Range").clicked() {
                self.curve_custom = false;
            }
            if ui.radio(self.curve_custom, "Custom").clicked() {
                self.curve_custom = true;
            }
        });
        ui.add_enabled_ui(self.curve_custom, |ui| {
            ui.horizontal(|ui| {
                ui.label("Min");
                ui.add(DragValue::new(&mut self.curve_from).speed(0.01));
                ui.label("Max");
                ui.add(DragValue::new(&mut self.curve_to).speed(0.01));
            });
        });
        ui.checkbox(&mut self.plot_on_graph, "Plot on Source Graph");
        ui.checkbox(&mut self.update_legend, "Update Legend on Source Graph");
        ui.weak(
            "Fitted values and residuals go next to the column; the curve is added to a graph that shows it.              Origin's other options here (plot type, stacking with the residual plot, preview on the graph) are not offered.",
        );
    }

    fn find_xy_ui(&mut self, ui: &mut egui::Ui) {
        ui.weak("Find X from Y and Find Y from X are not offered yet.");
    }

    fn output_ui(&mut self, ui: &mut egui::Ui) {
        ui.strong("Output");
        ui.label("The parameters, the statistics and the fitted curve go to new sheets of the workbook that holds the data; the fitted values and residuals go next to the fitted column.");
        ui.label("The whole result is one undoable step.");
        ui.weak("Origin's choices of names, tables and the results sheet arrangement are not offered.");
    }

    fn advanced_ui(&mut self, ui: &mut egui::Ui) {
        ui.strong("Fit Control");
        ui.horizontal(|ui| {
            ui.label("Maximum iterations");
            ui.add(DragValue::new(&mut self.max_iterations).range(1..=100_000).speed(1.0));
        });
        ui.horizontal(|ui| {
            ui.label("Tolerance 1e-");
            ui.add(DragValue::new(&mut self.tolerance_digits).range(3..=15))
                .on_hover_text("Nonlinear fits only; a straight line or polynomial is solved directly.");
        });
        ui.add_space(4.0);
        ui.strong("Quantities");
        ui.horizontal(|ui| {
            ui.label("Confidence level");
            ui.add(DragValue::new(&mut self.confidence).range(50.0..=99.99).speed(0.1).suffix(" %"));
        });
        ui.weak("Parameters and statistics go to new sheets.");
    }

    /// The Parameters page: starting or fixed values.
    fn parameters_ui(&mut self, ui: &mut egui::Ui) {
        if let Some(p) = &self.model_problem {
            ui.colored_label(egui::Color32::from_rgb(170, 30, 30), p);
            return;
        }
        if self.params.is_empty() {
            return;
        }
        // A linear fit can fix the intercept and the slope; a polynomial only the intercept.
        let closed_form = matches!(self.kind, FitKind::Linear | FitKind::Polynomial);
        egui::Grid::new("fit_params").num_columns(3).spacing([14.0, 4.0]).striped(true).show(ui, |ui| {
            ui.strong("Meaning");
            ui.strong(if closed_form { "" } else { "Value" });
            ui.strong("Fixed");
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
                ui.end_row();
            }
        });
    }

    /// The Bounds page: lower and upper limits of nonlinear fits.
    fn bounds_ui(&mut self, ui: &mut egui::Ui) {
        if let Some(p) = &self.model_problem {
            ui.colored_label(egui::Color32::from_rgb(170, 30, 30), p);
            return;
        }
        if matches!(self.kind, FitKind::Linear | FitKind::Polynomial) {
            ui.add_enabled_ui(false, |ui| ui.label("A straight line or polynomial is solved directly: no bounds."));
            return;
        }
        egui::Grid::new("fit_bounds").num_columns(3).spacing([14.0, 4.0]).striped(true).show(ui, |ui| {
            ui.strong("Param");
            ui.strong("Lower Bounds");
            ui.strong("Upper Bounds");
            ui.end_row();
            for p in self.params.iter_mut() {
                ui.label(&p.name);
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
                ui.end_row();
            }
        });
    }

    /// Makes the lower panel's drawing for the table as it is now, unless it is up to date: the
    /// fit run with no iterations on a copy of the project, so the curve is the model at the
    /// values of the Parameters table.
    fn refresh_preview(&mut self, project: &Project) {
        let key = self.request();
        if self.preview.as_ref().map(|(k, _)| k) == key.as_ref() {
            return;
        }
        let Some(key) = key else {
            self.preview = None;
            return;
        };
        let mut r = key.clone();
        r.max_iterations = 0;
        r.curve_points = r.curve_points.min(400);
        r.bands = BandChoice::None;
        r.add_to_graphs = false;
        let mut copy = project.clone();
        let mut preview = Preview::default();
        match copy.fit(&r) {
            Err(e) => preview.error = Some(e.to_string()),
            Ok(o) => {
                if let Some((_, sheet, col)) = copy.locate(self.y) {
                    let index = sheet.column_index(self.y).expect("located");
                    let ys = col.numeric().unwrap_or(&[]);
                    let xs: Vec<f64> = match sheet.x_for(index).and_then(|c| c.numeric()) {
                        Some(x) => (0..ys.len()).map(|i| x.get(i).copied().unwrap_or(f64::NAN)).collect(),
                        None => (1..=ys.len()).map(|i| i as f64).collect(),
                    };
                    let finite = |p: &(f64, f64)| p.0.is_finite() && p.1.is_finite();
                    preview.data = xs.iter().copied().zip(ys.iter().copied()).filter(finite).collect();
                    if let Some(res) = copy.numeric(o.residual_column) {
                        preview.residual = xs.iter().copied().zip(res.iter().copied()).filter(finite).collect();
                    }
                }
                if let (Some(cx), Some(cy)) = (copy.numeric(o.curve_x), copy.numeric(o.curve_y)) {
                    preview.curve = cx
                        .iter()
                        .copied()
                        .zip(cy.iter().copied())
                        .filter(|p| p.0.is_finite() && p.1.is_finite())
                        .collect();
                }
            }
        }
        self.preview = Some((key, preview));
    }

    /// The lower panel: Fit Curve, Residual, Formula and Messages.
    fn lower_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            for t in LowerTab::ALL {
                ui.selectable_value(&mut self.lower, t, t.label());
            }
        });
        let size = egui::vec2(ui.available_width(), 150.0);
        let blue = egui::Color32::from_rgb(30, 60, 160);
        let red = egui::Color32::from_rgb(190, 30, 30);
        match self.lower {
            LowerTab::FitCurve | LowerTab::Residual => {
                let residual = self.lower == LowerTab::Residual;
                match self.preview.as_ref().map(|(_, p)| p) {
                    None => {
                        ui.weak("No Preview");
                    }
                    Some(p) if p.error.is_some() => {
                        ui.colored_label(egui::Color32::from_rgb(170, 30, 30), p.error.clone().unwrap_or_default());
                    }
                    Some(p) if residual => {
                        draw_xy(ui, size, &[(&p.residual, Series::Dots, blue)], true);
                        ui.weak(format!(
                            "Residuals at the values of the Parameters table: {} points",
                            p.residual.len()
                        ));
                    }
                    Some(p) => {
                        draw_xy(ui, size, &[(&p.data, Series::Dots, blue), (&p.curve, Series::Line, red)], false);
                        ui.weak(format!("Data: {} points; fit curve: {} points", p.data.len(), p.curve.len()));
                    }
                }
            }
            LowerTab::Formula => {
                ui.add(egui::Label::new(RichText::new(self.formula_text()).monospace()).wrap());
                let names: Vec<&str> = self.params.iter().map(|p| p.name.as_str()).collect();
                if !names.is_empty() {
                    ui.label(format!("Parameters: {}", names.join(", ")));
                }
            }
            LowerTab::Messages => {
                egui::ScrollArea::vertical()
                    .id_salt("fit_messages")
                    .max_height(size.y)
                    .auto_shrink([false, false])
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        if self.log.is_empty() {
                            ui.weak("Nothing yet: 1 Iteration and Fit until converged report here.");
                        }
                        for m in &self.log {
                            ui.label(m);
                        }
                    });
            }
        }
    }

    /// Test hook: what the Fit Curve tab draws (points of the data, points of the curve).
    #[cfg(test)]
    pub(crate) fn preview_for_test(&self) -> Option<(usize, usize, Option<String>)> {
        self.preview.as_ref().map(|(_, p)| (p.data.len(), p.curve.len(), p.error.clone()))
    }

    #[cfg(test)]
    pub(crate) fn log_for_test(&self) -> Vec<String> {
        self.log.clone()
    }

    /// Draws the window. Fit until converged and 1 Iteration try the fit on a copy of the project
    /// and keep the window open; Fit finishes the iterations and writes the results; Done writes
    /// them at the values of the table; Cancel writes nothing.
    pub fn show(&mut self, ctx: &egui::Context, doc: &mut Document) -> FitDialogOutcome {
        if doc.project().column(self.y).is_none() || doc.project().workbook(self.book).is_none() {
            return FitDialogOutcome::Closed;
        }
        if self.lower_open {
            self.refresh_preview(doc.project());
        }
        let mut open = true;
        let mut fit_and_write = false;
        let mut done = false;
        let mut trial = false;
        let mut one_iteration = false;
        let mut revert = false;
        let mut cancel = false;
        let mut column_changed = false;
        let title = format!("NLFit ({})", self.function_label());
        egui::Window::new(title)
            .id(egui::Id::new("fit"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(760.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    for t in FitTab::ALL {
                        ui.selectable_value(&mut self.tab, t, t.label());
                    }
                });
                ui.separator();
                let mut model_changed = false;
                ui.allocate_ui_with_layout(
                    egui::vec2(740.0, 250.0),
                    egui::Layout::left_to_right(egui::Align::Min),
                    |ui| {
                        if self.tab == FitTab::Settings {
                            ui.vertical(|ui| {
                                ui.set_width(150.0);
                                for page in FitPage::ALL {
                                    if ui.selectable_label(self.page == page, page.label()).clicked() {
                                        self.page = page;
                                    }
                                }
                            });
                            ui.separator();
                        }
                        ui.vertical(|ui| {
                            ui.set_width(if self.tab == FitTab::Settings { 560.0 } else { 730.0 });
                            egui::ScrollArea::vertical()
                                .id_salt("fit_page")
                                .max_height(246.0)
                                .auto_shrink([false, false])
                                .show(ui, |ui| match self.tab {
                                    FitTab::Settings => match self.page {
                                        FitPage::FunctionSelection => model_changed |= self.function_selection_ui(ui),
                                        FitPage::DataSelection => column_changed |= self.data_selection_ui(ui),
                                        FitPage::FittedCurves => self.fitted_curves_ui(ui),
                                        FitPage::FindXY => self.find_xy_ui(ui),
                                        FitPage::Advanced => self.advanced_ui(ui),
                                        FitPage::Output => self.output_ui(ui),
                                    },
                                    FitTab::Code => model_changed |= self.code_ui(ui),
                                    FitTab::Parameters => self.parameters_ui(ui),
                                    FitTab::Bounds => self.bounds_ui(ui),
                                });
                        });
                    },
                );
                if model_changed {
                    self.refresh_params();
                    self.message = None;
                    self.history.clear();
                    self.iterations = 0;
                    self.converged_at = None;
                }
                ui.separator();
                if let Some(p) = self.model_problem.as_ref().or(self.problem.as_ref()) {
                    ui.colored_label(egui::Color32::from_rgb(170, 30, 30), p);
                } else if let Some(m) = &self.message {
                    ui.colored_label(egui::Color32::from_rgb(20, 70, 170), m);
                } else {
                    ui.weak(
                        "1 Iteration and Fit until converged try the fit and keep this window; Fit and Done write the results.",
                    );
                }
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    let ok = self.model_problem.is_none();
                    let can_step = ok && self.iterative();
                    one_iteration = ui.add_enabled(can_step, egui::Button::new("1 Iteration")).clicked();
                    trial = ui.add_enabled(ok, egui::Button::new("Fit until converged")).clicked();
                    revert = ui.add_enabled(!self.history.is_empty(), egui::Button::new("Revert")).clicked();
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        cancel = ui.button("Cancel").clicked();
                        done = ui.add_enabled(ok, egui::Button::new("Done")).clicked();
                        fit_and_write = ui.add_enabled(ok, egui::Button::new("Fit")).clicked();
                        let label = if self.lower_open { "Hide lower panel" } else { "Show lower panel" };
                        if ui.small_button(label).clicked() {
                            self.lower_open = !self.lower_open;
                        }
                    });
                });
                if self.lower_open {
                    ui.separator();
                    self.lower_ui(ui);
                }
            });
        if column_changed {
            self.column_changed(doc.project());
        }

        if revert {
            self.revert();
        }
        if one_iteration {
            self.trial(doc.project(), Some(1));
        } else if trial {
            self.trial(doc.project(), None);
        }
        if (fit_and_write || done)
            && let Some(mut request) = self.request()
        {
            // Done makes no iterations of its own: it reports the values of the table. Once the
            // table holds a converged fit, one more pass changes nothing and gives the status.
            if done && self.iterative() && !self.converged() {
                request.max_iterations = 0;
            }
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

/// What the lower panel draws.
#[derive(Debug, Clone, Default)]
struct Preview {
    data: Vec<(f64, f64)>,
    curve: Vec<(f64, f64)>,
    residual: Vec<(f64, f64)>,
    error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Series {
    Dots,
    Line,
}

/// A small plot drawn with the painter: the series scaled to a frame with the end values of both
/// axes written at the edges, and a line at zero if asked.
fn draw_xy(ui: &mut egui::Ui, size: egui::Vec2, series: &[(&[(f64, f64)], Series, egui::Color32)], zero_line: bool) {
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, ui.visuals().extreme_bg_color);
    painter.rect_stroke(rect, 0.0, ui.visuals().widgets.noninteractive.bg_stroke, egui::StrokeKind::Inside);
    let (mut x0, mut x1, mut y0, mut y1) = (f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY);
    for (x, y) in series.iter().flat_map(|(p, _, _)| p.iter()) {
        (x0, x1, y0, y1) = (x0.min(*x), x1.max(*x), y0.min(*y), y1.max(*y));
    }
    if zero_line {
        (y0, y1) = (y0.min(0.0), y1.max(0.0));
    }
    if !(x0.is_finite() && y0.is_finite()) {
        return;
    }
    let (dx, dy) = ((x1 - x0).max(f64::MIN_POSITIVE), (y1 - y0).max(f64::MIN_POSITIVE));
    let inner = rect.shrink2(egui::vec2(46.0, 14.0));
    let map = |(x, y): (f64, f64)| {
        egui::pos2(
            inner.left() + ((x - x0) / dx) as f32 * inner.width(),
            inner.bottom() - ((y - y0) / dy) as f32 * inner.height(),
        )
    };
    let text = ui.visuals().text_color();
    let font = egui::FontId::proportional(10.0);
    let g = |v: f64| sheaf_core::format::format_general(v, 4);
    painter.text(inner.left_bottom() + egui::vec2(0.0, 2.0), egui::Align2::LEFT_TOP, g(x0), font.clone(), text);
    painter.text(inner.right_bottom() + egui::vec2(0.0, 2.0), egui::Align2::RIGHT_TOP, g(x1), font.clone(), text);
    painter.text(inner.left_bottom() + egui::vec2(-3.0, 0.0), egui::Align2::RIGHT_BOTTOM, g(y0), font.clone(), text);
    painter.text(inner.left_top() + egui::vec2(-3.0, 0.0), egui::Align2::RIGHT_TOP, g(y1), font, text);
    if zero_line {
        painter
            .line_segment([map((x0, 0.0)), map((x1, 0.0))], egui::Stroke::new(1.0_f32, ui.visuals().weak_text_color()));
    }
    for (points, kind, color) in series {
        match kind {
            Series::Dots => {
                for p in points.iter() {
                    painter.circle_filled(map(*p), 2.0, *color);
                }
            }
            Series::Line => {
                let line: Vec<egui::Pos2> = points.iter().map(|p| map(*p)).collect();
                painter.add(egui::Shape::line(line, egui::Stroke::new(1.5_f32, *color)));
            }
        }
    }
}

const CAT_LINEAR: &str = "Linear Fit";
const CAT_POLY: &str = "Polynomial Fit";
const CAT_FORMULA: &str = "User Defined";
const CAT_FDF: &str = "Origin .FDF File";

/// The tabs along the top of the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FitTab {
    Settings,
    Code,
    Parameters,
    Bounds,
}

impl FitTab {
    const ALL: [FitTab; 4] = [FitTab::Settings, FitTab::Code, FitTab::Parameters, FitTab::Bounds];

    fn label(self) -> &'static str {
        match self {
            FitTab::Settings => "Settings",
            FitTab::Code => "Code",
            FitTab::Parameters => "Parameters",
            FitTab::Bounds => "Bounds",
        }
    }
}

/// The pages of the list at the left of the Settings tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FitPage {
    FunctionSelection,
    DataSelection,
    FittedCurves,
    FindXY,
    Advanced,
    Output,
}

impl FitPage {
    const ALL: [FitPage; 6] = [
        FitPage::FunctionSelection,
        FitPage::DataSelection,
        FitPage::FittedCurves,
        FitPage::FindXY,
        FitPage::Advanced,
        FitPage::Output,
    ];

    fn label(self) -> &'static str {
        match self {
            FitPage::FunctionSelection => "Function Selection",
            FitPage::DataSelection => "Data Selection",
            FitPage::FittedCurves => "Fitted Curves",
            FitPage::FindXY => "Find X/Y",
            FitPage::Advanced => "Advanced",
            FitPage::Output => "Output",
        }
    }
}

/// The tabs of the lower panel (Origin also has Sample Curve, Function File and Hints).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LowerTab {
    FitCurve,
    Residual,
    Formula,
    Messages,
}

impl LowerTab {
    const ALL: [LowerTab; 4] = [LowerTab::FitCurve, LowerTab::Residual, LowerTab::Formula, LowerTab::Messages];

    fn label(self) -> &'static str {
        match self {
            LowerTab::FitCurve => "Fit Curve",
            LowerTab::Residual => "Residual",
            LowerTab::Formula => "Formula",
            LowerTab::Messages => "Messages",
        }
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
        assert_eq!((r.bands, r.band_level, r.max_iterations, r.tolerance), (BandChoice::None, 0.95, 400, 1e-9));
        d.bands = BandChoice::Both;
        d.band_level = 90.0;
        d.max_iterations = 50;
        d.tolerance_digits = 12;
        let r = d.request().unwrap();
        assert_eq!((r.bands, r.band_level, r.max_iterations, r.tolerance), (BandChoice::Both, 0.9, 50, 1e-12));

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

    #[test]
    fn the_fitted_curves_page_reaches_the_request() {
        let (p, book, y) = project();
        let mut d = FitDialog::new(&p, book, Some(y), FitKind::Linear).unwrap();
        let r = d.request().unwrap();
        assert_eq!((r.curve_points, r.curve_range, r.add_to_graphs, r.update_legend), (1000, None, true, false));
        d.curve_points = 50;
        d.curve_custom = true;
        d.curve_from = -2.0;
        d.curve_to = 30.0;
        d.plot_on_graph = false;
        d.update_legend = true;
        let r = d.request().unwrap();
        assert_eq!(
            (r.curve_points, r.curve_range, r.add_to_graphs, r.update_legend),
            (50, Some((-2.0, 30.0)), false, true)
        );
    }
}
