//! Statistics > Exponential Smoothing Forecast as a tool of `Project::analyze` (T374; Origin 2024b's Time Series
//! Analysis app, not in Origin 2016, so the layout is Sheaf's own). The numerics are in `sheaf_calc::expsmooth`;
//! `docs/spec/statistics.md` section 42 says what is computed and written.
//!
//! Every chosen column is a series of its own, in row order; it must be complete in the chosen rows.

use super::dist::{GraphPlot, Output, XyGraph};
use super::statistics::num;
use super::*;
use crate::graph::{PlotKind, ScaleKind};
use calc::expsmooth::smooth_with;
pub use calc::expsmooth::{Damping, SmoothModel};

/// What [`Tool::ExpSmooth`] does.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExpSmoothSpec {
    pub model: SmoothModel,
    /// Points per season (Holt-Winters only).
    pub period: usize,
    /// Fixed smoothing constants `[alpha, beta, gamma]` (as far as the model uses them); `None` fits them.
    pub constants: Option<[f64; 3]>,
    /// Steps to forecast after the last value.
    pub horizon: usize,
    pub graph: bool,
    /// Trend damping (T427).
    pub damping: Damping,
    /// The level of the prediction intervals (T427).
    pub confidence: f64,
}

impl Default for ExpSmoothSpec {
    fn default() -> Self {
        ExpSmoothSpec {
            model: SmoothModel::Holt,
            period: 12,
            constants: None,
            horizon: 10,
            graph: true,
            damping: Damping::None,
            confidence: 0.95,
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

pub(super) fn run(spec: &ExpSmoothSpec, inputs: &[Input<'_>]) -> Result<Output, AnalysisError> {
    let k = spec.model.constants();
    let mut table =
        ResultTable::new(&["Alpha", "Beta", "Gamma", "SSE", "RMSE", "Final level", "Final trend", "Phi (damping)"]);
    let pct = num(100.0 * spec.confidence);
    let mut notes = Vec::new();
    let mut on_sheet = Vec::new();
    let mut graphs = Vec::new();
    for input in inputs {
        let fixed = spec.constants.map(|c| c[..k].to_vec());
        let s = smooth_with(
            input.ys(),
            spec.model,
            spec.period,
            fixed.as_deref(),
            spec.damping,
            spec.horizon,
            spec.confidence,
        )
        .map_err(|e| AnalysisError::BadOption(format!("{}: {e}", input.label)))?;
        let n = s.fitted.len();
        let rmse = (s.sse / n as f64).sqrt();
        let c = |i: usize| s.constants.get(i).copied().unwrap_or(f64::NAN);
        let mut units = vec![String::new(); 8];
        units[5].clone_from(&input.units);
        units[6].clone_from(&input.units);
        let trend = if spec.model == SmoothModel::Simple { f64::NAN } else { s.trend };
        let phi = if spec.damping == Damping::None { f64::NAN } else { s.phi };
        table.push_row(input.label.clone(), vec![c(0), c(1), c(2), s.sse, rmse, s.level, trend, phi], units);
        let names = ["alpha", "beta", "gamma"];
        let constants: Vec<String> = (0..k).map(|i| format!("{} = {}", names[i], num(c(i)))).collect();
        notes.push(format!(
            "{}: {}{} smoothing, {} {}; RMSE of the one-step forecasts {}; {} forecast(s) from {} to {}.",
            input.label,
            match spec.damping {
                Damping::None => String::new(),
                Damping::Fitted => format!("damped (phi {} fitted) ", num(s.phi)),
                Damping::Given(_) => format!("damped (phi {}) ", num(s.phi)),
            },
            spec.model.name(),
            if spec.constants.is_some() { "given" } else { "fitted" },
            constants.join(", "),
            num(rmse),
            spec.horizon,
            s.forecast.first().map_or("-".to_string(), |v| num(*v)),
            s.forecast.last().map_or("-".to_string(), |v| num(*v))
        ));
        let comment = format!("Exponential smoothing of {}", input.name);
        let xs = input.xs().to_vec();
        let step = if xs.len() >= 2 { xs[xs.len() - 1] - xs[xs.len() - 2] } else { 1.0 };
        let step = if step.is_finite() && step != 0.0 { step } else { 1.0 };
        let last_x = xs.last().copied().unwrap_or(0.0);
        let x = on_sheet.len();
        on_sheet.push(column_of(&input.x_name, &comment, Designation::X, xs));
        on_sheet.push(column_of(&input.name, &comment, Designation::Y, input.ys().to_vec()));
        on_sheet.push(column_of("One-step forecast", &comment, Designation::Y, s.fitted.clone()));
        on_sheet.push(column_of(
            "Residual",
            &comment,
            Designation::Y,
            input.ys().iter().zip(&s.fitted).map(|(y, f)| y - f).collect(),
        ));
        let fx = on_sheet.len();
        on_sheet.push(column_of(
            &format!("Forecast {}", input.x_name),
            &comment,
            Designation::X,
            (1..=spec.horizon).map(|h| last_x + step * h as f64).collect(),
        ));
        on_sheet.push(column_of("Forecast", &comment, Designation::Y, s.forecast.clone()));
        on_sheet.push(column_of(&format!("Lower {pct}%"), &comment, Designation::Y, s.lower.clone()));
        on_sheet.push(column_of(&format!("Upper {pct}%"), &comment, Designation::Y, s.upper.clone()));
        let with_bands = spec.model != SmoothModel::WintersMultiplicative;
        if spec.graph {
            graphs.push(XyGraph {
                plots: vec![
                    GraphPlot { x, y: x + 1, kind: PlotKind::LineSymbol, label: input.label.clone(), line_of: None },
                    GraphPlot { x, y: x + 2, kind: PlotKind::Line, label: "One-step forecast".into(), line_of: None },
                    GraphPlot { x: fx, y: fx + 1, kind: PlotKind::LineSymbol, label: "Forecast".into(), line_of: None },
                ]
                .into_iter()
                .chain(
                    with_bands
                        .then(|| {
                            [(2, "Lower"), (3, "Upper")].map(|(i, side)| GraphPlot {
                                x: fx,
                                y: fx + i,
                                kind: PlotKind::Line,
                                label: format!("{side} {pct}% prediction limit"),
                                line_of: None,
                            })
                        })
                        .into_iter()
                        .flatten(),
                )
                .collect(),
                x_scale: ScaleKind::Linear,
                y_scale: ScaleKind::Linear,
                x_title: input.x_name.clone(),
                y_title: input.name.clone(),
                look: Default::default(),
            });
        }
    }
    notes.push(if spec.model == SmoothModel::WintersMultiplicative {
        "The multiplicative season has no closed-form prediction intervals: the limit columns are empty.".to_string()
    } else {
        format!(
            "{pct} % prediction intervals: forecast -+ z sigma_h with sigma_h^2 = sigma^2 (1 + sum c_j^2) of the additive-error state space model, sigma^2 = SSE / n."
        )
    });
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

    fn request(columns: Vec<ColumnId>, spec: ExpSmoothSpec) -> Request {
        Request { columns, rows: RowRange::All, tool: Tool::ExpSmooth(spec) }
    }

    #[test]
    fn fixed_simple_smoothing_and_the_forecast_columns() {
        let mut p = Project::new();
        let book = p.add_workbook("ts");
        p.add_column(book, 0, Designation::X, ColumnData::numeric(vec![10.0, 20.0, 30.0])).unwrap();
        let y = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![1.0, 2.0, 3.0])).unwrap();
        let spec = ExpSmoothSpec {
            model: SmoothModel::Simple,
            constants: Some([0.5, 0.0, 0.0]),
            horizon: 2,
            ..Default::default()
        };
        let out = p.analyze(&request(vec![y], spec)).unwrap();
        let t = out.table.as_ref().unwrap();
        assert_eq!(t.rows[0].1[0], 0.5);
        assert!(t.rows[0].1[1].is_nan() && t.rows[0].1[6].is_nan());
        assert!((t.rows[0].1[3] - (0.0 + 1.0 + 2.25)).abs() < 1e-12, "SSE of errors 0, 1, 1.5");
        let cols = super::super::dist::plot_columns(&p, out.new_graph.unwrap());
        assert_eq!(cols[1].1, "One-step forecast");
        assert_eq!((cols[2].0.as_str(), cols[2].1.as_str()), ("Forecast A", "Forecast"), "{cols:?}");
        let g = p.graph(out.new_graph.unwrap()).unwrap();
        let fx = g.layers[0].plots[2].x.unwrap();
        assert_eq!(p.column(fx).unwrap().numeric().unwrap(), [40.0, 50.0], "the X step goes on");
        let cols = &p.workbook(book).unwrap().sheets.last().unwrap().columns;
        let lower = cols.iter().find(|c| c.long_name == "Lower 95%").unwrap().numeric().unwrap().to_vec();
        // Simple smoothing: sigma^2 (1 + (h - 1) a^2) with sigma^2 = SSE / 3; the forecast is 2.25.
        let half = 1.959963984540054 * (3.25f64 / 3.0 * (1.0 + 0.25)).sqrt();
        assert!((lower[1] - (2.25 - half)).abs() < 1e-9, "{lower:?}");
        let damped = ExpSmoothSpec { damping: Damping::Given(0.9), graph: false, horizon: 2, ..Default::default() };
        let out = p.analyze(&request(vec![y], damped)).unwrap();
        assert_eq!(out.table.as_ref().unwrap().rows[0].1[7], 0.9);
        assert!(out.notes[0].contains("damped (phi 0.9)"), "{:?}", out.notes);
        let bad = ExpSmoothSpec { model: SmoothModel::WintersAdditive, period: 4, ..Default::default() };
        assert!(p.analyze(&request(vec![y], bad)).is_err());
    }
}
