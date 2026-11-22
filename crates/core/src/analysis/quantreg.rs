//! Statistics > Regression > Quantile Regression as a tool of `Project::analyze` (T342; Origin's Quantile
//! Regression app of Origin 2019b, not in Origin 2016, so the layout is Sheaf's own). The numerics are in
//! `sheaf_calc::quantreg`; `docs/spec/statistics.md` section 25 says what is computed and written.
//!
//! The chosen columns are the independent variables followed, as the last one, by the dependent variable
//! (as the ROC curve takes its state column last).

use super::dist::{GraphPlot, Output, XyGraph};
use super::statistics::num;
use super::*;
use crate::graph::{PlotKind, ScaleKind};
use calc::quantreg::quantile_regression;

/// What [`Tool::QuantReg`] does.
#[derive(Debug, Clone, PartialEq)]
pub struct QuantRegSpec {
    /// The quantiles to fit, each between 0 and 1 (default 0.1, 0.25, 0.5, 0.75 and 0.9).
    pub quantiles: Vec<f64>,
    /// Fit an intercept (default on).
    pub intercept: bool,
    /// With one independent variable, make a graph of the data with one fitted line per quantile.
    pub graph: bool,
}

impl Default for QuantRegSpec {
    fn default() -> Self {
        QuantRegSpec { quantiles: vec![0.1, 0.25, 0.5, 0.75, 0.9], intercept: true, graph: true }
    }
}

fn column_of(name: &str, comments: &str, designation: Designation, data: ColumnData) -> SheetColumn {
    SheetColumn { designation, long_name: name.to_string(), units: String::new(), comments: comments.to_string(), data }
}

pub(super) fn run(
    spec: &QuantRegSpec,
    sheet: &Worksheet,
    columns: &[ColumnId],
    rows: RowRange,
) -> Result<Output, AnalysisError> {
    if spec.quantiles.is_empty() {
        return Err(AnalysisError::BadOption("give at least one quantile".into()));
    }
    if let Some(bad) = spec.quantiles.iter().find(|t| !(**t > 0.0 && **t < 1.0)) {
        return Err(AnalysisError::BadOption(format!("a quantile must lie between 0 and 1 (got {bad})")));
    }
    if columns.len() < 2 {
        return Err(AnalysisError::BadOption(
            "quantile regression needs the independent column(s) first and the dependent column last".into(),
        ));
    }
    let column = |id: ColumnId| sheet.column(id).ok_or(AnalysisError::NoSuchColumn(id));
    let (y_id, x_ids) = columns.split_last().expect("two columns");
    let yc = column(*y_id)?;
    let read = |c: &Column| -> Result<Vec<f64>, AnalysisError> {
        sheet.values_of(c).map(|v| v.into_owned()).ok_or_else(|| AnalysisError::NotNumeric(c.short_name.clone()))
    };
    let y_all = read(yc)?;
    let xcs: Vec<&Column> = x_ids.iter().map(|id| column(*id)).collect::<Result<_, _>>()?;
    let x_all: Vec<Vec<f64>> = xcs.iter().map(|c| read(c)).collect::<Result<_, _>>()?;
    let n = x_all.iter().map(Vec::len).chain([y_all.len()]).max().unwrap_or(0);
    if n == 0 {
        return Err(AnalysisError::Empty(yc.short_name.clone()));
    }
    let (first, last) = resolve_rows(rows, &yc.short_name, &x_all[0], n)?;
    let cut = |v: &[f64]| -> Vec<f64> { (first..=last).map(|r| v.get(r).copied().unwrap_or(f64::NAN)).collect() };
    let y = cut(&y_all);
    let xs: Vec<Vec<f64>> = x_all.iter().map(|v| cut(v)).collect();
    let x_refs: Vec<&[f64]> = xs.iter().map(Vec::as_slice).collect();

    let names: Vec<String> = xcs.iter().map(|c| column_label(c)).collect();
    let mut headers: Vec<String> = vec!["N".into()];
    if spec.intercept {
        headers.push("Intercept".into());
    }
    headers.extend(names.iter().cloned());
    headers.extend(["Sum of check losses".to_string(), "Pseudo R-square".to_string()]);
    let header_refs: Vec<&str> = headers.iter().map(String::as_str).collect();
    let mut table = ResultTable::new(&header_refs);
    let mut notes = Vec::new();
    let mut fits = Vec::new();
    for &tau in &spec.quantiles {
        let r = quantile_regression(&x_refs, &y, tau, spec.intercept)
            .map_err(|e| AnalysisError::BadOption(format!("quantile {tau}: {e}")))?;
        let mut values = vec![r.n as f64];
        values.extend(r.coefficients.iter().copied());
        values.extend([r.loss, r.pseudo_r2]);
        table.push_row(num(tau), values, vec![String::new(); headers.len()]);
        let terms: Vec<String> = r
            .coefficients
            .iter()
            .skip(usize::from(spec.intercept))
            .zip(&names)
            .map(|(b, name)| format!("{} {name}", num(*b)))
            .collect();
        let intercept = if spec.intercept { format!("{} + ", num(r.coefficients[0])) } else { String::new() };
        notes.push(format!(
            "Quantile {}: {} = {intercept}{} ({} points, sum of check losses {}).",
            num(tau),
            column_label(yc),
            terms.join(" + "),
            r.n,
            num(r.loss)
        ));
        fits.push(r);
    }
    notes.push(
        "Coefficients minimize the sum of the check losses (the exact linear-programme solution); no standard errors \
         are computed. Pseudo R-square is Koenker and Machado's 1 - loss / loss of the intercept-only model."
            .into(),
    );

    let mut on_sheet = vec![column_of(
        "Quantile",
        "",
        Designation::Label,
        ColumnData::text(table.rows.iter().map(|(l, _)| l.clone()).collect()),
    )];
    for (i, h) in table.headers.iter().enumerate() {
        on_sheet.push(column_of(
            h,
            "",
            Designation::Y,
            ColumnData::numeric(table.rows.iter().map(|(_, v)| v[i]).collect()),
        ));
    }
    let mut graph = None;
    if spec.graph && xs.len() == 1 {
        let used: Vec<(f64, f64)> =
            xs[0].iter().zip(&y).filter(|(a, b)| a.is_finite() && b.is_finite()).map(|(a, b)| (*a, *b)).collect();
        let (lo, hi) = used.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(l, h), (a, _)| (l.min(*a), h.max(*a)));
        let mut plots = Vec::new();
        let x = on_sheet.len();
        on_sheet.push(column_of(
            &names[0],
            "Data",
            Designation::X,
            ColumnData::numeric(used.iter().map(|p| p.0).collect()),
        ));
        on_sheet.push(column_of(
            &column_label(yc),
            "Data",
            Designation::Y,
            ColumnData::numeric(used.iter().map(|p| p.1).collect()),
        ));
        plots.push(GraphPlot { x, y: x + 1, kind: PlotKind::Scatter, label: column_label(yc), line_of: None });
        for r in &fits {
            let (a, b) = if spec.intercept { (r.coefficients[0], r.coefficients[1]) } else { (0.0, r.coefficients[0]) };
            let label = format!("Quantile {}", num(r.tau));
            let x = on_sheet.len();
            on_sheet.push(column_of(&names[0], &label, Designation::X, ColumnData::numeric(vec![lo, hi])));
            on_sheet.push(column_of(&label, &label, Designation::Y, ColumnData::numeric(vec![a + b * lo, a + b * hi])));
            plots.push(GraphPlot { x, y: x + 1, kind: PlotKind::Line, label, line_of: None });
        }
        graph = Some(XyGraph {
            plots,
            x_scale: ScaleKind::Linear,
            y_scale: ScaleKind::Linear,
            x_title: names[0].clone(),
            y_title: column_label(yc),
            look: Default::default(),
        });
    }
    Ok(Output { table: Some(table), on_sheet, notes, graph, more_graphs: Vec::new() })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(columns: Vec<ColumnId>, spec: QuantRegSpec) -> Request {
        Request { columns, rows: RowRange::All, tool: Tool::QuantReg(spec) }
    }

    fn project() -> (Project, ColumnId, ColumnId) {
        let mut p = Project::new();
        let book = p.add_workbook("qr");
        let xs: Vec<f64> = (1..=40).map(|i| f64::from(i) / 4.0).collect();
        let ys: Vec<f64> = (1..=40).map(|i| 1.0 + 0.5 * f64::from(i) / 4.0 + (f64::from(i) * 7.3).sin()).collect();
        let x = p.add_column(book, 0, Designation::X, ColumnData::numeric(xs)).unwrap();
        let y = p.add_column(book, 0, Designation::Y, ColumnData::numeric(ys)).unwrap();
        (p, x, y)
    }

    #[test]
    fn one_row_per_quantile_and_a_graph_of_the_lines() {
        let (mut p, x, y) = project();
        let spec = QuantRegSpec { quantiles: vec![0.25, 0.5, 0.75], ..Default::default() };
        let out = p.analyze(&request(vec![x, y], spec)).unwrap();
        let t = out.table.as_ref().unwrap();
        assert_eq!(t.headers, ["N", "Intercept", "A", "Sum of check losses", "Pseudo R-square"]);
        assert_eq!(t.rows.iter().map(|(l, _)| l.as_str()).collect::<Vec<_>>(), ["0.25", "0.5", "0.75"]);
        // The lines are ordered: the slope is about 0.5 and the 0.75 line lies above the 0.25 line.
        let (q1, q3) = (&t.rows[0].1, &t.rows[2].1);
        assert_eq!(q1[0], 40.0);
        assert!(q3[1] > q1[1] && (q1[2] - 0.5).abs() < 0.1 && q1[4] > 0.0 && q1[4] < 1.0, "{q1:?} {q3:?}");
        let check =
            calc::quantreg::quantile_regression(&[p.numeric(x).unwrap()], p.numeric(y).unwrap(), 0.5, true).unwrap();
        assert_eq!(t.rows[1].1[1..3], check.coefficients[..]);
        assert!(out.new_graph.is_some(), "one X column gets a graph");
    }

    #[test]
    fn bad_requests_are_refused() {
        let (mut p, x, y) = project();
        let bad = QuantRegSpec { quantiles: vec![1.5], ..Default::default() };
        assert!(p.analyze(&request(vec![x, y], bad)).is_err());
        let none = QuantRegSpec { quantiles: vec![], ..Default::default() };
        assert!(p.analyze(&request(vec![x, y], none)).is_err());
        assert!(p.analyze(&request(vec![y], QuantRegSpec::default())).is_err());
    }
}
