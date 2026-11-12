//! Curve fitting applied to worksheet columns, and where the results go
//! (`docs/adr/0006-fit-results.md`).
//!
//! The numerics are in the `sheaf-fit` crate. This module connects them to
//! the project: it gathers the X, Y and Y-error values of one column over
//! the chosen rows, hands them to a fit engine, and places the resulting
//! [`FitReport`] in the workbook:
//!
//! - a parameter sheet (`FitLinear1`, `FitPolynomial1` or `FitNL1`) with one
//!   row per parameter: value, standard error, t-value, Prob>|t|, confidence
//!   limits, dependency, and whether the parameter was fixed;
//! - a statistics sheet with the same name plus `Stats`: model, equation,
//!   goodness of fit and the ANOVA table as name and value rows;
//! - the fitted values and the residuals as two new Y columns directly after
//!   the source column (missing outside the fitted rows);
//! - a sheet `FitCurve1` with the model evaluated on equally spaced X values
//!   over the fitted X range, for drawing;
//! - the fitted curve added as a line to every graph layer that plots the
//!   source column.
//!
//! Nothing changes in the project when the fit fails.

use crate::analysis::{AnalysisError, RowRange, column_label, free_sheet_name, resolve_rows};
use crate::color::Color;
use crate::column::{Column, ColumnData, Designation};
use crate::graph::{LineStyle, Plot, PlotKind};
use crate::ids::{ColumnId, WindowId};
use crate::project::Project;
use crate::worksheet::Worksheet;
use sheaf_fit::{FitError, FitReport, FitStatus};

/// Which kind of model is fitted. It decides the name of the report sheet
/// and which engine of `sheaf-fit` runs.
#[derive(Debug, Clone, PartialEq)]
pub enum FitModel {
    /// A straight line, parameters `Intercept` and `Slope` (Origin's "Linear Fit").
    Linear,
    /// A polynomial of the given order, parameters `Intercept`, `B1`, `B2`, ...
    Polynomial { order: usize },
    /// A function of the built-in library, by name (for example `Gauss`).
    BuiltIn(String),
    /// A formula typed by the user, for example `y0 + A*exp(-x/t)`, with its parameter names.
    Formula { formula: String, params: Vec<String> },
    /// The text of an Origin `.FDF` fit-definition file.
    Fdf(String),
}

impl FitModel {
    /// Prefix of the report sheet's name, as Origin names its report sheets.
    pub fn sheet_prefix(&self) -> &'static str {
        match self {
            FitModel::Linear => "FitLinear",
            FitModel::Polynomial { .. } => "FitPolynomial",
            _ => "FitNL",
        }
    }
}

/// Starting value, bounds and fixed flag of one parameter, by name.
/// Settings for names the model does not have are an error.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ParamSetting {
    pub name: String,
    /// Starting value; `None` keeps the model's own starting value (or initial guess).
    pub initial: Option<f64>,
    /// Hold the parameter at `initial` instead of fitting it.
    pub fixed: bool,
    pub lower: Option<f64>,
    pub upper: Option<f64>,
}

/// How points are weighted (Origin's names in brackets).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FitWeighting {
    /// Every point counts the same ("No Weighting").
    #[default]
    None,
    /// `w = 1 / err^2` from the Y-error column ("Instrumental").
    Instrumental,
    /// `w = 1 / abs(y)` ("Statistical").
    Statistical,
}

/// One fit: a Y column, its rows and the model.
#[derive(Debug, Clone, PartialEq)]
pub struct FitRequest {
    /// The column to fit. Its X column follows from the designations of the sheet.
    pub y: ColumnId,
    /// The Y-error column for instrumental weighting; `None` uses the error column that belongs
    /// to `y` in the sheet, if there is one.
    pub y_error: Option<ColumnId>,
    pub rows: RowRange,
    pub model: FitModel,
    pub params: Vec<ParamSetting>,
    pub weighting: FitWeighting,
    /// Confidence level of the parameter limits, for example 0.95.
    pub confidence_level: f64,
    /// Number of points of the fitted curve (ADR 0006: 1000).
    pub curve_points: usize,
    /// Add the fitted curve to the graphs that show the source column.
    pub add_to_graphs: bool,
}

impl FitRequest {
    /// A request with Origin's defaults: all rows, no weighting, 95 % confidence, a curve of
    /// 1000 points that is added to the graphs of the data.
    pub fn new(y: ColumnId, model: FitModel) -> Self {
        FitRequest {
            y,
            y_error: None,
            rows: RowRange::All,
            model,
            params: Vec::new(),
            weighting: FitWeighting::None,
            confidence_level: 0.95,
            curve_points: 1000,
            add_to_graphs: true,
        }
    }
}

/// The values a fit engine receives, already restricted to the chosen rows. Missing values are
/// left in place (as NaN); the engine drops those points and reports fitted values and residuals
/// in the same order as these slices.
#[derive(Debug, Clone, PartialEq)]
pub struct FitData {
    pub x: Vec<f64>,
    pub y: Vec<f64>,
    /// Only present for instrumental weighting.
    pub y_error: Option<Vec<f64>>,
    pub weighting: FitWeighting,
    pub confidence_level: f64,
}

/// What a fit engine returns: the report and the fitted model as a function of x, for the curve.
pub struct FittedModel {
    pub report: FitReport,
    pub curve: Box<dyn Fn(f64) -> f64>,
    /// Quantities computed from the fitted parameters, such as `FWHM` of a peak or the
    /// half-life of a decay, as (name, value).
    pub derived: Vec<(String, f64)>,
}

/// What [`Project::fit_with`] did.
#[derive(Debug, Clone, PartialEq)]
pub struct FitOutcome {
    pub report: FitReport,
    pub book: WindowId,
    /// Index of the sheet that holds the source column.
    pub sheet: usize,
    /// Index of the parameter sheet; the statistics sheet follows it, then the curve sheet.
    pub param_sheet: usize,
    pub stats_sheet: usize,
    pub curve_sheet: usize,
    pub fitted_column: ColumnId,
    pub residual_column: ColumnId,
    pub curve_x: ColumnId,
    pub curve_y: ColumnId,
    /// Graphs to which the fitted curve was added.
    pub graphs: Vec<WindowId>,
    /// Derived quantities (see [`FittedModel::derived`]); also on the statistics sheet.
    pub derived: Vec<(String, f64)>,
    /// One sentence for a status line.
    pub message: String,
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum FitRequestError {
    #[error(transparent)]
    Column(#[from] AnalysisError),
    #[error("instrumental weighting needs a Y-error column for {0}; set one column's designation to Y error")]
    NoErrorColumn(String),
    #[error("column {0} holds text and cannot be used as Y errors")]
    ErrorColumnNotNumeric(String),
    #[error("the confidence level must be between 0 and 1 (exclusive), not {0}")]
    BadConfidence(f64),
    #[error("{0}")]
    BadOption(String),
    #[error(transparent)]
    Fit(#[from] FitError),
}

/// The colors tried for a fitted curve, in order; the first one that is not close to a color
/// already used in the layer is taken.
const CURVE_COLORS: [Color; 5] = [
    Color::rgb(220, 30, 30),
    Color::rgb(30, 80, 220),
    Color::rgb(0, 150, 70),
    Color::rgb(235, 140, 0),
    Color::rgb(0, 0, 0),
];

fn color_distance(a: Color, b: Color) -> i32 {
    let d = |p: u8, q: u8| (i32::from(p) - i32::from(q)).abs();
    d(a.r, b.r) + d(a.g, b.g) + d(a.b, b.b)
}

/// Equally spaced values from `lo` to `hi`, both included.
fn linspace(lo: f64, hi: f64, n: usize) -> Vec<f64> {
    match n {
        0 => Vec::new(),
        1 => vec![lo],
        _ => (0..n).map(|i| if i + 1 == n { hi } else { lo + (hi - lo) * i as f64 / (n - 1) as f64 }).collect(),
    }
}

fn status_text(s: &FitStatus) -> String {
    match s {
        FitStatus::Converged => "Converged".to_string(),
        FitStatus::MaxIterations => "Stopped at the iteration limit".to_string(),
        FitStatus::Exact => "Exact (closed form)".to_string(),
        FitStatus::Other(t) => t.clone(),
    }
}

/// A new column for a result sheet.
fn sheet_column(
    project: &mut Project,
    ws: &mut Worksheet,
    designation: Designation,
    long_name: &str,
    units: &str,
    data: ColumnData,
) -> ColumnId {
    let id = project.new_column_id();
    let name = ws.next_short_name();
    ws.columns.push(Column::new(id, name, designation, data).with_labels(long_name, units, ""));
    id
}

/// Rows of the statistics sheet: name, number, text.
fn stats_rows(r: &FitReport, derived: &[(String, f64)], source: &str, rows_note: &str) -> Vec<(String, f64, String)> {
    let nan = f64::NAN;
    let t = |n: &str, s: &str| (n.to_string(), nan, s.to_string());
    let v = |n: &str, x: f64| (n.to_string(), x, String::new());
    let mut out = vec![
        t("Model", &r.model),
        t("Equation", &r.equation),
        t("Data", source),
        t("Rows", rows_note),
        v("Number of points", r.stats.n_points as f64),
        v("Degrees of freedom", r.stats.dof as f64),
        v("Residual sum of squares", r.stats.rss),
        v("Reduced chi-square", r.stats.reduced_chi_sqr),
        v("R-square (COD)", r.stats.r_squared),
        v("Adj. R-square", r.stats.adj_r_squared),
    ];
    if let Some(p) = r.stats.pearson_r {
        out.push(v("Pearson's r", p));
    }
    if let Some(i) = r.stats.iterations {
        out.push(v("Iterations", i as f64));
    }
    out.push(t("Status", &status_text(&r.status)));
    out.push(v("Confidence level", r.confidence_level));
    if let Some(a) = &r.anova {
        out.extend([
            v("ANOVA model DF", a.model_df as f64),
            v("ANOVA model sum of squares", a.model_ss),
            v("ANOVA model mean square", a.model_ms),
            v("ANOVA error DF", a.error_df as f64),
            v("ANOVA error sum of squares", a.error_ss),
            v("ANOVA error mean square", a.error_ms),
            v("ANOVA corrected total DF", a.total_df as f64),
            v("ANOVA corrected total sum of squares", a.total_ss),
        ]);
        if a.uncorrected_total_ss.is_finite() {
            out.push(v("ANOVA uncorrected total sum of squares", a.uncorrected_total_ss));
        }
        out.extend([v("F value", a.f_value), v("Prob>F", a.prob_f)]);
    }
    for (name, value) in derived {
        out.push((name.clone(), *value, "derived from the fitted parameters".to_string()));
    }
    for w in &r.warnings {
        out.push(t("Warning", w));
    }
    out
}

impl Project {
    /// Fits one column with the given engine and places the results (see the module text).
    ///
    /// `engine` receives the X, Y and Y-error values of the chosen rows and returns the report
    /// with the fitted model. Callers normally use [`Project::fit`], which picks the engine of
    /// `sheaf-fit` from the request's model; tests and special models pass their own.
    pub fn fit_with(
        &mut self,
        request: &FitRequest,
        engine: impl FnOnce(&FitData) -> Result<FittedModel, FitError>,
    ) -> Result<FitOutcome, FitRequestError> {
        if !(request.confidence_level > 0.0 && request.confidence_level < 1.0) {
            return Err(FitRequestError::BadConfidence(request.confidence_level));
        }
        if request.curve_points < 2 || request.curve_points > 1_000_000 {
            return Err(FitRequestError::BadOption(format!(
                "the fitted curve needs between 2 and 1000000 points, not {}",
                request.curve_points
            )));
        }

        // ----- gather the input (no changes yet) ----------------------------------------
        let id = request.y;
        let (book, sheet, column) = self.locate(id).ok_or(AnalysisError::NoSuchColumn(id))?;
        let book_id = book.id;
        let sheet_index = book.sheets.iter().position(|s| s.column(id).is_some()).expect("located here");
        let index = sheet.column_index(id).expect("located here");
        let y_all = column.numeric().ok_or_else(|| AnalysisError::NotNumeric(column.short_name.clone()))?;
        if y_all.is_empty() {
            return Err(AnalysisError::Empty(column.short_name.clone()).into());
        }
        let x_col = sheet.x_for(index).filter(|x| x.id != id && x.data.is_numeric());
        let x_all: Vec<f64> = match x_col.and_then(Column::numeric) {
            Some(v) => (0..y_all.len()).map(|i| v.get(i).copied().unwrap_or(f64::NAN)).collect(),
            None => (1..=y_all.len()).map(|i| i as f64).collect(),
        };
        let (first, last) = resolve_rows(request.rows, &column.short_name, &x_all, y_all.len())?;
        let y_error = match request.weighting {
            FitWeighting::Instrumental => {
                let err_col = match request.y_error {
                    Some(e) => Some(self.column(e).ok_or(AnalysisError::NoSuchColumn(e))?),
                    None => sheet.y_error_for(index),
                }
                .ok_or_else(|| FitRequestError::NoErrorColumn(column.short_name.clone()))?;
                let e = err_col
                    .numeric()
                    .ok_or_else(|| FitRequestError::ErrorColumnNotNumeric(err_col.short_name.clone()))?;
                Some((first..=last).map(|i| e.get(i).copied().unwrap_or(f64::NAN)).collect())
            }
            _ => None,
        };
        let data = FitData {
            x: x_all[first..=last].to_vec(),
            y: y_all[first..=last].to_vec(),
            y_error,
            weighting: request.weighting,
            confidence_level: request.confidence_level,
        };
        let source_label = column_label(column);
        let source_name = column.display_name().to_string();
        let y_units = column.units.clone();
        let x_name = x_col.map_or_else(|| "Row".to_string(), |c| c.display_name().to_string());
        let x_units = x_col.map_or_else(String::new, |c| c.units.clone());
        let rows_note = if first == 0 && last + 1 == y_all.len() {
            format!("all {} rows", y_all.len())
        } else {
            format!("{} to {}", first + 1, last + 1)
        };
        let n_rows = y_all.len();

        // ----- fit (still no changes) ----------------------------------------------------
        let fitted = engine(&data)?;
        let report = fitted.report;
        if report.fitted.len() != data.y.len() || report.residuals.len() != data.y.len() {
            return Err(FitRequestError::BadOption(format!(
                "the fit engine returned {} fitted values for {} points",
                report.fitted.len(),
                data.y.len()
            )));
        }
        // The curve spans the X range of the points that took part in the fit.
        let used = data
            .x
            .iter()
            .zip(&data.y)
            .zip(&report.fitted)
            .filter(|((x, y), f)| x.is_finite() && y.is_finite() && f.is_finite())
            .map(|((x, _), _)| *x);
        let (lo, hi) = used.fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), x| (a.min(x), b.max(x)));
        if !(lo.is_finite() && hi.is_finite()) {
            return Err(FitError::TooFewPoints { have: 0, need: report.params.len().max(1) }.into());
        }
        let curve_x = linspace(lo, hi, request.curve_points);
        let curve_y: Vec<f64> = curve_x.iter().map(|x| (fitted.curve)(*x)).collect();

        // ----- change the project ----------------------------------------------------------
        let align = |values: &[f64]| {
            let mut out = vec![f64::NAN; n_rows];
            out[first..=last].copy_from_slice(values);
            out
        };
        let fitted_values = align(&report.fitted);
        let residuals = align(&report.residuals);
        let fitted_id = self.new_column_id();
        let residual_id = self.new_column_id();
        {
            let ws = &mut self.workbook_mut(book_id).expect("workbook exists").sheets[sheet_index];
            let mut at = ws.column_index(id).expect("source column exists") + 1;
            while at < ws.columns.len()
                && matches!(ws.columns[at].designation, Designation::YError | Designation::XError | Designation::Label)
            {
                at += 1;
            }
            let fitted_name = ws.next_short_name();
            ws.columns.insert(
                at,
                Column::new(fitted_id, fitted_name, Designation::Y, ColumnData::numeric(fitted_values)).with_labels(
                    &format!("Fitted {source_name}"),
                    &y_units,
                    &report.model,
                ),
            );
            let residual_name = ws.next_short_name();
            ws.columns.insert(
                at + 1,
                Column::new(residual_id, residual_name, Designation::Y, ColumnData::numeric(residuals)).with_labels(
                    &format!("Residual of {source_name}"),
                    &y_units,
                    &report.model,
                ),
            );
        }

        let sheets = &self.workbook(book_id).expect("workbook exists").sheets;
        // The parameter sheet and its "Stats" companion share a number; take the first number
        // for which neither name is in use (a sheet may have been renamed or deleted since).
        let taken = |name: &str| sheets.iter().any(|s| s.name.eq_ignore_ascii_case(name));
        let param_name = (1..)
            .map(|n| format!("{}{n}", request.model.sheet_prefix()))
            .find(|name| !taken(name) && !taken(&format!("{name}Stats")))
            .expect("the name sequence is unbounded");
        let mut param_ws = Worksheet::new(param_name.clone());
        let mut stats_ws = Worksheet::new(format!("{param_name}Stats"));
        let mut curve_ws = Worksheet::new(free_sheet_name(sheets, "FitCurve"));

        let p = &report.params;
        let numbers = |f: &dyn Fn(&sheaf_fit::Parameter) -> f64| ColumnData::numeric(p.iter().map(f).collect());
        let level = format!("{}%", (report.confidence_level * 100.0 * 1e6).round() / 1e6);
        let param_columns: Vec<(Designation, String, ColumnData)> = vec![
            (Designation::Label, "Parameter".into(), ColumnData::text(p.iter().map(|q| q.name.clone()).collect())),
            (Designation::Y, "Value".into(), numbers(&|q| q.value)),
            (Designation::Y, "Standard Error".into(), numbers(&|q| q.std_error)),
            (Designation::Y, "t-Value".into(), numbers(&|q| q.t_value)),
            (Designation::Y, "Prob>|t|".into(), numbers(&|q| q.prob_t)),
            (Designation::Y, format!("LCL ({level})"), numbers(&|q| q.lcl)),
            (Designation::Y, format!("UCL ({level})"), numbers(&|q| q.ucl)),
            (Designation::Y, "Dependency".into(), numbers(&|q| q.dependency)),
            (
                Designation::Label,
                "Fixed".into(),
                ColumnData::text(p.iter().map(|q| if q.fixed { "Fixed".into() } else { String::new() }).collect()),
            ),
        ];
        for (designation, name, data) in param_columns {
            sheet_column(self, &mut param_ws, designation, &name, "", data);
        }

        let rows = stats_rows(&report, &fitted.derived, &source_label, &rows_note);
        sheet_column(
            self,
            &mut stats_ws,
            Designation::Label,
            "Quantity",
            "",
            ColumnData::text(rows.iter().map(|r| r.0.clone()).collect()),
        );
        sheet_column(
            self,
            &mut stats_ws,
            Designation::Y,
            "Value",
            "",
            ColumnData::numeric(rows.iter().map(|r| r.1).collect()),
        );
        sheet_column(
            self,
            &mut stats_ws,
            Designation::Label,
            "Text",
            "",
            ColumnData::text(rows.iter().map(|r| r.2.clone()).collect()),
        );

        let curve_x_id =
            sheet_column(self, &mut curve_ws, Designation::X, &x_name, &x_units, ColumnData::numeric(curve_x));
        let curve_y_id = sheet_column(
            self,
            &mut curve_ws,
            Designation::Y,
            &format!("Fit of {source_name}"),
            &y_units,
            ColumnData::numeric(curve_y),
        );
        if let Some(c) = curve_ws.column_mut(curve_y_id) {
            c.comments = report.model.clone();
        }

        let param_name = param_ws.name.clone();
        let b = self.workbook_mut(book_id).expect("workbook exists");
        b.sheets.push(param_ws);
        let param_sheet = b.sheets.len() - 1;
        b.sheets.push(stats_ws);
        b.sheets.push(curve_ws);
        let (stats_sheet, curve_sheet) = (param_sheet + 1, param_sheet + 2);

        let mut graphs = Vec::new();
        if request.add_to_graphs {
            let label = format!("{} fit", report.model);
            for window in &mut self.windows {
                let crate::project::Window::Graph(g) = window else { continue };
                let mut touched = false;
                for layer in &mut g.layers {
                    if !layer.plots.iter().any(|pl| pl.y == id) {
                        continue;
                    }
                    let used: Vec<Color> = layer.plots.iter().map(|pl| pl.line.color).collect();
                    let color = CURVE_COLORS
                        .iter()
                        .copied()
                        .find(|c| used.iter().all(|u| color_distance(*c, *u) > 120))
                        .unwrap_or(CURVE_COLORS[0]);
                    layer.plots.push(Plot {
                        x: Some(curve_x_id),
                        y: curve_y_id,
                        y_error: None,
                        rows: None,
                        kind: PlotKind::Line,
                        line: LineStyle::solid(color, layer.default_line.width_pt.max(1.5)),
                        symbol: crate::graph::SymbolStyle { color, ..layer.default_symbol.clone() },
                        label: Some(label.clone()),
                        visible: true,
                        auto_color: false,
                    });
                    touched = true;
                }
                if touched {
                    graphs.push(g.id);
                }
            }
        }

        let r2 = if report.stats.r_squared.is_finite() {
            format!(", R\u{b2} = {}", crate::format::format_general(report.stats.r_squared, 6))
        } else {
            String::new()
        };
        let message = format!(
            "Fit {} to {}: {} points{r2}; parameters in sheet {param_name}{}",
            report.model,
            source_label,
            report.stats.n_points,
            if graphs.is_empty() { String::new() } else { ", curve added to the graph".to_string() },
        );
        Ok(FitOutcome {
            report,
            book: book_id,
            sheet: sheet_index,
            param_sheet,
            stats_sheet,
            curve_sheet,
            fitted_column: fitted_id,
            residual_column: residual_id,
            curve_x: curve_x_id,
            curve_y: curve_y_id,
            graphs,
            derived: fitted.derived,
            message,
        })
    }

    /// Fits one column with the engine of `sheaf-fit` that the request's model calls for, and
    /// places the results (see the module text and [`Project::fit_with`]).
    pub fn fit(&mut self, request: &FitRequest) -> Result<FitOutcome, FitRequestError> {
        let model = request.model.clone();
        let settings = request.params.clone();
        self.fit_with(request, move |data| run_engine(&model, &settings, data))
    }

    /// Makes a graph that shows the fitted data as symbols and the fitted curve as a line
    /// (for a fit whose data is not plotted anywhere yet). Returns the new graph.
    pub fn plot_fit(
        &mut self,
        outcome: &FitOutcome,
        source: ColumnId,
    ) -> Result<WindowId, crate::project::ProjectError> {
        let graph = self.plot_columns(crate::template::Template::Scatter, &[source])?;
        let g = self.graph_mut(graph).expect("just made");
        let layer = &mut g.layers[0];
        let data_color = layer.plots[0].line.color;
        let color =
            CURVE_COLORS.iter().copied().find(|c| color_distance(*c, data_color) > 120).unwrap_or(CURVE_COLORS[0]);
        layer.plots.push(Plot {
            x: Some(outcome.curve_x),
            y: outcome.curve_y,
            y_error: None,
            rows: None,
            kind: PlotKind::Line,
            line: LineStyle::solid(color, layer.default_line.width_pt.max(1.5)),
            symbol: crate::graph::SymbolStyle { color, ..layer.default_symbol.clone() },
            label: Some(format!("{} fit", outcome.report.model)),
            visible: true,
            auto_color: false,
        });
        Ok(graph)
    }
}

pub use sheaf_fit::library::FunctionInfo;

/// The built-in fit functions with their category, parameters and formula, for menus and
/// `sheaf fit --list`.
pub fn builtin_functions() -> Vec<FunctionInfo> {
    sheaf_fit::library::list()
}

/// The parameter names of a model, in the order the report lists them. For an `.FDF` model the
/// text is parsed; a file that cannot be used gives an error that says why.
pub fn model_parameters(model: &FitModel) -> Result<Vec<String>, FitError> {
    Ok(match model {
        FitModel::Linear => vec!["Intercept".into(), "Slope".into()],
        FitModel::Polynomial { order } => {
            std::iter::once("Intercept".to_string()).chain((1..=*order).map(|k| format!("B{k}"))).collect()
        }
        FitModel::BuiltIn(name) => sheaf_fit::library::get(name)
            .ok_or_else(|| FitError::BadFunction(format!("there is no built-in fit function named '{name}'")))?
            .params
            .into_iter()
            .map(|p| p.name)
            .collect(),
        FitModel::Formula { params, .. } => params.clone(),
        FitModel::Fdf(text) => {
            sheaf_fit::fdf::parse_fdf(text)?.to_fit_function()?.params.into_iter().map(|p| p.name).collect()
        }
    })
}

/// Turns the request's weighting into the engines' form.
fn weighting(d: &FitData) -> Result<sheaf_fit::Weighting, FitError> {
    Ok(match d.weighting {
        FitWeighting::None => sheaf_fit::Weighting::None,
        FitWeighting::Statistical => sheaf_fit::Weighting::Statistical,
        FitWeighting::Instrumental => sheaf_fit::Weighting::Instrumental(
            d.y_error.clone().ok_or_else(|| FitError::BadFunction("instrumental weighting needs Y errors".into()))?,
        ),
    })
}

/// Fails with a message that names the unknown parameter and the ones that exist.
fn unknown_param(name: &str, known: &[&str]) -> FitError {
    FitError::BadFunction(format!("the model has no parameter '{name}'; its parameters are {}", known.join(", ")))
}

/// The value a fixed parameter is held at.
fn fixed_value(s: &ParamSetting) -> Result<f64, FitError> {
    match s.initial {
        Some(v) if v.is_finite() => Ok(v),
        _ => Err(FitError::BadFunction(format!("parameter {} is fixed but has no value to hold it at", s.name))),
    }
}

/// Settings of a linear or polynomial fit: only fixed values are possible, for the named parameters.
fn closed_form_fixed(settings: &[ParamSetting], allowed: &[&str]) -> Result<Vec<(String, f64)>, FitError> {
    let mut fixed = Vec::new();
    for s in settings {
        if !allowed.contains(&s.name.as_str()) {
            return Err(if s.name == "Intercept" || s.name == "Slope" || s.name.starts_with('B') {
                FitError::BadFunction(format!(
                    "parameter {} cannot be fixed in this fit; only {} can",
                    s.name,
                    allowed.join(" and ")
                ))
            } else {
                unknown_param(&s.name, allowed)
            });
        }
        if s.lower.is_some() || s.upper.is_some() {
            return Err(FitError::BadFunction(format!(
                "a linear or polynomial fit has no bounds (parameter {}); use a nonlinear fit of the same formula",
                s.name
            )));
        }
        if s.fixed {
            fixed.push((s.name.clone(), fixed_value(s)?));
        } else if s.initial.is_some() {
            return Err(FitError::BadFunction(format!(
                "a linear or polynomial fit needs no start values (parameter {}); to hold it at a value, fix it",
                s.name
            )));
        }
    }
    Ok(fixed)
}

/// The value of a polynomial with coefficients in increasing order (`Intercept`, `B1`, ...).
fn polynomial(coefficients: &[f64], x: f64) -> f64 {
    coefficients.iter().rev().fold(0.0, |acc, c| acc * x + c)
}

fn run_engine(model: &FitModel, settings: &[ParamSetting], d: &FitData) -> Result<FittedModel, FitError> {
    use sheaf_fit::{linear, nlsf};
    let w = weighting(d)?;
    match model {
        FitModel::Linear => {
            let fixed = closed_form_fixed(settings, &["Intercept", "Slope"])?;
            let get = |n: &str| fixed.iter().find(|(name, _)| name == n).map(|(_, v)| *v);
            let opts = linear::LinearOptions {
                fix_intercept: get("Intercept"),
                fix_slope: get("Slope"),
                weighting: w,
                confidence_level: d.confidence_level,
                ..Default::default()
            };
            let report = linear::fit_linear(&d.x, &d.y, &opts)?;
            let c: Vec<f64> = report.params.iter().map(|p| p.value).collect();
            Ok(FittedModel { report, curve: Box::new(move |x| polynomial(&c, x)), derived: Vec::new() })
        }
        FitModel::Polynomial { order } => {
            let fixed = closed_form_fixed(settings, &["Intercept"])?;
            let opts = linear::PolyOptions {
                fix_intercept: fixed.first().map(|(_, v)| *v),
                weighting: w,
                confidence_level: d.confidence_level,
                ..Default::default()
            };
            let report = linear::fit_polynomial(&d.x, &d.y, *order, &opts)?;
            let c: Vec<f64> = report.params.iter().map(|p| p.value).collect();
            Ok(FittedModel { report, curve: Box::new(move |x| polynomial(&c, x)), derived: Vec::new() })
        }
        FitModel::BuiltIn(_) | FitModel::Formula { .. } | FitModel::Fdf(_) => {
            let (mut func, library_name) = match model {
                FitModel::BuiltIn(name) => {
                    let f = sheaf_fit::library::get(name).ok_or_else(|| {
                        FitError::BadFunction(format!(
                            "there is no built-in fit function named '{name}' (sheaf fit --list shows them all)"
                        ))
                    })?;
                    let n = f.name.clone();
                    (f, Some(n))
                }
                FitModel::Formula { formula, params } => {
                    let names: Vec<&str> = params.iter().map(String::as_str).collect();
                    (nlsf::FitFunction::from_formula("User defined", formula, &names)?, None)
                }
                FitModel::Fdf(text) => (sheaf_fit::fdf::parse_fdf(text)?.to_fit_function()?, None),
                _ => unreachable!("matched above"),
            };
            for s in settings {
                let Some(at) = func.params.iter().position(|p| p.name == s.name) else {
                    let known: Vec<&str> = func.params.iter().map(|p| p.name.as_str()).collect();
                    return Err(unknown_param(&s.name, &known));
                };
                let p = &mut func.params[at];
                if let Some(v) = s.initial {
                    p.initial = v;
                }
                if s.fixed {
                    p.initial = fixed_value(s)?;
                    p.fixed = true;
                }
                if s.lower.is_some() {
                    p.lower = s.lower;
                }
                if s.upper.is_some() {
                    p.upper = s.upper;
                }
                if let (Some(lo), Some(hi)) = (p.lower, p.upper)
                    && lo > hi
                {
                    return Err(FitError::BadFunction(format!(
                        "the lower bound of {} ({lo}) is above its upper bound ({hi})",
                        p.name
                    )));
                }
                // A value the user gave must lie within the bounds (the function's own bounds
                // included); otherwise the fit would start, or stay, outside them.
                if s.initial.is_some() || s.fixed {
                    let v = p.initial;
                    let what = if p.fixed { "fixed value" } else { "start value" };
                    if let Some(lo) = p.lower.filter(|lo| v < *lo) {
                        return Err(FitError::BadFunction(format!(
                            "the {what} of {} ({v}) is below its lower bound ({lo})",
                            p.name
                        )));
                    }
                    if let Some(hi) = p.upper.filter(|hi| v > *hi) {
                        return Err(FitError::BadFunction(format!(
                            "the {what} of {} ({v}) is above its upper bound ({hi})",
                            p.name
                        )));
                    }
                }
            }
            let opts = nlsf::FitOptions { weighting: w, confidence_level: d.confidence_level, ..Default::default() };
            let report = nlsf::fit(&func, &d.x, &d.y, &opts)?;
            let values: Vec<f64> = report.params.iter().map(|p| p.value).collect();
            let derived =
                library_name.map(|n| sheaf_fit::library::derived_values(&n, &report.params)).unwrap_or_default();
            Ok(FittedModel { report, curve: Box::new(move |x| func.eval(&values, x)), derived })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::template::Template;
    use sheaf_fit::{Anova, Parameter, Statistics};

    /// A straight-line least-squares engine written out by hand, enough to test the placement
    /// of results. The real engines live in `sheaf-fit`.
    fn line_engine(d: &FitData) -> Result<FittedModel, FitError> {
        let pts: Vec<(f64, f64)> =
            d.x.iter().zip(&d.y).filter(|(x, y)| x.is_finite() && y.is_finite()).map(|(x, y)| (*x, *y)).collect();
        let n = pts.len();
        if n < 3 {
            return Err(FitError::TooFewPoints { have: n, need: 2 });
        }
        let nf = n as f64;
        let (sx, sy) = pts.iter().fold((0.0, 0.0), |(a, b), (x, y)| (a + x, b + y));
        let (mx, my) = (sx / nf, sy / nf);
        let sxx: f64 = pts.iter().map(|(x, _)| (x - mx).powi(2)).sum();
        let sxy: f64 = pts.iter().map(|(x, y)| (x - mx) * (y - my)).sum();
        let slope = sxy / sxx;
        let intercept = my - slope * mx;
        let f = move |x: f64| intercept + slope * x;
        let fitted: Vec<f64> =
            d.x.iter().zip(&d.y).map(|(x, y)| if x.is_finite() && y.is_finite() { f(*x) } else { f64::NAN }).collect();
        let residuals: Vec<f64> = d.y.iter().zip(&fitted).map(|(y, f)| y - f).collect();
        let rss: f64 = residuals.iter().filter(|r| r.is_finite()).map(|r| r * r).sum();
        let param = |name: &str, value: f64| Parameter {
            name: name.into(),
            value,
            std_error: 0.1,
            fixed: false,
            t_value: value / 0.1,
            prob_t: 0.0,
            lcl: value - 0.2,
            ucl: value + 0.2,
            dependency: f64::NAN,
        };
        let report = FitReport {
            model: "Linear".into(),
            equation: "y = a + b*x".into(),
            params: vec![param("Intercept", intercept), param("Slope", slope)],
            stats: Statistics {
                n_points: n,
                dof: n - 2,
                rss,
                reduced_chi_sqr: rss / (n - 2) as f64,
                r_squared: 0.99,
                adj_r_squared: 0.98,
                pearson_r: Some(0.995),
                iterations: None,
            },
            anova: Some(Anova {
                model_df: 1,
                model_ss: 10.0,
                model_ms: 10.0,
                error_df: n - 2,
                error_ss: rss,
                error_ms: rss / (n - 2) as f64,
                total_df: n - 1,
                total_ss: 10.0 + rss,
                uncorrected_total_ss: f64::NAN,
                f_value: 100.0,
                prob_f: 1e-6,
            }),
            covariance: vec![0.0; 4],
            confidence_level: d.confidence_level,
            fitted,
            residuals,
            status: FitStatus::Exact,
            warnings: vec!["a test warning".into()],
        };
        Ok(FittedModel { report, curve: Box::new(f), derived: vec![("Twice the slope".into(), 2.0 * slope)] })
    }

    /// Book1 with X = 0..9 (V), Y = 2 + 3x with one missing value (mA), and Y errors.
    fn sample() -> (Project, WindowId, ColumnId, ColumnId, ColumnId) {
        let mut p = Project::new();
        let book = p.add_workbook("data");
        let xs: Vec<f64> = (0..10).map(f64::from).collect();
        let mut ys: Vec<f64> = xs.iter().map(|x| 2.0 + 3.0 * x).collect();
        ys[4] = f64::NAN;
        let x = p.add_column(book, 0, Designation::X, ColumnData::numeric(xs)).unwrap();
        let y = p.add_column(book, 0, Designation::Y, ColumnData::numeric(ys)).unwrap();
        let e = p.add_column(book, 0, Designation::YError, ColumnData::numeric(vec![0.5; 10])).unwrap();
        p.column_mut(x).unwrap().units = "V".into();
        let c = p.column_mut(y).unwrap();
        c.long_name = "Current".into();
        c.units = "mA".into();
        (p, book, x, y, e)
    }

    #[test]
    fn results_go_beside_the_source_and_into_three_sheets() {
        let (mut p, book, _, y, e) = sample();
        let out = p.fit_with(&FitRequest::new(y, FitModel::Linear), line_engine).unwrap();
        let b = p.workbook(book).unwrap();
        let names: Vec<&str> = b.sheets.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["Sheet1", "FitLinear1", "FitLinear1Stats", "FitCurve1"]);
        assert_eq!((out.param_sheet, out.stats_sheet, out.curve_sheet), (1, 2, 3));

        // Fitted and residual columns come right after the source and its error column.
        let ws = &b.sheets[0];
        let order: Vec<ColumnId> = ws.columns.iter().map(|c| c.id).collect();
        let at = order.iter().position(|c| *c == e).unwrap();
        assert_eq!(order[at + 1], out.fitted_column);
        assert_eq!(order[at + 2], out.residual_column);
        let fitted = p.column(out.fitted_column).unwrap();
        assert_eq!(fitted.long_name, "Fitted Current");
        assert_eq!(fitted.units, "mA");
        let f = fitted.numeric().unwrap();
        assert!((f[9] - 29.0).abs() < 1e-12);
        assert!(f[4].is_nan(), "the missing point stays missing");
        assert!(p.numeric(out.residual_column).unwrap()[0].abs() < 1e-12);

        // Parameter sheet.
        let ps = &b.sheets[1];
        assert_eq!(ps.columns[0].text().unwrap(), ["Intercept", "Slope"]);
        let value = ps.column_by_name("Value").unwrap().numeric().unwrap();
        assert!((value[0] - 2.0).abs() < 1e-12 && (value[1] - 3.0).abs() < 1e-12);
        assert!(ps.columns.iter().any(|c| c.long_name == "LCL (95%)"));

        // Statistics sheet: names and values line up; text rows carry text.
        let ss = &b.sheets[2];
        let names = ss.columns[0].text().unwrap();
        let values = ss.columns[1].numeric().unwrap();
        let texts = ss.columns[2].text().unwrap();
        let row = |n: &str| names.iter().position(|x| x == n).unwrap();
        assert_eq!(values[row("Number of points")], 9.0);
        assert_eq!(values[row("Degrees of freedom")], 7.0);
        assert_eq!(texts[row("Model")], "Linear");
        assert_eq!(texts[row("Rows")], "all 10 rows");
        assert_eq!(texts[row("Warning")], "a test warning");
        assert!(names.iter().any(|n| n == "F value"));
        assert!(!names.iter().any(|n| n.contains("uncorrected")), "NaN uncorrected total is left out");

        // The curve: 1000 points over the fitted X range, with the model's values.
        let cx = p.numeric(out.curve_x).unwrap();
        let cy = p.numeric(out.curve_y).unwrap();
        assert_eq!(cx.len(), 1000);
        assert_eq!((cx[0], cx[999]), (0.0, 9.0));
        assert!((cy[999] - 29.0).abs() < 1e-12);
        assert_eq!(p.column(out.curve_x).unwrap().units, "V");
        assert!(out.message.contains("FitLinear1"), "{}", out.message);
    }

    #[test]
    fn the_curve_is_added_to_graphs_that_show_the_data_in_a_different_color() {
        let (mut p, _, _, y, _) = sample();
        let g = p.plot_columns(Template::Line, &[y]).unwrap();
        let other = p.add_graph(Template::Line);
        let out = p.fit_with(&FitRequest::new(y, FitModel::Linear), line_engine).unwrap();
        assert_eq!(out.graphs, vec![g]);
        let layer = &p.graph(g).unwrap().layers[0];
        assert_eq!(layer.plots.len(), 2);
        let fit = &layer.plots[1];
        assert_eq!((fit.x, fit.y, fit.kind), (Some(out.curve_x), out.curve_y, PlotKind::Line));
        assert!(color_distance(fit.line.color, layer.plots[0].line.color) > 120);
        assert!(!fit.auto_color, "adding more plots must not recolor the fit");
        assert!(p.graph(other).unwrap().layers[0].plots.is_empty());

        // Without add_to_graphs nothing is added.
        let mut req = FitRequest::new(y, FitModel::Linear);
        req.add_to_graphs = false;
        let out2 = p.fit_with(&req, line_engine).unwrap();
        assert!(out2.graphs.is_empty());
        assert_eq!(p.graph(g).unwrap().layers[0].plots.len(), 2);
        // Second fit numbers its sheets on.
        let b = p.workbook(out2.book).unwrap();
        assert_eq!(b.sheets[out2.param_sheet].name, "FitLinear2");
        assert_eq!(b.sheets[out2.curve_sheet].name, "FitCurve2");
    }

    #[test]
    fn report_sheet_names_never_collide_with_kept_stats_sheets() {
        let (mut p, book, _, y, _) = sample();
        let first = p.fit_with(&FitRequest::new(y, FitModel::Linear), line_engine).unwrap();
        // Rename the parameter sheet but keep its statistics sheet, then fit again.
        p.workbook_mut(book).unwrap().sheets[first.param_sheet].name = "My parameters".into();
        let second = p.fit_with(&FitRequest::new(y, FitModel::Linear), line_engine).unwrap();
        let b = p.workbook(book).unwrap();
        assert_eq!(b.sheets[second.param_sheet].name, "FitLinear2");
        assert_eq!(b.sheets[second.stats_sheet].name, "FitLinear2Stats");
        let mut names: Vec<String> = b.sheets.iter().map(|s| s.name.to_ascii_lowercase()).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), b.sheets.len(), "all sheet names differ");
    }

    #[test]
    fn a_new_graph_shows_data_as_symbols_and_the_fit_as_a_line() {
        let (mut p, _, _, y, _) = sample();
        let out = p.fit_with(&FitRequest::new(y, FitModel::Linear), line_engine).unwrap();
        let g = p.plot_fit(&out, y).unwrap();
        let plots = &p.graph(g).unwrap().layers[0].plots;
        assert_eq!(plots.len(), 2);
        assert_eq!(plots[0].kind, PlotKind::Scatter);
        assert_eq!(plots[1].kind, PlotKind::Line);
        assert_eq!(plots[1].y, out.curve_y);
    }

    #[test]
    fn row_ranges_restrict_the_input_and_the_outputs() {
        let (mut p, _, _, y, _) = sample();
        let mut req = FitRequest::new(y, FitModel::Linear);
        req.rows = RowRange::Rows(6, 10);
        let mut seen = Vec::new();
        let out = p
            .fit_with(&req, |d| {
                seen = d.x.clone();
                line_engine(d)
            })
            .unwrap();
        assert_eq!(seen, [5.0, 6.0, 7.0, 8.0, 9.0]);
        let f = p.numeric(out.fitted_column).unwrap();
        assert!(f[..5].iter().all(|v| v.is_nan()));
        assert!(f[5..].iter().all(|v| v.is_finite()));
        assert_eq!(p.numeric(out.curve_x).unwrap()[0], 5.0);
    }

    #[test]
    fn instrumental_weighting_passes_the_error_column() {
        let (mut p, _, _, y, _) = sample();
        let mut req = FitRequest::new(y, FitModel::Linear);
        req.weighting = FitWeighting::Instrumental;
        let mut errors = None;
        p.fit_with(&req, |d| {
            errors = d.y_error.clone();
            line_engine(d)
        })
        .unwrap();
        assert_eq!(errors.unwrap(), vec![0.5; 10]);
    }

    #[test]
    fn failures_change_nothing() {
        let (mut p, book, _, y, e) = sample();
        let text = p.add_column(book, 0, Designation::Y, ColumnData::text(vec!["a".into()])).unwrap();
        let req = |f: &dyn Fn(&mut FitRequest)| {
            let mut r = FitRequest::new(y, FitModel::Linear);
            f(&mut r);
            r
        };
        let failing = |_: &FitData| -> Result<FittedModel, FitError> { Err(FitError::Failed("no convergence".into())) };

        let before = p.clone();
        let err = p.fit_with(&req(&|_| ()), failing).unwrap_err();
        assert!(matches!(err, FitRequestError::Fit(_)), "{err}");
        let err = p.fit_with(&req(&|r| r.confidence_level = 1.0), line_engine).unwrap_err();
        assert!(matches!(err, FitRequestError::BadConfidence(_)), "{err}");
        let err = p.fit_with(&req(&|r| r.curve_points = 1), line_engine).unwrap_err();
        assert!(matches!(err, FitRequestError::BadOption(_)), "{err}");
        let err = p.fit_with(&req(&|r| r.rows = RowRange::Rows(20, 30)), line_engine).unwrap_err();
        assert!(matches!(err, FitRequestError::Column(AnalysisError::BadRows(..))), "{err}");
        let err = p.fit_with(&req(&|r| r.y = text), line_engine).unwrap_err();
        assert!(matches!(err, FitRequestError::Column(AnalysisError::NotNumeric(_))), "{err}");
        // An engine that returns the wrong number of fitted values.
        let short = |d: &FitData| {
            let mut m = line_engine(d)?;
            m.report.fitted.pop();
            Ok(m)
        };
        assert!(matches!(p.fit_with(&req(&|_| ()), short).unwrap_err(), FitRequestError::BadOption(_)));
        assert_eq!(p, before);

        // Instrumental weighting without an error column.
        p.column_mut(e).unwrap().designation = Designation::Y;
        let before = p.clone();
        let err = p.fit_with(&req(&|r| r.weighting = FitWeighting::Instrumental), line_engine).unwrap_err();
        assert!(matches!(err, FitRequestError::NoErrorColumn(_)), "{err}");
        assert_eq!(p, before);
    }

    #[test]
    fn report_sheet_prefixes_follow_origin() {
        assert_eq!(FitModel::Linear.sheet_prefix(), "FitLinear");
        assert_eq!(FitModel::Polynomial { order: 3 }.sheet_prefix(), "FitPolynomial");
        assert_eq!(FitModel::BuiltIn("Gauss".into()).sheet_prefix(), "FitNL");
        assert_eq!(FitModel::Fdf(String::new()).sheet_prefix(), "FitNL");
    }

    /// A sheet with X = 0..40 step 0.25 and the given Y values.
    fn sheet_with(f: impl Fn(f64) -> f64) -> (Project, ColumnId) {
        let mut p = Project::new();
        let book = p.add_workbook("data");
        let xs: Vec<f64> = (0..=160).map(|i| f64::from(i) * 0.25).collect();
        let ys: Vec<f64> = xs.iter().map(|x| f(*x)).collect();
        p.add_column(book, 0, Designation::X, ColumnData::numeric(xs)).unwrap();
        let y = p.add_column(book, 0, Designation::Y, ColumnData::numeric(ys)).unwrap();
        (p, y)
    }

    fn value(out: &FitOutcome, name: &str) -> f64 {
        out.report.param(name).unwrap_or_else(|| panic!("no parameter {name}")).value
    }

    #[test]
    fn real_engines_recover_known_models() {
        // Straight line, polynomial, a built-in peak and a typed formula, all without noise.
        let (mut p, y) = sheet_with(|x| 1.5 - 0.25 * x);
        let out = p.fit(&FitRequest::new(y, FitModel::Linear)).unwrap();
        assert!((value(&out, "Intercept") - 1.5).abs() < 1e-9 && (value(&out, "Slope") + 0.25).abs() < 1e-9);
        let curve = p.numeric(out.curve_y).unwrap();
        assert!((curve[999] - (1.5 - 0.25 * 40.0)).abs() < 1e-9);

        let (mut p, y) = sheet_with(|x| 2.0 - x + 0.05 * x * x);
        let out = p.fit(&FitRequest::new(y, FitModel::Polynomial { order: 2 })).unwrap();
        let c: Vec<f64> = out.report.params.iter().map(|q| q.value).collect();
        assert!((c[0] - 2.0).abs() < 1e-8 && (c[1] + 1.0).abs() < 1e-8 && (c[2] - 0.05).abs() < 1e-9, "{c:?}");
        assert_eq!(p.workbook(out.book).unwrap().sheets[out.param_sheet].name, "FitPolynomial1");

        // Gauss in Origin's area form: y0 + A/(w*sqrt(pi/2)) * exp(-2((x-xc)/w)^2).
        let gauss = |x: f64| {
            0.5 + 12.0 / (3.0 * (std::f64::consts::PI / 2.0).sqrt()) * (-2.0 * ((x - 18.0) / 3.0).powi(2)).exp()
        };
        let (mut p, y) = sheet_with(gauss);
        let out = p.fit(&FitRequest::new(y, FitModel::BuiltIn("gauss".into()))).unwrap();
        for (name, want) in [("y0", 0.5), ("xc", 18.0), ("w", 3.0), ("A", 12.0)] {
            assert!((value(&out, name) - want).abs() < 1e-6, "{name}: {}", value(&out, name));
        }
        assert_eq!(p.workbook(out.book).unwrap().sheets[out.param_sheet].name, "FitNL1");
        // The curve follows the fitted parameters.
        let (cx, cy) = (p.numeric(out.curve_x).unwrap(), p.numeric(out.curve_y).unwrap());
        assert!(cx.iter().zip(cy).all(|(x, y)| (gauss(*x) - y).abs() < 1e-6));

        let (mut p, y) = sheet_with(|x| 3.0 + 7.0 * (-x / 4.0).exp());
        let model =
            FitModel::Formula { formula: "y0 + A*exp(-x/t)".into(), params: vec!["y0".into(), "A".into(), "t".into()] };
        let mut req = FitRequest::new(y, model);
        req.params = vec![
            ParamSetting { name: "A".into(), initial: Some(5.0), ..Default::default() },
            ParamSetting { name: "t".into(), initial: Some(2.0), ..Default::default() },
        ];
        let out = p.fit(&req).unwrap();
        for (name, want) in [("y0", 3.0), ("A", 7.0), ("t", 4.0)] {
            assert!((value(&out, name) - want).abs() < 1e-6, "{name}: {}", value(&out, name));
        }
    }

    #[test]
    fn parameter_settings_are_applied_and_checked() {
        let (mut p, y) = sheet_with(|x| 1.0 + 2.0 * x);
        // A fixed intercept of a linear fit.
        let mut req = FitRequest::new(y, FitModel::Linear);
        req.params =
            vec![ParamSetting { name: "Intercept".into(), initial: Some(0.0), fixed: true, ..Default::default() }];
        let out = p.fit(&req).unwrap();
        assert_eq!(value(&out, "Intercept"), 0.0);
        assert!(out.report.param("Intercept").unwrap().fixed);

        // A fixed parameter of a nonlinear fit stays where it was put.
        let mut req =
            FitRequest::new(y, FitModel::Formula { formula: "a + b*x".into(), params: vec!["a".into(), "b".into()] });
        req.params = vec![ParamSetting { name: "a".into(), initial: Some(1.0), fixed: true, ..Default::default() }];
        let out = p.fit(&req).unwrap();
        assert_eq!(value(&out, "a"), 1.0);
        assert!((value(&out, "b") - 2.0).abs() < 1e-8);

        let before = p.clone();
        for (settings, model, why) in [
            (ParamSetting { name: "Nope".into(), ..Default::default() }, FitModel::Linear, "unknown name"),
            (
                ParamSetting { name: "B1".into(), initial: Some(1.0), fixed: true, ..Default::default() },
                FitModel::Polynomial { order: 2 },
                "only the intercept can be fixed",
            ),
            (
                ParamSetting { name: "Slope".into(), lower: Some(0.0), ..Default::default() },
                FitModel::Linear,
                "no bounds",
            ),
            (
                ParamSetting { name: "Slope".into(), fixed: true, ..Default::default() },
                FitModel::Linear,
                "fixed without a value",
            ),
            (
                ParamSetting { name: "w".into(), lower: Some(2.0), upper: Some(1.0), ..Default::default() },
                FitModel::BuiltIn("Gauss".into()),
                "crossed bounds",
            ),
        ] {
            let mut req = FitRequest::new(y, model);
            req.params = vec![settings];
            let err = p.fit(&req).unwrap_err();
            assert!(matches!(err, FitRequestError::Fit(FitError::BadFunction(_))), "{why}: {err}");
        }
        // Values outside the bounds, and start values of a closed-form fit.
        for (settings, model, why) in [
            (
                ParamSetting { name: "w".into(), initial: Some(3.0), fixed: true, lower: Some(5.0), upper: Some(6.0) },
                FitModel::BuiltIn("Gauss".into()),
                "fixed value below the lower bound",
            ),
            (
                ParamSetting { name: "w".into(), initial: Some(9.0), upper: Some(6.0), ..Default::default() },
                FitModel::BuiltIn("Gauss".into()),
                "start value above the upper bound",
            ),
            (
                ParamSetting { name: "w".into(), initial: Some(-1.0), ..Default::default() },
                FitModel::BuiltIn("Gauss".into()),
                "start value below the function's own lower bound of 0",
            ),
            (
                ParamSetting { name: "Intercept".into(), initial: Some(100.0), ..Default::default() },
                FitModel::Linear,
                "a start value for a closed-form fit",
            ),
        ] {
            let mut req = FitRequest::new(y, model);
            req.params = vec![settings];
            let err = p.fit(&req).unwrap_err();
            assert!(matches!(err, FitRequestError::Fit(FitError::BadFunction(_))), "{why}: {err}");
        }
        let err = p.fit(&FitRequest::new(y, FitModel::BuiltIn("NoSuchFunction".into()))).unwrap_err();
        assert!(err.to_string().contains("--list"), "{err}");
        assert_eq!(p, before, "failed fits change nothing");
    }

    #[test]
    fn built_in_peaks_report_derived_quantities() {
        let (mut p, y) = sheet_with(|x| {
            12.0 / (3.0 * (std::f64::consts::PI / 2.0).sqrt()) * (-2.0 * ((x - 18.0) / 3.0).powi(2)).exp()
        });
        let out = p.fit(&FitRequest::new(y, FitModel::BuiltIn("Gauss".into()))).unwrap();
        if let Some((_, fwhm)) = out.derived.iter().find(|(n, _)| n.eq_ignore_ascii_case("FWHM")) {
            // For Origin's Gauss, w is twice the standard deviation: FWHM = w * sqrt(ln 4).
            assert!((fwhm - 3.0 * 4f64.ln().sqrt()).abs() < 1e-5, "{fwhm}");
            let ss = &p.workbook(out.book).unwrap().sheets[out.stats_sheet];
            assert!(ss.columns[0].text().unwrap().iter().any(|n| n.eq_ignore_ascii_case("FWHM")));
        } else {
            panic!("Gauss has no FWHM among {:?}", out.derived);
        }
    }

    #[test]
    fn the_test_engine_derived_value_reaches_the_stats_sheet() {
        let (mut p, _, _, y, _) = sample();
        let out = p.fit_with(&FitRequest::new(y, FitModel::Linear), line_engine).unwrap();
        assert_eq!(out.derived.len(), 1);
        let ss = &p.workbook(out.book).unwrap().sheets[out.stats_sheet];
        let names = ss.columns[0].text().unwrap();
        let at = names.iter().position(|n| n == "Twice the slope").unwrap();
        assert!((ss.columns[1].numeric().unwrap()[at] - 6.0).abs() < 1e-12);
    }

    #[test]
    fn linspace_hits_both_ends() {
        assert_eq!(linspace(0.0, 1.0, 3), [0.0, 0.5, 1.0]);
        assert_eq!(linspace(2.0, 2.0, 1), [2.0]);
        let v = linspace(0.1, 0.7, 7);
        assert_eq!(v[6], 0.7);
    }
}
