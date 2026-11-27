//! Statistics > Monte Carlo Simulation (T393, Origin 2027's app; Sheaf's own layout, not measured): the
//! inputs are drawn (`sheaf_calc::montecarlo`), the formula is evaluated for each draw with the inputs
//! as variables, and the draws, the output summary, capability and sensitivity are written
//! (`docs/spec/statistics.md` section 51). With controls (T490) the inputs named move their centres within ranges, the
//! draws kept, to the setting that best meets the goal; the output there is reported too.

use super::dist::{GraphPlot, Output, XyGraph};
use super::*;
use crate::graph::{PlotKind, ScaleKind};
use calc::correlation::{CorrKind, correlation};
use calc::montecarlo::{InputDist, McControl, McGoal, draw_correlated, goal_score, search_box, sensitivity, summarize};

/// What [`Tool::MonteCarlo`] runs (it reads no data; the results go next to the chosen sheet).
#[derive(Debug, Clone, PartialEq)]
pub struct MonteCarloSpec {
    /// The inputs, each with its distribution, in the order of the sheet's columns.
    pub inputs: Vec<(String, InputDist)>,
    /// The model: an expression of the input names, for example `a * b + c`.
    pub formula: String,
    pub samples: usize,
    pub seed: u64,
    /// Specification limits of the output (either or both).
    pub lsl: Option<f64>,
    pub usl: Option<f64>,
    /// Inputs whose centres the optimisation moves (T490; none: no optimisation).
    pub optimize: Vec<McControl>,
    /// What the optimisation looks for.
    pub goal: McGoal,
    /// Correlations of input pairs by name (T496; a Gaussian copula).
    pub correlations: Vec<(String, String, f64)>,
    /// A histogram graph of the output (T496).
    pub graph: bool,
}

impl Default for MonteCarloSpec {
    fn default() -> Self {
        MonteCarloSpec {
            inputs: vec![
                ("a".into(), InputDist::Normal { mean: 10.0, sd: 1.0 }),
                ("b".into(), InputDist::Uniform { low: 1.0, high: 2.0 }),
            ],
            formula: "a * b".into(),
            samples: 10_000,
            seed: 1,
            lsl: None,
            usl: None,
            optimize: Vec::new(),
            goal: McGoal::Capability,
            correlations: Vec::new(),
            graph: true,
        }
    }
}

/// The bins of the output histogram (T496): about the square root of the count, 5 to 50, over the values' range.
fn histogram(values: &[f64], bins: Option<(f64, f64, usize)>) -> (f64, f64, usize, Vec<f64>) {
    let finite: Vec<f64> = values.iter().copied().filter(|v| v.is_finite()).collect();
    let (lo, hi, k) = bins.unwrap_or_else(|| {
        let (lo, hi) = finite.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), v| (a.min(*v), b.max(*v)));
        let k = ((finite.len() as f64).sqrt().ceil() as usize).clamp(5, 50);
        (lo, hi, k)
    });
    let w = if hi > lo { (hi - lo) / k as f64 } else { 1.0 };
    let mut counts = vec![0.0; k];
    for v in &finite {
        if *v >= lo && *v <= hi {
            counts[(((v - lo) / w) as usize).min(k - 1)] += 1.0;
        }
    }
    (lo, w, k, counts)
}

/// Most draws of one run.
pub const MONTE_CARLO_MAX: usize = 1_000_000;

/// Most draws the optimisation's search uses (T490); the setting found is then run with all of them.
pub const MONTE_CARLO_SEARCH_DRAWS: usize = 20_000;

/// Most inputs one optimisation moves (T490).
pub const MONTE_CARLO_MAX_CONTROLS: usize = 4;

/// The formula over the first `rows` rows of the input columns.
fn evaluate(
    expr: &Expr,
    inputs: &[(String, InputDist)],
    columns: &[Vec<f64>],
    rows: usize,
) -> Result<Vec<f64>, String> {
    let mut host = sheaf_expr::SimpleHost::new();
    let mut output = Vec::with_capacity(rows);
    for r in 0..rows {
        for ((name, _), column) in inputs.iter().zip(columns) {
            host.set_var(name, column[r]);
        }
        output.push(expr.eval(&host).map_err(|e| e.to_string())?);
    }
    Ok(output)
}

/// The setting found by the optimisation (T490).
struct Optimum {
    /// The inputs moved (indices into the spec's inputs) and their centres.
    which: Vec<usize>,
    centres: Vec<f64>,
    /// Every input's draws at the setting, and the output.
    draws: Vec<Vec<f64>>,
    output: Vec<f64>,
}

/// `x` to about 1e-4 of `range`, so the settings read as short numbers.
fn tidy(x: f64, range: f64) -> f64 {
    let dec = (4 - range.log10().floor() as i32).clamp(0, 15) as usize;
    format!("{x:.dec$}").parse().unwrap_or(x)
}

fn optimize(spec: &MonteCarloSpec, expr: &Expr, draws: &[Vec<f64>]) -> Result<Optimum, AnalysisError> {
    let bad = |m: String| AnalysisError::BadOption(m);
    if spec.optimize.len() > MONTE_CARLO_MAX_CONTROLS {
        return Err(bad(format!("the optimisation moves at most {MONTE_CARLO_MAX_CONTROLS} inputs")));
    }
    let mut which = Vec::new();
    for c in &spec.optimize {
        let j = spec
            .inputs
            .iter()
            .position(|(n, _)| *n == c.name)
            .ok_or_else(|| bad(format!("the control {} is not one of the inputs", c.name)))?;
        if which.contains(&j) {
            return Err(bad(format!("the input {} is controlled twice", c.name)));
        }
        if !(c.low.is_finite() && c.high.is_finite() && c.low < c.high) {
            return Err(bad(format!("the range of {} must run from a lower to a higher number", c.name)));
        }
        if matches!(spec.inputs[j].1, InputDist::Lognormal { .. }) && c.low <= 0.0 {
            return Err(bad(format!("{} is lognormal: its centre (the median) must stay above 0", c.name)));
        }
        which.push(j);
    }
    match spec.goal {
        McGoal::Capability if spec.lsl.is_none() && spec.usl.is_none() => {
            return Err(bad("give a specification limit to optimise Cpk, or aim the output at a target".into()));
        }
        McGoal::Target(t) if !t.is_finite() => return Err(bad("the target must be a number".into())),
        _ => {}
    }
    let m = spec.samples.min(MONTE_CARLO_SEARCH_DRAWS);
    let base: Vec<Vec<f64>> = draws.iter().map(|c| c[..m].to_vec()).collect();
    let mut columns = base.clone();
    let lo: Vec<f64> = spec.optimize.iter().map(|c| c.low).collect();
    let hi: Vec<f64> = spec.optimize.iter().map(|c| c.high).collect();
    let mut failure = None;
    let (best, score) = search_box(
        &mut |x| {
            for (k, &j) in which.iter().enumerate() {
                columns[j] = spec.inputs[j].1.move_draws(&base[j], x[k]);
            }
            match evaluate(expr, &spec.inputs, &columns, m) {
                Ok(out) => goal_score(spec.goal, &out, spec.lsl, spec.usl),
                Err(e) => {
                    failure.get_or_insert(e);
                    f64::INFINITY
                }
            }
        },
        &lo,
        &hi,
    );
    if !score.is_finite() {
        let why = failure.map(|e| format!(" ({e})")).unwrap_or_default();
        return Err(bad(format!("no setting of the controls gives an output the goal can be measured on{why}")));
    }
    let centres: Vec<f64> =
        best.iter().zip(&spec.optimize).map(|(x, c)| tidy(*x, c.high - c.low).clamp(c.low, c.high)).collect();
    let mut moved = draws.to_vec();
    for (k, &j) in which.iter().enumerate() {
        moved[j] = spec.inputs[j].1.move_draws(&draws[j], centres[k]);
    }
    let output = evaluate(expr, &spec.inputs, &moved, spec.samples).map_err(|e| bad(format!("the formula: {e}")))?;
    Ok(Optimum { which, centres, draws: moved, output })
}

pub(super) fn run(spec: &MonteCarloSpec) -> Result<Output, AnalysisError> {
    let bad = |m: String| AnalysisError::BadOption(m);
    if spec.inputs.is_empty() {
        return Err(bad("give at least one input, for example a = normal(10, 1)".into()));
    }
    if !(1..=MONTE_CARLO_MAX).contains(&spec.samples) {
        return Err(bad(format!("the number of draws must be 1 to {MONTE_CARLO_MAX}")));
    }
    for (i, (n, _)) in spec.inputs.iter().enumerate() {
        if spec.inputs[..i].iter().any(|(m, _)| m == n) {
            return Err(bad(format!("the input name {n} is used twice")));
        }
    }
    if let (Some(l), Some(u)) = (spec.lsl, spec.usl)
        && l >= u
    {
        return Err(bad("the lower specification limit must be below the upper one".into()));
    }
    let expr = Expr::parse(&spec.formula).map_err(|e| bad(format!("the formula: {e}")))?;
    let dists: Vec<InputDist> = spec.inputs.iter().map(|(_, d)| *d).collect();
    // T496: correlations of input pairs, by name.
    let names: Vec<&str> = spec.inputs.iter().map(|(n, _)| n.as_str()).collect();
    let mut pairs = Vec::new();
    for (a, b, r) in &spec.correlations {
        let index =
            |n: &str| names.iter().position(|m| *m == n).ok_or_else(|| bad(format!("{n} is not one of the inputs")));
        pairs.push((index(a)?, index(b)?, *r));
    }
    let draws = draw_correlated(&dists, spec.samples, spec.seed, &pairs).map_err(bad)?;
    let output = evaluate(&expr, &spec.inputs, &draws, spec.samples).map_err(|e| bad(format!("the formula: {e}")))?;
    let optimum = if spec.optimize.is_empty() { None } else { Some(optimize(spec, &expr, &draws)?) };
    let out = summarize(&output, spec.lsl, spec.usl);
    let sens = sensitivity(&draws, &output);
    let mut table = ResultTable::new(&[
        "Mean",
        "SD",
        "5th percentile",
        "Median",
        "95th percentile",
        "Min",
        "Max",
        "Rank correlation with output",
    ]);
    let row = |s: &calc::montecarlo::McSummary, r: f64| vec![s.mean, s.sd, s.p05, s.median, s.p95, s.min, s.max, r];
    table.push_row(format!("Output = {}", spec.formula.trim()), row(&out, 1.0), vec![String::new(); 8]);
    for (j, (name, d)) in spec.inputs.iter().enumerate() {
        table.push_row(
            format!("{name} = {}", d.text()),
            row(&summarize(&draws[j], None, None), sens[j]),
            vec![String::new(); 8],
        );
    }
    // T490: the output and the moved inputs at the setting found.
    let at = optimum.as_ref().map(|o| (o, summarize(&o.output, spec.lsl, spec.usl), sensitivity(&o.draws, &o.output)));
    if let Some((o, s, sens)) = &at {
        table.push_row("Output at the optimum".to_string(), row(s, 1.0), vec![String::new(); 8]);
        for (k, &j) in o.which.iter().enumerate() {
            let (name, d) = &spec.inputs[j];
            table.push_row(
                format!("{name} at the optimum = {}", d.centred_at(o.centres[k]).text()),
                row(&summarize(&o.draws[j], None, None), sens[j]),
                vec![String::new(); 8],
            );
        }
    }
    let mut notes =
        vec![format!("{} draws (seed {}); {} of them gave a number for the output.", spec.samples, spec.seed, out.n)];
    if spec.lsl.is_some() || spec.usl.is_some() {
        let limits = match (spec.lsl, spec.usl) {
            (Some(l), Some(u)) => format!("{l} to {u}"),
            (Some(l), None) => format!("at least {l}"),
            (None, Some(u)) => format!("at most {u}"),
            (None, None) => unreachable!(),
        };
        let mut t = format!("Specification {limits}: {:.4} % of the draws outside", 100.0 * out.out_of_spec);
        if out.cp.is_finite() {
            t.push_str(&format!(", Cp = {:.4}", out.cp));
        }
        if out.cpk.is_finite() {
            t.push_str(&format!(", Cpk = {:.4}", out.cpk));
        }
        t.push('.');
        notes.push(t);
    }
    if let Some((j, r)) =
        sens.iter().enumerate().filter(|(_, r)| r.is_finite()).max_by(|a, b| a.1.abs().total_cmp(&b.1.abs()))
    {
        notes.push(format!("The output follows {} most (rank correlation {r:.4}).", spec.inputs[j].0));
    }
    if !pairs.is_empty() {
        let shown: Vec<String> = pairs
            .iter()
            .map(|&(i, j, r)| {
                let rho = correlation(&draws[i], &draws[j], CorrKind::Spearman).r;
                format!("{} and {} {r} (rank correlation of the draws {rho:.4})", names[i], names[j])
            })
            .collect();
        notes.push(format!("Correlated inputs (Gaussian copula of the normal scores): {}.", shown.join(", ")));
    }
    if let Some((o, s, _)) = &at {
        let ranges: Vec<String> = spec.optimize.iter().map(|c| format!("{} {} to {}", c.name, c.low, c.high)).collect();
        let settings: Vec<String> =
            o.which.iter().zip(&o.centres).map(|(&j, c)| format!("{} = {c}", spec.inputs[j].0)).collect();
        let goal = match spec.goal {
            McGoal::Capability => "the highest Cpk".to_string(),
            McGoal::Target(t) => format!("a mean of {t} with the least spread"),
        };
        let result = match spec.goal {
            McGoal::Capability => {
                let mut t = format!("Cpk = {:.4} ({:.4} before)", s.cpk, out.cpk);
                if s.out_of_spec.is_finite() {
                    t.push_str(&format!(", {:.4} % of the draws outside", 100.0 * s.out_of_spec));
                }
                t
            }
            McGoal::Target(t) => format!(
                "mean {:.4} and SD {:.4} (mean squared deviation {:.4}; {:.4} before)",
                s.mean,
                s.sd,
                goal_score(spec.goal, &o.output, None, None),
                (out.mean - t).powi(2) + out.sd.powi(2)
            ),
        };
        notes.push(format!(
            "Optimum for {goal} (centres within {}; the search used {} draws): {} {} {result}.",
            ranges.join(", "),
            spec.samples.min(MONTE_CARLO_SEARCH_DRAWS),
            settings.join(", "),
            if settings.len() == 1 { "gives" } else { "give" }
        ));
        for (c, x) in spec.optimize.iter().zip(&o.centres) {
            let edge = (c.high - c.low) * 1e-4;
            if *x <= c.low + edge || *x >= c.high - edge {
                let which = if *x <= c.low + edge { "lower" } else { "upper" };
                notes.push(format!("{} stops at its {which} bound {x}; a wider range may do better.", c.name));
            }
        }
    }
    let mut on_sheet: Vec<SheetColumn> = spec
        .inputs
        .iter()
        .zip(draws)
        .map(|((name, d), v)| SheetColumn {
            designation: Designation::Y,
            long_name: name.clone(),
            units: String::new(),
            comments: d.text(),
            data: ColumnData::numeric(v),
        })
        .collect();
    // T496: the histogram of the output (and of the output at the optimum, on the same bins).
    let hist = spec.graph.then(|| {
        let (lo, w, k, counts) = histogram(&output, None);
        let at_optimum = optimum.as_ref().map(|o| histogram(&o.output, Some((lo, lo + w * k as f64, k))).3);
        (lo, w, k, counts, at_optimum)
    });
    on_sheet.push(SheetColumn {
        designation: Designation::Y,
        long_name: "Output".into(),
        units: String::new(),
        comments: spec.formula.trim().to_string(),
        data: ColumnData::numeric(output),
    });
    if let Some(o) = optimum {
        for (k, &j) in o.which.iter().enumerate() {
            let (name, d) = &spec.inputs[j];
            on_sheet.push(SheetColumn {
                designation: Designation::Y,
                long_name: format!("{name} at optimum"),
                units: String::new(),
                comments: d.centred_at(o.centres[k]).text(),
                data: ColumnData::numeric(o.draws[j].clone()),
            });
        }
        on_sheet.push(SheetColumn {
            designation: Designation::Y,
            long_name: "Output at optimum".into(),
            units: String::new(),
            comments: spec.formula.trim().to_string(),
            data: ColumnData::numeric(o.output),
        });
    }
    let graph = hist.map(|(lo, w, k, counts, at_optimum)| {
        let column = |name: &str, designation: Designation, values: Vec<f64>| SheetColumn {
            designation,
            long_name: name.to_string(),
            units: String::new(),
            comments: "Histogram of the output".into(),
            data: ColumnData::numeric(values),
        };
        let top = counts.iter().chain(at_optimum.iter().flatten()).copied().fold(0.0, f64::max);
        let x = on_sheet.len();
        on_sheet.push(column("Bin center", Designation::X, (0..k).map(|i| lo + (i as f64 + 0.5) * w).collect()));
        on_sheet.push(column("Count", Designation::Y, counts));
        let mut plots = vec![GraphPlot { x, y: x + 1, kind: PlotKind::Column, label: "Output".into(), line_of: None }];
        if let Some(c) = at_optimum {
            let y = on_sheet.len();
            on_sheet.push(column("Count at optimum", Designation::Y, c));
            plots.push(GraphPlot { x, y, kind: PlotKind::Column, label: "Output at optimum".into(), line_of: None });
        }
        for (name, limit) in [("LSL", spec.lsl), ("USL", spec.usl)] {
            let Some(l) = limit else { continue };
            let lx = on_sheet.len();
            on_sheet.push(column(&format!("{name} X"), Designation::X, vec![l, l]));
            on_sheet.push(column(name, Designation::Y, vec![0.0, top * 1.1]));
            plots.push(GraphPlot {
                x: lx,
                y: lx + 1,
                kind: PlotKind::Line,
                label: format!("{name} = {l}"),
                line_of: Some(0),
            });
        }
        XyGraph {
            plots,
            x_scale: ScaleKind::Linear,
            y_scale: ScaleKind::Linear,
            x_title: format!("Output = {}", spec.formula.trim()),
            y_title: "Count".into(),
            look: Default::default(),
        }
    });
    Ok(Output { table: Some(table), on_sheet, notes, graph, more_graphs: Vec::new() })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_simulation_writes_draws_summary_capability_and_sensitivity() {
        let mut p = Project::new();
        let book = p.add_workbook("mc");
        let host = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![0.0])).unwrap();
        let spec = MonteCarloSpec {
            inputs: vec![
                ("len".into(), InputDist::Normal { mean: 10.0, sd: 0.1 }),
                ("wid".into(), InputDist::Uniform { low: 1.9, high: 2.1 }),
            ],
            formula: "len * wid".into(),
            samples: 5000,
            seed: 4,
            lsl: Some(19.0),
            usl: Some(21.0),
            optimize: Vec::new(),
            goal: McGoal::Capability,
            correlations: Vec::new(),
            graph: true,
        };
        let r = p
            .analyze(&Request { columns: vec![host], rows: RowRange::All, tool: Tool::MonteCarlo(spec.clone()) })
            .unwrap();
        let t = r.table.as_ref().unwrap();
        assert_eq!(t.rows.len(), 3);
        assert!((t.rows[0].1[0] - 20.0).abs() < 0.02, "{:?}", t.rows[0]);
        assert!(t.rows[2].1[7] > t.rows[1].1[7], "the width (5 % spread) matters more than the length (1 %)");
        assert!(r.notes.iter().any(|n| n.contains("Cp = ") && n.contains("% of the draws outside")), "{:?}", r.notes);
        let ws = &p.workbook(book).unwrap().sheets[r.new_sheet.unwrap()];
        let names: Vec<&str> = ws.columns.iter().map(|c| c.long_name.as_str()).collect();
        assert_eq!(names, ["len", "wid", "Output", "Bin center", "Count", "LSL X", "LSL", "USL X", "USL"]);
        // T496: the histogram graph of the output with the limits; its counts add up to the draws.
        let g = r.new_graph.expect("a histogram");
        let cols = super::super::dist::plot_columns(&p, g);
        assert_eq!(cols[0], ("Bin center".to_string(), "Count".to_string()));
        assert_eq!(cols.len(), 3, "{cols:?}");
        assert_eq!(ws.columns[4].numeric().unwrap().iter().sum::<f64>(), 5000.0);
        assert_eq!(ws.columns[2].numeric().unwrap().len(), 5000);
        // Correlated inputs: the length and width rise together; a pair of unknown names is refused.
        let tied =
            MonteCarloSpec { correlations: vec![("len".into(), "wid".into(), 0.9)], graph: false, ..spec.clone() };
        let c = p.analyze(&Request { columns: vec![host], rows: RowRange::All, tool: Tool::MonteCarlo(tied) }).unwrap();
        let note = c.notes.iter().find(|n| n.starts_with("Correlated inputs")).expect("a correlation note");
        assert!(note.contains("len and wid 0.9 (rank correlation of the draws 0.8"), "{note}");
        assert!(c.new_graph.is_none(), "no graph asked for");
        assert!(c.table.unwrap().rows[0].1[1] > t.rows[0].1[1], "a correlated product spreads more");
        let wrong = MonteCarloSpec { correlations: vec![("len".into(), "depth".into(), 0.5)], ..spec.clone() };
        let e = p
            .analyze(&Request { columns: vec![host], rows: RowRange::All, tool: Tool::MonteCarlo(wrong) })
            .unwrap_err();
        assert!(e.to_string().contains("depth is not one of the inputs"), "{e}");
        // The same seed gives the same draws; a formula of an unknown name is refused.
        let again = p
            .analyze(&Request { columns: vec![host], rows: RowRange::All, tool: Tool::MonteCarlo(spec.clone()) })
            .unwrap();
        assert_eq!(again.table.unwrap().rows[0].1[0], t.rows[0].1[0]);
        let wrong = MonteCarloSpec { formula: "len * depth".into(), ..spec };
        let e = p
            .analyze(&Request { columns: vec![host], rows: RowRange::All, tool: Tool::MonteCarlo(wrong) })
            .unwrap_err();
        assert!(e.to_string().contains("depth"), "{e}");
    }

    #[test]
    fn the_optimisation_moves_controls_to_the_best_setting() {
        let mut p = Project::new();
        let book = p.add_workbook("mc");
        let host = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![0.0])).unwrap();
        // A gap = housing - shaft, wanted between 0.1 and 0.5: the shaft starts too wide (gap 0.05).
        let spec = MonteCarloSpec {
            inputs: vec![
                ("housing".into(), InputDist::Normal { mean: 20.0, sd: 0.04 }),
                ("shaft".into(), InputDist::Normal { mean: 19.95, sd: 0.03 }),
            ],
            formula: "housing - shaft".into(),
            samples: 4000,
            seed: 2,
            lsl: Some(0.1),
            usl: Some(0.5),
            optimize: vec![McControl { name: "shaft".into(), low: 19.4, high: 19.95 }],
            goal: McGoal::Capability,
            correlations: Vec::new(),
            graph: true,
        };
        let run = |p: &mut Project, s: &MonteCarloSpec| {
            p.analyze(&Request { columns: vec![host], rows: RowRange::All, tool: Tool::MonteCarlo(s.clone()) })
        };
        let r = run(&mut p, &spec).unwrap();
        let t = r.table.as_ref().unwrap();
        assert_eq!(t.rows.len(), 5, "{:?}", t.rows.iter().map(|r| &r.0).collect::<Vec<_>>());
        assert_eq!(t.rows[3].0, "Output at the optimum");
        // The gap centres in the limits: shaft near 20 - 0.3.
        assert!((t.rows[3].1[0] - 0.3).abs() < 0.01, "{:?}", t.rows[3]);
        assert!(
            t.rows[4].0.starts_with("shaft at the optimum = normal(19.") && t.rows[4].0.ends_with(", 0.03)"),
            "{}",
            t.rows[4].0
        );
        let note = r.notes.iter().find(|n| n.starts_with("Optimum for the highest Cpk")).expect("an optimum note");
        assert!(note.contains("shaft = 19.") && note.contains("before)"), "{note}");
        let ws = &p.workbook(book).unwrap().sheets[r.new_sheet.unwrap()];
        let names: Vec<&str> = ws.columns.iter().map(|c| c.long_name.as_str()).collect();
        assert_eq!(
            names,
            [
                "housing",
                "shaft",
                "Output",
                "shaft at optimum",
                "Output at optimum",
                "Bin center",
                "Count",
                "Count at optimum",
                "LSL X",
                "LSL",
                "USL X",
                "USL"
            ]
        );
        // A range that stops short of the best place says so.
        let short = MonteCarloSpec {
            optimize: vec![McControl { name: "shaft".into(), low: 19.8, high: 19.9 }],
            ..spec.clone()
        };
        let r = run(&mut p, &short).unwrap();
        assert!(r.notes.iter().any(|n| n.contains("shaft stops at its lower bound 19.8")), "{:?}", r.notes);
        // Aiming at a target: the gap's mean comes to 0.2.
        let aim = MonteCarloSpec { goal: McGoal::Target(0.2), lsl: None, usl: None, ..spec.clone() };
        let r = run(&mut p, &aim).unwrap();
        let t = r.table.as_ref().unwrap();
        assert!((t.rows[3].1[0] - 0.2).abs() < 0.005, "{:?}", t.rows[3]);
        assert!(
            r.notes.iter().any(|n| n.starts_with("Optimum for a mean of 0.2 with the least spread")),
            "{:?}",
            r.notes
        );
        // Refusals: an unknown control, Cpk without limits, too many controls.
        let e = run(
            &mut p,
            &MonteCarloSpec { optimize: vec![McControl { name: "bore".into(), low: 0.0, high: 1.0 }], ..spec.clone() },
        )
        .unwrap_err();
        assert!(e.to_string().contains("bore is not one of the inputs"), "{e}");
        let e = run(&mut p, &MonteCarloSpec { lsl: None, usl: None, ..spec.clone() }).unwrap_err();
        assert!(e.to_string().contains("specification limit"), "{e}");
        let many = vec![McControl { name: "shaft".into(), low: 19.0, high: 20.0 }; 5];
        let e = run(&mut p, &MonteCarloSpec { optimize: many, ..spec }).unwrap_err();
        assert!(e.to_string().contains("at most 4"), "{e}");
    }
}
