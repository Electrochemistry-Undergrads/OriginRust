//! Statistics > Monte Carlo Simulation (T393, Origin 2027's app; Sheaf's own layout, not measured): the
//! inputs are drawn (`sheaf_calc::montecarlo`), the formula is evaluated for each draw with the inputs
//! as variables, and the draws, the output summary, capability and sensitivity are written
//! (`docs/spec/statistics.md` section 51).

use super::dist::Output;
use super::*;
use calc::montecarlo::{InputDist, draw, sensitivity, summarize};

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
        }
    }
}

/// Most draws of one run.
pub const MONTE_CARLO_MAX: usize = 1_000_000;

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
    let draws = draw(&dists, spec.samples, spec.seed).map_err(bad)?;
    let mut host = sheaf_expr::SimpleHost::new();
    let mut output = Vec::with_capacity(spec.samples);
    for r in 0..spec.samples {
        for ((name, _), column) in spec.inputs.iter().zip(&draws) {
            host.set_var(name, column[r]);
        }
        output.push(expr.eval(&host).map_err(|e| bad(format!("the formula: {e}")))?);
    }
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
    on_sheet.push(SheetColumn {
        designation: Designation::Y,
        long_name: "Output".into(),
        units: String::new(),
        comments: spec.formula.trim().to_string(),
        data: ColumnData::numeric(output),
    });
    Ok(Output { table: Some(table), on_sheet, notes, graph: None, more_graphs: Vec::new() })
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
        assert_eq!(names, ["len", "wid", "Output"]);
        assert_eq!(ws.columns[2].numeric().unwrap().len(), 5000);
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
}
