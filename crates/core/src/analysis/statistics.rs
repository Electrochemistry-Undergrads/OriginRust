//! The statistics tools of `Project::analyze`: t-tests, one-way ANOVA, normality tests and the histogram
//! (frequency counts). The numerics are in `sheaf_calc`; this module arranges the chosen columns, writes the
//! result tables and the plain-English lines about them. What each tool reports follows Origin 2016, see
//! `docs/spec/statistics.md`.

use super::*;
use sheaf_calc::{TTest, anova_one_way, freq_counts};

/// What a statistics tool produces; `analyze` turns it into a sheet (and a graph).
pub(super) struct Output {
    pub table: Option<ResultTable>,
    pub on_sheet: Vec<SheetColumn>,
    pub notes: Vec<String>,
    /// Pairs of indices into `on_sheet` (X column, Y column) for the plots of a histogram graph.
    pub plots: Vec<(usize, usize)>,
}

pub(super) fn run(tool: &Tool, inputs: &[Input<'_>]) -> Result<Output, AnalysisError> {
    match tool {
        Tool::TTest(spec) => ttest(spec, inputs),
        Tool::Anova(spec) => anova(spec, inputs),
        Tool::Histogram(spec) => histogram(spec, inputs),
        Tool::Normality(spec) => normality(spec, inputs),
        _ => unreachable!("only the statistics tools come here"),
    }
}

fn check_alpha(alpha: f64) -> Result<(), AnalysisError> {
    if alpha > 0.0 && alpha < 1.0 {
        Ok(())
    } else {
        Err(AnalysisError::BadOption(format!("the significance level must lie between 0 and 1, not {alpha}")))
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

fn number_column(name: &str, units: &str, values: Vec<f64>) -> SheetColumn {
    SheetColumn {
        designation: Designation::Y,
        long_name: name.to_string(),
        units: units.to_string(),
        comments: String::new(),
        data: ColumnData::numeric(values),
    }
}

/// The unit all inputs have, or empty where they differ.
fn shared_units(inputs: &[Input<'_>]) -> String {
    match inputs.first() {
        Some(f) if inputs.iter().all(|i| i.units == f.units) => f.units.clone(),
        _ => String::new(),
    }
}

/// A number for the plain-English lines: four significant digits, no exponent unless needed.
fn num(v: f64) -> String {
    if v.is_nan() {
        return "--".into();
    }
    if v == 0.0 || !v.is_finite() {
        return format!("{v}");
    }
    let magnitude = v.abs().log10().floor() as i32;
    if !(-4..=6).contains(&magnitude) {
        return format!("{v:.3e}");
    }
    let text = format!("{:.*}", (3 - magnitude).max(0) as usize, v);
    if text.contains('.') { text.trim_end_matches('0').trim_end_matches('.').to_string() } else { text }
}

fn tail_words(tail: Tail) -> &'static str {
    match tail {
        Tail::Two => "differs from",
        Tail::Upper => "is larger than",
        Tail::Lower => "is smaller than",
    }
}

fn tail_name(tail: Tail) -> &'static str {
    match tail {
        Tail::Two => "two-tailed",
        Tail::Upper => "upper-tailed",
        Tail::Lower => "lower-tailed",
    }
}

fn verdict(t: &TTest) -> String {
    if t.rejected() {
        format!("the null hypothesis is rejected at the {} level", num(t.alpha))
    } else {
        format!("the null hypothesis is not rejected at the {} level", num(t.alpha))
    }
}

// ----- t-tests ---------------------------------------------------------------------------------------

fn ttest(spec: &TTestSpec, inputs: &[Input<'_>]) -> Result<Output, AnalysisError> {
    check_alpha(spec.alpha)?;
    let two = |what: &str| {
        if inputs.len() == 2 {
            Ok(())
        } else {
            Err(AnalysisError::BadOption(format!("the {what} needs exactly two columns, {} were chosen", inputs.len())))
        }
    };
    let few = |name: &str| AnalysisError::BadOption(format!("{name} has fewer than two numbers in the chosen rows"));
    let mut notes = Vec::new();
    let table = match spec.kind {
        TTestKind::OneSample { mean } => {
            let mut t = ResultTable::new(&[
                "N",
                "Mean",
                "SD",
                "SEM",
                "Hypothesized mean",
                "t",
                "DF",
                "Prob",
                "Confidence level",
                "LCL",
                "UCL",
                "Reject",
            ]);
            for inp in inputs {
                let r = calc::ttest_one(inp.ys(), mean, spec.tail, spec.alpha).ok_or_else(|| few(&inp.name))?;
                let s = r.samples[0];
                let u = &inp.units;
                t.push_row(
                    inp.label.clone(),
                    vec![
                        s.n as f64,
                        s.mean,
                        s.sd,
                        s.sem,
                        mean,
                        r.t,
                        r.df,
                        r.p,
                        r.level,
                        r.lcl,
                        r.ucl,
                        f64::from(u8::from(r.rejected())),
                    ],
                    ["", u, u, u, u, "", "", "", "", u, u, ""].map(str::to_string).to_vec(),
                );
                notes.push(format!(
                    "{}: one-sample t-test, {}: the mean {} {} ({} = {}, t({}) = {}, p = {}); {}",
                    inp.name,
                    tail_name(spec.tail),
                    tail_words(spec.tail),
                    num(mean),
                    inp.name,
                    num(s.mean),
                    num(r.df),
                    num(r.t),
                    num(r.p),
                    verdict(&r)
                ));
            }
            t
        }
        TTestKind::TwoSample { .. } | TTestKind::Paired { .. } => {
            let (name, difference) = match spec.kind {
                TTestKind::TwoSample { equal_variance: true, difference } => ("two-sample t-test", difference),
                TTestKind::TwoSample { difference, .. } => {
                    ("two-sample t-test with unequal variances (Welch)", difference)
                }
                TTestKind::Paired { difference } => ("paired t-test", difference),
                TTestKind::OneSample { .. } => unreachable!(),
            };
            two(name)?;
            let (a, b) = (&inputs[0], &inputs[1]);
            let r = match spec.kind {
                TTestKind::TwoSample { equal_variance, difference } => {
                    calc::ttest_two(a.ys(), b.ys(), difference, spec.tail, spec.alpha, equal_variance)
                }
                TTestKind::Paired { difference } => {
                    // Rows pair up by position within the chosen rows.
                    calc::ttest_paired(a.ys(), b.ys(), difference, spec.tail, spec.alpha)
                }
                TTestKind::OneSample { .. } => unreachable!(),
            }
            .ok_or_else(|| {
                AnalysisError::BadOption(format!(
                    "the {name} needs at least two numbers{} in each column",
                    if matches!(spec.kind, TTestKind::Paired { .. }) { " in rows where both have one" } else { "" }
                ))
            })?;
            let mut t = ResultTable::new(&[
                "N1",
                "Mean1",
                "SD1",
                "SEM1",
                "N2",
                "Mean2",
                "SD2",
                "SEM2",
                "Difference",
                "SE of difference",
                "Hypothesized difference",
                "t",
                "DF",
                "Prob",
                "Confidence level",
                "LCL",
                "UCL",
                "Reject",
            ]);
            let (s1, s2) = (r.samples[0], r.samples[1]);
            let (u1, u2) = (&a.units, &b.units);
            let ud = if u1 == u2 { u1 } else { &String::new() };
            t.push_row(
                format!("{} vs {}", a.name, b.name),
                vec![
                    s1.n as f64,
                    s1.mean,
                    s1.sd,
                    s1.sem,
                    s2.n as f64,
                    s2.mean,
                    s2.sd,
                    s2.sem,
                    r.estimate,
                    r.se,
                    difference,
                    r.t,
                    r.df,
                    r.p,
                    r.level,
                    r.lcl,
                    r.ucl,
                    f64::from(u8::from(r.rejected())),
                ],
                ["", u1, u1, u1, "", u2, u2, u2, ud, ud, ud, "", "", "", "", ud, ud, ""].map(str::to_string).to_vec(),
            );
            let what = if matches!(spec.kind, TTestKind::Paired { .. }) {
                "the mean of the differences"
            } else {
                "the difference of the means"
            };
            notes.push(format!(
                "{} vs {}: {name}, {}: {what} {} {} ({} = {}, t({}) = {}, p = {}); {}",
                a.name,
                b.name,
                tail_name(spec.tail),
                tail_words(spec.tail),
                num(difference),
                if matches!(spec.kind, TTestKind::Paired { .. }) { "mean difference" } else { "difference" },
                num(r.estimate),
                num(r.df),
                num(r.t),
                num(r.p),
                verdict(&r)
            ));
            t
        }
    };
    notes.push(format!(
        "The confidence interval has level {} (a one-tailed test gives the interval of level 1 - 2 alpha, as Origin does).",
        num(1.0 - if spec.tail == Tail::Two { spec.alpha } else { 2.0 * spec.alpha })
    ));
    Ok(Output { table: Some(table), on_sheet: Vec::new(), notes, plots: Vec::new() })
}

// ----- one-way ANOVA ---------------------------------------------------------------------------------

fn anova(spec: &AnovaSpec, inputs: &[Input<'_>]) -> Result<Output, AnalysisError> {
    check_alpha(spec.alpha)?;
    let groups: Vec<&[f64]> = inputs.iter().map(Input::ys).collect();
    let r = anova_one_way(&groups).map_err(|e| AnalysisError::BadOption(e.to_string()))?;
    let units = shared_units(inputs);
    let mut on_sheet = vec![
        label_column("Group", inputs.iter().map(|i| i.label.clone()).collect()),
        number_column("N", "", r.groups.iter().map(|g| g.n as f64).collect()),
        number_column("Mean", &units, r.groups.iter().map(|g| g.mean).collect()),
        number_column("SD", &units, r.groups.iter().map(|g| g.sd).collect()),
        number_column("SEM", &units, r.groups.iter().map(|g| g.sem).collect()),
    ];

    let mut table = ResultTable::new(&["DF", "Sum of Squares", "Mean Square", "F", "Prob>F"]);
    let rows =
        [("Model", r.model, r.f, r.p), ("Error", r.error, f64::NAN, f64::NAN), ("Total", r.total, f64::NAN, f64::NAN)];
    for (name, row, f, p) in rows {
        table.push_row(name.to_string(), vec![row.df, row.sum_sq, row.mean_sq, f, p], vec![String::new(); 5]);
    }
    on_sheet.push(label_column("Source", rows.iter().map(|r| r.0.to_string()).collect()));
    for (i, h) in table.headers.iter().enumerate() {
        on_sheet.push(number_column(h, "", table.rows.iter().map(|(_, v)| v[i]).collect()));
    }

    on_sheet
        .push(label_column("Statistic", ["R-squared", "Root MSE", "COV", "Grand mean"].map(str::to_string).to_vec()));
    on_sheet.push(number_column("Value", "", vec![r.r_squared, r.root_mse, r.cov, r.grand_mean]));

    let mut notes = vec![format!(
        "One-way ANOVA of {} groups: F({}, {}) = {}, p = {}; the group means {} at the {} level.",
        r.groups.len(),
        num(r.model.df),
        num(r.error.df),
        num(r.f),
        num(r.p),
        if r.p < spec.alpha { "differ significantly" } else { "do not differ significantly" },
        num(spec.alpha)
    )];
    notes.push(format!(
        "R-squared {}, root MSE {}, COV {}, grand mean {}.",
        num(r.r_squared),
        num(r.root_mse),
        num(r.cov),
        num(r.grand_mean)
    ));
    if let Some(method) = spec.comparison {
        let comparisons = r.compare(method, spec.alpha);
        let stat_name = match method {
            MeansComparison::Tukey => "q",
            MeansComparison::Scheffe => "F",
            _ => "|t|",
        };
        let c = |f: fn(&sheaf_calc::Comparison) -> f64| comparisons.iter().map(f).collect::<Vec<f64>>();
        on_sheet.push(label_column(
            &format!("{} comparison", method.label()),
            comparisons.iter().map(|c| format!("{} - {}", inputs[c.j].name, inputs[c.i].name)).collect(),
        ));
        on_sheet.push(number_column("Difference", &units, c(|c| c.diff)));
        on_sheet.push(number_column("SE", &units, c(|c| c.se)));
        on_sheet.push(number_column(&format!("Statistic ({stat_name})"), "", c(|c| c.stat)));
        on_sheet.push(number_column("Prob", "", c(|c| c.p)));
        on_sheet.push(number_column("LCL", &units, c(|c| c.lcl)));
        on_sheet.push(number_column("UCL", &units, c(|c| c.ucl)));
        on_sheet.push(number_column("Significant", "", c(|c| f64::from(u8::from(c.significant)))));
        notes.push(format!("{} means comparison at the {} level:", method.label(), num(spec.alpha)));
        for c in &comparisons {
            notes.push(format!(
                "  {} - {}: difference {}, p = {}, {}",
                inputs[c.j].name,
                inputs[c.i].name,
                num(c.diff),
                num(c.p),
                if c.significant { "significant" } else { "not significant" }
            ));
        }
    }
    Ok(Output { table: Some(table), on_sheet, notes, plots: Vec::new() })
}

// ----- histogram -------------------------------------------------------------------------------------

fn histogram(spec: &HistogramSpec, inputs: &[Input<'_>]) -> Result<Output, AnalysisError> {
    let mut table = ResultTable::new(&["N", "Bins", "First edge", "Bin size", "Largest count"]);
    let mut on_sheet: Vec<SheetColumn> = Vec::new();
    let mut plots = Vec::new();
    let mut notes = Vec::new();
    for inp in inputs {
        let f = freq_counts(inp.ys(), &spec.freq).ok_or_else(|| {
            AnalysisError::BadOption(format!(
                "no frequency counts can be made of {}: it has no numbers in the chosen rows, or the bin range or size is not usable",
                inp.name
            ))
        })?;
        let u = &inp.units;
        table.push_row(
            inp.label.clone(),
            vec![
                f.total as f64,
                f.rows.len() as f64,
                f.bin_start,
                f.bin_size,
                f.rows.iter().map(|r| r.count).max().unwrap_or(0) as f64,
            ],
            ["", "", u, u, ""].map(str::to_string).to_vec(),
        );
        let first = on_sheet.len();
        let comments = inp.label.clone();
        let with = |mut c: SheetColumn, d: Designation| {
            c.designation = d;
            c.comments = comments.clone();
            c
        };
        let name = &inp.name;
        on_sheet.push(with(number_column("Bin center", u, f.rows.iter().map(|r| r.center).collect()), Designation::X));
        on_sheet
            .push(with(number_column("Bin end", u, f.rows.iter().map(|r| r.end).collect()), Designation::Disregard));
        on_sheet.push(with(
            number_column(&format!("Count ({name})"), "", f.rows.iter().map(|r| r.count as f64).collect()),
            Designation::Y,
        ));
        on_sheet.push(with(
            number_column("Cumulative count", "", f.cumulative().iter().map(|c| *c as f64).collect()),
            Designation::Y,
        ));
        on_sheet.push(with(
            number_column("Frequency (%)", "", f.relative().iter().map(|r| r * 100.0).collect()),
            Designation::Y,
        ));
        plots.push((first, first + 2));
        notes.push(format!(
            "{}: {} values in {} bins of width {} from {}.",
            inp.name,
            f.total,
            f.rows.len(),
            num(f.bin_size),
            num(f.bin_start)
        ));
    }
    if !spec.graph {
        plots.clear();
    }
    Ok(Output { table: Some(table), on_sheet, notes, plots })
}

// ----- normality tests -------------------------------------------------------------------------------

fn normality(spec: &NormalitySpec, inputs: &[Input<'_>]) -> Result<Output, AnalysisError> {
    check_alpha(spec.alpha)?;
    let mut table = ResultTable::new(&[
        "N",
        "Shapiro-Wilk W",
        "Shapiro-Wilk Prob",
        "Kolmogorov-Smirnov D",
        "Kolmogorov-Smirnov Prob",
        "Lilliefors D",
        "Lilliefors Prob",
        "Anderson-Darling A2",
        "Anderson-Darling Prob",
        "D'Agostino K2",
        "D'Agostino Prob",
    ]);
    let mut notes = Vec::new();
    for inp in inputs {
        let data = inp.ys();
        let n = data.iter().filter(|x| !x.is_nan()).count();
        if n < 3 {
            return Err(AnalysisError::BadOption(format!(
                "{} has {n} numbers in the chosen rows; the normality tests need at least 3 (the Shapiro-Wilk test) or more",
                inp.name
            )));
        }
        let results: [(&str, Option<calc::TestResult>); 5] = [
            ("Shapiro-Wilk", calc::shapiro_wilk(data)),
            ("Kolmogorov-Smirnov", calc::kolmogorov_smirnov(data)),
            ("Lilliefors", calc::lilliefors(data)),
            ("Anderson-Darling", calc::anderson_darling(data)),
            ("D'Agostino-Pearson", calc::dagostino(data).omnibus),
        ];
        let mut values = vec![n as f64];
        for (_, r) in &results {
            values.push(r.map_or(f64::NAN, |r| r.statistic));
            values.push(r.map_or(f64::NAN, |r| r.p));
        }
        table.push_row(inp.label.clone(), values, vec![String::new(); 11]);
        let rejected: Vec<&str> =
            results.iter().filter(|(_, r)| r.is_some_and(|r| r.p < spec.alpha)).map(|(name, _)| *name).collect();
        let missing: Vec<&str> = results.iter().filter(|(_, r)| r.is_none()).map(|(name, _)| *name).collect();
        let mut line = if rejected.is_empty() {
            format!("{}: no test rejects normality at the {} level", inp.name, num(spec.alpha))
        } else {
            format!("{}: normality is rejected at the {} level by {}", inp.name, num(spec.alpha), rejected.join(", "))
        };
        if !missing.is_empty() {
            line.push_str(&format!(" ({} not available for {n} numbers, or for constant data)", missing.join(", ")));
        }
        line.push('.');
        notes.push(line);
    }
    Ok(Output { table: Some(table), on_sheet: Vec::new(), notes, plots: Vec::new() })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::PlotKind;

    /// The samples of the Origin reference runs (see `crates/calc/tests/origin_stats_tests.rs`): 8, 10 and 7 numbers.
    fn project() -> (Project, WindowId, Vec<ColumnId>) {
        let a: Vec<f64> = (1..=8).map(|i| 10.0 + 2.0 * (3.0 * f64::from(i)).sin() + f64::from(i) / 4.0).collect();
        let b: Vec<f64> = (1..=10).map(|i| 11.5 + 2.5 * (2.0 * f64::from(i)).cos() + f64::from(i) / 5.0).collect();
        let g: Vec<f64> = (1..=7).map(|i| 12.2 + 1.8 * (7.0 * f64::from(i)).sin() + f64::from(i) / 8.0).collect();
        let mut p = Project::new();
        let book = p.add_workbook("icp");
        let ids = [a, b, g]
            .into_iter()
            .map(|v| p.add_column(book, 0, Designation::Y, ColumnData::numeric(v)).unwrap())
            .collect();
        (p, book, ids)
    }

    fn run(p: &mut Project, columns: &[ColumnId], tool: Tool) -> Report {
        p.analyze(&Request { columns: columns.to_vec(), rows: RowRange::All, tool }).unwrap()
    }

    fn request(columns: &[ColumnId], tool: Tool) -> Request {
        Request { columns: columns.to_vec(), rows: RowRange::All, tool }
    }

    fn cell(p: &Project, r: &Report, name: &str) -> Vec<f64> {
        let ws = &p.workbook(r.book).unwrap().sheets[r.new_sheet.unwrap()];
        let c = ws.columns.iter().find(|c| c.long_name == name).unwrap_or_else(|| panic!("no column {name}"));
        c.numeric().unwrap().to_vec()
    }

    #[test]
    fn two_sample_t_test_gives_the_origin_numbers_on_a_new_sheet() {
        let (mut p, _, ids) = project();
        let spec =
            TTestSpec { kind: TTestKind::TwoSample { equal_variance: true, difference: 0.0 }, ..Default::default() };
        let r = run(&mut p, &ids[..2], Tool::TTest(spec));
        assert!(r.message.contains("Two-sample t-test") && r.message.contains("TTest1"), "{}", r.message);
        let close = |name: &str, want: f64| {
            let got = cell(&p, &r, name)[0];
            assert!((got - want).abs() <= 1e-4 * want.abs().max(1e-3), "{name}: {got} vs {want}");
        };
        close("t", -1.9072589691306436);
        close("DF", 16.0);
        close("Prob", 0.07460412595719583);
        close("LCL", -3.3411796677281207);
        close("UCL", 0.1764242747222089);
        close("N1", 8.0);
        close("N2", 10.0);
        assert_eq!(cell(&p, &r, "Reject"), vec![0.0]);
        let ws = &p.workbook(r.book).unwrap().sheets[r.new_sheet.unwrap()];
        assert_eq!(ws.columns[0].text().unwrap(), ["A vs B"]);
        assert!(r.notes.iter().any(|n| n.contains("not rejected")), "{:?}", r.notes);

        let welch = TTestSpec { kind: TTestKind::TwoSample { equal_variance: false, difference: 0.0 }, ..spec };
        let w = run(&mut p, &ids[..2], Tool::TTest(welch));
        assert!((cell(&p, &w, "DF")[0] - 15.634490446322753).abs() < 1e-8);
        assert!((cell(&p, &w, "t")[0] + 1.9937117971415532).abs() < 1e-8);
    }

    #[test]
    fn one_sample_t_test_has_a_row_per_column_and_paired_needs_two() {
        let (mut p, _, ids) = project();
        let spec = TTestSpec { kind: TTestKind::OneSample { mean: 10.0 }, ..Default::default() };
        let r = run(&mut p, &ids[..2], Tool::TTest(spec));
        let t = cell(&p, &r, "t");
        assert_eq!(t.len(), 2);
        assert!((t[0] - 2.1305388295862757).abs() < 1e-9);
        let paired = TTestSpec { kind: TTestKind::Paired { difference: 0.0 }, ..Default::default() };
        let err = p.analyze(&request(&ids, Tool::TTest(paired))).unwrap_err();
        assert!(err.to_string().contains("exactly two columns"), "{err}");
        let bad = TTestSpec { alpha: 1.5, ..spec };
        assert!(p.analyze(&request(&ids, Tool::TTest(bad))).is_err());
    }

    #[test]
    fn anova_sheet_has_groups_table_and_tukey_comparisons() {
        let (mut p, _, ids) = project();
        let r = run(&mut p, &ids, Tool::Anova(AnovaSpec::default()));
        assert!((cell(&p, &r, "F")[0] - 2.9133115443579345).abs() < 1e-9);
        assert_eq!(cell(&p, &r, "DF"), vec![2.0, 22.0, 24.0]);
        assert_eq!(cell(&p, &r, "N"), vec![8.0, 10.0, 7.0]);
        assert_eq!(cell(&p, &r, "Prob").len(), 3);
        assert_eq!(cell(&p, &r, "Significant").len(), 3);
        assert!(r.notes[0].contains("F(2, 22)"), "{:?}", r.notes);
        let none = run(&mut p, &ids, Tool::Anova(AnovaSpec { comparison: None, ..Default::default() }));
        let ws = &p.workbook(none.book).unwrap().sheets[none.new_sheet.unwrap()];
        assert!(!ws.columns.iter().any(|c| c.long_name == "Significant"));
        assert!(p.analyze(&request(&ids[..1], Tool::Anova(AnovaSpec::default()))).is_err());
    }

    #[test]
    fn histogram_makes_a_sheet_and_a_column_graph() {
        let (mut p, _, ids) = project();
        let freq = FreqOptions { min: Some(9.0), max: Some(14.4), size: BinSize::Width(0.9), ..Default::default() };
        let r = run(&mut p, &ids[..1], Tool::Histogram(HistogramSpec { freq, graph: true }));
        assert_eq!(cell(&p, &r, "Bin center").len(), 6);
        assert_eq!(cell(&p, &r, "Count (A)"), vec![0.0, 5.0, 1.0, 1.0, 1.0, 0.0]);
        assert_eq!(cell(&p, &r, "Cumulative count").last(), Some(&8.0));
        let g = p.graph(r.new_graph.expect("a graph")).unwrap();
        assert_eq!(g.layers[0].plots.len(), 1);
        assert_eq!(g.layers[0].plots[0].kind, PlotKind::Column);
        let x = p.column(g.layers[0].plots[0].x.unwrap()).unwrap();
        assert_eq!(x.long_name, "Bin center");
        let again = run(&mut p, &ids[..1], Tool::Histogram(HistogramSpec { freq, graph: false }));
        assert!(again.new_graph.is_none());
    }

    #[test]
    fn normality_table_lists_every_test() {
        let (mut p, book, ids) = project();
        let r = run(&mut p, &ids[..2], Tool::Normality(NormalitySpec::default()));
        let w = cell(&p, &r, "Shapiro-Wilk W");
        assert_eq!(w.len(), 2);
        assert!(w.iter().all(|w| *w > 0.0 && *w <= 1.0));
        assert_eq!(cell(&p, &r, "N"), vec![8.0, 10.0]);
        assert_eq!(r.notes.len(), 2);
        let tiny = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![1.0, 2.0])).unwrap();
        assert!(p.analyze(&request(&[tiny], Tool::Normality(NormalitySpec::default()))).is_err());
    }
}
