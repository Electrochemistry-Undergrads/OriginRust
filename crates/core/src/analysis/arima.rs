//! Statistics > ARIMA Forecast as a tool of `Project::analyze` (T376; Origin 2024b's Time Series Analysis app, not
//! in Origin 2016, so the layout is Sheaf's own). The numerics are in `sheaf_calc::arima`; `docs/spec/statistics.md`
//! section 43 says what is computed and written.
//!
//! Every chosen column is a series of its own, in row order; it must be complete in the chosen rows.

use super::dist::{GraphPlot, Output, XyGraph};
use super::statistics::num;
use super::*;
use crate::graph::{PlotKind, ScaleKind};
pub use calc::arima::Method as ArimaMethod;
pub use calc::arima::Seasonal as ArimaSeasonal;
use calc::arima::{ArimaOrder, fit_auto_with, fit_with};

/// What [`Tool::Arima`] does.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArimaSpec {
    pub p: usize,
    pub d: usize,
    pub q: usize,
    /// A constant: the mean of the differenced series (a drift when `d = 1`).
    pub constant: bool,
    pub horizon: usize,
    /// Confidence level of the forecast bands (0.95).
    pub confidence: f64,
    pub graph: bool,
    /// Choose `p` and `q` by AIC up to these maxima instead of using `p` and `q` (T428).
    pub auto: Option<(usize, usize)>,
    /// The seasonal orders `(P, D, Q)` and period (T446); all 0 for none.
    pub seasonal: ArimaSeasonal,
    /// Conditional sum of squares (the default) or exact maximum likelihood (T495).
    pub method: ArimaMethod,
}

impl Default for ArimaSpec {
    fn default() -> Self {
        ArimaSpec {
            p: 1,
            d: 0,
            q: 0,
            constant: true,
            horizon: 10,
            confidence: 0.95,
            graph: true,
            auto: None,
            seasonal: ArimaSeasonal::default(),
            method: ArimaMethod::Css,
        }
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

pub(super) fn run(spec: &ArimaSpec, inputs: &[Input<'_>]) -> Result<Output, AnalysisError> {
    if !(spec.confidence > 0.0 && spec.confidence < 1.0) {
        return Err(AnalysisError::BadOption("the confidence level must lie between 0 and 1".into()));
    }
    let order = ArimaOrder { p: spec.p, d: spec.d, q: spec.q, constant: spec.constant, seasonal: spec.seasonal };
    let z = calc::standard_quantile(calc::Family::Normal, 1.0, 0.5 + spec.confidence / 2.0);
    let pct = num(100.0 * spec.confidence);
    let mut table = ResultTable::new(&["Value", "Standard error", "z", "Prob>|z|"]);
    let mut notes = Vec::new();
    let mut on_sheet = Vec::new();
    let mut graphs = Vec::new();
    for input in inputs {
        let m = match spec.auto {
            Some((max_p, max_q)) => {
                fit_auto_with(input.ys(), spec.d, max_p, max_q, spec.constant, spec.seasonal, spec.horizon, spec.method)
            }
            None => fit_with(input.ys(), order, spec.horizon, spec.method),
        }
        .map_err(|e| AnalysisError::BadOption(format!("{}: {e}", input.label)))?;
        let (p, q) = (m.order.p, m.order.q);
        // The fitted seasonal orders (the AIC search may choose smaller ones, T452).
        let s = m.order.seasonal;
        let seasonal_text =
            if s.p + s.d + s.q > 0 { format!("({},{},{}){}", s.p, s.d, s.q, s.period) } else { String::new() };
        let mut names = Vec::new();
        if spec.constant {
            names.push(if spec.d == 0 { "Mean".to_string() } else { "Constant (drift)".to_string() });
        }
        names.extend((1..=p).map(|i| format!("AR {i}")));
        names.extend((1..=q).map(|i| format!("MA {i}")));
        names.extend((1..=s.p).map(|i| format!("Seasonal AR {i}")));
        names.extend((1..=s.q).map(|i| format!("Seasonal MA {i}")));
        for ((name, v), se) in names.iter().zip(&m.params).zip(&m.se) {
            let z = v / se;
            let prob = 2.0 * calc::dist_cdf(calc::Family::Normal, &[0.0, 1.0], -z.abs());
            table.push_row(format!("{}, {name}", input.label), vec![*v, *se, z, prob], vec![String::new(); 4]);
        }
        for (name, v) in [("Sum of squares", m.ss), ("Sigma^2", m.sigma2), ("AIC", m.aic), ("Log-likelihood", m.loglik)]
        {
            let nan = f64::NAN;
            table.push_row(format!("{}, {name}", input.label), vec![v, nan, nan, nan], vec![String::new(); 4]);
        }
        let coefs: Vec<String> = names.iter().zip(&m.params).map(|(n, v)| format!("{n} {}", num(*v))).collect();
        notes.push(format!(
            "{}: ARIMA({},{},{}){}{} by {}: {}; sigma^2 {}, AIC {}; {} forecast(s) from {} to {}.",
            input.label,
            p,
            spec.d,
            q,
            seasonal_text,
            match spec.auto {
                Some((mp, mq)) if spec.seasonal.p + spec.seasonal.q > 0 => format!(
                    " (the smallest AIC of p <= {mp}, q <= {mq}, P <= {}, Q <= {})",
                    spec.seasonal.p, spec.seasonal.q
                ),
                Some((mp, mq)) => format!(" (the smallest AIC of p <= {mp}, q <= {mq})"),
                None => String::new(),
            },
            match spec.method {
                ArimaMethod::Css => "conditional least squares",
                ArimaMethod::Ml => "exact maximum likelihood",
            },
            if coefs.is_empty() { "no coefficients".to_string() } else { coefs.join(", ") },
            num(m.sigma2),
            num(m.aic),
            spec.horizon,
            m.forecast.first().map_or("-".to_string(), |v| num(*v)),
            m.forecast.last().map_or("-".to_string(), |v| num(*v))
        ));
        let comment = format!("ARIMA({p},{},{q}){seasonal_text} of {}", spec.d, input.name);
        let xs = input.xs().to_vec();
        let step = if xs.len() >= 2 { xs[xs.len() - 1] - xs[xs.len() - 2] } else { 1.0 };
        let step = if step.is_finite() && step != 0.0 { step } else { 1.0 };
        let last_x = xs.last().copied().unwrap_or(0.0);
        let x = on_sheet.len();
        on_sheet.push(column_of(&input.x_name, &comment, Designation::X, xs));
        on_sheet.push(column_of(&input.name, &comment, Designation::Y, input.ys().to_vec()));
        on_sheet.push(column_of("One-step prediction", &comment, Designation::Y, m.fitted.clone()));
        on_sheet.push(column_of(
            "Residual",
            &comment,
            Designation::Y,
            input.ys().iter().zip(&m.fitted).map(|(y, f)| y - f).collect(),
        ));
        let fx = on_sheet.len();
        on_sheet.push(column_of(
            &format!("Forecast {}", input.x_name),
            &comment,
            Designation::X,
            (1..=spec.horizon).map(|h| last_x + step * h as f64).collect(),
        ));
        on_sheet.push(column_of("Forecast", &comment, Designation::Y, m.forecast.clone()));
        on_sheet.push(column_of(
            &format!("Lower {pct}%"),
            &comment,
            Designation::Y,
            m.forecast.iter().zip(&m.forecast_se).map(|(f, s)| f - z * s).collect(),
        ));
        on_sheet.push(column_of(
            &format!("Upper {pct}%"),
            &comment,
            Designation::Y,
            m.forecast.iter().zip(&m.forecast_se).map(|(f, s)| f + z * s).collect(),
        ));
        if spec.graph {
            graphs.push(XyGraph {
                plots: vec![
                    GraphPlot { x, y: x + 1, kind: PlotKind::LineSymbol, label: input.label.clone(), line_of: None },
                    GraphPlot { x, y: x + 2, kind: PlotKind::Line, label: "One-step prediction".into(), line_of: None },
                    GraphPlot { x: fx, y: fx + 1, kind: PlotKind::LineSymbol, label: "Forecast".into(), line_of: None },
                    GraphPlot {
                        x: fx,
                        y: fx + 2,
                        kind: PlotKind::Line,
                        label: format!("Lower {pct}%"),
                        line_of: Some(2),
                    },
                    GraphPlot {
                        x: fx,
                        y: fx + 3,
                        kind: PlotKind::Line,
                        label: format!("Upper {pct}%"),
                        line_of: Some(2),
                    },
                ],
                x_scale: ScaleKind::Linear,
                y_scale: ScaleKind::Linear,
                x_title: input.x_name.clone(),
                y_title: input.name.clone(),
                look: Default::default(),
            });
        }
    }
    notes.push(format!(
        "Forecast bands: forecast -+ {} times the standard error from the psi weights (future errors taken as 0).",
        num(z)
    ));
    notes.push(
        "Coefficient standard errors: sigma^2 (J'J)^-1 with J the derivatives of the conditional errors (Gauss-Newton); z = coefficient / SE with normal probabilities.".to_string(),
    );
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
    let (graph, more_graphs) = if graphs.is_empty() {
        (None, Vec::new())
    } else {
        let first = graphs.remove(0);
        (Some(first), graphs)
    };
    Ok(Output { table: Some(table), on_sheet: sheet, notes, graph, more_graphs }.shift_graphs(shift))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(columns: Vec<ColumnId>, spec: ArimaSpec) -> Request {
        Request { columns, rows: RowRange::All, tool: Tool::Arima(spec) }
    }

    #[test]
    fn a_drifting_walk_and_its_forecast_bands() {
        let mut p = Project::new();
        let book = p.add_workbook("ts");
        let walk = vec![0.0, 1.0, 3.0, 4.0, 6.0, 7.0, 9.0];
        let y = p.add_column(book, 0, Designation::Y, ColumnData::numeric(walk)).unwrap();
        let spec = ArimaSpec { p: 0, d: 1, q: 0, horizon: 2, ..Default::default() };
        let out = p.analyze(&request(vec![y], spec)).unwrap();
        let t = out.table.as_ref().unwrap();
        assert_eq!(t.rows[0].0, "A, Constant (drift)");
        assert!((t.rows[0].1[0] - 1.5).abs() < 1e-9, "{:?}", t.rows[0]);
        let cols = super::super::dist::plot_columns(&p, out.new_graph.unwrap());
        assert_eq!(cols[2].1, "Forecast", "{cols:?}");
        assert_eq!(cols[4].1, "Upper 95%");
        let g = p.graph(out.new_graph.unwrap()).unwrap();
        let fc = p.column(g.layers[0].plots[2].y).unwrap().numeric().unwrap().to_vec();
        assert!((fc[1] - 12.0).abs() < 1e-8, "{fc:?}");
        // The drift's standard error is sigma / sqrt(n - 1) (T428).
        let sigma2 = t.rows[2].1[0];
        assert!((t.rows[0].1[1] - (sigma2 / 6.0).sqrt()).abs() < 1e-9, "{:?}", t.rows[0]);
        assert!(t.rows[1].1[1].is_nan(), "no SE for the sum of squares");
        let auto = ArimaSpec { d: 1, auto: Some((1, 1)), graph: false, horizon: 1, ..Default::default() };
        let out = p.analyze(&request(vec![y], auto)).unwrap();
        assert!(out.notes[0].contains("the smallest AIC of p <= 1, q <= 1"), "{:?}", out.notes);
        let none = ArimaSpec { p: 0, d: 0, q: 0, constant: false, ..Default::default() };
        assert!(p.analyze(&request(vec![y], none)).is_err());
        // T495: exact maximum likelihood, with its log-likelihood row.
        let ml =
            ArimaSpec { p: 0, d: 1, q: 0, horizon: 2, graph: false, method: ArimaMethod::Ml, ..Default::default() };
        let out = p.analyze(&request(vec![y], ml)).unwrap();
        let t = out.table.as_ref().unwrap();
        assert!((t.rows[0].1[0] - 1.5).abs() < 1e-6, "the drift is the mean step either way: {:?}", t.rows[0]);
        assert_eq!(t.rows[4].0, "A, Log-likelihood");
        assert!(t.rows[4].1[0].is_finite());
        assert!(out.notes[0].contains("by exact maximum likelihood"), "{:?}", out.notes);
    }
}
