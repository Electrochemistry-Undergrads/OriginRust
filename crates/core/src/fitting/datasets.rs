//! Nonlinear Implicit Curve Fit and Compare Datasets on worksheet columns (`docs/spec/fitting.md`, sections 17 and
//! 18). The numerics are in `sheaf_fit::implicit` and `sheaf_fit::compare_data`; this file gathers the columns and
//! places the results in the workbook. Nothing changes in the project when one of these fails.

use sheaf_fit::compare_data::{ALPHA, DatasetComparison, compare_datasets};
use sheaf_fit::implicit::{ImplicitFunction, ImplicitOptions, fit_implicit};

use super::{FitData, FitReport, FitRequest, FitRequestError, Project, run_engine, sheet_column, stats_rows};
use crate::analysis::{AnalysisError, RowRange, column_label, free_sheet_name, resolve_rows};
use crate::column::{Column, ColumnData, Designation};
use crate::ids::{ColumnId, WindowId};
use crate::worksheet::Worksheet;

/// A nonlinear implicit curve fit `f(x, y, params) = 0` of the points of an X and a Y column.
#[derive(Debug, Clone, PartialEq)]
pub struct ImplicitRequest {
    pub x: ColumnId,
    pub y: ColumnId,
    /// `Circle` or `Ellipse` (any case).
    pub function: String,
    /// Starting values in the order of the function's parameters; empty estimates them from the data.
    pub initial: Vec<f64>,
    pub rows: RowRange,
    pub confidence_level: f64,
    pub max_iterations: usize,
    pub tolerance: f64,
}

impl ImplicitRequest {
    /// Origin's defaults: all rows, 400 iterations, tolerance 1e-9, 95 % limits.
    pub fn new(x: ColumnId, y: ColumnId, function: &str) -> Self {
        ImplicitRequest {
            x,
            y,
            function: function.to_string(),
            initial: Vec::new(),
            rows: RowRange::All,
            confidence_level: 0.95,
            max_iterations: 400,
            tolerance: 1e-9,
        }
    }
}

/// What [`Project::fit_implicit`] did.
#[derive(Debug, Clone, PartialEq)]
pub struct ImplicitOutcome {
    pub report: FitReport,
    pub book: WindowId,
    /// The parameter sheet (`FitImplicit<n>`); the statistics sheet follows it, then the curve sheet.
    pub param_sheet: usize,
    pub stats_sheet: usize,
    pub curve_sheet: usize,
    /// The column of signed orthogonal distances next to the Y column.
    pub distance_column: ColumnId,
    pub message: String,
}

/// What [`Project::compare_datasets`] writes.
#[derive(Debug, Clone, PartialEq)]
pub struct CompareDatasetsOptions {
    /// Origin's "Fit Parameters" box (ticked by default).
    pub parameters: bool,
    /// Origin's "Fit Statistics" box (ticked by default).
    pub statistics: bool,
    /// Origin's "1st Dataset Name" and "2nd Dataset Name" (`Data1`, `Data2`).
    pub names: [String; 2],
}

impl Default for CompareDatasetsOptions {
    fn default() -> Self {
        CompareDatasetsOptions { parameters: true, statistics: true, names: ["Data1".into(), "Data2".into()] }
    }
}

/// What [`Project::compare_datasets`] did.
#[derive(Debug, Clone, PartialEq)]
pub struct CompareDatasetsOutcome {
    pub comparison: DatasetComparison,
    /// The fit of dataset 1, of dataset 2 and of both pooled.
    pub reports: Vec<FitReport>,
    pub book: WindowId,
    pub sheet: usize,
    pub message: String,
}

type Row = (String, f64, f64, String);

impl Project {
    fn numeric_values(&self, id: ColumnId) -> Result<(String, Vec<f64>), FitRequestError> {
        let (_, sheet, c) = self.locate(id).ok_or(AnalysisError::NoSuchColumn(id))?;
        // Masked cells and filter-hidden rows are missing values (T256).
        let v = sheet.values_of(c).ok_or_else(|| AnalysisError::NotNumeric(c.short_name.clone()))?;
        Ok((c.short_name.clone(), v.into_owned()))
    }

    /// Fits a circle or ellipse to the points of two columns by orthogonal distance regression and places the
    /// parameter sheet `FitImplicit<n>`, its statistics sheet `FitImplicit<n>Stats`, a sheet `FitImplicitCurve<n>`
    /// with the fitted curve, and the signed distances as a new column next to the Y column.
    pub fn fit_implicit(&mut self, request: &ImplicitRequest) -> Result<ImplicitOutcome, FitRequestError> {
        let function = ImplicitFunction::by_name(&request.function).ok_or_else(|| {
            FitRequestError::BadOption(format!(
                "there is no implicit function '{}' (Circle and Ellipse are available)",
                request.function
            ))
        })?;
        if !(request.confidence_level > 0.0 && request.confidence_level < 1.0) {
            return Err(FitRequestError::BadConfidence(request.confidence_level));
        }
        let (_, x_all) = self.numeric_values(request.x)?;
        let (y_short, y_all) = self.numeric_values(request.y)?;
        if x_all.len() != y_all.len() {
            return Err(FitRequestError::BadOption(format!(
                "the X and Y columns have different lengths ({} and {})",
                x_all.len(),
                y_all.len()
            )));
        }
        if y_all.is_empty() {
            return Err(AnalysisError::Empty(y_short).into());
        }
        let (first, last) = resolve_rows(request.rows, &y_short, &x_all, y_all.len())?;
        let opts = ImplicitOptions {
            initial: request.initial.clone(),
            max_iterations: request.max_iterations,
            tolerance: request.tolerance,
            confidence_level: request.confidence_level,
        };
        let fit = fit_implicit(function, &x_all[first..=last], &y_all[first..=last], &opts)?;
        let report = fit.report;
        let values: Vec<f64> = report.params.iter().map(|p| p.value).collect();

        let (book_id, sheet_index, source_label, x_label, y_name, y_units, x_name, x_units) = {
            let (book, _, y_col) = self.locate(request.y).ok_or(AnalysisError::NoSuchColumn(request.y))?;
            let x_col = self.column(request.x).ok_or(AnalysisError::NoSuchColumn(request.x))?;
            let si = book.sheets.iter().position(|s| s.column(request.y).is_some()).expect("located here");
            (
                book.id,
                si,
                column_label(y_col),
                column_label(x_col),
                y_col.display_name().to_string(),
                y_col.units.clone(),
                x_col.display_name().to_string(),
                x_col.units.clone(),
            )
        };
        let rows_note = if first == 0 && last + 1 == y_all.len() {
            format!("all {} rows", y_all.len())
        } else {
            format!("{} to {}", first + 1, last + 1)
        };

        // ----- change the project ----------------------------------------------------------
        let mut distances = vec![f64::NAN; y_all.len()];
        distances[first..=last].copy_from_slice(&fit.distances);
        let distance_id = self.new_column_id();
        {
            let ws = &mut self.workbook_mut(book_id).expect("workbook exists").sheets[sheet_index];
            let mut at = ws.column_index(request.y).expect("source column exists") + 1;
            while at < ws.columns.len()
                && matches!(ws.columns[at].designation, Designation::YError | Designation::XError | Designation::Label)
            {
                at += 1;
            }
            let name = ws.next_short_name();
            ws.columns.insert(
                at,
                Column::new(distance_id, name, Designation::Y, ColumnData::numeric(distances))
                    .with_labels(&format!("Distance of {y_name}"), &y_units, &report.model)
                    .as_fit_result(),
            );
        }
        let sheets = &self.workbook(book_id).expect("workbook exists").sheets;
        let taken = |name: &str| sheets.iter().any(|s| s.name.eq_ignore_ascii_case(name));
        let n = (1..)
            .find(|n| {
                !taken(&format!("FitImplicit{n}"))
                    && !taken(&format!("FitImplicit{n}Stats"))
                    && !taken(&format!("FitImplicitCurve{n}"))
            })
            .expect("the number sequence is unbounded");
        let param_name = format!("FitImplicit{n}");
        let mut param_ws = Worksheet::new(param_name.clone());
        let mut stats_ws = Worksheet::new(format!("{param_name}Stats"));
        let mut curve_ws = Worksheet::new(format!("FitImplicitCurve{n}"));

        let p = &report.params;
        let numbers = |f: &dyn Fn(&sheaf_fit::Parameter) -> f64| ColumnData::numeric(p.iter().map(f).collect());
        let level = format!("{}%", (report.confidence_level * 100.0 * 1e6).round() / 1e6);
        let columns: Vec<(Designation, String, ColumnData)> = vec![
            (Designation::Label, "Parameter".into(), ColumnData::text(p.iter().map(|q| q.name.clone()).collect())),
            (Designation::Y, "Value".into(), numbers(&|q| q.value)),
            (Designation::Y, "Standard Error".into(), numbers(&|q| q.std_error)),
            (Designation::Y, "t-Value".into(), numbers(&|q| q.t_value)),
            (Designation::Y, "Prob>|t|".into(), numbers(&|q| q.prob_t)),
            (Designation::Y, format!("LCL ({level})"), numbers(&|q| q.lcl)),
            (Designation::Y, format!("UCL ({level})"), numbers(&|q| q.ucl)),
            (Designation::Y, "Dependency".into(), numbers(&|q| q.dependency)),
        ];
        for (designation, name, data) in columns {
            sheet_column(self, &mut param_ws, designation, &name, "", data);
        }
        let data_note = format!("{source_label} against {x_label}");
        let mut rows = stats_rows(&report, &[], &data_note, &rows_note);
        let at = rows.iter().position(|r| r.0 == "Status").unwrap_or(rows.len());
        rows.insert(
            at,
            (
                "Distance".into(),
                f64::NAN,
                "orthogonal (geometric) distance of each point to the curve, not the algebraic residual of f".into(),
            ),
        );
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
        let curve: Vec<(f64, f64)> = function.curve(&values, 360);
        sheet_column(
            self,
            &mut curve_ws,
            Designation::X,
            &x_name,
            &x_units,
            ColumnData::numeric(curve.iter().map(|c| c.0).collect()),
        );
        let cy = sheet_column(
            self,
            &mut curve_ws,
            Designation::Y,
            &format!("Fit of {y_name}"),
            &y_units,
            ColumnData::numeric(curve.iter().map(|c| c.1).collect()),
        );
        if let Some(c) = curve_ws.column_mut(cy) {
            c.comments = report.model.clone();
        }
        let b = self.workbook_mut(book_id).expect("workbook exists");
        b.sheets.push(param_ws);
        b.sheets.push(stats_ws);
        b.sheets.push(curve_ws);
        let count = b.sheets.len();
        let message = format!(
            "{} fit of {source_label} against {x_label}: {}; residual sum of squares {:.6}; report in sheet {param_name}",
            report.model,
            report.params.iter().map(|q| format!("{} = {:.6}", q.name, q.value)).collect::<Vec<_>>().join(", "),
            report.stats.rss,
        );
        Ok(ImplicitOutcome {
            report,
            book: book_id,
            param_sheet: count - 3,
            stats_sheet: count - 2,
            curve_sheet: count - 1,
            distance_column: distance_id,
            message,
        })
    }

    /// Compares the fits of one model to two datasets (the Y columns of `a` and `b`, each with its X column): both
    /// are fitted separately and pooled with all parameters shared, then compared with Sheaf's F-test
    /// (`sheaf_fit::compare_data`; Origin's report was not measured). The report goes to a new sheet
    /// `FitCompareData<n>` of the workbook of the first dataset.
    pub fn compare_datasets(
        &mut self,
        a: &FitRequest,
        b: &FitRequest,
        options: &CompareDatasetsOptions,
    ) -> Result<CompareDatasetsOutcome, FitRequestError> {
        if a.model != b.model || a.params != b.params {
            return Err(FitRequestError::BadOption(
                "both datasets must be fitted with the same model and the same parameter settings".into(),
            ));
        }
        if a.y == b.y && a.rows == b.rows {
            return Err(FitRequestError::BadOption("choose two different datasets".into()));
        }
        let data = [self.collect_fit_data(a)?, self.collect_fit_data(b)?];
        let fit_one = |d: &FitData| run_engine(&a.model, &a.params, d);
        let ra = fit_one(&data[0])?.report;
        let rb = fit_one(&data[1])?.report;
        let join = |p: &Option<Vec<f64>>, q: &Option<Vec<f64>>| match (p, q) {
            (Some(p), Some(q)) => Some(p.iter().chain(q).copied().collect::<Vec<f64>>()),
            _ => None,
        };
        let pooled = FitData {
            x: data[0].x.iter().chain(&data[1].x).copied().collect(),
            y: data[0].y.iter().chain(&data[1].y).copied().collect(),
            y_error: join(&data[0].y_error, &data[1].y_error),
            x_error: join(&data[0].x_error, &data[1].x_error),
            ..data[0].clone()
        };
        let rs = fit_one(&pooled)?.report;
        let comparison = compare_datasets(&ra, &rb, &rs)?;
        let (book_id, label_a, label_b) = {
            let (book, _, ca) = self.locate(a.y).ok_or(AnalysisError::NoSuchColumn(a.y))?;
            let (_, _, cb) = self.locate(b.y).ok_or(AnalysisError::NoSuchColumn(b.y))?;
            (book.id, column_label(ca), column_label(cb))
        };

        let nan = f64::NAN;
        let c = &comparison;
        let mut rows: Vec<Row> = Vec::new();
        let mut row = |q: &str, x: f64, y: f64, t: &str| rows.push((q.to_string(), x, y, t.to_string()));
        row("Function", nan, nan, &format!("{}: {}", ra.model, ra.equation));
        row("Data", nan, nan, &format!("{label_a} | {label_b}"));
        if options.parameters {
            for d in &c.differences {
                row(&d.name, d.value[0], d.value[1], "");
                row(&format!("{} Standard Error", d.name), d.std_error[0], d.std_error[1], "");
                row(
                    &format!("{} difference", d.name),
                    d.difference,
                    nan,
                    &format!("t = {:.6}, Prob>|t| = {:.6}", d.t, d.prob),
                );
            }
        }
        if options.statistics {
            row("Number of points", ra.stats.n_points as f64, rb.stats.n_points as f64, "");
            row("Degrees of freedom", ra.stats.dof as f64, rb.stats.dof as f64, "");
            row("Residual sum of squares", ra.stats.rss, rb.stats.rss, "");
            row("Reduced chi-square", ra.stats.reduced_chi_sqr, rb.stats.reduced_chi_sqr, "");
            row("R-square (COD)", ra.stats.r_squared, rb.stats.r_squared, "");
            row("Adj. R-square", ra.stats.adj_r_squared, rb.stats.adj_r_squared, "");
        }
        row(
            "Residual sum of squares, one parameter set",
            c.rss_shared,
            nan,
            &format!("both datasets fitted together with shared parameters ({} points)", c.n_shared),
        );
        row("F value", c.f, nan, "F-test: one parameter set fits both datasets (Sheaf's test, not Origin's)");
        row("F-test df1", c.df1 as f64, nan, "");
        row("F-test df2", c.df2 as f64, nan, "");
        row("Prob>F", c.prob, nan, &format!("significance level {ALPHA}"));
        let verdict = match c.different() {
            Some(true) => format!("The parameters of {} and {} are different", options.names[0], options.names[1]),
            Some(false) => format!("No significant difference between {} and {}", options.names[0], options.names[1]),
            None => "No conclusion".to_string(),
        };
        row("Conclusion", nan, nan, &verdict);
        for w in &comparison.warnings {
            row("Warning", nan, nan, w);
        }

        let sheets = &self.workbook(book_id).expect("workbook exists").sheets;
        let mut ws = Worksheet::new(free_sheet_name(sheets, "FitCompareData"));
        let name = ws.name.clone();
        let text = |f: fn(&Row) -> &str| ColumnData::text(rows.iter().map(|r| f(r).to_string()).collect());
        sheet_column(self, &mut ws, Designation::Label, "Quantity", "", text(|r| r.0.as_str()));
        sheet_column(
            self,
            &mut ws,
            Designation::Y,
            &options.names[0],
            "",
            ColumnData::numeric(rows.iter().map(|r| r.1).collect()),
        );
        sheet_column(
            self,
            &mut ws,
            Designation::Y,
            &options.names[1],
            "",
            ColumnData::numeric(rows.iter().map(|r| r.2).collect()),
        );
        sheet_column(self, &mut ws, Designation::Label, "Text", "", text(|r| r.3.as_str()));
        let bk = self.workbook_mut(book_id).expect("workbook exists");
        bk.sheets.push(ws);
        let sheet = bk.sheets.len() - 1;
        let message = format!(
            "Compared {} and {}: {verdict} (F = {:.6}, Prob>F = {:.6}); report in sheet {name}",
            options.names[0], options.names[1], c.f, c.prob
        );
        Ok(CompareDatasetsOutcome { reports: vec![ra, rb, rs], comparison, book: book_id, sheet, message })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::column::{ColumnData, Designation};

    #[test]
    fn the_implicit_fit_leaves_masked_cells_out() {
        // T256: a masked cell is a missing value, as in the main fit.
        let mut p = Project::new();
        let book = p.add_workbook("circle");
        let t: Vec<f64> = (0..24).map(|i| f64::from(i) * std::f64::consts::TAU / 24.0).collect();
        let xs: Vec<f64> =
            t.iter().enumerate().map(|(i, a)| 2.0 + 3.0 * a.cos() + 0.02 * ((i * 7 % 5) as f64 - 2.0)).collect();
        let ys: Vec<f64> = t.iter().map(|a| -1.0 + 3.0 * a.sin()).collect();
        let cx = p.add_column(book, 0, Designation::X, ColumnData::numeric(xs)).unwrap();
        let cy = p.add_column(book, 0, Designation::Y, ColumnData::numeric(ys)).unwrap();
        let circle = |mut q: Project| {
            let out = q.fit_implicit(&ImplicitRequest::new(cx, cy, "Circle")).unwrap();
            format!("{:?}", out.report.params.iter().map(|r| r.value).collect::<Vec<_>>())
        };
        let mut gone = p.clone();
        gone.column_mut(cy).unwrap().numeric_mut().unwrap()[5] = f64::NAN;
        let want = circle(gone);
        assert_ne!(circle(p.clone()), want);
        let mut masked = p.clone();
        masked.column_mut(cy).unwrap().mask.insert(5);
        assert_eq!(circle(masked), want);
    }
}
