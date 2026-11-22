//! Statistics > Autocorrelation (ACF and PACF) as a tool of `Project::analyze` (T363; Origin 2024b's Time Series
//! Analysis app, "inspect autocorrelation", not in Origin 2016, so the layout is Sheaf's own). The numerics are in
//! `sheaf_calc::acf`; `docs/spec/statistics.md` section 34 says what is computed and written.
//!
//! Every chosen column is a series of its own, in row order with missing values left out.

use super::dist::{GraphPlot, Output, XyGraph};
use super::statistics::num;
use super::*;
use crate::graph::{PlotKind, ScaleKind};
use calc::acf::{autocorrelation, default_lags};

/// What [`Tool::Acf`] does.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AcfSpec {
    /// The largest lag; 0 picks `min(10 log10 n, n - 1)` for each series.
    pub lags: usize,
    /// The confidence level of the bounds (default 0.95).
    pub confidence: f64,
    /// Make the ACF and PACF graphs.
    pub graph: bool,
}

impl Default for AcfSpec {
    fn default() -> Self {
        AcfSpec { lags: 0, confidence: 0.95, graph: true }
    }
}

fn column_of(name: &str, comments: &str, designation: Designation, values: Vec<f64>) -> SheetColumn {
    SheetColumn {
        designation,
        long_name: name.to_string(),
        units: String::new(),
        comments: comments.to_string(),
        data: ColumnData::numeric(values),
    }
}

pub(super) fn run(spec: &AcfSpec, inputs: &[Input<'_>]) -> Result<Output, AnalysisError> {
    if !(spec.confidence > 0.0 && spec.confidence < 1.0) {
        return Err(AnalysisError::BadOption("the confidence level must lie between 0 and 1".into()));
    }
    let z = calc::standard_quantile(calc::Family::Normal, 1.0, 0.5 + spec.confidence / 2.0);
    let pct = num(100.0 * spec.confidence);
    let mut table = ResultTable::new(&[
        "Lag",
        "ACF",
        "ACF standard error",
        "ACF lower bound",
        "ACF upper bound",
        "PACF",
        "PACF bound",
        "Ljung-Box Q",
        "Prob>Q",
    ]);
    let mut notes = Vec::new();
    let mut on_sheet = Vec::new();
    let (mut acf_plots, mut pacf_plots) = (Vec::new(), Vec::new());
    for input in inputs {
        let values: Vec<f64> = input.ys().iter().copied().filter(|v| v.is_finite()).collect();
        let lags = if spec.lags == 0 { default_lags(values.len()) } else { spec.lags };
        let r =
            autocorrelation(&values, lags).map_err(|e| AnalysisError::BadOption(format!("{}: {e}", input.label)))?;
        for l in &r {
            table.push_row(
                format!("{}, lag {}", input.label, l.lag),
                vec![
                    l.lag as f64,
                    l.acf,
                    l.acf_se,
                    -z * l.acf_se,
                    z * l.acf_se,
                    l.pacf,
                    z * l.pacf_se,
                    l.ljung_box,
                    l.p,
                ],
                vec![String::new(); 9],
            );
        }
        let outside = r.iter().filter(|l| l.acf.abs() > z * l.acf_se).count();
        let last = r.last().expect("at least one lag");
        notes.push(format!(
            "{}: {} values, lags 1 to {lags}; {outside} autocorrelation(s) outside the {pct}% bounds; Ljung-Box Q({lags}) = {}, Prob>Q = {}.",
            input.label,
            values.len(),
            num(last.ljung_box),
            num(last.p)
        ));
        if spec.graph {
            let lag_col: Vec<f64> = r.iter().map(|l| l.lag as f64).collect();
            let mut add = |plots: &mut Vec<GraphPlot>, what: &str, ys: Vec<f64>, bound: Vec<f64>| {
                let label = format!("{what} of {}", input.name);
                let x = on_sheet.len();
                on_sheet.push(column_of("Lag", &label, Designation::X, lag_col.clone()));
                on_sheet.push(column_of(what, &label, Designation::Y, ys));
                let first = plots.len();
                plots.push(GraphPlot { x, y: x + 1, kind: PlotKind::Column, label: label.clone(), line_of: None });
                for (side, sign) in [("upper", 1.0), ("lower", -1.0)] {
                    let name = format!("{what} {side} {pct}% bound");
                    on_sheet.push(column_of(&name, &label, Designation::Y, bound.iter().map(|b| sign * b).collect()));
                    plots.push(GraphPlot {
                        x,
                        y: on_sheet.len() - 1,
                        kind: PlotKind::Line,
                        label: format!("{name} of {}", input.name),
                        line_of: Some(first),
                    });
                }
            };
            add(&mut acf_plots, "ACF", r.iter().map(|l| l.acf).collect(), r.iter().map(|l| z * l.acf_se).collect());
            add(&mut pacf_plots, "PACF", r.iter().map(|l| l.pacf).collect(), r.iter().map(|l| z * l.pacf_se).collect());
        }
    }
    notes.push(format!(
        "ACF bounds from Bartlett's standard error sqrt((1 + 2 sum of earlier r^2) / n), PACF bounds +-z/sqrt(n) (z = {}); PACF by the Durbin-Levinson recursion; Prob>Q from the chi-square distribution on lag degrees of freedom.",
        num(z)
    ));
    let graph_of = |plots: Vec<GraphPlot>, y_title: &str| XyGraph {
        plots,
        x_scale: ScaleKind::Linear,
        y_scale: ScaleKind::Linear,
        x_title: "Lag".into(),
        y_title: y_title.into(),
        look: Default::default(),
    };
    let (graph, more_graphs) = if spec.graph && !acf_plots.is_empty() {
        (Some(graph_of(acf_plots, "ACF")), vec![graph_of(pacf_plots, "PACF")])
    } else {
        (None, Vec::new())
    };
    let mut sheet = vec![SheetColumn {
        designation: Designation::Label,
        long_name: "Data".into(),
        units: String::new(),
        comments: String::new(),
        data: ColumnData::text(table.rows.iter().map(|(l, _)| l.clone()).collect()),
    }];
    for (i, h) in table.headers.iter().enumerate() {
        sheet.push(column_of(h, "", Designation::Y, table.rows.iter().map(|(_, v)| v[i]).collect()));
    }
    let shift = sheet.len();
    sheet.extend(on_sheet);
    Ok(Output { table: Some(table), on_sheet: sheet, notes, graph, more_graphs }.shift_graphs(shift))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(columns: Vec<ColumnId>, spec: AcfSpec) -> Request {
        Request { columns, rows: RowRange::All, tool: Tool::Acf(spec) }
    }

    #[test]
    fn lags_bounds_and_both_graphs_are_made() {
        let mut p = Project::new();
        let book = p.add_workbook("ts");
        let mut ys: Vec<f64> =
            (0..100).map(|i| (f64::from(i) * 0.6).sin() + 0.1 * (f64::from(i * i) * 0.37).sin()).collect();
        ys[5] = f64::NAN;
        let y = p.add_column(book, 0, Designation::Y, ColumnData::numeric(ys.clone())).unwrap();
        let out = p.analyze(&request(vec![y], AcfSpec::default())).unwrap();
        let t = out.table.as_ref().unwrap();
        // 99 values: default lags floor(10 log10 99) = 19.
        assert_eq!(t.rows.len(), 19);
        let values: Vec<f64> = ys.iter().copied().filter(|v| v.is_finite()).collect();
        let direct = autocorrelation(&values, 19).unwrap();
        assert!((t.rows[0].1[1] - direct[0].acf).abs() < 1e-15);
        assert!((t.rows[0].1[6] - 1.959_963_984_540_054 / 99f64.sqrt()).abs() < 1e-12);
        assert_eq!(t.rows[0].1[3], -t.rows[0].1[4]);
        assert!(out.new_graph.is_some());
        assert_eq!(out.more_graphs.len(), 1);
        let cols = super::super::dist::plot_columns(&p, out.new_graph.unwrap());
        assert_eq!((cols[0].0.as_str(), cols[0].1.as_str()), ("Lag", "ACF"), "the graph draws its own columns");
        assert_eq!(cols[1].1, "ACF upper 95% bound");
        let cols = super::super::dist::plot_columns(&p, out.more_graphs[0]);
        assert_eq!(cols[0].1, "PACF");

        let spec = AcfSpec { lags: 5, confidence: 0.99, graph: false };
        let out = p.analyze(&request(vec![y], spec)).unwrap();
        assert_eq!(out.table.as_ref().unwrap().rows.len(), 5);
        assert!(out.new_graph.is_none());
        assert!(p.analyze(&request(vec![y], AcfSpec { lags: 200, ..Default::default() })).is_err());
        assert!(p.analyze(&request(vec![y], AcfSpec { confidence: 1.0, ..Default::default() })).is_err());
    }
}
