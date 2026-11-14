//! The distribution tools of `Project::analyze`: distribution fit, probability plot and Q-Q plot. The numerics
//! are in `sheaf_calc` (`distfit`); what the tools compute and report is written down in
//! `docs/spec/statistics.md`, section 12.

use super::statistics::num;
use super::*;
use crate::graph::{PlotKind, ScaleKind};
use calc::Family;

/// What a tool of this module produces.
pub(super) struct Output {
    pub table: Option<ResultTable>,
    pub on_sheet: Vec<SheetColumn>,
    pub notes: Vec<String>,
    pub graph: Option<XyGraph>,
}

/// The graph of a probability plot: plots of columns of the result sheet (by position), axis scales and titles.
pub(super) struct XyGraph {
    pub plots: Vec<GraphPlot>,
    pub x_scale: ScaleKind,
    pub y_scale: ScaleKind,
    pub x_title: String,
    pub y_title: String,
}

pub(super) struct GraphPlot {
    pub x: usize,
    pub y: usize,
    pub kind: PlotKind,
    pub label: String,
    /// A reference line: the index of the plot of points it belongs to (it gets that colour).
    pub line_of: Option<usize>,
}

pub(super) fn run(tool: &Tool, inputs: &[Input<'_>]) -> Result<Output, AnalysisError> {
    match tool {
        Tool::DistFit(spec) => dist_fit(spec, inputs),
        Tool::ProbPlot(spec) => prob_plot(spec, inputs),
        _ => unreachable!("only the distribution tools come here"),
    }
}

fn label_column(name: &str, values: Vec<String>) -> SheetColumn {
    SheetColumn {
        designation: Designation::Label,
        long_name: name.to_string(),
        units: String::new(),
        comments: String::new(),
        data: ColumnData::text(values),
    }
}

fn number_column(name: &str, units: &str, designation: Designation, values: Vec<f64>) -> SheetColumn {
    SheetColumn {
        designation,
        long_name: name.to_string(),
        units: units.to_string(),
        comments: String::new(),
        data: ColumnData::numeric(values),
    }
}

// ----- distribution fit ------------------------------------------------------------------------------------

/// What [`Tool::DistFit`] does: fit each family to each chosen column by maximum likelihood.
#[derive(Debug, Clone, PartialEq)]
pub struct DistFitSpec {
    /// The families to fit, in the order of the result rows. A family that cannot take a column (negative
    /// numbers for the lognormal) is left out for that column with a note.
    pub families: Vec<Family>,
}

impl Default for DistFitSpec {
    fn default() -> Self {
        DistFitSpec { families: Family::ALL.to_vec() }
    }
}

/// The unit of each parameter of the family for data in `units`: location and scale parameters carry the unit
/// of the data, shapes and the parameters of the logarithm none.
fn param_units(family: Family, units: &str) -> [String; 2] {
    let u = units.to_string();
    match family {
        Family::Normal => [u.clone(), u],
        Family::Lognormal => [String::new(), String::new()],
        Family::Exponential => [u, String::new()],
        Family::Gamma | Family::Weibull => [String::new(), u],
    }
}

fn param_text(family: Family, params: &[f64]) -> String {
    family.param_names().iter().zip(params).map(|(n, v)| format!("{n} = {}", num(*v))).collect::<Vec<_>>().join(", ")
}

fn dist_fit(spec: &DistFitSpec, inputs: &[Input<'_>]) -> Result<Output, AnalysisError> {
    if spec.families.is_empty() {
        return Err(AnalysisError::BadOption("choose at least one distribution to fit".into()));
    }
    let mut families: Vec<Family> = Vec::new();
    for f in &spec.families {
        if !families.contains(f) {
            families.push(*f);
        }
    }
    let mut table = ResultTable::new(&[
        "N",
        "Parameter 1",
        "Parameter 2",
        "Log-likelihood",
        "AIC",
        "BIC",
        "KS D",
        "KS Prob",
        "Anderson-Darling A2",
        "Anderson-Darling Prob",
    ]);
    let mut notes = Vec::new();
    // Text columns of the result sheet: data, distribution, the two parameter names.
    let (mut data_names, mut dist_names, mut p1_names, mut p2_names) = (vec![], vec![], vec![], vec![]);
    for inp in inputs {
        let mut best: Option<(f64, Family)> = None;
        let mut fitted = 0;
        for &family in &families {
            let r = match calc::dist_fit(family, inp.ys()) {
                Ok(r) => r,
                Err(e) => {
                    notes.push(format!("{}: the {} distribution was not fitted: {e}.", inp.label, family.name()));
                    continue;
                }
            };
            fitted += 1;
            let names = family.param_names();
            let units = param_units(family, &inp.units);
            let p2 = r.params.get(1).copied().unwrap_or(f64::NAN);
            let mut row_units = vec![String::new(); 10];
            row_units[1] = units[0].clone();
            row_units[2] = units[1].clone();
            table.push_row(
                format!("{} - {}", inp.label, family.name()),
                vec![r.n as f64, r.params[0], p2, r.log_likelihood, r.aic, r.bic, r.ks_d, r.ks_p, r.ad, r.ad_p],
                row_units,
            );
            data_names.push(inp.label.clone());
            dist_names.push(family.name().to_string());
            p1_names.push(names[0].to_string());
            p2_names.push(names.get(1).map_or(String::new(), |n| n.to_string()));
            let ad_p = if r.ad_p.is_nan() { String::new() } else { format!(" (approximate p = {})", num(r.ad_p)) };
            notes.push(format!(
                "{}, {}: {}; log-likelihood {}, AIC {}, BIC {}; Kolmogorov-Smirnov D = {} (p = {} for a distribution known in advance); Anderson-Darling A2 = {}{ad_p}.",
                inp.label,
                family.name(),
                param_text(family, &r.params),
                num(r.log_likelihood),
                num(r.aic),
                num(r.bic),
                num(r.ks_d),
                num(r.ks_p),
                num(r.ad),
            ));
            if best.is_none_or(|(a, _)| r.aic < a) {
                best = Some((r.aic, family));
            }
        }
        if fitted == 0 {
            return Err(AnalysisError::BadOption(format!(
                "no distribution can be fitted to {}: it needs at least two numbers with some spread in the chosen rows",
                inp.name
            )));
        }
        if let (Some((_, f)), true) = (best, fitted > 1) {
            notes.push(format!("{}: the lowest AIC is that of the {} distribution.", inp.label, f.name()));
        }
    }
    let mut on_sheet = vec![label_column("Data", data_names), label_column("Distribution", dist_names)];
    let col = |i: usize| table.rows.iter().map(|(_, v)| v[i]).collect::<Vec<f64>>();
    on_sheet.push(number_column("N", "", Designation::Y, col(0)));
    on_sheet.push(label_column("Parameter 1", p1_names));
    on_sheet.push(number_column("Value 1", "", Designation::Y, col(1)));
    on_sheet.push(label_column("Parameter 2", p2_names));
    on_sheet.push(number_column("Value 2", "", Designation::Y, col(2)));
    for (i, h) in table.headers.iter().enumerate().skip(3) {
        on_sheet.push(number_column(h, "", Designation::Y, col(i)));
    }
    Ok(Output { table: Some(table), on_sheet, notes, graph: None })
}

// ----- probability plot and Q-Q plot -----------------------------------------------------------------------

/// Which plot [`ProbPlotSpec`] makes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbPlotKind {
    /// The data on X against the plotting position in percent on Y, on a probability axis; the reference is the
    /// distribution function of the maximum likelihood fit.
    Probability,
    /// The quantiles of the distribution on X against the ordered data on Y (`scipy.stats.probplot`); the
    /// reference is the least-squares line through the points.
    QQ,
}

/// What [`Tool::ProbPlot`] does: one plot of each chosen column against one family.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProbPlotSpec {
    pub kind: ProbPlotKind,
    pub family: Family,
    /// Make the graph (default); without it only the result sheet is made.
    pub graph: bool,
}

impl Default for ProbPlotSpec {
    fn default() -> Self {
        ProbPlotSpec { kind: ProbPlotKind::Probability, family: Family::Normal, graph: true }
    }
}

/// The scale of the percent axis that straightens the distribution function of the family.
fn percent_scale(family: Family) -> ScaleKind {
    match family {
        Family::Weibull | Family::Exponential => ScaleKind::Weibull,
        _ => ScaleKind::Probability,
    }
}

fn prob_plot(spec: &ProbPlotSpec, inputs: &[Input<'_>]) -> Result<Output, AnalysisError> {
    let family = spec.family;
    let kind = spec.kind;
    let mut table = match kind {
        ProbPlotKind::Probability => ResultTable::new(&["N", "Parameter 1", "Parameter 2", "Correlation r"]),
        ProbPlotKind::QQ => ResultTable::new(&["N", "Shape", "Slope", "Intercept", "Correlation r"]),
    };
    let mut on_sheet: Vec<SheetColumn> = Vec::new();
    let mut plots: Vec<GraphPlot> = Vec::new();
    let mut notes = Vec::new();
    let all_positive = inputs.iter().all(|i| i.ys().iter().all(|v| v.is_nan() || *v > 0.0));
    let log_x = kind == ProbPlotKind::Probability
        && (matches!(family, Family::Lognormal | Family::Weibull) || (family == Family::Exponential && all_positive));
    for inp in inputs {
        let bad = |e: calc::FitError| AnalysisError::BadOption(format!("{}: {e}", inp.name));
        let pp = calc::probplot(family, inp.ys(), None).map_err(bad)?;
        let n = pp.ordered.len();
        let first = on_sheet.len();
        let u = &inp.units;
        let with = |mut c: SheetColumn| {
            c.comments = inp.label.clone();
            c
        };
        match kind {
            ProbPlotKind::Probability => {
                let fit = calc::dist_fit(family, inp.ys()).map_err(bad)?;
                let percent: Vec<f64> = pp.positions.iter().map(|p| p * 100.0).collect();
                // The fitted distribution function over the range of the data, where its percent stays
                // inside what the probability axes can show.
                let (lo, hi) = (pp.ordered[0], pp.ordered[n - 1]);
                let (mut rx, mut ry) = (Vec::new(), Vec::new());
                for i in 0..=100 {
                    let t = f64::from(i) / 100.0;
                    let x = if log_x && lo > 0.0 { lo * (hi / lo).powf(t) } else { lo + (hi - lo) * t };
                    let p = calc::dist_cdf(family, &fit.params, x) * 100.0;
                    if (0.01..=99.99).contains(&p) {
                        rx.push(x);
                        ry.push(p);
                    }
                }
                on_sheet.push(with(number_column("Data (ordered)", u, Designation::X, pp.ordered.clone())));
                on_sheet.push(with(number_column("Percent", "%", Designation::Y, percent)));
                on_sheet.push(with(number_column("Fit X", u, Designation::X, rx)));
                on_sheet.push(with(number_column("Fit percent", "%", Designation::Y, ry)));
                let units = param_units(family, u);
                let p2 = fit.params.get(1).copied().unwrap_or(f64::NAN);
                table.push_row(
                    inp.label.clone(),
                    vec![n as f64, fit.params[0], p2, pp.r],
                    vec![String::new(), units[0].clone(), units[1].clone(), String::new()],
                );
                plots.push(GraphPlot {
                    x: first,
                    y: first + 1,
                    kind: PlotKind::Scatter,
                    label: inp.name.clone(),
                    line_of: None,
                });
                plots.push(GraphPlot {
                    x: first + 2,
                    y: first + 3,
                    kind: PlotKind::Line,
                    label: format!("{} fit of {}", family.name(), inp.name),
                    line_of: Some(plots.len() - 1),
                });
                notes.push(format!(
                    "{}: {} probability plot of {n} values, fitted {}.",
                    inp.label,
                    family.name(),
                    param_text(family, &fit.params)
                ));
            }
            ProbPlotKind::QQ => {
                let (tmin, tmax) = (pp.theoretical[0], pp.theoretical[n - 1]);
                on_sheet.push(with(number_column(
                    &format!("{} quantile", family.name()),
                    "",
                    Designation::X,
                    pp.theoretical.clone(),
                )));
                on_sheet.push(with(number_column("Data (ordered)", u, Designation::Y, pp.ordered.clone())));
                on_sheet.push(with(number_column("Line X", "", Designation::X, vec![tmin, tmax])));
                on_sheet.push(with(number_column(
                    "Line Y",
                    u,
                    Designation::Y,
                    vec![pp.slope * tmin + pp.intercept, pp.slope * tmax + pp.intercept],
                )));
                table.push_row(
                    inp.label.clone(),
                    vec![n as f64, pp.shape.unwrap_or(f64::NAN), pp.slope, pp.intercept, pp.r],
                    vec![String::new(), String::new(), u.clone(), u.clone(), String::new()],
                );
                plots.push(GraphPlot {
                    x: first,
                    y: first + 1,
                    kind: PlotKind::Scatter,
                    label: inp.name.clone(),
                    line_of: None,
                });
                plots.push(GraphPlot {
                    x: first + 2,
                    y: first + 3,
                    kind: PlotKind::Line,
                    label: format!("Line through the points of {}", inp.name),
                    line_of: Some(plots.len() - 1),
                });
                let shape = pp.shape.map_or(String::new(), |s| format!(" with shape {}", num(s)));
                notes.push(format!(
                    "{}: {} Q-Q plot of {n} values{shape}; line y = {} x + {}, r = {}.",
                    inp.label,
                    family.name(),
                    num(pp.slope),
                    num(pp.intercept),
                    num(pp.r)
                ));
            }
        }
    }
    let name_of = |inp: &Input<'_>| {
        if inp.units.is_empty() { inp.name.clone() } else { format!("{} ({})", inp.name, inp.units) }
    };
    let data_title = if inputs.len() == 1 { name_of(&inputs[0]) } else { "Data".to_string() };
    let graph = spec.graph.then(|| match kind {
        ProbPlotKind::Probability => XyGraph {
            plots,
            x_scale: if log_x { ScaleKind::Log10 } else { ScaleKind::Linear },
            y_scale: percent_scale(family),
            x_title: data_title,
            y_title: "Cumulative probability (%)".to_string(),
        },
        ProbPlotKind::QQ => XyGraph {
            plots,
            x_scale: ScaleKind::Linear,
            y_scale: ScaleKind::Linear,
            x_title: format!("{} quantile", family.name()),
            y_title: data_title,
        },
    });
    Ok(Output { table: Some(table), on_sheet, notes, graph })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A project with a gamma-like column (A), a symmetric one (B) and one with a negative number (C).
    fn project() -> (Project, Vec<ColumnId>) {
        let a: Vec<f64> = (1..=24).map(|i| 1.0 + 0.4 * f64::from(i) + (f64::from(i) * 1.7).sin().abs() * 3.0).collect();
        let b: Vec<f64> = (1..=20).map(|i| 10.0 + 2.0 * (f64::from(i) * 2.3).sin()).collect();
        let mut c = b.clone();
        c[3] = -4.0;
        let mut p = Project::new();
        let book = p.add_workbook("fits");
        let ids = [a, b, c]
            .into_iter()
            .map(|v| p.add_column(book, 0, Designation::Y, ColumnData::numeric(v)).unwrap())
            .collect();
        (p, ids)
    }

    fn run(p: &mut Project, columns: &[ColumnId], tool: Tool) -> Report {
        p.analyze(&Request { columns: columns.to_vec(), rows: RowRange::All, tool }).unwrap()
    }

    fn cell(p: &Project, r: &Report, name: &str) -> Vec<f64> {
        let ws = &p.workbook(r.book).unwrap().sheets[r.new_sheet.unwrap()];
        ws.columns
            .iter()
            .find(|c| c.long_name == name)
            .unwrap_or_else(|| panic!("no column {name}"))
            .numeric()
            .unwrap()
            .to_vec()
    }

    fn texts(p: &Project, r: &Report, name: &str) -> Vec<String> {
        let ws = &p.workbook(r.book).unwrap().sheets[r.new_sheet.unwrap()];
        ws.columns.iter().find(|c| c.long_name == name).unwrap().text().unwrap().to_vec()
    }

    #[test]
    fn distribution_fit_lists_every_family_with_its_criteria() {
        let (mut p, ids) = project();
        let r = run(&mut p, &ids[..1], Tool::DistFit(DistFitSpec::default()));
        assert!(r.message.contains("Distribution fit") && r.message.contains("DistFit1"), "{}", r.message);
        assert_eq!(texts(&p, &r, "Distribution"), ["Normal", "Lognormal", "Exponential", "Gamma", "Weibull"]);
        assert_eq!(texts(&p, &r, "Parameter 1")[3], "Shape");
        let (ll, aic, bic) = (cell(&p, &r, "Log-likelihood"), cell(&p, &r, "AIC"), cell(&p, &r, "BIC"));
        let (k, n) = ([2.0, 2.0, 1.0, 2.0, 2.0], 24.0_f64);
        for i in 0..5 {
            assert!((aic[i] - (2.0 * k[i] - 2.0 * ll[i])).abs() < 1e-9);
            assert!((bic[i] - (k[i] * n.ln() - 2.0 * ll[i])).abs() < 1e-9);
        }
        // The normal fit of A: the mean and the standard deviation with divisor n.
        let a = p.workbook(r.book).unwrap().sheets[0].columns[0].numeric().unwrap().to_vec();
        let mean = a.iter().sum::<f64>() / n;
        let sd = (a.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n).sqrt();
        assert!((cell(&p, &r, "Value 1")[0] - mean).abs() < 1e-12);
        assert!((cell(&p, &r, "Value 2")[0] - sd).abs() < 1e-12);
        assert!(cell(&p, &r, "Value 2")[2].is_nan(), "the exponential family has one parameter");
        assert!(r.notes.iter().any(|n| n.contains("lowest AIC")), "{:?}", r.notes);
        let table = r.table.unwrap();
        assert_eq!(table.rows.len(), 5);
        assert_eq!(table.rows[3].0, "A - Gamma");
    }

    #[test]
    fn families_that_cannot_take_the_data_are_skipped_with_a_note() {
        let (mut p, ids) = project();
        let r = run(&mut p, &ids[2..3], Tool::DistFit(DistFitSpec::default()));
        assert_eq!(texts(&p, &r, "Distribution"), ["Normal"]);
        assert!(r.notes.iter().any(|n| n.contains("Lognormal distribution was not fitted")), "{:?}", r.notes);
        let only_gamma = DistFitSpec { families: vec![Family::Gamma] };
        let e = p.analyze(&Request { columns: vec![ids[2]], rows: RowRange::All, tool: Tool::DistFit(only_gamma) });
        assert!(matches!(e, Err(AnalysisError::BadOption(_))), "{e:?}");
        let none = DistFitSpec { families: vec![] };
        assert!(p.analyze(&Request { columns: vec![ids[0]], rows: RowRange::All, tool: Tool::DistFit(none) }).is_err());
    }

    #[test]
    fn a_probability_plot_makes_a_graph_with_points_a_fit_line_and_a_probability_axis() {
        let (mut p, ids) = project();
        let spec = ProbPlotSpec { kind: ProbPlotKind::Probability, family: Family::Normal, graph: true };
        let r = run(&mut p, &ids[1..2], Tool::ProbPlot(spec));
        assert!(r.message.contains("Probability plot") && r.message.contains("ProbPlot1"), "{}", r.message);
        let ordered = cell(&p, &r, "Data (ordered)");
        assert!(ordered.windows(2).all(|w| w[0] <= w[1]));
        let percent = cell(&p, &r, "Percent");
        assert_eq!(percent.len(), 20);
        assert!(percent.iter().all(|v| *v > 0.0 && *v < 100.0));
        let g = p.graph(r.new_graph.expect("a graph")).unwrap();
        let l = &g.layers[0];
        assert_eq!(l.plots.len(), 2);
        assert_eq!((l.plots[0].kind, l.plots[1].kind), (PlotKind::Scatter, PlotKind::Line));
        assert_eq!(l.y_axis.scale, ScaleKind::Probability);
        assert_eq!(l.x_axis.scale, ScaleKind::Linear);
        assert_eq!(l.plots[1].line.color, l.plots[0].symbol.color);
        // The reference line is the fitted normal distribution function: 50 % at the mean.
        let fit_x = cell(&p, &r, "Fit X");
        let fit_p = cell(&p, &r, "Fit percent");
        assert_eq!(fit_x.len(), fit_p.len());
        let table = r.table.unwrap();
        let mean = table.rows[0].1[1];
        let i = fit_x.iter().position(|x| *x >= mean).unwrap();
        assert!((fit_p[i] - 50.0).abs() < 5.0);
    }

    #[test]
    fn weibull_and_lognormal_plots_use_log_x_and_the_matching_percent_axis() {
        let (mut p, ids) = project();
        let w = ProbPlotSpec { kind: ProbPlotKind::Probability, family: Family::Weibull, graph: true };
        let r = run(&mut p, &ids[..1], Tool::ProbPlot(w));
        let l = &p.graph(r.new_graph.unwrap()).unwrap().layers[0];
        assert_eq!((l.x_axis.scale, l.y_axis.scale), (ScaleKind::Log10, ScaleKind::Weibull));
        let ln = ProbPlotSpec { family: Family::Lognormal, ..w };
        let r = run(&mut p, &ids[..1], Tool::ProbPlot(ln));
        let l = &p.graph(r.new_graph.unwrap()).unwrap().layers[0];
        assert_eq!((l.x_axis.scale, l.y_axis.scale), (ScaleKind::Log10, ScaleKind::Probability));
        let g = ProbPlotSpec { family: Family::Gamma, ..w };
        let r = run(&mut p, &ids[..1], Tool::ProbPlot(g));
        let l = &p.graph(r.new_graph.unwrap()).unwrap().layers[0];
        assert_eq!((l.x_axis.scale, l.y_axis.scale), (ScaleKind::Linear, ScaleKind::Probability));
    }

    #[test]
    fn an_exponential_plot_has_a_log_x_axis_only_when_every_number_is_positive() {
        let (mut p, ids) = project();
        let spec = ProbPlotSpec { kind: ProbPlotKind::Probability, family: Family::Exponential, graph: true };
        // all positive: log X and the Weibull percent scale
        let r = run(&mut p, &ids[..1], Tool::ProbPlot(spec));
        let l = &p.graph(r.new_graph.unwrap()).unwrap().layers[0];
        assert_eq!((l.x_axis.scale, l.y_axis.scale), (ScaleKind::Log10, ScaleKind::Weibull));
        // a zero (allowed for the exponential): linear X, a log axis cannot show it
        let mut q = Project::new();
        let book = q.add_workbook("with zero");
        let mut v: Vec<f64> = (1..=15).map(|i| f64::from(i) * 0.7).collect();
        let w = q.add_column(book, 0, Designation::Y, ColumnData::numeric(v.clone())).unwrap();
        v[0] = 0.0;
        let z = q.add_column(book, 0, Designation::Y, ColumnData::numeric(v)).unwrap();
        let r = run(&mut q, &[w], Tool::ProbPlot(spec));
        let l = &q.graph(r.new_graph.unwrap()).unwrap().layers[0];
        assert_eq!(l.x_axis.scale, ScaleKind::Log10);
        // a zero (allowed for the exponential): linear X, a log axis cannot show it
        let r = run(&mut q, &[z], Tool::ProbPlot(spec));
        let l = &q.graph(r.new_graph.unwrap()).unwrap().layers[0];
        assert_eq!((l.x_axis.scale, l.y_axis.scale), (ScaleKind::Linear, ScaleKind::Weibull));
        // a zero in any of several columns makes the axis linear (one axis for the graph)
        let r = run(&mut q, &[w, z], Tool::ProbPlot(spec));
        let l = &q.graph(r.new_graph.unwrap()).unwrap().layers[0];
        assert_eq!(l.x_axis.scale, ScaleKind::Linear);
    }

    #[test]
    fn a_qq_plot_has_the_quantiles_the_ordered_data_and_the_least_squares_line() {
        let (mut p, ids) = project();
        let spec = ProbPlotSpec { kind: ProbPlotKind::QQ, family: Family::Normal, graph: true };
        let r = run(&mut p, &ids[..2], Tool::ProbPlot(spec));
        let g = p.graph(r.new_graph.unwrap()).unwrap();
        assert_eq!(g.layers[0].plots.len(), 4, "points and a line for each of the two columns");
        let table = r.table.clone().unwrap();
        assert_eq!(table.headers, ["N", "Shape", "Slope", "Intercept", "Correlation r"]);
        let (slope, intercept, rr) = (table.rows[1].1[2], table.rows[1].1[3], table.rows[1].1[4]);
        // Column B is a sine: close to normal in the middle, so r is high; the line runs through the mean.
        assert!(rr > 0.9 && rr <= 1.0 && slope > 0.0);
        assert!((intercept - 10.0).abs() < 1.0);
        assert!(table.rows[0].1[1].is_nan(), "the normal family has no shape");
        assert_eq!(cell(&p, &r, "Normal quantile").len(), 24, "the first column has 24 numbers");
    }

    #[test]
    fn plots_without_a_graph_still_fill_the_sheet() {
        let (mut p, ids) = project();
        let spec = ProbPlotSpec { graph: false, ..Default::default() };
        let r = run(&mut p, &ids[1..2], Tool::ProbPlot(spec));
        assert!(r.new_graph.is_none());
        assert!(r.new_sheet.is_some());
        // A family the data cannot take.
        let bad = ProbPlotSpec { family: Family::Lognormal, ..Default::default() };
        assert!(p.analyze(&Request { columns: vec![ids[2]], rows: RowRange::All, tool: Tool::ProbPlot(bad) }).is_err());
    }
}
