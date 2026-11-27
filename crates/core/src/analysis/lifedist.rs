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
use calc::lifedata::{Obs, fit_arbitrary, fit_right_censored, plotting_positions};

/// What [`Tool::LifeDist`] does.
#[derive(Debug, Clone, PartialEq)]
pub struct LifeDistSpec {
    pub dist: LifeDist,
    /// The value of the censor column that marks a censored case (default 1, as the Kaplan-Meier estimator).
    pub censor: String,
    /// Make the four graphs: probability plot, density, survival and hazard.
    pub graph: bool,
    /// The last chosen column holds each row's frequency (count) (T394).
    pub frequency: bool,
}

impl Default for LifeDistSpec {
    fn default() -> Self {
        LifeDistSpec { dist: LifeDist::Weibull, censor: "1".into(), graph: true, frequency: false }
    }
}

/// What [`Tool::LifeDistArbitrary`] does: the chosen columns are the start and the end of each failure window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LifeDistArbitrarySpec {
    pub dist: LifeDist,
    pub graph: bool,
    /// The last chosen column holds each row's frequency (count) (T394).
    pub frequency: bool,
}

impl Default for LifeDistArbitrarySpec {
    fn default() -> Self {
        LifeDistArbitrarySpec { dist: LifeDist::Weibull, graph: true, frequency: false }
    }
}

/// What [`Tool::LifeRegression`] does: the chosen columns are the covariates, then the time and the censor column.
#[derive(Debug, Clone, PartialEq)]
pub struct LifeRegressionSpec {
    /// Weibull, lognormal or exponential.
    pub dist: LifeDist,
    /// The value of the censor column that marks a censored case (default 1).
    pub censor: String,
    /// The last chosen column holds each row's frequency (count) (T394).
    pub frequency: bool,
    /// Fractions failed whose percentile life is predicted (T437); empty for none.
    pub percentiles: Vec<f64>,
    /// The covariate values of the prediction, one per covariate; `None`: the covariate means.
    pub at: Option<Vec<f64>>,
    /// Further covariate settings to predict at, each one value per covariate (T504).
    pub also_at: Vec<Vec<f64>>,
    /// A graph of the percentile lives against the first covariate over its range, the others at their means (T509).
    pub graph: bool,
    /// The level of the percentile intervals.
    pub confidence: f64,
}

impl Default for LifeRegressionSpec {
    fn default() -> Self {
        LifeRegressionSpec {
            dist: LifeDist::Weibull,
            censor: "1".into(),
            frequency: false,
            percentiles: vec![0.1, 0.5, 0.9],
            at: None,
            also_at: Vec::new(),
            graph: false,
            confidence: 0.95,
        }
    }
}

/// What [`Tool::Cox`] does: the chosen columns are the covariates, then the time and the censor column (T408).
#[derive(Debug, Clone, PartialEq)]
pub struct CoxSpec {
    /// The value of the censor column that marks a censored case (default 1).
    pub censor: String,
    /// Confidence level of the hazard-ratio intervals (0.95).
    pub confidence: f64,
    /// Make the graph of the survival at the covariate means.
    pub graph: bool,
    /// The last chosen column holds each row's frequency (count) (T429).
    pub frequency: bool,
    /// Efron's handling of tied failure times instead of Breslow's (T445).
    pub efron: bool,
    /// A stratum column follows the censor column (T451): risk sets and baselines within strata.
    pub strata: bool,
    /// Also draw the survival at these covariate values, one per covariate (T457).
    pub at: Option<Vec<f64>>,
    /// Write the martingale, deviance and Schoenfeld residuals (T457).
    pub residuals: bool,
    /// Test the proportional-hazards assumption (T458).
    pub ph_test: bool,
    /// The time scale of that test (T503): rank (default), Kaplan-Meier, identity or log.
    pub ph_transform: calc::cox::PhTransform,
}

impl Default for CoxSpec {
    fn default() -> Self {
        CoxSpec {
            censor: "1".into(),
            confidence: 0.95,
            graph: true,
            frequency: false,
            efron: false,
            strata: false,
            at: None,
            residuals: false,
            ph_test: false,
            ph_transform: calc::cox::PhTransform::Rank,
        }
    }
}

/// What [`Tool::Turnbull`] does: the chosen columns are the start and the end of each failure window, as for
/// [`Tool::LifeDistArbitrary`] (T409).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TurnbullSpec {
    /// Make the survival graph.
    pub graph: bool,
    /// The last chosen column holds each row's frequency (count).
    pub frequency: bool,
}

impl Default for TurnbullSpec {
    fn default() -> Self {
        TurnbullSpec { graph: true, frequency: false }
    }
}

/// The chosen columns without the frequency column, and the frequencies by row when the last chosen column holds them
/// (T394; `None`: every row counts once).
pub(super) fn split_frequency<'a>(
    sheet: &Worksheet,
    columns: &'a [ColumnId],
    on: bool,
) -> Result<(&'a [ColumnId], Option<Vec<f64>>), AnalysisError> {
    if !on {
        return Ok((columns, None));
    }
    let Some((last, rest)) = columns.split_last() else {
        return Err(AnalysisError::BadOption("choose the frequency column last".into()));
    };
    let c = sheet.column(*last).ok_or(AnalysisError::NoSuchColumn(*last))?;
    let v = sheet.values_of(c).ok_or_else(|| AnalysisError::NotNumeric(c.short_name.clone()))?.into_owned();
    Ok((rest, Some(v)))
}

/// How many times row `r` counts: once without frequencies; a missing, negative or fractional frequency is refused.
pub(super) fn count_of(counts: &Option<Vec<f64>>, r: usize) -> Result<usize, AnalysisError> {
    let Some(v) = counts else { return Ok(1) };
    let c = v.get(r).copied().unwrap_or(f64::NAN);
    if c >= 0.0 && c.fract() == 0.0 && c <= 1e6 {
        Ok(c as usize)
    } else {
        Err(AnalysisError::BadOption(format!(
            "row {}: the frequency must be a whole number of 0 or more (got {c})",
            r + 1
        )))
    }
}

/// The frequencies may not add up to more than a million rows.
pub(super) fn check_total(n: usize) -> Result<(), AnalysisError> {
    if n > 1_000_000 {
        return Err(AnalysisError::BadOption("the frequencies add up to more than 1,000,000 rows".into()));
    }
    Ok(())
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
    let (columns, counts) = split_frequency(sheet, columns, spec.frequency)?;
    let [time_id, censor_id] = columns else {
        return Err(AnalysisError::BadOption(
            "the distribution overview takes two columns: the time and the censor column (and the frequency last)"
                .into(),
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
        for _ in 0..count_of(&counts, row)? {
            t.push(v);
            c.push(is_censored);
        }
        check_total(t.len())?;
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

pub(super) fn run_regression(
    spec: &LifeRegressionSpec,
    sheet: &Worksheet,
    columns: &[ColumnId],
    rows: RowRange,
) -> Result<Output, AnalysisError> {
    let censor = spec.censor.trim();
    if censor.is_empty() {
        return Err(AnalysisError::BadOption("give the value of the censor column that marks a censored case".into()));
    }
    let (columns, counts) = split_frequency(sheet, columns, spec.frequency)?;
    if columns.len() < 3 {
        return Err(AnalysisError::BadOption(
            "life-data regression takes the covariate column(s), then the time and the censor column (and the frequency last)".into(),
        ));
    }
    let column = |id: ColumnId| sheet.column(id).ok_or(AnalysisError::NoSuchColumn(id));
    let read = |c: &Column| -> Result<Vec<f64>, AnalysisError> {
        sheet.values_of(c).map(|v| v.into_owned()).ok_or_else(|| AnalysisError::NotNumeric(c.short_name.clone()))
    };
    let k = columns.len() - 2;
    let xcs: Vec<&Column> = columns[..k].iter().map(|id| column(*id)).collect::<Result<_, _>>()?;
    let tc = column(columns[k])?;
    let cc = column(columns[k + 1])?;
    let x_all: Vec<Vec<f64>> = xcs.iter().map(|c| read(c)).collect::<Result<_, _>>()?;
    let t_all = read(tc)?;
    let n = x_all.iter().map(Vec::len).chain([t_all.len(), cc.data.len()]).max().unwrap_or(0);
    if n == 0 {
        return Err(AnalysisError::Empty(tc.short_name.clone()));
    }
    let (first, last) = resolve_rows(rows, &tc.short_name, &[], n)?;
    let censor_number = censor.parse::<f64>().ok();
    let (mut x, mut t, mut failed) = (Vec::new(), Vec::new(), Vec::new());
    for r in first..=last {
        let row: Vec<f64> = x_all.iter().map(|c| c.get(r).copied().unwrap_or(f64::NAN)).collect();
        let v = t_all.get(r).copied().unwrap_or(f64::NAN);
        if !v.is_finite() || row.iter().any(|a| !a.is_finite()) || cell_left_out(sheet, cc, r) {
            continue;
        }
        let Some(is_censored) = cell_equals(cc, r, censor, censor_number) else { continue };
        for _ in 0..count_of(&counts, r)? {
            x.push(row.clone());
            t.push(v);
            failed.push(!is_censored);
        }
        check_total(t.len())?;
    }
    if spec.percentiles.iter().any(|p| !(*p > 0.0 && *p < 1.0)) {
        return Err(AnalysisError::BadOption("the percentiles must lie between 0 and 100 percent".into()));
    }
    if !(spec.confidence > 0.0 && spec.confidence < 1.0) {
        return Err(AnalysisError::BadOption("the confidence level must lie between 0 and 1".into()));
    }
    if let Some(at) = spec.at.iter().chain(&spec.also_at).find(|at| at.len() != k) {
        return Err(AnalysisError::BadOption(format!(
            "give one value per covariate for the percentiles ({k}, got {})",
            at.len()
        )));
    }
    let fit = calc::lifereg::life_regression(&x, &t, &failed, spec.dist)
        .map_err(|e| AnalysisError::BadOption(e.to_string()))?;
    let mut table = ResultTable::new(&["Coefficient", "Standard error", "z", "Prob>|z|"]);
    table.push_row(
        "Intercept".into(),
        vec![fit.coefficients[0], fit.se[0], fit.z[0], fit.p[0]],
        vec![String::new(); 4],
    );
    for (i, c) in xcs.iter().enumerate() {
        table.push_row(
            column_label(c),
            vec![fit.coefficients[i + 1], fit.se[i + 1], fit.z[i + 1], fit.p[i + 1]],
            vec![String::new(); 4],
        );
    }
    table.push_row("Scale (sigma)".into(), vec![fit.scale, fit.scale_se, f64::NAN, f64::NAN], vec![String::new(); 4]);
    let free = fit.coefficients.len() + usize::from(spec.dist != LifeDist::Exponential);
    let mut notes = vec![format!(
        "{} accelerated failure time model of ln {} on {} covariate(s): {} rows, {} failures; log-likelihood {}, AIC {}.",
        spec.dist.name(),
        column_label(tc),
        k,
        fit.n,
        fit.failures,
        num(fit.log_likelihood),
        num(2.0 * free as f64 - 2.0 * fit.log_likelihood)
    )];
    notes.push(match spec.dist {
        LifeDist::Weibull => format!(
            "Coefficients act on log time (exp(b) multiplies the life); Weibull shape = 1 / scale = {}.",
            num(1.0 / fit.scale)
        ),
        LifeDist::Exponential => {
            "Coefficients act on log time (exp(b) multiplies the mean life); the scale is fixed at 1.".into()
        }
        _ => "Coefficients act on log time (exp(b) multiplies the median life).".into(),
    });
    let mut sheet_cols = vec![SheetColumn {
        designation: Designation::Label,
        long_name: "Term".into(),
        units: String::new(),
        comments: String::new(),
        data: ColumnData::text(table.rows.iter().map(|(l, _)| l.clone()).collect()),
    }];
    for (i, h) in table.headers.iter().enumerate() {
        sheet_cols.push(column_of(h, "", Designation::Y, table.rows.iter().map(|(_, v)| v[i]).collect()));
    }
    // At the given covariates, else at their means over the rows used (T437), then at each further setting (T504).
    let first = spec
        .at
        .clone()
        .unwrap_or_else(|| (0..k).map(|j| x.iter().map(|r| r[j]).sum::<f64>() / x.len() as f64).collect());
    let settings: Vec<(Vec<f64>, bool)> =
        std::iter::once((first, spec.at.is_some())).chain(spec.also_at.iter().map(|a| (a.clone(), true))).collect();
    let several = settings.len() > 1;
    for (s, (at, given)) in settings.into_iter().enumerate() {
        if spec.percentiles.is_empty() {
            break;
        }
        let suffix = if several { format!(" ({})", s + 1) } else { String::new() };
        let pct = num(100.0 * spec.confidence);
        let q: Vec<calc::lifereg::Percentile> =
            spec.percentiles.iter().map(|p| fit.percentile(&at, *p, spec.confidence)).collect();
        let place: Vec<String> =
            xcs.iter().zip(&at).map(|(c, v)| format!("{} = {}", column_label(c), num(*v))).collect();
        let place = if place.is_empty() { "no covariates".to_string() } else { place.join(", ") };
        let comment = format!("Percentile life at {place} ({})", if given { "given" } else { "the covariate means" });
        sheet_cols.push(column_of(
            &format!("Percent{suffix}"),
            &comment,
            Designation::X,
            q.iter().map(|v| 100.0 * v.p).collect(),
        ));
        sheet_cols.push(column_of(
            &format!("Percentile life{suffix}"),
            &comment,
            Designation::Y,
            q.iter().map(|v| v.estimate).collect(),
        ));
        sheet_cols.push(column_of(
            &format!("Lower {pct}%{suffix}"),
            &comment,
            Designation::Y,
            q.iter().map(|v| v.lower).collect(),
        ));
        sheet_cols.push(column_of(
            &format!("Upper {pct}%{suffix}"),
            &comment,
            Designation::Y,
            q.iter().map(|v| v.upper).collect(),
        ));
        let list: Vec<String> = q
            .iter()
            .map(|v| format!("{} %: {} ({} to {})", num(100.0 * v.p), num(v.estimate), num(v.lower), num(v.upper)))
            .collect();
        notes.push(format!(
            "Percentile life at {place} ({}), {pct} % intervals from the delta method on log time: {}.",
            if given { "given" } else { "the covariate means" },
            list.join("; ")
        ));
    }
    // T509: the percentile lives against the first covariate, the others at their means.
    let mut graph = None;
    if spec.graph && k > 0 && !spec.percentiles.is_empty() {
        let means: Vec<f64> = (0..k).map(|j| x.iter().map(|r| r[j]).sum::<f64>() / x.len() as f64).collect();
        let (lo, hi) = x.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(l, h), r| (l.min(r[0]), h.max(r[0])));
        let steps = 40;
        let grid: Vec<f64> = (0..=steps).map(|i| lo + (hi - lo) * f64::from(i) / f64::from(steps)).collect();
        let name = column_label(xcs[0]);
        let comment = "Percentile life against the first covariate (the others at their means)";
        let gx = sheet_cols.len();
        sheet_cols.push(column_of(&name, comment, Designation::X, grid.clone()));
        let pct = num(100.0 * spec.confidence);
        let mut plots = Vec::new();
        let mut dotted = Vec::new();
        for p in &spec.percentiles {
            let q: Vec<calc::lifereg::Percentile> = grid
                .iter()
                .map(|v| {
                    let mut at = means.clone();
                    at[0] = *v;
                    fit.percentile(&at, *p, spec.confidence)
                })
                .collect();
            let label = format!("{} % life", num(100.0 * p));
            let y = sheet_cols.len();
            sheet_cols.push(column_of(&label, comment, Designation::Y, q.iter().map(|v| v.estimate).collect()));
            let main = plots.len();
            plots.push(GraphPlot { x: gx, y, kind: PlotKind::Line, label: label.clone(), line_of: None });
            for (side, values) in [
                ("lower", q.iter().map(|v| v.lower).collect::<Vec<_>>()),
                ("upper", q.iter().map(|v| v.upper).collect()),
            ] {
                let c = sheet_cols.len();
                sheet_cols.push(column_of(&format!("{label} {pct}% {side}"), comment, Designation::Y, values));
                dotted.push(plots.len());
                plots.push(GraphPlot {
                    x: gx,
                    y: c,
                    kind: PlotKind::Line,
                    label: format!("{label} {pct}% {side}"),
                    line_of: Some(main),
                });
            }
        }
        notes.push(format!(
            "The graph draws the percentile lives against {name} from {} to {} with the other covariates at their means, \
             their {pct} % intervals dotted.",
            num(lo),
            num(hi)
        ));
        graph = Some(XyGraph {
            plots,
            x_scale: ScaleKind::Linear,
            y_scale: ScaleKind::Log10,
            x_title: name,
            y_title: format!("Percentile life of {}", column_label(tc)),
            look: super::dist::Look { dotted, ..Default::default() },
        });
    }
    Ok(Output { table: Some(table), on_sheet: sheet_cols, notes, graph, more_graphs: Vec::new() })
}

pub(super) fn run_cox(
    spec: &CoxSpec,
    sheet: &Worksheet,
    columns: &[ColumnId],
    rows: RowRange,
) -> Result<Output, AnalysisError> {
    let censor = spec.censor.trim();
    if censor.is_empty() {
        return Err(AnalysisError::BadOption("give the value of the censor column that marks a censored case".into()));
    }
    let (columns, counts) = split_frequency(sheet, columns, spec.frequency)?;
    let extra = 2 + usize::from(spec.strata);
    if columns.len() < extra + 1 {
        return Err(AnalysisError::BadOption(
            "Cox regression takes the covariate column(s), then the time and the censor column (then the strata, and the frequency last)"
                .into(),
        ));
    }
    let column = |id: ColumnId| sheet.column(id).ok_or(AnalysisError::NoSuchColumn(id));
    let read = |c: &Column| -> Result<Vec<f64>, AnalysisError> {
        sheet.values_of(c).map(|v| v.into_owned()).ok_or_else(|| AnalysisError::NotNumeric(c.short_name.clone()))
    };
    let k = columns.len() - extra;
    let sc = if spec.strata { Some(column(columns[k + 2])?) } else { None };
    if let Some(at) = &spec.at
        && at.len() != k
    {
        return Err(AnalysisError::BadOption(format!(
            "give one value per covariate for the survival curve ({k}, got {})",
            at.len()
        )));
    }
    let xcs: Vec<&Column> = columns[..k].iter().map(|id| column(*id)).collect::<Result<_, _>>()?;
    let tc = column(columns[k])?;
    let cc = column(columns[k + 1])?;
    let x_all: Vec<Vec<f64>> = xcs.iter().map(|c| read(c)).collect::<Result<_, _>>()?;
    let t_all = read(tc)?;
    let n = x_all.iter().map(Vec::len).chain([t_all.len(), cc.data.len()]).max().unwrap_or(0);
    if n == 0 {
        return Err(AnalysisError::Empty(tc.short_name.clone()));
    }
    let (first, last) = resolve_rows(rows, &tc.short_name, &[], n)?;
    let censor_number = censor.parse::<f64>().ok();
    let (mut x, mut t, mut failed, mut labels) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    let mut source_rows = Vec::new();
    for r in first..=last {
        let row: Vec<f64> = x_all.iter().map(|c| c.get(r).copied().unwrap_or(f64::NAN)).collect();
        let v = t_all.get(r).copied().unwrap_or(f64::NAN);
        if !v.is_finite() || row.iter().any(|a| !a.is_finite()) || cell_left_out(sheet, cc, r) {
            continue;
        }
        let Some(is_censored) = cell_equals(cc, r, censor, censor_number) else { continue };
        // A row without a stratum (or a masked one) is left out.
        let label = match sc {
            Some(s) if cell_left_out(sheet, s, r) => continue,
            Some(s) => match super::survival::group_cell(s, r) {
                Some(l) => l,
                None => continue,
            },
            None => String::new(),
        };
        for _ in 0..count_of(&counts, r)? {
            x.push(row.clone());
            t.push(v);
            failed.push(!is_censored);
            labels.push(label.clone());
            source_rows.push((r + 1) as f64);
        }
        check_total(t.len())?;
    }
    // Strata in the order of their labels (numbers by value, otherwise as text).
    let mut names: Vec<String> = labels.clone();
    names.sort();
    names.dedup();
    if names.iter().all(|l| l.parse::<f64>().is_ok()) {
        names.sort_by(|a, b| a.parse::<f64>().unwrap_or(0.0).total_cmp(&b.parse::<f64>().unwrap_or(0.0)));
    }
    let strata: Vec<usize> = labels.iter().map(|l| names.iter().position(|n| n == l).unwrap_or(0)).collect();
    let ties = if spec.efron { calc::cox::Ties::Efron } else { calc::cox::Ties::Breslow };
    let fit = calc::cox::cox_stratified(&x, &t, &failed, &strata, spec.confidence, ties)
        .map_err(|e| AnalysisError::BadOption(e.to_string()))?;
    let pct = num(100.0 * spec.confidence);
    let low = format!("HR lower {pct}%");
    let high = format!("HR upper {pct}%");
    let mut table = ResultTable::new(&["Coefficient", "Standard error", "z", "Prob>|z|", "Hazard ratio", &low, &high]);
    for (i, c) in xcs.iter().enumerate() {
        table.push_row(
            column_label(c),
            vec![
                fit.coefficients[i],
                fit.se[i],
                fit.z[i],
                fit.p[i],
                fit.hazard_ratio[i],
                fit.hr_low[i],
                fit.hr_high[i],
            ],
            vec![String::new(); 7],
        );
    }
    let mut notes = vec![
        format!(
            "Cox proportional hazards ({} ties) of {} on {k} covariate(s): {} cases, {} events; log-likelihood {} (null {}).",
            if spec.efron { "Efron" } else { "Breslow" },
            column_label(tc),
            fit.n,
            fit.events,
            num(fit.log_likelihood),
            num(fit.log_likelihood_null)
        ),
        format!(
            "Tests of all coefficients = 0 ({k} df): likelihood ratio {} (Prob {}), Wald {} (Prob {}), score {} (Prob {}).",
            num(fit.lr.0),
            num(fit.lr.1),
            num(fit.wald.0),
            num(fit.wald.1),
            num(fit.score.0),
            num(fit.score.1)
        ),
    ];
    let mut sheet_cols = vec![SheetColumn {
        designation: Designation::Label,
        long_name: "Covariate".into(),
        units: String::new(),
        comments: String::new(),
        data: ColumnData::text(table.rows.iter().map(|(l, _)| l.clone()).collect()),
    }];
    for (i, h) in table.headers.iter().enumerate() {
        sheet_cols.push(column_of(h, "", Designation::Y, table.rows.iter().map(|(_, v)| v[i]).collect()));
    }
    if let Some(s) = sc {
        notes.push(format!(
            "Stratified by {} ({} strata: {}): risk sets and baselines within each stratum, shared coefficients.",
            column_label(s),
            names.len(),
            names.join(", ")
        ));
    }
    let shift = sheet_cols.len();
    // The survival at the covariate means as steps: flat until each failure time, then down; a curve per stratum.
    let mut plots = Vec::new();
    for (i, (times, survival)) in fit.curves.iter().enumerate() {
        let (mut sx, mut sy) = (vec![0.0], vec![1.0]);
        let mut prev = 1.0;
        for (tk, s) in times.iter().zip(survival) {
            sx.extend([*tk, *tk]);
            sy.extend([prev, *s]);
            prev = *s;
        }
        let comment = if spec.efron {
            "Baseline survival at the covariate means (Efron)"
        } else {
            "Baseline survival at the covariate means (Breslow)"
        };
        let x0 = sheet_cols.len() - shift;
        let (time_name, surv_name, label) = match sc {
            Some(_) => {
                let n = names.get(i).cloned().unwrap_or_default();
                (format!("Time ({n})"), format!("Survival at the means ({n})"), format!("Stratum {n}"))
            }
            None => {
                ("Time".to_string(), "Survival at the means".to_string(), "Survival at the covariate means".to_string())
            }
        };
        sheet_cols.push(column_of(&time_name, comment, Designation::X, sx));
        sheet_cols.push(column_of(&surv_name, comment, Designation::Y, sy));
        plots.push(GraphPlot { x: x0, y: x0 + 1, kind: PlotKind::Line, label, line_of: None });
    }
    // The survival at the given covariates (T457), a curve per stratum beside the ones at the means.
    if let Some(at) = &spec.at {
        let place: Vec<String> =
            xcs.iter().zip(at).map(|(c, v)| format!("{} = {}", column_label(c), num(*v))).collect();
        let place = place.join(", ");
        for (i, (times, survival)) in fit.survival_at(at).iter().enumerate() {
            let (mut sx, mut sy) = (vec![0.0], vec![1.0]);
            let mut prev = 1.0;
            for (tk, s) in times.iter().zip(survival) {
                sx.extend([*tk, *tk]);
                sy.extend([prev, *s]);
                prev = *s;
            }
            let tail = match sc {
                Some(_) => format!(" ({})", names.get(i).cloned().unwrap_or_default()),
                None => String::new(),
            };
            let comment = format!("Survival at {place}");
            let x0 = sheet_cols.len() - shift;
            sheet_cols.push(column_of(&format!("Time at the values{tail}"), &comment, Designation::X, sx));
            sheet_cols.push(column_of(&format!("Survival at the values{tail}"), &comment, Designation::Y, sy));
            plots.push(GraphPlot {
                x: x0,
                y: x0 + 1,
                kind: PlotKind::Line,
                label: format!("{place}{tail}"),
                line_of: None,
            });
        }
        notes.push(format!(
            "The graph adds the survival at {place}: the curve at the means to the power exp((x - mean)'b)."
        ));
    }
    if spec.ph_test {
        let test = fit.ph_test_with(spec.ph_transform);
        let scale = match spec.ph_transform {
            calc::cox::PhTransform::Rank => "the rank of the failure time",
            calc::cox::PhTransform::Km => "1 - the Kaplan-Meier estimate before the failure time",
            calc::cox::PhTransform::Identity => "the failure time",
            calc::cox::PhTransform::Log => "the log of the failure time",
        };
        let each: Vec<String> = xcs
            .iter()
            .zip(&test.per_covariate)
            .map(|(c, (stat, p))| format!("{} chi-square {} (Prob {})", column_label(c), num(*stat), num(*p)))
            .collect();
        notes.push(format!(
            "Proportional hazards (Grambsch-Therneau, scaled Schoenfeld residuals against {scale}): {}; global chi-square {} ({k} df, Prob {}).",
            each.join("; "),
            num(test.global.0),
            num(test.global.1)
        ));
    }
    // Residuals (T457): per case and per failure, on the sheet.
    if spec.residuals {
        let comment = "Cox residuals";
        sheet_cols.push(column_of("Row", comment, Designation::X, source_rows.clone()));
        sheet_cols.push(column_of("Case time", comment, Designation::Y, t.clone()));
        sheet_cols.push(column_of("Martingale residual", comment, Designation::Y, fit.martingale.clone()));
        sheet_cols.push(column_of("Deviance residual", comment, Designation::Y, fit.deviance.clone()));
        sheet_cols.push(column_of(
            "Failure row",
            "Schoenfeld residuals",
            Designation::X,
            fit.schoenfeld.iter().map(|(c, _)| source_rows[*c]).collect(),
        ));
        sheet_cols.push(column_of(
            "Failure time",
            "Schoenfeld residuals",
            Designation::Y,
            fit.schoenfeld.iter().map(|(c, _)| t[*c]).collect(),
        ));
        for (j, c) in xcs.iter().enumerate() {
            sheet_cols.push(column_of(
                &format!("Schoenfeld {}", column_label(c)),
                "Schoenfeld residuals",
                Designation::Y,
                fit.schoenfeld.iter().map(|(_, r)| r[j]).collect(),
            ));
        }
        notes.push(
            "Residuals: martingale d - H0(t) exp(x'b), deviance sign(M) sqrt(-2 (M + d ln(d - M))) per case; Schoenfeld x - the risk-set mean of x per failure.".to_string(),
        );
    }
    let graph = spec.graph.then(|| XyGraph {
        plots,
        x_scale: ScaleKind::Linear,
        y_scale: ScaleKind::Linear,
        x_title: column_label(tc),
        y_title: "Survival".into(),
        look: Default::default(),
    });
    Ok(Output { table: Some(table), on_sheet: sheet_cols, notes, graph, more_graphs: Vec::new() }.shift_graphs(shift))
}

/// The failure windows of the start and end columns (T384), each row as often as its frequency, and the label
/// "start to end".
fn failure_windows(
    sheet: &Worksheet,
    columns: &[ColumnId],
    counts: &Option<Vec<f64>>,
    rows: RowRange,
    what: &str,
) -> Result<(Vec<Obs>, String), AnalysisError> {
    let [start_id, end_id] = columns else {
        return Err(AnalysisError::BadOption(format!(
            "{what} takes two columns: the start and the end of each failure window"
        )));
    };
    let column = |id: ColumnId| sheet.column(id).ok_or(AnalysisError::NoSuchColumn(id));
    let sc = column(*start_id)?;
    let ec = column(*end_id)?;
    let read = |c: &Column| -> Result<Vec<f64>, AnalysisError> {
        sheet.values_of(c).map(|v| v.into_owned()).ok_or_else(|| AnalysisError::NotNumeric(c.short_name.clone()))
    };
    let starts = read(sc)?;
    let ends = read(ec)?;
    let n = starts.len().max(ends.len());
    if n == 0 {
        return Err(AnalysisError::Empty(sc.short_name.clone()));
    }
    let (first, last) = resolve_rows(rows, &sc.short_name, &[], n)?;
    let mut obs = Vec::new();
    for r in first..=last {
        let a = starts.get(r).copied().unwrap_or(f64::NAN);
        let b = ends.get(r).copied().unwrap_or(f64::NAN);
        if a.is_nan() && b.is_nan() {
            continue;
        }
        match Obs::from_columns(a, b) {
            Some(o) => {
                for _ in 0..count_of(counts, r)? {
                    obs.push(o);
                }
                check_total(obs.len())?;
            }
            None => {
                return Err(AnalysisError::BadOption(format!("row {}: the start {a} is after the end {b}", r + 1)));
            }
        }
    }
    Ok((obs, format!("{} to {}", column_label(sc), column_label(ec))))
}

/// The counts of exact, right-, left- and interval-censored rows.
fn censoring_counts(obs: &[Obs]) -> [f64; 4] {
    let count = |f: fn(&Obs) -> bool| obs.iter().filter(|o| f(o)).count() as f64;
    [
        count(|o| matches!(o, Obs::Exact(_))),
        count(|o| matches!(o, Obs::Right(_))),
        count(|o| matches!(o, Obs::Left(_))),
        count(|o| matches!(o, Obs::Interval(..))),
    ]
}

pub(super) fn run_turnbull(
    spec: &TurnbullSpec,
    sheet: &Worksheet,
    columns: &[ColumnId],
    rows: RowRange,
) -> Result<Output, AnalysisError> {
    let (columns, counts) = split_frequency(sheet, columns, spec.frequency)?;
    let (obs, label) =
        failure_windows(sheet, columns, &counts, rows, "the nonparametric analysis with arbitrary censoring")?;
    let fit = calc::turnbull::turnbull(&obs).map_err(|e| AnalysisError::BadOption(e.to_string()))?;
    let [exact, right, left, interval] = censoring_counts(&obs);
    let finite = |v: f64| if v.is_finite() { v } else { f64::NAN };
    let mut table = ResultTable::new(&["Lower", "Upper", "Probability", "Survival", "Cumulative failure"]);
    for j in 0..fit.probability.len() {
        let (lo, hi) = (fit.lower[j], fit.upper[j]);
        let name = if fit.lower_closed[j] && lo == hi {
            format!("[{}, {}]", num(lo), num(hi))
        } else {
            let lo_text = if lo.is_finite() { num(lo) } else { "-inf".into() };
            let hi_text = if hi.is_finite() { format!("{}]", num(hi)) } else { "inf)".into() };
            format!("{}{lo_text}, {hi_text}", if fit.lower_closed[j] { "[" } else { "(" })
        };
        table.push_row(
            name,
            vec![finite(lo), finite(hi), fit.probability[j], fit.survival[j], 1.0 - fit.survival[j]],
            vec![String::new(); 5],
        );
    }
    let mut notes = vec![
        format!(
            "Turnbull estimate of {label} from {} rows ({exact} exact, {right} right-, {left} left-, {interval} interval-censored): {} intervals, log-likelihood {}, {} self-consistency iterations.",
            fit.n,
            fit.probability.len(),
            num(fit.log_likelihood),
            fit.iterations
        ),
        "The estimate gives the probability of failing in each interval, not where inside it; the graph draws the survival straight across each interval.".into(),
    ];
    if !fit.converged {
        notes.push(format!(
            "The iteration stopped after {} steps before the Kuhn-Tucker condition held to 1e-12; the probabilities are close, not exact.",
            fit.iterations
        ));
    }
    let mut sheet_cols = vec![SheetColumn {
        designation: Designation::Label,
        long_name: "Interval".into(),
        units: String::new(),
        comments: String::new(),
        data: ColumnData::text(table.rows.iter().map(|(l, _)| l.clone()).collect()),
    }];
    for (i, h) in table.headers.iter().enumerate() {
        sheet_cols.push(column_of(h, "", Designation::Y, table.rows.iter().map(|(_, v)| v[i]).collect()));
    }
    let shift = sheet_cols.len();
    let mut graph = None;
    if spec.graph {
        // Flat between the intervals, straight across each; the curve starts at 0 (or the first time below it).
        let lowest = fit.lower.iter().chain(&fit.upper).copied().filter(|v| v.is_finite()).fold(0.0, f64::min);
        let (mut gx, mut gy) = (vec![lowest], vec![1.0]);
        let mut prev = 1.0;
        for j in 0..fit.probability.len() {
            if fit.lower[j].is_finite() {
                gx.push(fit.lower[j]);
                gy.push(prev);
            }
            if !fit.upper[j].is_finite() {
                break;
            }
            gx.push(fit.upper[j]);
            gy.push(fit.survival[j]);
            prev = fit.survival[j];
        }
        let comment = format!("Turnbull survival of {label}");
        sheet_cols.push(column_of("Time", &comment, Designation::X, gx));
        sheet_cols.push(column_of("Survival", &comment, Designation::Y, gy));
        graph = Some(XyGraph {
            plots: vec![GraphPlot {
                x: 0,
                y: 1,
                kind: PlotKind::Line,
                label: "Turnbull survival".into(),
                line_of: None,
            }],
            x_scale: ScaleKind::Linear,
            y_scale: ScaleKind::Linear,
            x_title: "Time".into(),
            y_title: "Survival probability".into(),
            look: Default::default(),
        });
    }
    Ok(Output { table: Some(table), on_sheet: sheet_cols, notes, graph, more_graphs: Vec::new() }.shift_graphs(shift))
}

pub(super) fn run_arbitrary(
    spec: &LifeDistArbitrarySpec,
    sheet: &Worksheet,
    columns: &[ColumnId],
    rows: RowRange,
) -> Result<Output, AnalysisError> {
    let (columns, counts) = split_frequency(sheet, columns, spec.frequency)?;
    let (obs, label) =
        failure_windows(sheet, columns, &counts, rows, "distribution analysis with arbitrary censoring")?;
    let fit = fit_arbitrary(&obs, spec.dist).map_err(|e| AnalysisError::BadOption(e.to_string()))?;
    let [exact, right, left, interval] = censoring_counts(&obs);
    let k = fit.params.len() as f64;
    let mut table = ResultTable::new(&[
        "N",
        "Exact",
        "Right-censored",
        "Left-censored",
        "Interval-censored",
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
            obs.len() as f64,
            exact,
            right,
            left,
            interval,
            fit.params[0],
            fit.params.get(1).copied().unwrap_or(f64::NAN),
            fit.log_likelihood,
            2.0 * k - 2.0 * fit.log_likelihood,
            fit.mean,
            fit.median,
        ],
        vec![String::new(); 11],
    );
    let names = spec.dist.param_names();
    let params: Vec<String> = names.iter().zip(&fit.params).map(|(n, v)| format!("{n} {}", num(*v))).collect();
    let mut notes = vec![
        format!(
            "{} fit to {} rows ({exact} exact, {right} right-, {left} left-, {interval} interval-censored): {}; log-likelihood {}, mean {}, median {}.",
            spec.dist.name(),
            obs.len(),
            params.join(", "),
            num(fit.log_likelihood),
            num(fit.mean),
            num(fit.median)
        ),
        "Rows: equal start and end = exact, empty end = still working at the start, empty start = failed before the end, otherwise failed between them.".into(),
    ];
    let mut sheet_cols = vec![SheetColumn {
        designation: Designation::Label,
        long_name: "Data".into(),
        units: String::new(),
        comments: String::new(),
        data: ColumnData::text(vec![label.clone()]),
    }];
    for (i, h) in table.headers.iter().enumerate() {
        sheet_cols.push(column_of(h, "", Designation::Y, table.rows.iter().map(|(_, v)| v[i]).collect()));
    }
    let shift = sheet_cols.len();
    let mut on_sheet = Vec::new();
    let mut graph = None;
    let mut more_graphs = Vec::new();
    if spec.graph {
        let times: Vec<f64> = obs
            .iter()
            .flat_map(|o| match *o {
                Obs::Exact(t) | Obs::Right(t) | Obs::Left(t) => vec![t],
                Obs::Interval(a, b) => vec![a, b],
            })
            .collect();
        let hi = times.iter().copied().fold(f64::NEG_INFINITY, f64::max) * 1.2;
        let lo = if spec.dist == LifeDist::Normal { fit.quantile(0.001).min(0.0) } else { 0.0 };
        let curve: Vec<f64> = (0..=200).map(|i| lo + (hi - lo) * f64::from(i) / 200.0).collect();
        let comment = format!("Distribution analysis of {label}");
        on_sheet.push(column_of("Time", &comment, Designation::X, curve.clone()));
        on_sheet.push(column_of(
            "Percent failed",
            &comment,
            Designation::Y,
            curve.iter().map(|t| 100.0 * fit.cdf(*t)).collect(),
        ));
        on_sheet.push(column_of(
            "Percent surviving",
            &comment,
            Designation::Y,
            curve.iter().map(|t| 100.0 * fit.survival(*t)).collect(),
        ));
        graph = Some(XyGraph {
            plots: vec![
                GraphPlot {
                    x: 0,
                    y: 1,
                    kind: PlotKind::Line,
                    label: format!("Fitted {} CDF", spec.dist.name()),
                    line_of: None,
                },
                GraphPlot { x: 0, y: 2, kind: PlotKind::Line, label: "Fitted survival".into(), line_of: None },
            ],
            x_scale: ScaleKind::Linear,
            y_scale: ScaleKind::Linear,
            x_title: "Time".into(),
            y_title: "Percent".into(),
            look: Default::default(),
        });
        // Probability plot (T438): the Turnbull estimate's percent failed at the upper end of each interval that holds
        // probability, on the fitted distribution's probability axis, with the fitted line.
        let tb = calc::turnbull::turnbull(&obs).map_err(|e| AnalysisError::BadOption(e.to_string()))?;
        let (px, py): (Vec<f64>, Vec<f64>) = tb
            .upper
            .iter()
            .zip(&tb.survival)
            .zip(&tb.probability)
            .filter(|((u, s), p)| {
                u.is_finite() && **p > 1e-9 && **s > 1e-12 && (spec.dist == LifeDist::Normal || **u > 0.0)
            })
            .map(|((u, s), _)| (*u, 100.0 * (1.0 - s)))
            .unzip();
        let normal = spec.dist == LifeDist::Normal;
        let positive: Vec<f64> = times.iter().copied().filter(|t| *t > 0.0).collect();
        let (a, b) = if normal {
            (fit.quantile(0.001), fit.quantile(0.999))
        } else {
            let lo = positive.iter().copied().fold(f64::INFINITY, f64::min);
            let hi = positive.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            (lo * 0.5, hi * 1.1)
        };
        let line: (Vec<f64>, Vec<f64>) = (0..=200)
            .map(|i| {
                let f = f64::from(i) / 200.0;
                if normal { a + (b - a) * f } else { a * (b / a).powf(f) }
            })
            .map(|t| (t, 100.0 * fit.cdf(t)))
            .filter(|(_, p)| (0.001..=99.999).contains(p))
            .unzip();
        let comment = format!("Probability plot of {label}");
        let x1 = on_sheet.len();
        on_sheet.push(column_of("Turnbull upper end", &comment, Designation::X, px));
        on_sheet.push(column_of("Percent failed (Turnbull)", &comment, Designation::Y, py));
        on_sheet.push(column_of("Line time", &comment, Designation::X, line.0));
        on_sheet.push(column_of("Fitted percent failed", &comment, Designation::Y, line.1));
        let (x_scale, y_scale) = match spec.dist {
            LifeDist::Weibull | LifeDist::Exponential => (ScaleKind::Log10, ScaleKind::Weibull),
            LifeDist::Lognormal => (ScaleKind::Log10, ScaleKind::Probability),
            LifeDist::Normal => (ScaleKind::Linear, ScaleKind::Probability),
        };
        more_graphs.push(XyGraph {
            plots: vec![
                GraphPlot {
                    x: x1,
                    y: x1 + 1,
                    kind: PlotKind::Scatter,
                    label: "Turnbull estimate".into(),
                    line_of: None,
                },
                GraphPlot {
                    x: x1 + 2,
                    y: x1 + 3,
                    kind: PlotKind::Line,
                    label: format!("Fitted {}", spec.dist.name()),
                    line_of: None,
                },
            ],
            x_scale,
            y_scale,
            x_title: "Time".into(),
            y_title: "Percent failed".into(),
            look: Default::default(),
        });
        notes.push(
            "Probability plot: the Turnbull estimate's percent failed at the upper end of each interval holding probability, with the fitted line.".into(),
        );
    }
    sheet_cols.extend(on_sheet);
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

    #[test]
    fn a_frequency_counts_a_row_as_often_as_copies_of_it() {
        let mut p = Project::new();
        let book = p.add_workbook("freq");
        // Rows with counts 2, 1, 3, 1, 2 and the same data written out row by row.
        let (t, c, f) = ([2.0, 4.0, 6.0, 8.0, 9.0], [0.0, 0.0, 1.0, 0.0, 1.0], [2.0, 1.0, 3.0, 1.0, 2.0]);
        let mut te = Vec::new();
        let mut ce = Vec::new();
        for i in 0..5 {
            for _ in 0..f[i] as usize {
                te.push(t[i]);
                ce.push(c[i]);
            }
        }
        let add = |p: &mut Project, v: Vec<f64>| p.add_column(book, 0, Designation::Y, ColumnData::numeric(v)).unwrap();
        let (tc, cc, fc) = (add(&mut p, t.to_vec()), add(&mut p, c.to_vec()), add(&mut p, f.to_vec()));
        let (tec, cec) = (add(&mut p, te), add(&mut p, ce));
        let spec = LifeDistSpec { graph: false, frequency: true, ..Default::default() };
        let weighted = p.analyze(&request(vec![tc, cc, fc], spec)).unwrap();
        let spec = LifeDistSpec { graph: false, ..Default::default() };
        let expanded = p.analyze(&request(vec![tec, cec], spec)).unwrap();
        let (w, e) = (&weighted.table.as_ref().unwrap().rows[0].1, &expanded.table.as_ref().unwrap().rows[0].1);
        assert_eq!(w, e, "the same fit");
        assert_eq!(w[0], 9.0);
        // The same with the start/end form: equal start and end are exact failures.
        let any = |cols: Vec<ColumnId>, frequency: bool| Request {
            columns: cols,
            rows: RowRange::All,
            tool: Tool::LifeDistArbitrary(LifeDistArbitrarySpec { graph: false, frequency, ..Default::default() }),
        };
        let ends = add(&mut p, vec![2.0, 4.0, f64::NAN, 8.0, f64::NAN]);
        let w = p.analyze(&any(vec![tc, ends, fc], true)).unwrap();
        let ends_e = add(&mut p, vec![2.0, 2.0, 4.0, f64::NAN, f64::NAN, f64::NAN, 8.0, f64::NAN, f64::NAN]);
        let e = p.analyze(&any(vec![tec, ends_e], false)).unwrap();
        assert_eq!(w.table.as_ref().unwrap().rows[0].1, e.table.as_ref().unwrap().rows[0].1);
        let bad = add(&mut p, vec![1.0, 0.5, 1.0, 1.0, 1.0]);
        let spec = LifeDistSpec { frequency: true, ..Default::default() };
        assert!(p.analyze(&request(vec![tc, cc, bad], spec)).is_err(), "a fractional frequency");
    }

    #[test]
    fn a_turnbull_estimate_reports_the_intervals_and_a_survival_graph() {
        let mut p = Project::new();
        let book = p.add_workbook("tb");
        let nan = f64::NAN;
        // Rows: failed before 10, between 10 and 20 (twice, by frequency), 20 and 30, at 5, after 30.
        let start = vec![nan, 10.0, 20.0, 5.0, 30.0];
        let end = vec![10.0, 20.0, 30.0, 5.0, nan];
        let freq = vec![1.0, 2.0, 1.0, 1.0, 1.0];
        let cols: Vec<ColumnId> = [start, end, freq]
            .into_iter()
            .map(|v| p.add_column(book, 0, Designation::Y, ColumnData::numeric(v)).unwrap())
            .collect();
        let spec = TurnbullSpec { frequency: true, ..Default::default() };
        let out =
            p.analyze(&Request { columns: cols.clone(), rows: RowRange::All, tool: Tool::Turnbull(spec) }).unwrap();
        let t = out.table.as_ref().unwrap();
        let names: Vec<&str> = t.rows.iter().map(|(l, _)| l.as_str()).collect();
        assert_eq!(names, ["[5, 5]", "(10, 20]", "(20, 30]", "(30, inf)"], "{:?}", out.notes);
        // 6 rows: the left-censored row's mass goes to [5, 5], the only interval before 10.
        assert!((t.rows[1].1[2] - 2.0 / 6.0).abs() < 1e-9 && (t.rows[0].1[2] - 2.0 / 6.0).abs() < 1e-9, "{t:?}");
        assert!(
            t.rows[3].1[1].is_nan()
                && out.notes[0].contains("6 rows (1 exact, 1 right-, 1 left-, 3 interval-censored)")
        );
        let gc = super::super::dist::plot_columns(&p, out.new_graph.unwrap());
        assert_eq!(gc[0].1, "Survival");
        let two =
            Request { columns: cols[..1].to_vec(), rows: RowRange::All, tool: Tool::Turnbull(Default::default()) };
        assert!(p.analyze(&two).is_err());
    }

    #[test]
    fn life_regression_percentiles_at_the_means_and_at_given_values() {
        let mut p = Project::new();
        let book = p.add_workbook("lrp");
        let x: Vec<f64> = (0..20).map(|i| f64::from(i % 2)).collect();
        let t: Vec<f64> =
            (0..20).map(|i| (2.0 - 0.8 * f64::from(i % 2) + 0.3 * (f64::from(i) * 1.3).sin()).exp()).collect();
        let c: Vec<f64> = (0..20).map(|i| f64::from(u8::from(i % 7 == 3))).collect();
        let cols: Vec<ColumnId> = [x, t, c]
            .into_iter()
            .map(|v| p.add_column(book, 0, Designation::Y, ColumnData::numeric(v)).unwrap())
            .collect();
        let run = |p: &mut Project, spec: LifeRegressionSpec| {
            p.analyze(&Request { columns: cols.clone(), rows: RowRange::All, tool: Tool::LifeRegression(spec) })
        };
        let out = run(&mut p, LifeRegressionSpec::default()).unwrap();
        assert!(out.notes[2].contains("the covariate means") && out.notes[2].contains("50 %"), "{:?}", out.notes);
        let spec = LifeRegressionSpec { at: Some(vec![1.0]), percentiles: vec![0.5], ..Default::default() };
        let one = run(&mut p, spec).unwrap();
        let spec = LifeRegressionSpec { at: Some(vec![0.0]), percentiles: vec![0.5], ..Default::default() };
        let zero = run(&mut p, spec).unwrap();
        let median = |o: &Report| {
            let s = &p.workbook(book).unwrap().sheets[o.new_sheet.unwrap()];
            s.columns.iter().find(|c| c.long_name == "Percentile life").unwrap().numeric().unwrap()[0]
        };
        // The dose coefficient is the log ratio of the medians.
        let coef = one.table.as_ref().unwrap().rows[1].1[0];
        assert!(((median(&one) / median(&zero)).ln() - coef).abs() < 1e-9);
        // T504: several settings at once, each with its columns and note; a setting of the wrong length is refused.
        let both = LifeRegressionSpec {
            at: Some(vec![1.0]),
            also_at: vec![vec![0.0]],
            percentiles: vec![0.5],
            ..Default::default()
        };
        let out2 = run(&mut p, both).unwrap();
        let notes: Vec<&String> = out2.notes.iter().filter(|n| n.starts_with("Percentile life at")).collect();
        assert_eq!(notes.len(), 2, "{:?}", out2.notes);
        let ws = &p.workbook(book).unwrap().sheets[out2.new_sheet.unwrap()];
        assert!(ws.columns.iter().any(|c| c.long_name == "Percentile life (2)"));
        // T509: the percentile lives against the covariate, with dotted bands.
        let drawn = LifeRegressionSpec { graph: true, percentiles: vec![0.1, 0.5], ..Default::default() };
        let out3 = run(&mut p, drawn).unwrap();
        let g = p.graph(out3.new_graph.expect("a graph")).unwrap();
        assert_eq!(g.layers[0].plots.len(), 6, "a line and two bands per percentile");
        assert_eq!(g.layers[0].y_axis.scale, ScaleKind::Log10);
        let cols = super::super::dist::plot_columns(&p, out3.new_graph.unwrap());
        assert_eq!(cols[0].1, "10 % life");
        assert!(out3.notes.iter().any(|n| n.contains("against") && n.contains("dotted")), "{:?}", out3.notes);
        let wrong = LifeRegressionSpec { also_at: vec![vec![1.0, 2.0]], ..Default::default() };
        assert!(run(&mut p, wrong).is_err());
        let bad = LifeRegressionSpec { at: Some(vec![1.0, 2.0]), ..Default::default() };
        assert!(run(&mut p, bad).is_err());
        let bad = LifeRegressionSpec { percentiles: vec![1.0], ..Default::default() };
        assert!(run(&mut p, bad).is_err());
    }

    #[test]
    fn cox_residuals_and_a_curve_at_given_values() {
        let mut p = Project::new();
        let book = p.add_workbook("coxres");
        let x: Vec<f64> = (0..30).map(|i| f64::from(i % 2)).collect();
        let t: Vec<f64> = (0..30).map(|i| (5.0 + f64::from(i * 7 % 11)) / (1.0 + f64::from(i % 2))).collect();
        let c: Vec<f64> = (0..30).map(|i| if i % 9 == 4 { 1.0 } else { 0.0 }).collect();
        let cols: Vec<ColumnId> = [x, t, c]
            .into_iter()
            .map(|v| p.add_column(book, 0, Designation::Y, ColumnData::numeric(v)).unwrap())
            .collect();
        let spec = CoxSpec { at: Some(vec![1.0]), residuals: true, ..Default::default() };
        let out = p.analyze(&Request { columns: cols.clone(), rows: RowRange::All, tool: Tool::Cox(spec) }).unwrap();
        let s = &p.workbook(book).unwrap().sheets[out.new_sheet.unwrap()];
        let col = |name: &str| s.columns.iter().find(|c| c.long_name == name).unwrap().numeric().unwrap().to_vec();
        let m = col("Martingale residual");
        assert_eq!(m.len(), 30);
        assert!(m.iter().sum::<f64>().abs() < 1e-9, "Breslow martingale residuals sum to 0");
        let sch: Vec<f64> =
            s.columns.iter().find(|c| c.long_name.starts_with("Schoenfeld ")).unwrap().numeric().unwrap().to_vec();
        assert!(sch.iter().sum::<f64>().abs() < 1e-7 && sch.len() == 27, "{}", sch.len());
        assert_eq!(col("Row")[0], 1.0);
        let gc = super::super::dist::plot_columns(&p, out.new_graph.unwrap());
        assert_eq!(gc.len(), 2, "the curve at the means and at A = 1");
        // The proportional-hazards test (T458).
        let ph = CoxSpec { ph_test: true, graph: false, ..Default::default() };
        let out =
            p.analyze(&Request { columns: cols.clone(), rows: RowRange::All, tool: Tool::Cox(ph.clone()) }).unwrap();
        assert!(out.notes.iter().any(|n| n.starts_with("Proportional hazards (Grambsch-Therneau")), "{:?}", out.notes);
        // T503: the Kaplan-Meier time scale.
        let km = CoxSpec { ph_transform: calc::cox::PhTransform::Km, ..ph };
        let out = p.analyze(&Request { columns: cols.clone(), rows: RowRange::All, tool: Tool::Cox(km) }).unwrap();
        assert!(out.notes.iter().any(|n| n.contains("against 1 - the Kaplan-Meier estimate")), "{:?}", out.notes);
        let bad = CoxSpec { at: Some(vec![1.0, 2.0]), ..Default::default() };
        assert!(p.analyze(&Request { columns: cols, rows: RowRange::All, tool: Tool::Cox(bad) }).is_err());
    }

    #[test]
    fn a_stratified_cox_model_draws_a_curve_per_stratum() {
        let mut p = Project::new();
        let book = p.add_workbook("coxstrata");
        let x: Vec<f64> = (0..24).map(|i| f64::from(i % 2)).collect();
        let t: Vec<f64> = (0..24).map(|i| (5.0 + f64::from(i * 7 % 11)) / (1.0 + f64::from(i % 2))).collect();
        let c: Vec<f64> = (0..24).map(|i| if i % 9 == 4 { 1.0 } else { 0.0 }).collect();
        let s: Vec<f64> = (0..24).map(|i| f64::from(i / 8)).collect();
        let cols: Vec<ColumnId> = [x, t, c, s]
            .into_iter()
            .map(|v| p.add_column(book, 0, Designation::Y, ColumnData::numeric(v)).unwrap())
            .collect();
        let spec = CoxSpec { strata: true, ..Default::default() };
        let out = p.analyze(&Request { columns: cols, rows: RowRange::All, tool: Tool::Cox(spec) }).unwrap();
        assert!(out.notes[2].contains("3 strata: 0, 1, 2"), "{:?}", out.notes);
        let gc = super::super::dist::plot_columns(&p, out.new_graph.unwrap());
        let names: Vec<&str> = gc.iter().map(|c| c.1.as_str()).collect();
        assert_eq!(names, ["Survival at the means (0)", "Survival at the means (1)", "Survival at the means (2)"]);
    }

    #[test]
    fn a_cox_frequency_counts_a_row_as_often_as_copies_of_it() {
        let mut p = Project::new();
        let book = p.add_workbook("coxfreq");
        let x = vec![0.0, 1.0, 0.0, 1.0, 1.0, 0.0];
        let t = vec![2.0, 5.0, 7.0, 3.0, 6.0, 4.0];
        let c = vec![0.0, 0.0, 1.0, 0.0, 0.0, 0.0];
        let f = vec![2.0, 1.0, 1.0, 3.0, 1.0, 1.0];
        let expand = |v: &[f64]| -> Vec<f64> { v.iter().zip(&f).flat_map(|(a, n)| vec![*a; *n as usize]).collect() };
        let mut add = |v: Vec<f64>| p.add_column(book, 0, Designation::Y, ColumnData::numeric(v)).unwrap();
        let weighted = vec![add(x.clone()), add(t.clone()), add(c.clone()), add(f.clone())];
        let copies = vec![add(expand(&x)), add(expand(&t)), add(expand(&c))];
        let spec = CoxSpec { graph: false, frequency: true, ..Default::default() };
        let a = p.analyze(&Request { columns: weighted, rows: RowRange::All, tool: Tool::Cox(spec) }).unwrap();
        let plain = CoxSpec { graph: false, ..Default::default() };
        let b = p.analyze(&Request { columns: copies, rows: RowRange::All, tool: Tool::Cox(plain) }).unwrap();
        let values = |o: &Report| o.table.as_ref().unwrap().rows.iter().map(|r| r.1.clone()).collect::<Vec<_>>();
        assert_eq!(values(&a), values(&b));
        assert!(a.notes[0].contains("9 cases"), "{:?}", a.notes);
    }

    #[test]
    fn a_cox_model_reports_hazard_ratios_and_a_survival_graph() {
        let mut p = Project::new();
        let book = p.add_workbook("cox");
        let x: Vec<f64> = (0..30).map(|i| f64::from(i % 2)).collect();
        // Group 1 fails about twice as fast.
        let t: Vec<f64> = (0..30).map(|i| (5.0 + f64::from(i * 7 % 11)) / (1.0 + f64::from(i % 2))).collect();
        let c: Vec<f64> = (0..30).map(|i| if i % 9 == 4 { 1.0 } else { 0.0 }).collect();
        let cols: Vec<ColumnId> = [x, t, c]
            .into_iter()
            .map(|v| p.add_column(book, 0, Designation::Y, ColumnData::numeric(v)).unwrap())
            .collect();
        let out =
            p.analyze(&Request { columns: cols, rows: RowRange::All, tool: Tool::Cox(Default::default()) }).unwrap();
        let t = out.table.as_ref().unwrap();
        assert!(t.rows[0].1[4] > 1.5, "the hazard ratio of group 1: {:?}", t.rows[0]);
        assert!(out.notes[0].contains("30 cases") && out.notes[1].contains("likelihood ratio"), "{:?}", out.notes);
        let gc = super::super::dist::plot_columns(&p, out.new_graph.unwrap());
        assert_eq!(gc[0].1, "Survival at the means");
    }

    #[test]
    fn a_covariate_shortens_life_in_the_regression() {
        let mut p = Project::new();
        let book = p.add_workbook("lr");
        let x: Vec<f64> = (0..20).map(|i| f64::from(i % 2)).collect();
        let t: Vec<f64> =
            (0..20).map(|i| (2.0 - 0.8 * f64::from(i % 2) + 0.3 * (f64::from(i) * 1.3).sin()).exp()).collect();
        let c: Vec<f64> = (0..20).map(|i| if i % 7 == 3 { 1.0 } else { 0.0 }).collect();
        let cols: Vec<ColumnId> = [x, t, c]
            .into_iter()
            .map(|v| p.add_column(book, 0, Designation::Y, ColumnData::numeric(v)).unwrap())
            .collect();
        let out = p
            .analyze(&Request { columns: cols, rows: RowRange::All, tool: Tool::LifeRegression(Default::default()) })
            .unwrap();
        let t = out.table.as_ref().unwrap();
        assert_eq!(t.rows.len(), 3, "intercept, covariate, scale");
        assert!((t.rows[1].1[0] + 0.8).abs() < 0.2, "{:?}", t.rows[1]);
        assert!(out.notes[0].contains("20 rows, 17 failures"), "{:?}", out.notes);
    }

    #[test]
    fn interval_and_left_censored_rows_are_counted_and_fitted() {
        let mut p = Project::new();
        let book = p.add_workbook("ic");
        let s = p
            .add_column(book, 0, Designation::Y, ColumnData::numeric(vec![f64::NAN, 10.0, 20.0, 5.0, 30.0, 12.0]))
            .unwrap();
        let e = p
            .add_column(book, 0, Designation::Y, ColumnData::numeric(vec![10.0, 20.0, 30.0, 5.0, f64::NAN, 18.0]))
            .unwrap();
        let out = p
            .analyze(&Request {
                columns: vec![s, e],
                rows: RowRange::All,
                tool: Tool::LifeDistArbitrary(Default::default()),
            })
            .unwrap();
        let row = &out.table.as_ref().unwrap().rows[0].1;
        assert_eq!(&row[..5], &[6.0, 1.0, 1.0, 1.0, 3.0]);
        assert!(row[5] > 0.0 && row[6] > 0.0, "{row:?}");
        let gc = super::super::dist::plot_columns(&p, out.new_graph.unwrap());
        assert_eq!(gc[1].1, "Percent surviving");
        let bad = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![40.0])).unwrap();
        assert!(
            p.analyze(&Request {
                columns: vec![bad, e],
                rows: RowRange::All,
                tool: Tool::LifeDistArbitrary(Default::default())
            })
            .is_err()
        );
    }
}
