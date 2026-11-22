//! Statistics > Survival Analysis > Distribution Overview Plot as a tool of `Project::analyze` (T366; Origin
//! 2026b, not in Origin 2016, so the layout is Sheaf's own). The numerics are in `sheaf_calc::lifedata`;
//! `docs/spec/statistics.md` section 37 says what is computed and written.
//!
//! The chosen columns are the time column and the censor column, as for the Kaplan-Meier estimator: a case is
//! right-censored when its censor cell equals the censoring value.

use super::dist::{GraphPlot, Output, XyGraph};
use super::roc::is_positive as cell_equals;
use super::statistics::{cell_left_out, num};
use super::*;
use crate::graph::{PlotKind, ScaleKind};
pub use calc::lifedata::LifeDist;
use calc::lifedata::{fit_right_censored, plotting_positions};

/// What [`Tool::LifeDist`] does.
#[derive(Debug, Clone, PartialEq)]
pub struct LifeDistSpec {
    pub dist: LifeDist,
    /// The value of the censor column that marks a censored case (default 1, as the Kaplan-Meier estimator).
    pub censor: String,
    /// Make the four graphs: probability plot, density, survival and hazard.
    pub graph: bool,
}

impl Default for LifeDistSpec {
    fn default() -> Self {
        LifeDistSpec { dist: LifeDist::Weibull, censor: "1".into(), graph: true }
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

pub(super) fn run(
    spec: &LifeDistSpec,
    sheet: &Worksheet,
    columns: &[ColumnId],
    rows: RowRange,
) -> Result<Output, AnalysisError> {
    let censor = spec.censor.trim();
    if censor.is_empty() {
        return Err(AnalysisError::BadOption("give the value of the censor column that marks a censored case".into()));
    }
    let [time_id, censor_id] = columns else {
        return Err(AnalysisError::BadOption(
            "the distribution overview takes two columns: the time and the censor column".into(),
        ));
    };
    let column = |id: ColumnId| sheet.column(id).ok_or(AnalysisError::NoSuchColumn(id));
    let time_col = column(*time_id)?;
    let censor_col = column(*censor_id)?;
    let times = sheet.values_of(time_col).ok_or_else(|| AnalysisError::NotNumeric(time_col.short_name.clone()))?;
    let n = time_col.data.len().max(censor_col.data.len());
    if n == 0 {
        return Err(AnalysisError::Empty(time_col.short_name.clone()));
    }
    let (first, last) = resolve_rows(rows, &time_col.short_name, &[], n)?;
    let censor_number = censor.parse::<f64>().ok();
    let (mut t, mut c) = (Vec::new(), Vec::new());
    for row in first..=last {
        let v = times.get(row).copied().unwrap_or(f64::NAN);
        if v.is_nan() || cell_left_out(sheet, censor_col, row) {
            continue;
        }
        let Some(is_censored) = cell_equals(censor_col, row, censor, censor_number) else { continue };
        t.push(v);
        c.push(is_censored);
    }
    let fit = fit_right_censored(&t, &c, spec.dist).map_err(|e| AnalysisError::BadOption(e.to_string()))?;
    let label = column_label(time_col);
    let units = time_col.units.clone();
    let k = fit.params.len() as f64;
    let mut table = ResultTable::new(&[
        "N",
        "Failures",
        "Censored",
        "Parameter 1",
        "Parameter 2",
        "Log-likelihood",
        "AIC",
        "Mean",
        "Median",
    ]);
    table.push_row(
        label.clone(),
        vec![
            fit.n as f64,
            fit.failures as f64,
            (fit.n - fit.failures) as f64,
            fit.params[0],
            fit.params.get(1).copied().unwrap_or(f64::NAN),
            fit.log_likelihood,
            2.0 * k - 2.0 * fit.log_likelihood,
            fit.mean,
            fit.median,
        ],
        vec![
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            units.clone(),
            units.clone(),
        ],
    );
    let names = spec.dist.param_names();
    let params: Vec<String> = names.iter().zip(&fit.params).map(|(n, v)| format!("{n} {}", num(*v))).collect();
    let mut notes = vec![
        format!(
            "{label}: {} fit to {} cases ({} failures, {} right-censored): {}; log-likelihood {}, mean {}, median {}.",
            spec.dist.name(),
            fit.n,
            fit.failures,
            fit.n - fit.failures,
            params.join(", "),
            num(fit.log_likelihood),
            num(fit.mean),
            num(fit.median)
        ),
        "Maximum likelihood with right censoring; points at the modified Kaplan-Meier failure probabilities.".into(),
    ];
    if names.len() == 1 {
        notes.push("Parameter 2 is not used by the exponential distribution.".into());
    }
    let mut on_sheet = Vec::new();
    let mut graphs = Vec::new();
    if spec.graph {
        let positions = plotting_positions(&t, &c);
        let comment = format!("Distribution overview of {label}");
        let lo_t = t.iter().copied().fold(f64::INFINITY, f64::min);
        let hi_t = t.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let normal = spec.dist == LifeDist::Normal;
        // Curve times: from half the smallest time (or below the data for the normal) to past the largest.
        let (a, b) = if normal {
            let pad = 0.1 * (hi_t - lo_t).max(fit.params[1]);
            (lo_t - pad, hi_t + pad)
        } else {
            (lo_t * 0.5, hi_t * 1.1)
        };
        let curve: Vec<f64> = (0..=200)
            .map(|i| {
                let f = f64::from(i) / 200.0;
                if normal { a + (b - a) * f } else { a * (b / a).powf(f) }
            })
            .collect();
        let mut add_curve = |name: &str, xs: Vec<f64>, ys: Vec<f64>, extra: Option<(Vec<f64>, Vec<f64>)>| {
            let x = on_sheet.len();
            on_sheet.push(column_of("Time", &comment, Designation::X, xs));
            on_sheet.push(column_of(name, &comment, Designation::Y, ys));
            let mut plots = Vec::new();
            if let Some((px, py)) = extra {
                let x0 = on_sheet.len();
                on_sheet.push(column_of("Failure time", &comment, Designation::X, px));
                on_sheet.push(column_of(&format!("{name} (Kaplan-Meier)"), &comment, Designation::Y, py));
                plots.push(GraphPlot {
                    x: x0,
                    y: x0 + 1,
                    kind: PlotKind::Scatter,
                    label: label.clone(),
                    line_of: None,
                });
            }
            plots.push(GraphPlot {
                x,
                y: x + 1,
                kind: PlotKind::Line,
                label: format!("Fitted {}", spec.dist.name()),
                line_of: None,
            });
            plots
        };
        // Probability plot: percent on the distribution's own probability axis, the fitted line kept to the part of
        // it the axis can show (a probability axis has no 0 or 100).
        let (fit_x, fit_pct): (Vec<f64>, Vec<f64>) =
            curve.iter().map(|t| (*t, 100.0 * fit.cdf(*t))).filter(|(_, p)| (0.001..=99.999).contains(p)).unzip();
        let pts: (Vec<f64>, Vec<f64>) = positions.iter().map(|(t, f)| (*t, 100.0 * f)).unzip();
        let (x_scale, y_scale) = match spec.dist {
            LifeDist::Weibull | LifeDist::Exponential => (ScaleKind::Log10, ScaleKind::Weibull),
            LifeDist::Lognormal => (ScaleKind::Log10, ScaleKind::Probability),
            LifeDist::Normal => (ScaleKind::Linear, ScaleKind::Probability),
        };
        let x_title = if units.is_empty() { label.clone() } else { format!("{label} ({units})") };
        let graph = |plots: Vec<GraphPlot>, x_scale, y_scale, y_title: &str| XyGraph {
            plots,
            x_scale,
            y_scale,
            x_title: x_title.clone(),
            y_title: y_title.into(),
            look: Default::default(),
        };
        let plots = add_curve("Percent failed", fit_x, fit_pct, Some(pts));
        graphs.push(graph(plots, x_scale, y_scale, "Percent"));
        let plots = add_curve("Probability density", curve.clone(), curve.iter().map(|t| fit.pdf(*t)).collect(), None);
        graphs.push(graph(plots, ScaleKind::Linear, ScaleKind::Linear, "Probability density"));
        let surv: (Vec<f64>, Vec<f64>) = positions.iter().map(|(t, f)| (*t, 100.0 * (1.0 - f))).unzip();
        let plots = add_curve(
            "Percent surviving",
            curve.clone(),
            curve.iter().map(|t| 100.0 * fit.survival(*t)).collect(),
            Some(surv),
        );
        graphs.push(graph(plots, ScaleKind::Linear, ScaleKind::Linear, "Percent surviving"));
        let plots = add_curve("Hazard rate", curve.clone(), curve.iter().map(|t| fit.hazard(*t)).collect(), None);
        graphs.push(graph(plots, ScaleKind::Linear, ScaleKind::Linear, "Hazard rate"));
    }
    let mut sheet_cols = vec![SheetColumn {
        designation: Designation::Label,
        long_name: "Data".into(),
        units: String::new(),
        comments: String::new(),
        data: ColumnData::text(vec![label]),
    }];
    for (i, h) in table.headers.iter().enumerate() {
        sheet_cols.push(column_of(h, "", Designation::Y, table.rows.iter().map(|(_, v)| v[i]).collect()));
    }
    let shift = sheet_cols.len();
    sheet_cols.extend(on_sheet);
    let (graph, more_graphs) = if graphs.is_empty() {
        (None, Vec::new())
    } else {
        let first = graphs.remove(0);
        (Some(first), graphs)
    };
    Ok(Output { table: Some(table), on_sheet: sheet_cols, notes, graph, more_graphs }.shift_graphs(shift))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(columns: Vec<ColumnId>, spec: LifeDistSpec) -> Request {
        Request { columns, rows: RowRange::All, tool: Tool::LifeDist(spec) }
    }

    #[test]
    fn censored_cases_fit_and_four_graphs_are_made() {
        let mut p = Project::new();
        let book = p.add_workbook("life");
        let t = p
            .add_column(book, 0, Designation::X, ColumnData::numeric(vec![2.0, 4.0, 6.0, 8.0, f64::NAN, 9.0]))
            .unwrap();
        let c = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![0.0, 0.0, 1.0, 0.0, 0.0, 1.0])).unwrap();
        let spec = LifeDistSpec { dist: LifeDist::Exponential, ..Default::default() };
        let out = p.analyze(&request(vec![t, c], spec)).unwrap();
        let row = &out.table.as_ref().unwrap().rows[0].1;
        assert_eq!((row[0], row[1], row[2]), (5.0, 3.0, 2.0), "the missing time is left out");
        assert!((row[3] - 29.0 / 3.0).abs() < 1e-12 && row[4].is_nan());
        assert!(out.new_graph.is_some());
        assert_eq!(out.more_graphs.len(), 3);
        let cols = super::super::dist::plot_columns(&p, out.new_graph.unwrap());
        assert_eq!(cols[0], ("Failure time".to_string(), "Percent failed (Kaplan-Meier)".to_string()), "{cols:?}");
        assert_eq!(cols[1], ("Time".to_string(), "Percent failed".to_string()));
        let cols = super::super::dist::plot_columns(&p, out.more_graphs[2]);
        assert_eq!(cols[0].1, "Hazard rate");

        let out = p.analyze(&request(vec![t, c], LifeDistSpec { graph: false, ..Default::default() })).unwrap();
        assert!(out.new_graph.is_none() && out.notes[0].contains("Weibull fit to 5 cases"), "{:?}", out.notes);
        assert!(p.analyze(&request(vec![t], LifeDistSpec::default())).is_err());
        assert!(p.analyze(&request(vec![t, c], LifeDistSpec { censor: " ".into(), ..Default::default() })).is_err());
    }
}
