//! Multiple linear regression, model comparison and model ranking on worksheet columns
//! (`docs/spec/fitting.md`, sections 11 and 12). The numerics are in `sheaf_fit::multi` and
//! `sheaf_fit::compare`; this file gathers the columns and places the results in the workbook.
//!
//! Nothing changes in the project when one of these fails.

use sheaf_fit::compare::{Comparison, F_TEST_ALPHA, ModelSummary, Ranked, Verdict, compare, rank};
use sheaf_fit::multi::{MultiOptions, fit_multiple};

use super::{FitModel, FitReport, FitRequest, FitRequestError, Project, run_engine, sheet_column, stats_rows};
use crate::analysis::{AnalysisError, RowRange, column_label, free_sheet_name, resolve_rows};
use crate::column::{Column, ColumnData, Designation};
use crate::ids::{ColumnId, WindowId};
use crate::worksheet::Worksheet;

/// A multiple linear regression `y = b0 + b1 x1 + ... + bp xp`.
#[derive(Debug, Clone, PartialEq)]
pub struct MultiRegRequest {
    pub y: ColumnId,
    /// The independent variables, in the order of the coefficients `b1`, `b2`...
    pub x: Vec<ColumnId>,
    /// Hold the intercept at this value.
    pub fix_intercept: Option<f64>,
    /// Rows counted from 1, as the worksheet shows them.
    pub rows: RowRange,
    pub confidence_level: f64,
}

impl MultiRegRequest {
    pub fn new(y: ColumnId, x: Vec<ColumnId>) -> Self {
        MultiRegRequest { y, x, fix_intercept: None, rows: RowRange::All, confidence_level: 0.95 }
    }
}

/// What [`Project::fit_multiple_regression`] did.
#[derive(Debug, Clone, PartialEq)]
pub struct MultiRegOutcome {
    pub report: FitReport,
    pub book: WindowId,
    pub param_sheet: usize,
    pub stats_sheet: usize,
    pub fitted_column: ColumnId,
    pub residual_column: ColumnId,
    pub message: String,
}

/// What [`Project::compare_models`] did.
#[derive(Debug, Clone, PartialEq)]
pub struct CompareOutcome {
    pub comparison: Comparison,
    /// The two fits' reports, in request order.
    pub reports: Vec<FitReport>,
    pub book: WindowId,
    pub sheet: usize,
    pub message: String,
}

/// What [`Project::rank_models`] did.
#[derive(Debug, Clone, PartialEq)]
pub struct RankOutcome {
    /// Best model first.
    pub ranking: Vec<Ranked>,
    /// Functions that could not be fitted, with the reason.
    pub skipped: Vec<(String, String)>,
    pub book: WindowId,
    pub sheet: usize,
    pub message: String,
}

fn verdict_text(v: Verdict, names: &[String; 2]) -> String {
    match v {
        Verdict::Model(i) => format!("{} is preferred", names[i]),
        Verdict::Inconclusive => "Inconclusive".to_string(),
        Verdict::NoConclusion => "No conclusion".to_string(),
    }
}

impl Project {
    /// Fits a plane (or hyperplane) to several independent columns and places the parameter and
    /// statistics sheets, and the fitted values and residuals next to the Y column.
    pub fn fit_multiple_regression(&mut self, request: &MultiRegRequest) -> Result<MultiRegOutcome, FitRequestError> {
        if !(request.confidence_level > 0.0 && request.confidence_level < 1.0) {
            return Err(FitRequestError::BadConfidence(request.confidence_level));
        }
        if request.x.is_empty() {
            return Err(FitRequestError::BadOption("choose at least one independent variable".into()));
        }
        let id = request.y;
        let (book, sheet, column) = self.locate(id).ok_or(AnalysisError::NoSuchColumn(id))?;
        let book_id = book.id;
        let sheet_index = book.sheets.iter().position(|s| s.column(id).is_some()).expect("located here");
        let y_all = column.numeric().ok_or_else(|| AnalysisError::NotNumeric(column.short_name.clone()))?.to_vec();
        if y_all.is_empty() {
            return Err(AnalysisError::Empty(column.short_name.clone()).into());
        }
        let n_rows = y_all.len();
        let source_label = column_label(column);
        let source_name = column.display_name().to_string();
        let y_units = column.units.clone();
        let mut xs_all: Vec<Vec<f64>> = Vec::new();
        let mut names: Vec<String> = Vec::new();
        for &xid in &request.x {
            let (_, _, c) = self.locate(xid).ok_or(AnalysisError::NoSuchColumn(xid))?;
            let v = c.numeric().ok_or_else(|| AnalysisError::NotNumeric(c.short_name.clone()))?;
            xs_all.push((0..n_rows).map(|i| v.get(i).copied().unwrap_or(f64::NAN)).collect());
            names.push(c.display_name().to_string());
        }
        let _ = sheet;
        let index: Vec<f64> = (1..=n_rows).map(|i| i as f64).collect();
        let (first, last) = resolve_rows(request.rows, &source_name, &index, n_rows)?;
        let xs: Vec<Vec<f64>> = xs_all.iter().map(|v| v[first..=last].to_vec()).collect();
        let y = y_all[first..=last].to_vec();
        let rows_note = if first == 0 && last + 1 == n_rows {
            format!("all {n_rows} rows")
        } else {
            format!("{} to {}", first + 1, last + 1)
        };

        let opts = MultiOptions { fix_intercept: request.fix_intercept, confidence_level: request.confidence_level };
        let report = fit_multiple(&xs, &names, &y, &opts)?;

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
            let mut at = ws.column_index(id).expect("source column exists") + 1;
            while at < ws.columns.len()
                && matches!(ws.columns[at].designation, Designation::YError | Designation::XError | Designation::Label)
            {
                at += 1;
            }
            let fitted_name = ws.next_short_name();
            ws.columns.insert(
                at,
                Column::new(fitted_id, fitted_name, Designation::Y, ColumnData::numeric(align(&report.fitted)))
                    .with_labels(&format!("Fitted {source_name}"), &y_units, &report.model)
                    .as_fit_result(),
            );
            let residual_name = ws.next_short_name();
            ws.columns.insert(
                at + 1,
                Column::new(residual_id, residual_name, Designation::Y, ColumnData::numeric(align(&report.residuals)))
                    .with_labels(&format!("Residual of {source_name}"), &y_units, &report.model)
                    .as_fit_result(),
            );
        }

        let sheets = &self.workbook(book_id).expect("workbook exists").sheets;
        let taken = |name: &str| sheets.iter().any(|s| s.name.eq_ignore_ascii_case(name));
        let param_name = (1..)
            .map(|n| format!("FitMR{n}"))
            .find(|name| !taken(name) && !taken(&format!("{name}Stats")))
            .expect("the name sequence is unbounded");
        let mut param_ws = Worksheet::new(param_name.clone());
        let mut stats_ws = Worksheet::new(format!("{param_name}Stats"));
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
            (
                Designation::Label,
                "Fixed".into(),
                ColumnData::text(p.iter().map(|q| if q.fixed { "Fixed".into() } else { String::new() }).collect()),
            ),
        ];
        for (designation, name, data) in columns {
            sheet_column(self, &mut param_ws, designation, &name, "", data);
        }
        let rows = stats_rows(&report, &[], &source_label, &rows_note);
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
        let b = self.workbook_mut(book_id).expect("workbook exists");
        b.sheets.push(param_ws);
        let param_sheet = b.sheets.len() - 1;
        b.sheets.push(stats_ws);

        let message = format!(
            "Multiple regression of {source_label} on {} variable{}: {} points, R\u{b2} = {}; coefficients in sheet {param_name}",
            request.x.len(),
            if request.x.len() == 1 { "" } else { "s" },
            report.stats.n_points,
            crate::format::format_general(report.stats.r_squared, 6),
        );
        Ok(MultiRegOutcome {
            report,
            book: book_id,
            param_sheet,
            stats_sheet: param_sheet + 1,
            fitted_column: fitted_id,
            residual_column: residual_id,
            message,
        })
    }

    /// Fits each request's model to its column and returns the reports (nothing is placed).
    fn fit_for_comparison(&self, requests: &[FitRequest]) -> Vec<Result<(String, FitReport), String>> {
        requests
            .iter()
            .map(|r| {
                let label = model_label(&r.model);
                let data = self.collect_fit_data(r).map_err(|e| e.to_string())?;
                let fitted = run_engine(&r.model, &r.params, &data).map_err(|e| e.to_string())?;
                Ok((label, fitted.report))
            })
            .collect()
    }

    /// Fits two models to the same data and compares them: corrected Akaike information
    /// criterion with Akaike weights, Bayesian information criterion and the F-test, as Origin's
    /// Compare Models report gives them. The report goes to a new sheet `FitCompare<n>`.
    ///
    /// Both requests need the same Y column (the other settings of each give its model).
    pub fn compare_models(&mut self, a: &FitRequest, b: &FitRequest) -> Result<CompareOutcome, FitRequestError> {
        if a.y != b.y {
            return Err(FitRequestError::BadOption("both models must be fitted to the same Y column".into()));
        }
        let fits = self.fit_for_comparison(&[a.clone(), b.clone()]);
        let mut done = Vec::new();
        for f in fits {
            done.push(f.map_err(FitRequestError::BadOption)?);
        }
        let (book_id, source_label) = {
            let (book, _, column) = self.locate(a.y).ok_or(AnalysisError::NoSuchColumn(a.y))?;
            (book.id, column_label(column))
        };
        let summaries: Vec<ModelSummary> = done
            .iter()
            .map(|(label, r)| {
                let mut s = ModelSummary::from_report(r);
                s.name = label.clone();
                s
            })
            .collect();
        let comparison = compare(&summaries[0], &summaries[1]);
        let names = [summaries[0].name.clone(), summaries[1].name.clone()];

        let nan = f64::NAN;
        let c = &comparison;
        let mut rows: Vec<(String, f64, f64, String)> = Vec::new();
        let mut row = |q: &str, x: f64, y: f64, t: &str| rows.push((q.to_string(), x, y, t.to_string()));
        row("Function", nan, nan, &format!("{} | {}", names[0], names[1]));
        row("Number of points", c.models[0].n as f64, c.models[1].n as f64, "");
        row("Number of parameters", c.models[0].params as f64, c.models[1].params as f64, "");
        row("Degrees of freedom", c.models[0].dof() as f64, c.models[1].dof() as f64, "");
        row("Residual sum of squares", c.models[0].rss, c.models[1].rss, "");
        row("AICc", c.aic[0], c.aic[1], "corrected Akaike information criterion");
        row("Akaike weight", c.aic_weight[0], c.aic_weight[1], "");
        row("AIC conclusion", nan, nan, &verdict_text(c.aic_verdict, &names));
        row("BIC", c.bic[0], c.bic[1], "");
        row("BIC minus the smaller BIC", c.bic_delta[0], c.bic_delta[1], "");
        row("BIC conclusion", nan, nan, &verdict_text(c.bic_verdict, &names));
        let null_text = c.ftest.null_model.map(|i| format!("null model: {}", names[i])).unwrap_or_default();
        row("F value", c.ftest.f, nan, &null_text);
        row("F-test df1", c.ftest.df1 as f64, nan, "");
        row("F-test df2", c.ftest.df2 as f64, nan, "");
        row("Prob>F", c.ftest.prob, nan, &format!("significance level {F_TEST_ALPHA}"));
        row("F-test conclusion", nan, nan, &verdict_text(c.ftest.verdict, &names));
        for w in &comparison.warnings {
            row("Warning", nan, nan, w);
        }

        let sheets = &self.workbook(book_id).expect("workbook exists").sheets;
        let mut ws = Worksheet::new(free_sheet_name(sheets, "FitCompare"));
        let name = ws.name.clone();
        sheet_column(
            self,
            &mut ws,
            Designation::Label,
            "Quantity",
            "",
            ColumnData::text(rows.iter().map(|r| r.0.clone()).collect()),
        );
        sheet_column(
            self,
            &mut ws,
            Designation::Y,
            "Model 1",
            "",
            ColumnData::numeric(rows.iter().map(|r| r.1).collect()),
        );
        sheet_column(
            self,
            &mut ws,
            Designation::Y,
            "Model 2",
            "",
            ColumnData::numeric(rows.iter().map(|r| r.2).collect()),
        );
        sheet_column(
            self,
            &mut ws,
            Designation::Label,
            "Text",
            "",
            ColumnData::text(rows.iter().map(|r| r.3.clone()).collect()),
        );
        let b = self.workbook_mut(book_id).expect("workbook exists");
        b.sheets.push(ws);
        let sheet = b.sheets.len() - 1;
        let message = format!(
            "Compared {} and {} on {source_label}: AICc prefers {}, F-test says {}; report in sheet {name}",
            names[0],
            names[1],
            match comparison.aic_verdict {
                Verdict::Model(i) => names[i].as_str(),
                _ => "neither",
            },
            verdict_text(comparison.ftest.verdict, &names),
        );
        Ok(CompareOutcome {
            reports: done.into_iter().map(|d| d.1).collect(),
            comparison,
            book: book_id,
            sheet,
            message,
        })
    }

    /// Fits each of the named built-in functions to the request's column and ranks them by AICc
    /// (best first), with Akaike weights, BIC and adjusted R-square. A function that cannot be
    /// fitted is skipped and listed. The table goes to a new sheet `FitRank<n>`.
    pub fn rank_models(&mut self, request: &FitRequest, functions: &[String]) -> Result<RankOutcome, FitRequestError> {
        if functions.len() < 2 {
            return Err(FitRequestError::BadOption("choose at least two functions to rank".into()));
        }
        let requests: Vec<FitRequest> = functions
            .iter()
            .map(|f| {
                let mut r = request.clone();
                r.model = FitModel::BuiltIn(f.clone());
                r.params = Vec::new();
                r
            })
            .collect();
        let fits = self.fit_for_comparison(&requests);
        let mut summaries = Vec::new();
        let mut adj = Vec::new();
        let mut skipped = Vec::new();
        for (f, fit) in functions.iter().zip(fits) {
            match fit {
                Ok((label, r)) if r.stats.rss.is_finite() => {
                    let mut s = ModelSummary::from_report(&r);
                    s.name = label;
                    adj.push((f.clone(), r.stats.adj_r_squared));
                    summaries.push(s);
                }
                Ok(_) => skipped.push((f.clone(), "the fit gave no finite residual sum of squares".to_string())),
                Err(e) => skipped.push((f.clone(), e)),
            }
        }
        if summaries.len() < 2 {
            let why = skipped.iter().map(|(f, e)| format!("{f}: {e}")).collect::<Vec<_>>().join("; ");
            return Err(FitRequestError::BadOption(format!("fewer than two of the functions could be fitted ({why})")));
        }
        let ranking = rank(&summaries);
        let (book_id, source_label) = {
            let (book, _, column) = self.locate(request.y).ok_or(AnalysisError::NoSuchColumn(request.y))?;
            (book.id, column_label(column))
        };
        let sheets = &self.workbook(book_id).expect("workbook exists").sheets;
        let mut ws = Worksheet::new(free_sheet_name(sheets, "FitRank"));
        let name = ws.name.clone();
        let adj_of = |model: &str| adj.iter().find(|(f, _)| f == model).map_or(f64::NAN, |a| a.1);
        let col = |f: &dyn Fn(&Ranked) -> f64| ColumnData::numeric(ranking.iter().map(f).collect());
        sheet_column(
            self,
            &mut ws,
            Designation::Y,
            "Rank",
            "",
            ColumnData::numeric((1..=ranking.len()).map(|i| i as f64).collect()),
        );
        sheet_column(
            self,
            &mut ws,
            Designation::Label,
            "Function",
            "",
            ColumnData::text(ranking.iter().map(|r| r.summary.name.clone()).collect()),
        );
        sheet_column(self, &mut ws, Designation::Y, "Parameters", "", col(&|r| r.summary.params as f64));
        sheet_column(self, &mut ws, Designation::Y, "Residual Sum of Squares", "", col(&|r| r.summary.rss));
        sheet_column(self, &mut ws, Designation::Y, "Adj. R-Square", "", col(&|r| adj_of(&r.summary.name)));
        sheet_column(self, &mut ws, Designation::Y, "AICc", "", col(&|r| r.aic));
        sheet_column(self, &mut ws, Designation::Y, "AICc minus the smallest", "", col(&|r| r.aic_delta));
        sheet_column(self, &mut ws, Designation::Y, "Akaike weight", "", col(&|r| r.aic_weight));
        sheet_column(self, &mut ws, Designation::Y, "BIC", "", col(&|r| r.bic));
        sheet_column(self, &mut ws, Designation::Y, "BIC minus the smallest", "", col(&|r| r.bic_delta));
        if !skipped.is_empty() {
            let text: Vec<String> = skipped.iter().map(|(f, e)| format!("{f}: {e}")).collect();
            let n = text.len();
            let mut padded = text;
            padded.resize(ranking.len().max(n), String::new());
            sheet_column(self, &mut ws, Designation::Label, "Skipped", "", ColumnData::text(padded));
        }
        let b = self.workbook_mut(book_id).expect("workbook exists");
        b.sheets.push(ws);
        let sheet = b.sheets.len() - 1;
        let message = format!(
            "Ranked {} functions on {source_label}: best is {}{}; table in sheet {name}",
            ranking.len(),
            ranking[0].summary.name,
            if skipped.is_empty() { String::new() } else { format!(" ({} skipped)", skipped.len()) },
        );
        Ok(RankOutcome { ranking, skipped, book: book_id, sheet, message })
    }
}

/// The model's name as the comparison and ranking sheets show it.
fn model_label(m: &FitModel) -> String {
    match m {
        FitModel::Linear => "Linear".into(),
        FitModel::LinearXError => "Linear (X error)".into(),
        FitModel::Polynomial { order } => format!("Polynomial (order {order})"),
        FitModel::BuiltIn(n) => n.clone(),
        FitModel::Formula { .. } => "Formula".into(),
        FitModel::Fdf(_) => "FDF".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project() -> (Project, ColumnId, ColumnId, ColumnId, ColumnId) {
        let mut p = Project::new();
        let book = p.add_workbook("data");
        let xs: Vec<f64> = (1..=20).map(f64::from).collect();
        let x2: Vec<f64> = xs.iter().map(|x| (x * 0.7).sin() * 3.0).collect();
        let y: Vec<f64> = xs
            .iter()
            .zip(&x2)
            .enumerate()
            .map(|(i, (a, b))| 1.0 + 2.0 * a - 0.5 * b + 0.05 * ((i * 7 % 5) as f64 - 2.0))
            .collect();
        let dec: Vec<f64> = xs.iter().map(|x| 0.5 + 3.0 * (-x / 4.0).exp() + 0.01 * ((x * 9.0).sin())).collect();
        let cx = p.add_column(book, 0, Designation::X, ColumnData::numeric(xs)).unwrap();
        let c2 = p.add_column(book, 0, Designation::Y, ColumnData::numeric(x2)).unwrap();
        let cy = p.add_column(book, 0, Designation::Y, ColumnData::numeric(y)).unwrap();
        let cd = p.add_column(book, 0, Designation::Y, ColumnData::numeric(dec)).unwrap();
        (p, cx, c2, cy, cd)
    }

    #[test]
    fn multiple_regression_places_sheets_and_recovers_the_coefficients() {
        let (mut p, cx, c2, cy, _) = project();
        let out = p.fit_multiple_regression(&MultiRegRequest::new(cy, vec![cx, c2])).unwrap();
        let v = |n: &str| out.report.params.iter().find(|q| q.name == n).unwrap().value;
        assert!((out.report.params[1].value - 2.0).abs() < 0.05, "{}", v("x1"));
        let b = p.workbook(out.book).unwrap();
        assert_eq!(b.sheets[out.param_sheet].name, "FitMR1");
        assert_eq!(b.sheets[out.stats_sheet].name, "FitMR1Stats");
        let again = p.fit_multiple_regression(&MultiRegRequest::new(cy, vec![cx])).unwrap();
        assert_eq!(p.workbook(again.book).unwrap().sheets[again.param_sheet].name, "FitMR2");
        let mut req = MultiRegRequest::new(cy, vec![cx]);
        req.fix_intercept = Some(0.0);
        let fixed = p.fit_multiple_regression(&req).unwrap();
        assert!(fixed.report.params[0].fixed);
        assert!(p.fit_multiple_regression(&MultiRegRequest::new(cy, vec![])).is_err());
    }

    #[test]
    fn compare_models_writes_the_report_and_prefers_the_right_model() {
        let (mut p, _, _, _, cd) = project();
        let a = FitRequest::new(cd, FitModel::BuiltIn("ExpDec1".into()));
        let b = FitRequest::new(cd, FitModel::Polynomial { order: 1 });
        let out = p.compare_models(&a, &b).unwrap();
        assert_eq!(out.comparison.aic_verdict, Verdict::Model(0));
        let ws = &p.workbook(out.book).unwrap().sheets[out.sheet];
        assert_eq!(ws.name, "FitCompare1");
        let other = FitRequest::new(ColumnId(9999), FitModel::Linear);
        assert!(p.compare_models(&a, &other).is_err());
    }

    #[test]
    fn rank_models_orders_the_functions_and_lists_the_ones_that_failed() {
        let (mut p, _, _, _, cd) = project();
        let req = FitRequest::new(cd, FitModel::Linear);
        let names = ["ExpDec1".to_string(), "Gauss".to_string(), "NoSuchFunction".to_string()];
        let out = p.rank_models(&req, &names).unwrap();
        assert_eq!(out.ranking[0].summary.name, "ExpDec1");
        assert_eq!(out.skipped.len(), 1);
        assert_eq!(p.workbook(out.book).unwrap().sheets[out.sheet].name, "FitRank1");
        assert!(p.rank_models(&req, &names[..1]).is_err());
    }

    #[test]
    fn linear_fit_with_x_error_needs_the_error_column() {
        let mut p = Project::new();
        let book = p.add_workbook("data");
        let xs: Vec<f64> = (1..=10).map(f64::from).collect();
        let y: Vec<f64> = xs.iter().map(|x| 1.0 + 2.0 * x + 0.1 * (x * 3.0).sin()).collect();
        p.add_column(book, 0, Designation::X, ColumnData::numeric(xs)).unwrap();
        let cy = p.add_column(book, 0, Designation::Y, ColumnData::numeric(y)).unwrap();
        let req = FitRequest::new(cy, FitModel::LinearXError);
        assert!(matches!(p.clone().fit(&req), Err(FitRequestError::NoXErrorColumn(_))));
        let ex = p.add_column(book, 0, Designation::XError, ColumnData::numeric(vec![0.1; 10])).unwrap();
        let out = p.fit(&req).unwrap();
        assert!((out.report.param("Slope").unwrap().value - 2.0).abs() < 0.1);
        let mut with_id = req.clone();
        with_id.x_error = Some(ex);
        assert!(p.fit(&with_id).is_ok());
    }
}
