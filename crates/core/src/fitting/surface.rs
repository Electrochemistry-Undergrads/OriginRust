//! Surface fits `z = f(x, y)` on three worksheet columns (`docs/spec/fitting.md`, section 15).
//! The numerics are in `sheaf_fit::nlsf::fit_xyz` and `sheaf_fit::surface`; this file gathers the
//! X, Y and Z columns and places the results in the workbook:
//!
//! - a parameter sheet `FitSurface1` and its `Stats` companion, laid out as for curve fits;
//! - the fitted Z values and the residuals as two new columns after the Z column;
//! - a sheet `FitSurfaceCurve1` with the fitted surface on a regular grid over the X and Y range
//!   of the points that took part, as three columns X, Y, Z.
//!
//! Nothing changes in the project when the fit fails.

use sheaf_fit::{FitError, nlsf};

use super::{
    BandChoice, FitData, FitModel, FitOutcome, FitRequest, FitRequestError, FitWeighting, FunctionInfo, Project,
    apply_settings, linspace, scale_errors, sheet_column, stats_rows, weighting,
};
use crate::analysis::{AnalysisError, column_label, free_sheet_name, resolve_rows};
use crate::column::{Column, ColumnData, Designation};
use crate::ids::ColumnId;
use crate::worksheet::Worksheet;

/// Points per side of the grid of the `FitSurfaceCurve` sheet.
const GRID: usize = 30;

/// Origin's category name of the surface functions.
pub const SURFACE_CATEGORY: &str = "Surface Fitting";

/// The surface functions with their parameters and formulas, for menus and `sheaf fit --list`
/// (they are not part of [`super::builtin_functions`], which lists the curve functions).
pub fn surface_functions() -> Vec<FunctionInfo> {
    sheaf_fit::surface::names()
        .into_iter()
        .map(|name| FunctionInfo {
            name: name.to_string(),
            category: SURFACE_CATEGORY.to_string(),
            priority: "P1".to_string(),
            params: sheaf_fit::surface::param_names(name).unwrap_or_default().iter().map(|p| p.to_string()).collect(),
            formula: sheaf_fit::surface::formula(name).map(str::to_string),
            derived: Vec::new(),
        })
        .collect()
}

/// The X and Y columns that belong to a Z column: the X column to its left (as for curve fits)
/// and the nearest column to its left designated Y that holds no fit result.
pub fn surface_columns(project: &Project, z: ColumnId) -> Option<(ColumnId, ColumnId)> {
    let (_, sheet, _) = project.locate(z)?;
    let index = sheet.column_index(z)?;
    let x = sheet.x_for(index).filter(|c| c.id != z && c.data.is_numeric())?.id;
    let y = sheet.columns[..index]
        .iter()
        .rev()
        .find(|c| c.designation == Designation::Y && c.data.is_numeric() && !c.fit_result && c.id != x)?
        .id;
    Some((x, y))
}

/// The function of a surface model with the settings applied.
fn surface_function(model: &FitModel, settings: &[super::ParamSetting]) -> Result<nlsf::FitFunction, FitError> {
    let mut func = match model {
        FitModel::BuiltIn(name) => sheaf_fit::surface::get(name).ok_or_else(|| {
            FitError::BadFunction(format!(
                "'{name}' is not a surface function; the surface functions are {}",
                sheaf_fit::surface::names().join(", ")
            ))
        })?,
        FitModel::Formula { formula, params } => {
            let names: Vec<&str> = params.iter().map(String::as_str).collect();
            nlsf::FitFunction::from_formula_xy("User defined", formula, &names)?
        }
        FitModel::Fdf(text) => {
            let f = sheaf_fit::fdf::parse_fdf(text)?.to_fit_function()?;
            if f.x2_name.is_none() {
                return Err(FitError::BadFunction("this .FDF function has one independent variable, not two".into()));
            }
            f
        }
        _ => {
            return Err(FitError::BadFunction(
                "a surface fit needs a surface function (z = f(x, y)), a formula in x and y, or an .FDF file with two independent variables"
                    .into(),
            ));
        }
    };
    apply_settings(&mut func, settings)?;
    Ok(func)
}

impl Project {
    /// Fits `z = f(x, y)` to the Z column `request.y` and the X and Y columns of
    /// `request.surface`, and places the results (see the module text). Called by
    /// [`Project::fit`] for a request with `surface` set.
    pub(super) fn fit_surface(&mut self, request: &FitRequest) -> Result<FitOutcome, FitRequestError> {
        let chosen = request.surface.unwrap_or_default();
        if !(request.confidence_level > 0.0 && request.confidence_level < 1.0) {
            return Err(FitRequestError::BadConfidence(request.confidence_level));
        }
        if !(request.tolerance > 0.0 && request.tolerance.is_finite()) {
            return Err(FitRequestError::BadOption(format!(
                "the tolerance must be a positive number, not {}",
                request.tolerance
            )));
        }
        if request.bands != BandChoice::None {
            return Err(FitRequestError::BadOption(
                "confidence and prediction bands are not available for a surface fit".into(),
            ));
        }
        let z_id = request.y;
        let (x_id, y_id) = match (chosen.x, chosen.y) {
            (Some(x), Some(y)) => (x, y),
            (x, y) => {
                let (dx, dy) = surface_columns(self, z_id).ok_or_else(|| {
                    FitRequestError::BadOption(
                        "a surface fit needs an X, a Y and a Z column; set the designations of the columns, or choose the X and Y columns"
                            .into(),
                    )
                })?;
                (x.unwrap_or(dx), y.unwrap_or(dy))
            }
        };
        if x_id == y_id || x_id == z_id || y_id == z_id {
            return Err(FitRequestError::BadOption(
                "the X, Y and Z columns of a surface fit must be three different columns".into(),
            ));
        }
        let func = surface_function(&request.model, &request.params)?;

        // ----- gather the input (no changes yet) ----------------------------------------
        let (book, sheet, z_col) = self.locate(z_id).ok_or(AnalysisError::NoSuchColumn(z_id))?;
        let book_id = book.id;
        let sheet_index = book.sheets.iter().position(|s| s.column(z_id).is_some()).expect("located here");
        let numbers = |id: ColumnId| -> Result<(Vec<f64>, &Column), FitRequestError> {
            let c = sheet.column(id).ok_or(AnalysisError::NoSuchColumn(id))?;
            let v = sheet.values_of(c).ok_or_else(|| AnalysisError::NotNumeric(c.short_name.clone()))?;
            Ok((v.into_owned(), c))
        };
        let (z_all, _) = numbers(z_id)?;
        let (x_raw, x_col) = numbers(x_id)?;
        let (y_raw, y_col) = numbers(y_id)?;
        if z_all.is_empty() {
            return Err(AnalysisError::Empty(z_col.short_name.clone()).into());
        }
        let n_rows = z_all.len();
        let fill = |v: &[f64]| -> Vec<f64> { (0..n_rows).map(|i| v.get(i).copied().unwrap_or(f64::NAN)).collect() };
        let (x_all, y_all) = (fill(&x_raw), fill(&y_raw));
        let (first, last) = resolve_rows(request.rows, &z_col.short_name, &x_all, n_rows)?;
        let z_index = sheet.column_index(z_id).expect("located here");
        let y_error = match request.weighting {
            FitWeighting::Instrumental | FitWeighting::Direct => {
                let err_col = match request.y_error {
                    Some(e) => Some(self.column(e).ok_or(AnalysisError::NoSuchColumn(e))?),
                    None => sheet.y_error_for(z_index),
                }
                .ok_or_else(|| FitRequestError::NoErrorColumn(z_col.short_name.clone()))?;
                let e = err_col
                    .numeric()
                    .ok_or_else(|| FitRequestError::ErrorColumnNotNumeric(err_col.short_name.clone()))?;
                Some((first..=last).map(|i| e.get(i).copied().unwrap_or(f64::NAN)).collect::<Vec<f64>>())
            }
            _ => None,
        };
        let data = FitData {
            x: x_all[first..=last].to_vec(),
            y: z_all[first..=last].to_vec(),
            y_error,
            x_error: None,
            weighting: request.weighting,
            confidence_level: request.confidence_level,
            max_iterations: request.max_iterations,
            tolerance: request.tolerance,
            scale_errors: request.scale_errors,
            algorithm: request.algorithm,
        };
        let ys = y_all[first..=last].to_vec();
        let source_label = format!("{}, {}, {}", column_label(x_col), column_label(y_col), column_label(z_col));
        let z_name = z_col.display_name().to_string();
        let (x_name, y_name) = (x_col.display_name().to_string(), y_col.display_name().to_string());
        let (x_units, y_units, z_units) = (x_col.units.clone(), y_col.units.clone(), z_col.units.clone());
        let rows_note = if first == 0 && last + 1 == n_rows {
            format!("all {n_rows} rows")
        } else {
            format!("{} to {}", first + 1, last + 1)
        };

        // ----- fit (still no changes) ----------------------------------------------------
        let opts = nlsf::FitOptions {
            weighting: weighting(&data)?,
            confidence_level: data.confidence_level,
            max_iterations: data.max_iterations,
            tolerance: data.tolerance,
            scale_errors: Some(scale_errors(&data)),
            algorithm: data.algorithm,
        };
        let report = nlsf::fit_xyz(&func, &data.x, &ys, &data.y, &opts)?;
        if report.fitted.len() != data.y.len() || report.residuals.len() != data.y.len() {
            return Err(FitRequestError::BadOption(format!(
                "the fit engine returned {} fitted values for {} points",
                report.fitted.len(),
                data.y.len()
            )));
        }
        let derived = match &request.model {
            FitModel::BuiltIn(_) => sheaf_fit::surface::derived_values(&func.name, &report.params),
            _ => Vec::new(),
        };
        let values: Vec<f64> = report.params.iter().map(|p| p.value).collect();
        let used: Vec<(f64, f64)> = data
            .x
            .iter()
            .zip(&ys)
            .zip(data.y.iter().zip(&report.fitted))
            .filter(|((x, y), (z, f))| x.is_finite() && y.is_finite() && z.is_finite() && f.is_finite())
            .map(|((x, y), _)| (*x, *y))
            .collect();
        let range = |pick: fn(&(f64, f64)) -> f64| {
            used.iter().map(pick).fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), v| (a.min(v), b.max(v)))
        };
        let ((xlo, xhi), (ylo, yhi)) = (range(|p| p.0), range(|p| p.1));
        if !(xlo.is_finite() && ylo.is_finite()) {
            return Err(FitError::TooFewPoints { have: 0, need: report.params.len().max(1) }.into());
        }
        let gx = linspace(xlo, xhi, GRID);
        let gy = linspace(ylo, yhi, GRID);
        let (mut grid_x, mut grid_y, mut grid_z) = (Vec::new(), Vec::new(), Vec::new());
        for y in &gy {
            for x in &gx {
                grid_x.push(*x);
                grid_y.push(*y);
                grid_z.push(func.eval2(&values, *x, *y));
            }
        }

        // ----- change the project ----------------------------------------------------------
        let align = |values: &[f64]| {
            let mut out = vec![f64::NAN; n_rows];
            out[first..=last].copy_from_slice(values);
            out
        };
        let fitted_id = self.new_column_id();
        let residual_id = self.new_column_id();
        {
            let ws = &mut self.workbook_mut(book_id).expect("workbook exists").sheets[sheet_index];
            let mut at = ws.column_index(z_id).expect("source column exists") + 1;
            while at < ws.columns.len()
                && matches!(ws.columns[at].designation, Designation::YError | Designation::XError | Designation::Label)
            {
                at += 1;
            }
            let fitted_name = ws.next_short_name();
            ws.columns.insert(
                at,
                Column::new(fitted_id, fitted_name, Designation::Y, ColumnData::numeric(align(&report.fitted)))
                    .with_labels(&format!("Fitted {z_name}"), &z_units, &report.model)
                    .as_fit_result(),
            );
            let residual_name = ws.next_short_name();
            ws.columns.insert(
                at + 1,
                Column::new(residual_id, residual_name, Designation::Y, ColumnData::numeric(align(&report.residuals)))
                    .with_labels(&format!("Residual of {z_name}"), &z_units, &report.model)
                    .as_fit_result(),
            );
        }

        let sheets = &self.workbook(book_id).expect("workbook exists").sheets;
        let taken = |name: &str| sheets.iter().any(|s| s.name.eq_ignore_ascii_case(name));
        let param_name = (1..)
            .map(|n| format!("FitSurface{n}"))
            .find(|name| !taken(name) && !taken(&format!("{name}Stats")))
            .expect("the name sequence is unbounded");
        let mut param_ws = Worksheet::new(param_name.clone());
        let mut stats_ws = Worksheet::new(format!("{param_name}Stats"));
        let mut grid_ws = Worksheet::new(free_sheet_name(sheets, "FitSurfaceCurve"));

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
        let rows = stats_rows(&report, &derived, &source_label, &rows_note);
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
        let curve_x = sheet_column(self, &mut grid_ws, Designation::X, &x_name, &x_units, ColumnData::numeric(grid_x));
        sheet_column(self, &mut grid_ws, Designation::Y, &y_name, &y_units, ColumnData::numeric(grid_y));
        let curve_y = sheet_column(
            self,
            &mut grid_ws,
            Designation::Z,
            &format!("Fit of {z_name}"),
            &z_units,
            ColumnData::numeric(grid_z),
        );
        if let Some(c) = grid_ws.column_mut(curve_y) {
            c.comments = report.model.clone();
        }

        let b = self.workbook_mut(book_id).expect("workbook exists");
        b.sheets.push(param_ws);
        let param_sheet = b.sheets.len() - 1;
        b.sheets.push(stats_ws);
        b.sheets.push(grid_ws);
        let (stats_sheet, curve_sheet) = (param_sheet + 1, param_sheet + 2);

        let r2 = if report.stats.r_squared.is_finite() {
            format!(", R\u{b2} = {}", crate::format::format_general(report.stats.r_squared, 6))
        } else {
            String::new()
        };
        let message = format!(
            "Surface fit {} to {}: {} points{r2}; parameters in sheet {param_name}",
            report.model, source_label, report.stats.n_points,
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
            curve_x,
            curve_y,
            band_columns: Vec::new(),
            bands: None,
            plot_bands: false,
            graphs: Vec::new(),
            derived,
            message,
            surface: true,
        })
    }
}
