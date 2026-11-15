//! The statistics tools of `Project::analyze`: t-tests, one-way and two-way ANOVA, nonparametric tests,
//! correlation coefficients, normality tests and the histogram (frequency counts). The numerics are in `sheaf_calc`; this module arranges the chosen columns, writes the
//! result tables and the plain-English lines about them. What each tool reports follows Origin, see
//! `docs/spec/statistics.md`.

use super::*;
use sheaf_calc::{Category, TTest, anova_one_way, bin2d, freq_counts};

/// What a statistics tool produces; `analyze` turns it into a sheet (and a graph).
pub(super) struct Output {
    pub table: Option<ResultTable>,
    pub on_sheet: Vec<SheetColumn>,
    pub notes: Vec<String>,
    /// Pairs of indices into `on_sheet` (X column, Y column) for the plots of a histogram graph.
    pub plots: Vec<(usize, usize)>,
    /// The groups (indices into the inputs, `(i, j)` of each row) of the rows of a means comparison, in the order of the
    /// report; empty for every other tool. Kept on the report sheet for the Multiple Comparisons Plot (T58).
    pub comparison_pairs: Vec<(usize, usize)>,
}

pub(super) fn run(tool: &Tool, inputs: &[Input<'_>]) -> Result<Output, AnalysisError> {
    match tool {
        Tool::TTest(spec) => ttest(spec, inputs),
        Tool::VarTest(spec) => var_test(spec, inputs),
        Tool::Anova(spec) => anova(spec, inputs),
        Tool::Histogram(spec) => histogram(spec, inputs),
        Tool::Binning2D(spec) => binning_2d(spec, inputs),
        Tool::Normality(spec) => normality(spec, inputs),
        Tool::MannWhitney(spec) => mann_whitney(spec, inputs),
        Tool::SignedRank(spec) => signed_rank(spec, inputs),
        Tool::KruskalWallis(spec) => kruskal_wallis(spec, inputs),
        Tool::Correlation(spec) => correlation(spec, inputs),
        Tool::SignTest(spec) => sign_test(spec, inputs),
        Tool::KsTwoSample(spec) => ks_two_sample(spec, inputs),
        Tool::MedianTest(spec) => median_test(spec, inputs),
        Tool::Friedman(spec) => friedman(spec, inputs),
        Tool::Grubbs(spec) => grubbs(spec, inputs),
        Tool::Dixon(spec) => dixon(spec, inputs),
        _ => unreachable!("only the statistics tools come here"),
    }
}

pub(super) fn check_alpha(alpha: f64) -> Result<(), AnalysisError> {
    if alpha > 0.0 && alpha < 1.0 {
        Ok(())
    } else {
        Err(AnalysisError::BadOption(format!("the significance level must lie between 0 and 1, not {alpha}")))
    }
}

pub(super) fn label_column(name: &str, values: Vec<String>) -> SheetColumn {
    SheetColumn {
        designation: Designation::Label,
        long_name: name.to_string(),
        units: String::new(),
        comments: String::new(),
        data: ColumnData::text(values),
    }
}

pub(super) fn number_column(name: &str, units: &str, values: Vec<f64>) -> SheetColumn {
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
pub(super) fn num(v: f64) -> String {
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
    let few = |name: &str| {
        AnalysisError::BadOption(format!(
            "{name} has fewer than two numbers in the chosen rows, or no spread and a mean equal to the test value"
        ))
    };
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
                    "the {name} needs at least two numbers{} in each column, and the result must not be 0/0 (identical constant columns)",
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
    Ok(Output { table: Some(table), on_sheet: Vec::new(), notes, plots: Vec::new(), comparison_pairs: Vec::new() })
}

// ----- tests of variances and proportions ------------------------------------------------------------
//
// The result sheets follow Origin's report: one group of columns for each of its tables (the descriptive
// statistics, the test statistics, the confidence intervals; for proportions one set for each method), the
// null and alternative hypotheses and the conclusion as notes. See docs/spec/statistics.md section 21.

/// Checks the confidence levels (percent) and returns them, or nothing when the intervals are off.
fn check_levels(intervals: bool, levels: &[f64]) -> Result<Vec<f64>, AnalysisError> {
    if !intervals {
        return Ok(Vec::new());
    }
    if levels.is_empty() {
        return Err(AnalysisError::BadOption("give at least one confidence level, or switch the intervals off".into()));
    }
    for &l in levels {
        if !(l > 0.0 && l < 100.0) {
            return Err(AnalysisError::BadOption(format!("a confidence level must lie between 0 and 100 %, not {l}")));
        }
    }
    Ok(levels.to_vec())
}

fn squared(unit: &str) -> String {
    if unit.is_empty() { String::new() } else { format!("{unit}^2") }
}

/// Origin's null and alternative hypothesis text, `name` the tested quantity and `value` the hypothesized one.
fn hypotheses(name: &str, value: f64, tail: Tail) -> [String; 2] {
    let (null, alt) = match tail {
        Tail::Two => ("=", "<>"),
        Tail::Upper => ("<=", ">"),
        Tail::Lower => (">=", "<"),
    };
    [
        format!("Null Hypothesis: {name} {null} {}", num(value)),
        format!("Alternative Hypothesis: {name} {alt} {}", num(value)),
    ]
}

/// The columns of Origin's "Confidence Intervals" table: for each input a row per confidence level.
fn interval_columns(rows: Vec<(String, f64, f64, f64)>) -> Vec<SheetColumn> {
    vec![
        label_column("Data", rows.iter().map(|r| r.0.clone()).collect()),
        number_column("Conf. Levels in %", "", rows.iter().map(|r| r.1).collect()),
        number_column("Lower Limits", "", rows.iter().map(|r| r.2).collect()),
        number_column("Upper Limits", "", rows.iter().map(|r| r.3).collect()),
    ]
}

fn var_test(spec: &VarTestSpec, inputs: &[Input<'_>]) -> Result<Output, AnalysisError> {
    check_alpha(spec.alpha)?;
    let levels = check_levels(spec.intervals, &spec.levels)?;
    let bad = |what: &str| {
        AnalysisError::BadOption(format!(
            "{what}: the test needs two or more numbers in each column and a positive hypothesized value"
        ))
    };
    let mut notes = Vec::new();
    let mut on_sheet = Vec::new();
    let level_pct = |l: &f64| l / 100.0;
    match spec.kind {
        VarTestKind::OneSample { variance } => {
            let (mut n, mut mean, mut sd, mut var) = (vec![], vec![], vec![], vec![]);
            let (mut chi, mut df, mut prob) = (vec![], vec![], vec![]);
            let mut intervals = Vec::new();
            for inp in inputs {
                let r = calc::vartest_one(inp.ys(), variance, spec.tail, spec.alpha).ok_or_else(|| bad(&inp.name))?;
                let s = r.samples[0];
                n.push(s.n as f64);
                mean.push(s.mean);
                sd.push(s.sd);
                var.push(r.estimate);
                chi.push(r.statistic);
                df.push(r.df.0);
                prob.push(r.p);
                for l in &levels {
                    let (lo, hi) = calc::variance_interval(s.n, r.estimate, spec.tail, level_pct(l));
                    intervals.push((inp.label.clone(), *l, lo, hi));
                }
                notes.push(format!(
                    "{}: At the {} level, the population variance is {}significantly {} the test variance({}).",
                    inp.name,
                    num(spec.alpha),
                    if r.rejected() { "" } else { "NOT " },
                    match spec.tail {
                        Tail::Two => "different from",
                        Tail::Upper => "greater than",
                        Tail::Lower => "less than",
                    },
                    num(variance)
                ));
            }
            let units = shared_units(inputs);
            let sq = squared(&units);
            on_sheet.push(label_column("Data", inputs.iter().map(|i| i.label.clone()).collect()));
            on_sheet.push(number_column("N", "", n));
            on_sheet.push(number_column("Mean", &units, mean));
            on_sheet.push(number_column("SD", &units, sd));
            on_sheet.push(number_column("Variance", &sq, var));
            on_sheet.push(number_column("Chi-Square", "", chi));
            on_sheet.push(number_column("DF", "", df));
            let prob_name = if spec.tail == Tail::Lower { "Prob<Chi-Square" } else { "Prob>Chi-Square" };
            on_sheet.push(number_column(prob_name, "", prob));
            if !levels.is_empty() {
                on_sheet.extend(interval_columns(intervals));
            }
            let [null, alt] = hypotheses("Variance", variance, spec.tail);
            notes.insert(0, alt);
            notes.insert(0, null);
        }
        VarTestKind::TwoSample { ratio } => {
            if inputs.len() != 2 {
                return Err(AnalysisError::BadOption(format!(
                    "the two-sample test for variance needs exactly two columns, {} were chosen",
                    inputs.len()
                )));
            }
            let (a, b) = (&inputs[0], &inputs[1]);
            let name = format!("{} vs {}", a.name, b.name);
            let r = calc::vartest_two(a.ys(), b.ys(), ratio, spec.tail, spec.alpha).ok_or_else(|| bad(&name))?;
            let (s1, s2) = (r.samples[0], r.samples[1]);
            on_sheet.push(label_column("Data", vec![a.label.clone(), b.label.clone()]));
            on_sheet.push(number_column("N", "", vec![s1.n as f64, s2.n as f64]));
            let (u1, u2) = (&a.units, &b.units);
            let same = u1 == u2;
            on_sheet.push(number_column("Mean", if same { u1 } else { "" }, vec![s1.mean, s2.mean]));
            on_sheet.push(number_column("SD", if same { u1 } else { "" }, vec![s1.sd, s2.sd]));
            on_sheet.push(number_column(
                "Variance",
                &if same { squared(u1) } else { String::new() },
                vec![s1.sd * s1.sd, s2.sd * s2.sd],
            ));
            on_sheet.push(number_column("F", "", vec![r.statistic]));
            on_sheet.push(number_column("Numer. DF", "", vec![r.df.0]));
            on_sheet.push(number_column("Denom. DF", "", vec![r.df.1]));
            // Origin keeps the heading "Prob > F" for the lower tail too (the one-sample test switches to "Prob<Chi-Square").
            on_sheet.push(number_column("Prob > F", "", vec![r.p]));
            if !levels.is_empty() {
                let rows = levels
                    .iter()
                    .map(|l| {
                        let (lo, hi) = calc::ratio_interval(s1.n, s2.n, r.estimate, spec.tail, level_pct(l));
                        (name.clone(), *l, lo, hi)
                    })
                    .collect();
                on_sheet.extend(interval_columns(rows));
            }
            let [null, alt] = hypotheses("Variance1/Variance2", ratio, spec.tail);
            notes.push(null);
            notes.push(alt);
            let rel = match spec.tail {
                Tail::Two => "different",
                Tail::Upper => "greater than",
                Tail::Lower => "less than",
            };
            // Origin's wording (measured): "the two population variances are [NOT] significantly different" and
            // "the ratio of two population variances is [NOT] significantly greater than 1".
            let (subject, test_value) = if spec.tail == Tail::Two {
                ("two population variances are", String::new())
            } else {
                ("ratio of two population variances is", format!(" {}", num(ratio)))
            };
            notes.push(format!(
                "At the {} level, the {subject} {}significantly {rel}{test_value}.",
                num(spec.alpha),
                if r.rejected() { "" } else { "NOT " },
            ));
        }
    }
    Ok(Output { table: None, on_sheet, notes, plots: Vec::new(), comparison_pairs: Vec::new() })
}

/// The warning line under the normal approximation (our wording).
const NORMAL_DOUBTFUL: &str =
    "The normal approximation may be unreliable with fewer than 10 successes or fewer than 10 failures in a sample.";

/// Origin's "Powers" table: one row per sample size, first the actual ones (when "Actual Power" is ticked),
/// then the typed hypothetical ones, each with the power analysis' own significance level.
fn power_table(
    on_sheet: &mut Vec<SheetColumn>,
    power: &PropPowerSpec,
    actual: Vec<f64>,
    f: impl Fn(f64, f64) -> f64,
) -> Result<(), AnalysisError> {
    if !(power.actual || power.hypothetical) {
        return Ok(());
    }
    if !(power.alpha > 0.0 && power.alpha < 1.0) {
        return Err(AnalysisError::BadOption(
            "the significance level of the power analysis must be between 0 and 1".into(),
        ));
    }
    let mut sizes = Vec::new();
    if power.actual {
        sizes.extend(actual);
    }
    if power.hypothetical {
        if power.sizes.is_empty() || power.sizes.contains(&0) {
            return Err(AnalysisError::BadOption("the hypothetical sample sizes must be whole numbers above 0".into()));
        }
        sizes.extend(power.sizes.iter().map(|n| *n as f64));
    }
    on_sheet.push(number_column("Alpha", "", vec![power.alpha; sizes.len()]));
    on_sheet.push(number_column("Sample Size", "", sizes.clone()));
    on_sheet.push(number_column("Power", "", sizes.iter().map(|n| f(*n, power.alpha)).collect()));
    Ok(())
}

pub(super) fn prop_test(spec: &PropTestSpec) -> Result<Output, AnalysisError> {
    check_alpha(spec.alpha)?;
    if !(spec.normal || spec.exact) {
        return Err(AnalysisError::BadOption("tick at least one of Normal Approximation and Binomial Test".into()));
    }
    let levels = check_levels(spec.intervals, &spec.levels)?;
    let bad = || {
        AnalysisError::BadOption(
            "the proportion test needs a sample size above 0, no more successes than the sample size and a test proportion strictly between 0 and 1 (a difference between -1 and 1 for two samples)"
                .into(),
        )
    };
    let exact_too_large = || {
        AnalysisError::BadOption(format!(
            "the exact test is only computed for sample sizes up to {}; use the normal approximation",
            calc::vartest::MAX_EXACT_SIZE
        ))
    };
    let tail = spec.tail;
    let head_level = 1.0 - spec.alpha;
    let head = |what: &str| format!("{}% {what}", num(head_level * 100.0));
    let mut on_sheet = Vec::new();
    let mut notes = Vec::new();
    // The headline LCL and UCL (level 1 - alpha) and the table of the confidence levels.
    let limits = |on_sheet: &mut Vec<SheetColumn>, f: &dyn Fn(f64) -> (f64, f64)| {
        if !spec.intervals {
            return;
        }
        let (lo, hi) = f(head_level);
        on_sheet.push(number_column(&head("LCL"), "", vec![lo]));
        on_sheet.push(number_column(&head("UCL"), "", vec![hi]));
    };
    let level_table = |on_sheet: &mut Vec<SheetColumn>, f: &dyn Fn(f64) -> (f64, f64)| {
        if levels.is_empty() {
            return;
        }
        let rows: Vec<(f64, f64)> = levels.iter().map(|l| f(l / 100.0)).collect();
        on_sheet.push(number_column("Conf. Levels in %", "", levels.clone()));
        on_sheet.push(number_column("Lower Limits", "", rows.iter().map(|r| r.0).collect()));
        on_sheet.push(number_column("Upper Limits", "", rows.iter().map(|r| r.1).collect()));
    };
    match spec.kind {
        PropTestKind::OneSample { successes, size, proportion } => {
            if spec.exact && size > calc::vartest::MAX_EXACT_SIZE {
                return Err(exact_too_large());
            }
            let r = calc::proportion_one(successes, size, proportion, tail).ok_or_else(bad)?;
            let [null, alt] = hypotheses("Proportion", proportion, tail);
            notes.push(null);
            notes.push(alt);
            let basics = |on_sheet: &mut Vec<SheetColumn>| {
                on_sheet.push(number_column("Success", "", vec![successes as f64]));
                on_sheet.push(number_column("N", "", vec![size as f64]));
                on_sheet.push(number_column("Sample P", "", vec![r.estimate]));
            };
            if spec.normal {
                basics(&mut on_sheet);
                on_sheet.push(number_column("Z", "", vec![r.z]));
                on_sheet.push(number_column("P", "", vec![r.p_normal]));
                let wald = |level: f64| calc::wald_interval(successes, size, tail, level);
                limits(&mut on_sheet, &wald);
                level_table(&mut on_sheet, &wald);
                if calc::vartest::normal_approximation_doubtful(successes, size) {
                    notes.push(
                        "The normal approximation may be unreliable with fewer than 10 successes or fewer than 10 failures."
                            .into(),
                    );
                }
            }
            if spec.exact {
                basics(&mut on_sheet);
                on_sheet.push(number_column("Exact P", "", vec![r.p_exact]));
                let exact = |level: f64| calc::exact_interval(successes, size, tail, level);
                limits(&mut on_sheet, &exact);
                level_table(&mut on_sheet, &exact);
            }
            let p1 = r.estimate;
            power_table(&mut on_sheet, &spec.power, vec![size as f64], |n, a| {
                calc::power_one_proportion(p1, proportion, n, a, tail)
            })?;
        }
        PropTestKind::TwoSample { first, second, difference } => {
            if spec.exact && difference == 0.0 && first.1.saturating_add(second.1) > calc::vartest::MAX_EXACT_SIZE {
                return Err(exact_too_large());
            }
            let r = calc::proportion_two(first, second, difference, spec.pooled, tail).ok_or_else(bad)?;
            let [null, alt] = hypotheses("Proportion1 - Proportion2", difference, tail);
            notes.push(null);
            notes.push(alt);
            // Each section (normal approximation, Fisher) repeats the sample summary and the difference.
            let summary = |on_sheet: &mut Vec<SheetColumn>| {
                on_sheet.push(label_column("Sample", vec!["Sample1".into(), "Sample2".into()]));
                on_sheet.push(number_column("Success", "", vec![first.0 as f64, second.0 as f64]));
                on_sheet.push(number_column("N", "", vec![first.1 as f64, second.1 as f64]));
                on_sheet.push(number_column(
                    "Sample P",
                    "",
                    vec![first.0 as f64 / first.1 as f64, second.0 as f64 / second.1 as f64],
                ));
                on_sheet.push(label_column("Difference", vec!["Sample1 - Sample2".into()]));
                on_sheet.push(number_column("Estimate for difference", "", vec![r.estimate]));
            };
            let wald = |level: f64| calc::wald_difference_interval(first, second, tail, level);
            if spec.normal {
                summary(&mut on_sheet);
                on_sheet.push(number_column("Z", "", vec![r.z]));
                on_sheet.push(number_column("P", "", vec![r.p_normal]));
                limits(&mut on_sheet, &wald);
                level_table(&mut on_sheet, &wald);
                if calc::vartest::two_sample_normal_doubtful(first, second) {
                    notes.push(NORMAL_DOUBTFUL.into());
                }
            }
            if let (true, Some(p)) = (spec.exact, r.p_fisher) {
                // Fisher's section shows the same Wald interval of the difference as the normal one.
                summary(&mut on_sheet);
                on_sheet.push(number_column("Exact P", "", vec![p]));
                limits(&mut on_sheet, &wald);
                level_table(&mut on_sheet, &wald);
            } else if spec.exact {
                // Origin leaves the Fisher section out for a test difference other than 0, without a word.
                notes.push("Fisher's exact test is only given for a hypothesized difference of 0.".into());
            }
            let rate = |(x, n): (u64, u64)| x as f64 / n as f64;
            let (p1, p2) = (rate(first), rate(second));
            // The "Actual Power" rows take each sample size as the size of both samples (once when equal).
            let mut actual = vec![first.1 as f64];
            if second.1 != first.1 {
                actual.push(second.1 as f64);
            }
            power_table(&mut on_sheet, &spec.power, actual, |n, a| calc::power_two_proportions(p1, p2, n, a, tail))?;
        }
    }
    Ok(Output { table: None, on_sheet, notes, plots: Vec::new(), comparison_pairs: Vec::new() })
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
    let mut comparison_pairs = Vec::new();
    if let Some(method) = spec.comparison {
        if method == MeansComparison::Dunnett && spec.control >= inputs.len() {
            return Err(AnalysisError::BadOption(format!(
                "Dunnett's control group is number {}, but only {} columns were chosen",
                spec.control + 1,
                inputs.len()
            )));
        }
        let comparisons = r.compare_with_control(method, spec.control, spec.alpha);
        comparison_pairs = comparisons.iter().map(|c| (c.i, c.j)).collect();
        let stat_name = match method {
            MeansComparison::Tukey => "q",
            MeansComparison::Scheffe => "F",
            m if m.is_holm() => "t",
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
        if method.is_holm() {
            // The step-down methods give no interval, but the level each pair was judged at.
            on_sheet.push(number_column("Alpha", "", c(|c| c.alpha)));
        } else {
            on_sheet.push(number_column("LCL", &units, c(|c| c.lcl)));
            on_sheet.push(number_column("UCL", &units, c(|c| c.ucl)));
        }
        on_sheet.push(number_column("Significant", "", c(|c| f64::from(u8::from(c.significant)))));
        notes.push(format!("{} means comparison at the {} level:", method.label(), num(spec.alpha)));
        if method == MeansComparison::Dunnett {
            notes.push(format!(
                "  Every group against the control {}; the p-values are adjusted for all {} comparisons and the limits are simultaneous at level {}.",
                inputs[spec.control].name,
                comparisons.len(),
                num(1.0 - spec.alpha)
            ));
        } else if method.is_holm() {
            notes.push("  (sorted by p-value; each pair is judged at the level of its rank, and after the first pair that is not significant no later pair is)".to_string());
        }
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
    Ok(Output { table: Some(table), on_sheet, notes, plots: Vec::new(), comparison_pairs })
}

// ----- histogram -------------------------------------------------------------------------------------

pub(super) fn histogram(spec: &HistogramSpec, inputs: &[Input<'_>]) -> Result<Output, AnalysisError> {
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
    Ok(Output { table: Some(table), on_sheet, notes, plots, comparison_pairs: Vec::new() })
}

// ----- 2D frequency counts and binning -----------------------------------------------------------------

/// `Bin Ends of A`, or `Bin Ends of "Potential"` when the column has a long name (as Origin names the first
/// result column).
fn bin_label_name(centers: bool, short: &str, display: &str) -> String {
    let what = if centers { "Bin Centers of" } else { "Bin Ends of" };
    if display == short { format!("{what} {short}") } else { format!("{what} \"{display}\"") }
}

fn binning_2d(spec: &Bin2dOptions, inputs: &[Input<'_>]) -> Result<Output, AnalysisError> {
    let (x, y, x_short, x_name, y_name) = match inputs {
        [one] => (one.xs(), one.ys(), one.x_short.as_str(), one.x_name.as_str(), one.name.as_str()),
        [a, b, ..] => (a.ys(), b.ys(), a.short.as_str(), a.name.as_str(), b.name.as_str()),
        [] => return Err(AnalysisError::NoColumns),
    };
    let r = bin2d(x, y, spec).ok_or_else(|| {
        AnalysisError::BadOption(format!(
            "no 2D frequency counts can be made of {y_name} against {x_name}: there are no points in the chosen              rows, or the bin range or size is not usable"
        ))
    })?;
    let mut on_sheet = Vec::with_capacity(r.y.bins.len() + 1);
    let mut first = number_column(
        &bin_label_name(spec.x.centers, x_short, x_name),
        "",
        (0..r.x.bins.len()).map(|i| r.x.label(i)).collect(),
    );
    first.designation = Designation::X;
    on_sheet.push(first);
    let units = if spec.stat == Bin2dStat::PercentFrequency { "%" } else { "" };
    for j in 0..r.y.bins.len() {
        let mut c = number_column(spec.stat.label(), units, r.cells.iter().map(|row| row[j]).collect());
        c.comments = r.y.range_text(j);
        on_sheet.push(c);
    }
    let n = x.len().min(y.len());
    let points = (0..n).filter(|&i| x[i].is_finite() && y[i].is_finite()).count();
    let mut notes = vec![format!(
        "{} points of {y_name} against {x_name} in {} X bins by {} Y bins ({}); {} outside the bins.",
        points,
        r.x.bins.len(),
        r.y.bins.len(),
        spec.stat.label().to_lowercase(),
        points - r.binned
    )];
    if inputs.len() > 2 {
        notes.push(format!(
            "Only the first two columns are used (X and Y); {} is ignored, as in Origin.",
            inputs[2..].iter().map(|i| i.short.as_str()).collect::<Vec<_>>().join(", ")
        ));
    }
    Ok(Output { table: None, on_sheet, notes, plots: Vec::new(), comparison_pairs: Vec::new() })
}

// ----- discrete frequency -------------------------------------------------------------------------------

/// The cells of a column as categories, from the first row to the last filled one (or the chosen rows).
fn categories(c: &Column, rows: RowRange) -> Result<Vec<Category>, AnalysisError> {
    let cell = |row: usize| match &c.data {
        ColumnData::Text(v) => Category::Text(v.get(row).cloned().unwrap_or_default()),
        ColumnData::Numeric(v) => Category::Number(v.get(row).copied().unwrap_or(f64::NAN)),
        ColumnData::Mixed { numbers, .. } => match c.data.text_cell(row) {
            Some(t) => Category::Text(t.to_string()),
            None => Category::Number(numbers.get(row).copied().unwrap_or(f64::NAN)),
        },
    };
    let empty = |v: &Category| match v {
        Category::Missing => true,
        Category::Number(x) => x.is_nan(),
        Category::Text(s) => s.is_empty(),
    };
    let mut all: Vec<Category> = (0..c.data.len()).map(cell).collect();
    while all.last().is_some_and(empty) {
        all.pop();
    }
    if all.is_empty() {
        return Err(AnalysisError::Empty(c.short_name.clone()));
    }
    match rows {
        RowRange::All => Ok(all),
        RowRange::Rows(..) => {
            let (first, last) = resolve_rows(rows, &c.short_name, &[], all.len())?;
            Ok(all[first..=last].to_vec())
        }
        RowRange::XBetween(..) => Err(AnalysisError::BadOption(
            "the discrete frequency has no X range to limit; choose rows with a row range".into(),
        )),
    }
}

pub(super) fn discrete_frequency(
    spec: &DiscreteFreqSpec,
    sheet: &Worksheet,
    columns: &[ColumnId],
    rows: RowRange,
) -> Result<Output, AnalysisError> {
    if !(spec.count || spec.relative || spec.cumulative) {
        return Err(AnalysisError::BadOption("tick at least one of Count, Relative and Cumulative Frequency".into()));
    }
    let mut on_sheet = Vec::new();
    let mut notes = Vec::new();
    let (scale, unit) = if spec.percent { (100.0, "%") } else { (1.0, "") };
    for id in columns {
        let c = sheet.column(*id).ok_or(AnalysisError::NoSuchColumn(*id))?;
        let f = sheaf_calc::discrete_frequency(&categories(c, rows)?, spec.case_sensitive, spec.sort);
        let with = |mut col: SheetColumn, d: Designation| {
            col.designation = d;
            col.comments = c.short_name.clone();
            col
        };
        let numeric = f.rows.iter().all(|r| !matches!(r.value, Category::Text(_)));
        let data = if numeric {
            number_column(
                "Data",
                "",
                f.rows
                    .iter()
                    .map(|r| match r.value {
                        Category::Number(x) => x,
                        _ => f64::NAN,
                    })
                    .collect(),
            )
        } else {
            let texts = f
                .rows
                .iter()
                .map(|r| match &r.value {
                    Category::Text(s) => s.clone(),
                    Category::Number(x) => format!("{x}"),
                    Category::Missing => String::new(),
                })
                .collect();
            label_column("Data", texts)
        };
        on_sheet.push(with(data, Designation::X));
        if spec.count {
            on_sheet.push(with(
                number_column("Count", "", f.rows.iter().map(|r| r.count as f64).collect()),
                Designation::Y,
            ));
        }
        if spec.relative {
            on_sheet.push(with(
                number_column("Relative Frequency", unit, f.relative().iter().map(|v| v * scale).collect()),
                Designation::Y,
            ));
        }
        if spec.cumulative {
            on_sheet.push(with(
                number_column("Cumulative Frequency", unit, f.cumulative().iter().map(|v| v * scale).collect()),
                Designation::Y,
            ));
        }
        notes.push(format!(
            "{}: {} cells, {} distinct values ({}{}).",
            c.display_name(),
            f.total,
            f.rows.len(),
            spec.sort.label().to_lowercase(),
            if spec.case_sensitive { ", case-sensitive" } else { "" }
        ));
    }
    Ok(Output { table: None, on_sheet, notes, plots: Vec::new(), comparison_pairs: Vec::new() })
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
    Ok(Output { table: Some(table), on_sheet: Vec::new(), notes, plots: Vec::new(), comparison_pairs: Vec::new() })
}

// ----- two-way ANOVA ---------------------------------------------------------------------------------

/// A factor cell as text: the cell itself, or a number written without a trailing ".0". `None` when empty.
pub(super) fn level_text(col: &Column, row: usize) -> Option<String> {
    match &col.data {
        ColumnData::Text(v) => v.get(row).map(|s| s.trim()).filter(|s| !s.is_empty()).map(str::to_string),
        ColumnData::Mixed { .. } if col.data.text_cell(row).is_some() => {
            col.data.text_cell(row).map(|s| s.trim().to_string())
        }
        ColumnData::Numeric(v) | ColumnData::Mixed { numbers: v, .. } => v
            .get(row)
            .copied()
            .filter(|x| !x.is_nan())
            .map(|x| if x.fract() == 0.0 && x.abs() < 1e15 { format!("{}", x as i64) } else { format!("{x}") }),
    }
}

/// The number of a level in `levels`, which grows by the levels in the order they first appear.
fn level_index(levels: &mut Vec<String>, name: &str) -> usize {
    levels.iter().position(|l| l == name).unwrap_or_else(|| {
        levels.push(name.to_string());
        levels.len() - 1
    })
}

pub(super) fn two_way_anova(
    spec: &TwoWayAnovaSpec,
    sheet: &Worksheet,
    columns: &[ColumnId],
    rows: RowRange,
) -> Result<Output, AnalysisError> {
    check_alpha(spec.alpha)?;
    if spec.comparison.is_some_and(MeansComparison::is_holm) {
        return Err(AnalysisError::BadOption(
            "the Holm comparisons are only available in the one-way ANOVA (Origin's two-way numbers for them were not checked)"
                .into(),
        ));
    }
    if spec.comparison == Some(MeansComparison::Dunnett) {
        return Err(AnalysisError::BadOption(
            "Dunnett's test needs one control group; it is available in the one-way ANOVA only".to_string(),
        ));
    }
    let column = |id: ColumnId| sheet.column(id).ok_or(AnalysisError::NoSuchColumn(id));
    fn numeric(c: &Column) -> Result<&[f64], AnalysisError> {
        c.numeric().ok_or_else(|| AnalysisError::NotNumeric(c.short_name.clone()))
    }
    let row_limits = |len: usize, short: &str| -> Result<(usize, usize), AnalysisError> {
        match rows {
            RowRange::All => Ok((0, len.saturating_sub(1))),
            RowRange::Rows(..) => resolve_rows(rows, short, &[], len),
            RowRange::XBetween(..) => Err(AnalysisError::BadOption(
                "the two-way ANOVA has no X column to limit; choose rows with a row range".into(),
            )),
        }
    };
    let (mut levels_a, mut levels_b): (Vec<String>, Vec<String>) = (Vec::new(), Vec::new());
    let (mut a, mut b, mut y): (Vec<usize>, Vec<usize>, Vec<f64>) = (Vec::new(), Vec::new(), Vec::new());
    let (name_a, name_b);
    let mut left_out = 0usize;
    match &spec.layout {
        TwoWayLayout::Indexed => {
            if columns.len() != 3 {
                return Err(AnalysisError::BadOption(format!(
                    "the two-way ANOVA with indexed data needs three columns (factor A, factor B, data), {} were chosen",
                    columns.len()
                )));
            }
            let (ca, cb, cy) = (column(columns[0])?, column(columns[1])?, column(columns[2])?);
            let ys = numeric(cy)?;
            let len = ca.data.len().max(cb.data.len()).max(ys.len());
            if len == 0 {
                return Err(AnalysisError::Empty(cy.short_name.clone()));
            }
            let (first, last) = row_limits(len, &cy.short_name)?;
            for row in first..=last.min(len - 1) {
                let value = ys.get(row).copied().unwrap_or(f64::NAN);
                let (la, lb) = (level_text(ca, row), level_text(cb, row));
                match (&la, &lb) {
                    (Some(la), Some(lb)) if !value.is_nan() => {
                        a.push(level_index(&mut levels_a, la));
                        b.push(level_index(&mut levels_b, lb));
                        y.push(value);
                    }
                    // Rows that are empty in all three columns are just the end of a shorter column.
                    _ => left_out += usize::from(!(value.is_nan() && la.is_none() && lb.is_none())),
                }
            }
            name_a = ca.display_name().to_string();
            name_b = cb.display_name().to_string();
        }
        TwoWayLayout::Raw { levels } => {
            if levels.len() != columns.len() {
                return Err(AnalysisError::BadOption(format!(
                    "give the levels of factor A and factor B for each of the {} chosen columns ({} were given)",
                    columns.len(),
                    levels.len()
                )));
            }
            for (id, (la, lb)) in columns.iter().zip(levels) {
                let c = column(*id)?;
                let data = numeric(c)?;
                if data.is_empty() {
                    return Err(AnalysisError::Empty(c.short_name.clone()));
                }
                let (first, last) = row_limits(data.len(), &c.short_name)?;
                let (ia, ib) = (level_index(&mut levels_a, la.trim()), level_index(&mut levels_b, lb.trim()));
                for &v in &data[first..=last.min(data.len() - 1)] {
                    if v.is_nan() {
                        continue;
                    }
                    a.push(ia);
                    b.push(ib);
                    y.push(v);
                }
            }
            name_a = "Factor A".to_string();
            name_b = "Factor B".to_string();
        }
    }
    let r = calc::anova_two_way(&a, &b, &y, spec.interaction).map_err(|e| AnalysisError::BadOption(e.to_string()))?;
    let name_ab = format!("{name_a}*{name_b}");
    let cell_name = |i: usize| format!("{} / {}", levels_a[i / levels_b.len()], levels_b[i % levels_b.len()]);

    // Descriptive statistics of the levels, the cells and all the data, in one table.
    let mut group = Vec::new();
    let mut stats = Vec::new();
    for (l, s) in levels_a.iter().zip(&r.levels_a) {
        group.push(format!("{name_a}: {l}"));
        stats.push(*s);
    }
    for (l, s) in levels_b.iter().zip(&r.levels_b) {
        group.push(format!("{name_b}: {l}"));
        stats.push(*s);
    }
    if spec.interaction {
        for (i, s) in r.cells.iter().enumerate() {
            group.push(format!("{name_ab}: {}", cell_name(i)));
            stats.push(*s);
        }
    }
    group.push("All data".to_string());
    stats.push(r.overall);
    let mut on_sheet = vec![
        label_column("Group", group),
        number_column("N", "", stats.iter().map(|s| s.n as f64).collect()),
        number_column("Mean", "", stats.iter().map(|s| s.mean).collect()),
        number_column("SD", "", stats.iter().map(|s| s.sd).collect()),
        number_column("SEM", "", stats.iter().map(|s| s.sem).collect()),
    ];

    // The ANOVA table.
    let mut table = ResultTable::new(&["DF", "Sum of Squares", "Mean Square", "F", "Prob>F"]);
    let mut rows_out: Vec<(String, calc::TwoWayRow)> = vec![(name_a.clone(), r.factor_a), (name_b.clone(), r.factor_b)];
    if let Some(ab) = r.interaction_row {
        rows_out.push((name_ab.clone(), ab));
    }
    rows_out.extend([("Model".to_string(), r.model), ("Error".to_string(), r.error), ("Total".to_string(), r.total)]);
    for (name, row) in &rows_out {
        table.push_row(name.clone(), vec![row.df, row.sum_sq, row.mean_sq, row.f, row.p], vec![String::new(); 5]);
    }
    on_sheet.push(label_column("Source", rows_out.iter().map(|(n, _)| n.clone()).collect()));
    for (i, h) in table.headers.iter().enumerate() {
        on_sheet.push(number_column(h, "", table.rows.iter().map(|(_, v)| v[i]).collect()));
    }
    on_sheet.push(label_column("Statistic", ["R-squared", "Root MSE"].map(str::to_string).to_vec()));
    on_sheet.push(number_column("Value", "", vec![r.r_squared, r.root_mse]));

    // Notes.
    let mut notes = Vec::new();
    let terms: Vec<String> = rows_out
        .iter()
        .take(if spec.interaction { 3 } else { 2 })
        .map(|(n, row)| {
            format!(
                "{n}: F({}, {}) = {}, p = {} ({})",
                num(row.df),
                num(r.error.df),
                num(row.f),
                num(row.p),
                if row.p < spec.alpha { "significant" } else { "not significant" }
            )
        })
        .collect();
    notes.push(format!(
        "Two-way ANOVA of {} numbers ({} levels of {name_a}, {} of {name_b}, {}): {}; at the {} level.",
        y.len(),
        levels_a.len(),
        levels_b.len(),
        if spec.interaction { "with interaction" } else { "without interaction" },
        terms.join("; "),
        num(spec.alpha)
    ));
    let sizes: Vec<usize> = r.cells.iter().map(|s| s.n).collect();
    if sizes.iter().any(|n| *n != sizes[0]) {
        notes.push(
            "The cells have different sizes: every sum of squares is adjusted for all the other terms (type III, as Origin does)."
                .to_string(),
        );
    }
    if left_out > 0 {
        notes.push(format!("{left_out} rows with a missing factor level or no number were left out."));
    }
    if r.error.df <= 0.0 {
        notes.push(
            "There are no error degrees of freedom (one number per cell): no F tests and no means comparisons."
                .to_string(),
        );
    }
    if let Some(method) = spec.comparison.filter(|_| r.error.df > 0.0) {
        let stat_name = match method {
            MeansComparison::Tukey => "q",
            MeansComparison::Scheffe => "F",
            _ => "t",
        };
        let mut label = Vec::new();
        let mut all = Vec::new();
        let mut add = |names: &[String], factor: &str, list: Vec<calc::Comparison>| {
            for c in list {
                label.push(format!("{factor}: {} - {}", names[c.j], names[c.i]));
                all.push(c);
            }
        };
        add(&levels_a, &name_a, r.compare_a(method, spec.alpha));
        add(&levels_b, &name_b, r.compare_b(method, spec.alpha));
        if spec.interaction {
            let cells: Vec<String> = (0..r.cells.len()).map(cell_name).collect();
            add(&cells, &name_ab, r.compare_cells(method, spec.alpha));
        }
        let c = |f: fn(&calc::Comparison) -> f64| all.iter().map(f).collect::<Vec<f64>>();
        on_sheet.push(label_column(&format!("{} comparison", method.label()), label.clone()));
        on_sheet.push(number_column("Difference", "", c(|c| c.diff)));
        on_sheet.push(number_column("SE", "", c(|c| c.se)));
        on_sheet.push(number_column(&format!("Statistic ({stat_name})"), "", c(|c| c.stat)));
        on_sheet.push(number_column("Prob", "", c(|c| c.p)));
        on_sheet.push(number_column("LCL", "", c(|c| c.lcl)));
        on_sheet.push(number_column("UCL", "", c(|c| c.ucl)));
        on_sheet.push(number_column("Significant", "", c(|c| f64::from(u8::from(c.significant)))));
        notes.push(format!("{} means comparison at the {} level:", method.label(), num(spec.alpha)));
        for (l, c) in label.iter().zip(&all) {
            notes.push(format!(
                "  {l}: difference {}, p = {}, {}",
                num(c.diff),
                num(c.p),
                if c.significant { "significant" } else { "not significant" }
            ));
        }
    }
    Ok(Output { table: Some(table), on_sheet, notes, plots: Vec::new(), comparison_pairs: Vec::new() })
}

// ----- Mann-Whitney, signed rank, Kruskal-Wallis ---------------------------------------------------------

fn reject_words(p: f64, alpha: f64) -> String {
    if p < alpha {
        format!("the null hypothesis is rejected at the {} level", num(alpha))
    } else {
        format!("the null hypothesis is not rejected at the {} level", num(alpha))
    }
}

fn mann_whitney(spec: &MannWhitneySpec, inputs: &[Input<'_>]) -> Result<Output, AnalysisError> {
    check_alpha(spec.alpha)?;
    if inputs.len() != 2 {
        return Err(AnalysisError::BadOption(format!(
            "the Mann-Whitney test needs exactly two columns, {} were chosen",
            inputs.len()
        )));
    }
    let (a, b) = (&inputs[0], &inputs[1]);
    let r = calc::mann_whitney(a.ys(), b.ys(), spec.tail, spec.exact).ok_or_else(|| {
        AnalysisError::BadOption("each column needs at least one number in the chosen rows".to_string())
    })?;
    let mut table = ResultTable::new(&[
        "N1",
        "N2",
        "Mean rank 1",
        "Mean rank 2",
        "Rank sum 1",
        "Rank sum 2",
        "U",
        "Z",
        "Prob (asymptotic)",
        "Prob (exact)",
        "Reject",
    ]);
    table.push_row(
        format!("{} vs {}", a.name, b.name),
        vec![
            r.n1 as f64,
            r.n2 as f64,
            r.mean_rank1,
            r.mean_rank2,
            r.rank_sum1,
            r.rank_sum2,
            r.u,
            r.z,
            r.p_asymptotic,
            r.p_exact.unwrap_or(f64::NAN),
            f64::from(u8::from(r.p() < spec.alpha)),
        ],
        vec![String::new(); 11],
    );
    let how = if r.p_exact.is_some() { "exact" } else { "asymptotic" };
    let claim = match spec.tail {
        Tail::Two => format!("the distributions of {} and {} differ", a.name, b.name),
        Tail::Upper => format!("{} tends to be larger than {}", a.name, b.name),
        Tail::Lower => format!("{} tends to be smaller than {}", a.name, b.name),
    };
    let mut notes = vec![format!(
        "{} vs {}: Mann-Whitney test, {}: {claim} (U = {}, z = {}, {how} p = {}); {}",
        a.name,
        b.name,
        tail_name(spec.tail),
        num(r.u),
        num(r.z),
        num(r.p()),
        reject_words(r.p(), spec.alpha)
    )];
    if spec.exact && r.p_exact.is_none() {
        notes.push("The exact p-value would take too long for this many numbers; the asymptotic one was used.".into());
    }
    Ok(Output { table: Some(table), on_sheet: Vec::new(), notes, plots: Vec::new(), comparison_pairs: Vec::new() })
}

fn signed_rank(spec: &SignedRankSpec, inputs: &[Input<'_>]) -> Result<Output, AnalysisError> {
    check_alpha(spec.alpha)?;
    let mut table = ResultTable::new(&[
        "N",
        "Zero differences",
        "Sum of positive ranks (T+)",
        "Sum of negative ranks (T-)",
        "Z",
        "Prob (asymptotic)",
        "Prob (exact)",
        "Reject",
    ]);
    let mut notes = Vec::new();
    let mut add = |label: String, claim: String, r: &calc::SignedRank| {
        let p = if spec.exact { r.p() } else { r.p_asymptotic };
        table.push_row(
            label.clone(),
            vec![
                r.n as f64,
                r.n_zero as f64,
                r.t_plus,
                r.t_minus,
                r.z,
                r.p_asymptotic,
                r.p_exact.unwrap_or(f64::NAN),
                f64::from(u8::from(p < spec.alpha)),
            ],
            vec![String::new(); 8],
        );
        let how = if spec.exact && r.p_exact.is_some() { "exact" } else { "asymptotic" };
        notes.push(format!(
            "{label}: Wilcoxon signed rank test, {}: {claim} (T+ = {}, z = {}, {how} p = {}); {}",
            tail_name(spec.tail),
            num(r.t_plus),
            num(r.z),
            num(p),
            reject_words(p, spec.alpha)
        ));
        if r.n == 0 {
            notes.push(format!("{label}: every difference is zero, so there is nothing to rank (p = 1)."));
        }
    };
    match spec.kind {
        SignedRankKind::OneSample { median } => {
            let side = match spec.tail {
                Tail::Two => "differs from",
                Tail::Upper => "is larger than",
                Tail::Lower => "is smaller than",
            };
            for inp in inputs {
                let r = calc::signed_rank_one(inp.ys(), median, spec.tail);
                add(inp.label.clone(), format!("the median of {} {side} {}", inp.name, num(median)), &r);
            }
        }
        SignedRankKind::Paired => {
            if inputs.len() != 2 {
                return Err(AnalysisError::BadOption(format!(
                    "the paired signed rank test needs exactly two columns, {} were chosen",
                    inputs.len()
                )));
            }
            let (a, b) = (&inputs[0], &inputs[1]);
            let r = calc::signed_rank_paired(a.ys(), b.ys(), spec.tail);
            let claim = match spec.tail {
                Tail::Two => format!("{} and {} differ in median", a.name, b.name),
                Tail::Upper => format!("{} tends to be larger than {}", a.name, b.name),
                Tail::Lower => format!("{} tends to be smaller than {}", a.name, b.name),
            };
            add(format!("{} vs {}", a.name, b.name), claim, &r);
        }
    }
    Ok(Output { table: Some(table), on_sheet: Vec::new(), notes, plots: Vec::new(), comparison_pairs: Vec::new() })
}

fn kruskal_wallis(spec: &KruskalWallisSpec, inputs: &[Input<'_>]) -> Result<Output, AnalysisError> {
    check_alpha(spec.alpha)?;
    let groups: Vec<&[f64]> = inputs.iter().map(Input::ys).collect();
    let r = calc::kruskal_wallis(&groups).map_err(|e| AnalysisError::BadOption(e.to_string()))?;
    let mut table = ResultTable::new(&["N", "Mean rank", "Rank sum"]);
    for (inp, g) in inputs.iter().zip(&r.groups) {
        table.push_row(inp.label.clone(), vec![g.n as f64, g.mean_rank, g.rank_sum], vec![String::new(); 3]);
    }
    let mut on_sheet = vec![label_column("Group", inputs.iter().map(|i| i.label.clone()).collect())];
    for (i, h) in table.headers.iter().enumerate() {
        on_sheet.push(number_column(h, "", table.rows.iter().map(|(_, v)| v[i]).collect()));
    }
    on_sheet.push(label_column("Statistic", ["Chi-square", "DF", "Prob>Chi-square"].map(str::to_string).to_vec()));
    on_sheet.push(number_column("Value", "", vec![r.chi_square, r.df, r.p]));
    let notes = vec![format!(
        "Kruskal-Wallis test of {} groups ({} numbers): chi-square({}) = {}, p = {}; the groups {} at the {} level.",
        r.groups.len(),
        r.n,
        num(r.df),
        num(r.chi_square),
        num(r.p),
        if r.p < spec.alpha { "differ significantly" } else { "do not differ significantly" },
        num(spec.alpha)
    )];
    Ok(Output { table: Some(table), on_sheet, notes, plots: Vec::new(), comparison_pairs: Vec::new() })
}

// ----- sign test, two-sample Kolmogorov-Smirnov, median test, Friedman ----------------------------------------

fn exactly_two<'a, 'b>(name: &str, inputs: &'a [Input<'b>]) -> Result<(&'a Input<'b>, &'a Input<'b>), AnalysisError> {
    if inputs.len() == 2 {
        Ok((&inputs[0], &inputs[1]))
    } else {
        Err(AnalysisError::BadOption(format!("the {name} needs exactly two columns, {} were chosen", inputs.len())))
    }
}

/// The table of a test with a statistics block: one row per group, then a text and a number column with the
/// statistics.
fn group_sheet(
    inputs: &[Input<'_>],
    table: &ResultTable,
    group_header: &str,
    stat_names: &[&str],
    values: Vec<f64>,
) -> Vec<SheetColumn> {
    let mut on_sheet = vec![label_column(group_header, inputs.iter().map(|i| i.label.clone()).collect())];
    for (i, h) in table.headers.iter().enumerate() {
        on_sheet.push(number_column(h, "", table.rows.iter().map(|(_, v)| v[i]).collect()));
    }
    on_sheet.push(label_column("Statistic", stat_names.iter().map(|s| s.to_string()).collect()));
    on_sheet.push(number_column("Value", "", values));
    on_sheet
}

fn sign_test(spec: &SignTestSpec, inputs: &[Input<'_>]) -> Result<Output, AnalysisError> {
    check_alpha(spec.alpha)?;
    let (a, b) = exactly_two("sign test", inputs)?;
    let r = calc::sign_test(a.ys(), b.ys(), spec.tail).ok_or_else(|| {
        AnalysisError::BadOption("the sign test needs at least two rows with a number in both columns".to_string())
    })?;
    let mut table = ResultTable::new(&[
        "Pairs",
        "First < second",
        "First > second",
        "Ties",
        "N",
        "Statistic",
        "Z",
        "Prob",
        "Reject",
    ]);
    table.push_row(
        format!("{} vs {}", a.name, b.name),
        vec![
            r.pairs as f64,
            r.first_smaller as f64,
            r.second_smaller as f64,
            r.ties as f64,
            r.n as f64,
            r.statistic,
            r.z.unwrap_or(f64::NAN),
            r.p,
            f64::from(u8::from(r.p < spec.alpha)),
        ],
        vec![String::new(); 9],
    );
    let claim = match spec.tail {
        Tail::Two => format!("{} and {} differ in median", a.name, b.name),
        Tail::Upper => format!("{} tends to be larger than {}", a.name, b.name),
        Tail::Lower => format!("{} tends to be smaller than {}", a.name, b.name),
    };
    let how = if r.z.is_some() { "normal approximation" } else { "exact" };
    let notes = vec![format!(
        "{} vs {}: sign test, {}: {claim} ({} of {} differing pairs have the first smaller, {how} p = {}); {}",
        a.name,
        b.name,
        tail_name(spec.tail),
        r.first_smaller,
        r.n,
        num(r.p),
        reject_words(r.p, spec.alpha)
    )];
    Ok(Output { table: Some(table), on_sheet: Vec::new(), notes, plots: Vec::new(), comparison_pairs: Vec::new() })
}

fn ks_two_sample(spec: &KsTwoSampleSpec, inputs: &[Input<'_>]) -> Result<Output, AnalysisError> {
    check_alpha(spec.alpha)?;
    let (a, b) = exactly_two("Kolmogorov-Smirnov test", inputs)?;
    let r = calc::ks_two_sample(a.ys(), b.ys(), spec.tail).ok_or_else(|| {
        AnalysisError::BadOption("each column needs at least two numbers in the chosen rows".to_string())
    })?;
    let mut table = ResultTable::new(&["N1", "N2", "D", "Z", "Prob", "Reject"]);
    table.push_row(
        format!("{} vs {}", a.name, b.name),
        vec![r.n1 as f64, r.n2 as f64, r.d, r.z, r.p, f64::from(u8::from(r.p < spec.alpha))],
        vec![String::new(); 6],
    );
    let claim = match spec.tail {
        Tail::Two => format!("the distributions of {} and {} differ", a.name, b.name),
        Tail::Upper => format!("the distribution function of {} lies above that of {}", a.name, b.name),
        Tail::Lower => format!("the distribution function of {} lies below that of {}", a.name, b.name),
    };
    let how = if r.exact { "exact" } else { "asymptotic" };
    let notes = vec![format!(
        "{} vs {}: Kolmogorov-Smirnov test, {}: {claim} (D = {}, z = {}, {how} p = {}); {}",
        a.name,
        b.name,
        tail_name(spec.tail),
        num(r.d),
        num(r.z),
        num(r.p),
        reject_words(r.p, spec.alpha)
    )];
    Ok(Output { table: Some(table), on_sheet: Vec::new(), notes, plots: Vec::new(), comparison_pairs: Vec::new() })
}

fn median_test(spec: &MedianTestSpec, inputs: &[Input<'_>]) -> Result<Output, AnalysisError> {
    check_alpha(spec.alpha)?;
    let groups: Vec<&[f64]> = inputs.iter().map(Input::ys).collect();
    let r = calc::median_test(&groups).ok_or_else(|| {
        AnalysisError::BadOption("the median test needs at least two columns that each have a number".to_string())
    })?;
    let mut table = ResultTable::new(&["N", "At or below median", "Above median"]);
    for (inp, g) in inputs.iter().zip(&r.groups) {
        table.push_row(
            inp.label.clone(),
            vec![g.n as f64, g.at_or_below as f64, g.above as f64],
            vec![String::new(); 3],
        );
    }
    let on_sheet = group_sheet(
        inputs,
        &table,
        "Group",
        &["Median", "Chi-square", "DF", "Prob>Chi-square"],
        vec![r.median, r.chi_square, r.df, r.p],
    );
    let notes = vec![format!(
        "Median test of {} groups (overall median {}): chi-square({}) = {}, p = {}; the medians {} at the {} level.",
        r.groups.len(),
        num(r.median),
        num(r.df),
        num(r.chi_square),
        num(r.p),
        if r.p < spec.alpha { "differ significantly" } else { "do not differ significantly" },
        num(spec.alpha)
    )];
    Ok(Output { table: Some(table), on_sheet, notes, plots: Vec::new(), comparison_pairs: Vec::new() })
}

fn friedman(spec: &FriedmanSpec, inputs: &[Input<'_>]) -> Result<Output, AnalysisError> {
    check_alpha(spec.alpha)?;
    let cols: Vec<&[f64]> = inputs.iter().map(Input::ys).collect();
    let r = calc::friedman(&cols).map_err(|e| AnalysisError::BadOption(e.to_string()))?;
    let mut table = ResultTable::new(&["N", "Mean rank", "Rank sum"]);
    for (inp, g) in inputs.iter().zip(&r.groups) {
        table.push_row(inp.label.clone(), vec![r.n as f64, g.mean_rank, g.rank_sum], vec![String::new(); 3]);
    }
    let on_sheet = group_sheet(
        inputs,
        &table,
        "Treatment",
        &["Chi-square", "DF", "Prob>Chi-square"],
        vec![r.chi_square, r.df, r.p],
    );
    let notes = vec![format!(
        "Friedman ANOVA of {} treatments in {} rows: chi-square({}) = {}, p = {}; the treatments {} at the {} level.",
        r.groups.len(),
        r.n,
        num(r.df),
        num(r.chi_square),
        num(r.p),
        if r.p < spec.alpha { "differ significantly" } else { "do not differ significantly" },
        num(spec.alpha)
    )];
    Ok(Output { table: Some(table), on_sheet, notes, plots: Vec::new(), comparison_pairs: Vec::new() })
}

// ----- outlier tests ----------------------------------------------------------------------------------------

fn outlier_sentence(
    label: &str,
    test: &str,
    alpha: f64,
    significant: bool,
    side: calc::OutlierSide,
    value: f64,
    row: usize,
    statistic: (&str, f64),
    critical: f64,
) -> String {
    if significant {
        format!(
            "{label}: {test}, at the {} level the {} value {} (row {row}) is a significant outlier ({} = {}, critical value {}).",
            num(alpha),
            side.word(),
            num(value),
            statistic.0,
            num(statistic.1),
            num(critical)
        )
    } else {
        format!(
            "{label}: {test}, at the {} level there is no significant outlier (largest {} = {} at the {} value {}, critical value {}).",
            num(alpha),
            statistic.0,
            num(statistic.1),
            side.word(),
            num(value),
            num(critical)
        )
    }
}

fn grubbs(spec: &OutlierSpec, inputs: &[Input<'_>]) -> Result<Output, AnalysisError> {
    check_alpha(spec.alpha)?;
    let mut table =
        ResultTable::new(&["N", "Suspect value", "Row", "G", "Critical value", "Prob (approximate)", "Outlier"]);
    let mut notes = Vec::new();
    for inp in inputs {
        let r =
            calc::grubbs(inp.ys(), spec.alpha).map_err(|e| AnalysisError::BadOption(format!("{}: {e}", inp.label)))?;
        let row = inp.first + r.index;
        let mut units = vec![String::new(); 7];
        units[1] = inp.units.clone();
        table.push_row(
            inp.label.clone(),
            vec![r.n as f64, r.value, row as f64, r.g, r.critical, r.p, f64::from(u8::from(r.significant))],
            units,
        );
        notes.push(outlier_sentence(
            &inp.label,
            "Grubbs test",
            spec.alpha,
            r.significant,
            r.side,
            r.value,
            row,
            ("G", r.g),
            r.critical,
        ));
    }
    Ok(Output { table: Some(table), on_sheet: Vec::new(), notes, plots: Vec::new(), comparison_pairs: Vec::new() })
}

fn dixon(spec: &OutlierSpec, inputs: &[Input<'_>]) -> Result<Output, AnalysisError> {
    let mut table = ResultTable::new(&["N", "Suspect value", "Row", "Q", "Critical Q", "Outlier"]);
    let mut notes = Vec::new();
    for inp in inputs {
        let r = calc::dixon_q(inp.ys(), spec.alpha).map_err(|e| match e {
            calc::OutlierError::BadAlpha => AnalysisError::BadOption(
                "Dixon's Q test has critical values for the levels 0.1, 0.05 and 0.01 only".to_string(),
            ),
            e => AnalysisError::BadOption(format!("{}: {e}", inp.label)),
        })?;
        let row = inp.first + r.index;
        let mut units = vec![String::new(); 6];
        units[1] = inp.units.clone();
        table.push_row(
            inp.label.clone(),
            vec![r.n as f64, r.value, row as f64, r.q, r.critical, f64::from(u8::from(r.significant))],
            units,
        );
        notes.push(outlier_sentence(
            &inp.label,
            "Dixon's Q test",
            spec.alpha,
            r.significant,
            r.side,
            r.value,
            row,
            ("Q", r.q),
            r.critical,
        ));
    }
    Ok(Output { table: Some(table), on_sheet: Vec::new(), notes, plots: Vec::new(), comparison_pairs: Vec::new() })
}

// ----- correlation coefficients ---------------------------------------------------------------------------

fn correlation(spec: &CorrelationSpec, inputs: &[Input<'_>]) -> Result<Output, AnalysisError> {
    check_alpha(spec.alpha)?;
    if inputs.len() < 2 {
        return Err(AnalysisError::BadOption("the correlation coefficients need at least two columns".into()));
    }
    let kinds: Vec<calc::CorrKind> = [
        (spec.pearson, calc::CorrKind::Pearson),
        (spec.spearman, calc::CorrKind::Spearman),
        (spec.kendall, calc::CorrKind::Kendall),
    ]
    .into_iter()
    .filter_map(|(on, kind)| on.then_some(kind))
    .collect();
    if kinds.is_empty() {
        return Err(AnalysisError::BadOption("choose at least one of Pearson, Spearman and Kendall".into()));
    }
    let columns: Vec<&[f64]> = inputs.iter().map(Input::ys).collect();
    let k = inputs.len();
    let names: Vec<String> = inputs.iter().map(|i| i.name.clone()).collect();
    let matrices: Vec<calc::CorrMatrix> =
        kinds.iter().map(|&kind| calc::correlation_matrix(&columns, kind, spec.listwise)).collect();

    // Descriptive statistics of each column.
    let described: Vec<calc::Stats> = columns.iter().map(|c| calc::stats(c)).collect();
    let mut on_sheet = vec![
        label_column("Variable", inputs.iter().map(|i| i.label.clone()).collect()),
        number_column("N", "", described.iter().map(|s| s.n as f64).collect()),
        number_column("Mean", "", described.iter().map(|s| s.mean).collect()),
        number_column("SD", "", described.iter().map(|s| s.sd).collect()),
        number_column("Sum", "", described.iter().map(|s| s.sum).collect()),
        number_column("Minimum", "", described.iter().map(|s| s.min).collect()),
        number_column("Maximum", "", described.iter().map(|s| s.max).collect()),
    ];
    // One block per coefficient: the matrix of r (the variable names down the side), the matrix of p-values and,
    // where missing values leave different numbers of pairs, the matrix of pair counts.
    for m in &matrices {
        on_sheet.push(label_column(&format!("{} coefficient", m.kind.label()), names.clone()));
        for (j, name) in names.iter().enumerate() {
            on_sheet.push(number_column(name, "", (0..k).map(|i| m.get(i, j).r).collect()));
        }
        for (j, name) in names.iter().enumerate() {
            on_sheet.push(number_column(&format!("Prob ({name})"), "", (0..k).map(|i| m.get(i, j).p).collect()));
        }
        let first = m.get(0, 1).n;
        if (0..k).any(|i| (0..k).any(|j| i != j && m.get(i, j).n != first)) {
            for (j, name) in names.iter().enumerate() {
                on_sheet.push(number_column(
                    &format!("N ({name})"),
                    "",
                    (0..k).map(|i| m.get(i, j).n as f64).collect(),
                ));
            }
        }
    }

    // The table of pairs, for the report and the command line.
    let mut headers = vec!["N".to_string()];
    for kind in &kinds {
        let symbol = match kind {
            calc::CorrKind::Pearson => "r",
            calc::CorrKind::Spearman => "rho",
            calc::CorrKind::Kendall => "tau",
        };
        headers.push(format!("{} {symbol}", kind.label()));
        headers.push(format!("{} Prob", kind.label()));
    }
    let mut table = ResultTable { headers, ..Default::default() };
    let mut significant: Vec<Vec<String>> = vec![Vec::new(); kinds.len()];
    for i in 0..k {
        for j in i + 1..k {
            let mut values = vec![matrices[0].get(i, j).n as f64];
            for (m, sig) in matrices.iter().zip(significant.iter_mut()) {
                let c = m.get(i, j);
                values.push(c.r);
                values.push(c.p);
                if c.p < spec.alpha {
                    sig.push(format!("{} and {} ({})", names[i], names[j], num(c.r)));
                }
            }
            let count = values.len();
            table.push_row(format!("{} vs {}", names[i], names[j]), values, vec![String::new(); count]);
        }
    }
    let mut notes = Vec::new();
    for (kind, sig) in kinds.iter().zip(&significant) {
        notes.push(if sig.is_empty() {
            format!("{}: no pair is significantly correlated at the {} level.", kind.label(), num(spec.alpha))
        } else {
            format!(
                "{}: significant correlation at the {} level between {}.",
                kind.label(),
                num(spec.alpha),
                sig.join(", ")
            )
        });
    }
    notes.push(format!(
        "Missing values are left out {}.",
        if spec.listwise { "row by row over all columns (listwise)" } else { "pair by pair (pairwise)" }
    ));
    Ok(Output { table: Some(table), on_sheet, notes, plots: Vec::new(), comparison_pairs: Vec::new() })
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

    /// The points of Origin's `xy_count` reference run (`crates/calc/tests/fixtures/bin2d_origin.rs`).
    fn binning_project() -> (Project, ColumnId, ColumnId) {
        let x = vec![0.5, 1.5, 2.5, 3.5, 4.5, 5.5, 6.5, 7.5, 8.5, 9.5, 1.2, 1.7, 2.2, 8.1, 8.8, 10.0, 0.0];
        let y = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 2.2, 2.7, 9.1, 0.4, 3.3, 5.0, 0.0];
        let mut p = Project::new();
        let book = p.add_workbook("bins");
        let a = p.add_column(book, 0, Designation::X, ColumnData::numeric(x)).unwrap();
        let b = p.add_column(book, 0, Designation::Y, ColumnData::numeric(y)).unwrap();
        (p, a, b)
    }

    #[test]
    fn variance_tests_give_origin_s_numbers_and_the_levels_asked() {
        let (mut p, _, ids) = project();
        let one = VarTestSpec {
            kind: VarTestKind::OneSample { variance: 2.0 },
            intervals: true,
            levels: vec![95.0, 99.0],
            ..Default::default()
        };
        let r = run(&mut p, &ids[..1], Tool::VarTest(one.clone()));
        assert!(r.message.contains("One-sample test for variance") && r.message.contains("VarTest1"), "{}", r.message);
        let close = |got: Vec<f64>, want: &[f64]| {
            assert_eq!(got.len(), want.len(), "{got:?}");
            for (g, w) in got.iter().zip(want) {
                assert!((g - w).abs() <= 1e-7 * w.abs(), "{got:?} {want:?}");
            }
        };
        close(cell(&p, &r, "Chi-Square"), &[6.378_838_199_014_966]);
        close(cell(&p, &r, "Prob>Chi-Square"), &[0.992_549_330_449_127_7]);
        close(cell(&p, &r, "Conf. Levels in %"), &[95.0, 99.0]);
        // Origin's report for the two-sided test: the limits of the 95 and 99 % intervals.
        let (lo, hi) = (cell(&p, &r, "Lower Limits"), cell(&p, &r, "Upper Limits"));
        close(lo[..1].to_vec(), &[0.796_719_176_361_770_8]);
        close(hi[..1].to_vec(), &[7.54950463751445]);
        assert!(lo[1] < lo[0] && hi[1] > hi[0]);
        assert!(r.notes.iter().any(|n| n.starts_with("Null Hypothesis: Variance = 2")), "{:?}", r.notes);
        assert!(r.notes.iter().any(|n| n.contains("NOT significantly different from the test variance(2)")));
        let plain = run(&mut p, &ids[..1], Tool::VarTest(VarTestSpec { intervals: false, ..one.clone() }));
        let ws = &p.workbook(plain.book).unwrap().sheets[plain.new_sheet.unwrap()];
        assert!(ws.columns.iter().all(|c| c.long_name != "Lower Limits"));
        // A lower-tailed test names its probability Prob<Chi-Square and has no lower limit.
        let low = run(&mut p, &ids[..1], Tool::VarTest(VarTestSpec { tail: Tail::Lower, ..one }));
        assert_eq!(cell(&p, &low, "Lower Limits"), [0.0, 0.0]);
        assert_eq!(cell(&p, &low, "Prob<Chi-Square").len(), 1);

        let two = VarTestSpec { kind: VarTestKind::TwoSample { ratio: 1.0 }, ..Default::default() };
        let r = run(&mut p, &ids[..2], Tool::VarTest(two.clone()));
        close(cell(&p, &r, "F"), &[0.453_230_229_143_458_5]);
        close(cell(&p, &r, "Prob > F"), &[0.309_348_277_840_741_8]);
        close(cell(&p, &r, "Numer. DF"), &[7.0]);
        close(cell(&p, &r, "Denom. DF"), &[9.0]);
        let said = |notes: &[String], text: &str| notes.iter().any(|n| n == text);
        assert!(
            said(&r.notes, "At the 0.05 level, the two population variances are NOT significantly different."),
            "{:?}",
            r.notes
        );
        // Origin's one-sided wording names the test ratio and keeps "Prob > F" for the lower tail.
        let up = run(&mut p, &ids[..2], Tool::VarTest(VarTestSpec { tail: Tail::Upper, ..two.clone() }));
        assert!(
            said(
                &up.notes,
                "At the 0.05 level, the ratio of two population variances is NOT significantly greater than 1."
            ),
            "{:?}",
            up.notes
        );
        let lo = run(&mut p, &ids[..2], Tool::VarTest(VarTestSpec { tail: Tail::Lower, ..two.clone() }));
        assert!(
            said(
                &lo.notes,
                "At the 0.05 level, the ratio of two population variances is NOT significantly less than 1."
            ),
            "{:?}",
            lo.notes
        );
        close(cell(&p, &lo, "Prob > F"), &[0.154_674_138_920_370_9]);
        assert!(p.analyze(&request(&ids, Tool::VarTest(two))).is_err());
        let bad = VarTestSpec { levels: vec![150.0], intervals: true, ..Default::default() };
        assert!(p.analyze(&request(&ids[..1], Tool::VarTest(bad))).is_err());
    }

    #[test]
    fn proportion_tests_need_no_data_and_show_the_methods_ticked() {
        let (mut p, _, ids) = project();
        let spec = PropTestSpec {
            kind: PropTestKind::OneSample { successes: 7, size: 10, proportion: 0.5 },
            intervals: true,
            exact: true,
            ..Default::default()
        };
        let r = run(&mut p, &ids[..1], Tool::PropTest(spec.clone()));
        assert!(r.message.contains("PropTest1"), "{}", r.message);
        assert!((cell(&p, &r, "Exact P")[0] - 0.34375).abs() < 1e-12);
        assert!((cell(&p, &r, "Z")[0] - 1.2649110640673518).abs() < 1e-12);
        assert_eq!(cell(&p, &r, "95% LCL").len(), 1);
        // Both methods: the normal section and the binomial section each have a table of levels.
        assert_eq!(cell(&p, &r, "Conf. Levels in %"), [90.0, 95.0, 99.0]);
        let ws = &p.workbook(r.book).unwrap().sheets[r.new_sheet.unwrap()];
        assert_eq!(ws.columns.iter().filter(|c| c.long_name == "Lower Limits").count(), 2);
        let two = PropTestSpec {
            kind: PropTestKind::TwoSample { first: (15, 40), second: (9, 35), difference: 0.0 },
            ..Default::default()
        };
        let r = run(&mut p, &ids[..1], Tool::PropTest(two));
        assert!(cell(&p, &r, "P")[0] > 0.0 && cell(&p, &r, "Estimate for difference").len() == 1);
        let none = PropTestSpec { normal: false, exact: false, ..spec.clone() };
        assert!(p.analyze(&request(&ids[..1], Tool::PropTest(none))).is_err());
        let over = PropTestSpec { kind: PropTestKind::OneSample { successes: 11, size: 10, proportion: 0.5 }, ..spec };
        assert!(p.analyze(&request(&ids[..1], Tool::PropTest(over))).is_err());
    }

    #[test]
    fn one_sample_proportion_report_has_a_powers_table() {
        let (mut p, _, ids) = project();
        let spec = PropTestSpec {
            kind: PropTestKind::OneSample { successes: 60, size: 100, proportion: 0.5 },
            power: PropPowerSpec { actual: true, hypothetical: true, ..Default::default() },
            ..Default::default()
        };
        let r = run(&mut p, &ids[..1], Tool::PropTest(spec));
        // Origin: the actual size first, then the typed ones (100 appears twice).
        assert_eq!(cell(&p, &r, "Sample Size"), [100.0, 50.0, 100.0, 200.0]);
        let want = [0.516323421027342, 0.28904912956061, 0.516323421027342, 0.812291820591136];
        for (g, w) in cell(&p, &r, "Power").iter().zip(want) {
            assert!((g - w).abs() < 1e-9, "{g} {w}");
        }
    }

    #[test]
    fn two_sample_proportion_report_has_fisher_and_power_sections() {
        let (mut p, _, ids) = project();
        let spec = PropTestSpec {
            kind: PropTestKind::TwoSample { first: (15, 40), second: (25, 35), difference: 0.0 },
            exact: true,
            intervals: true,
            pooled: false,
            power: PropPowerSpec { actual: true, hypothetical: true, alpha: 0.05, sizes: vec![50, 100, 200] },
            ..Default::default()
        };
        let r = run(&mut p, &ids[..1], Tool::PropTest(spec));
        // Numbers of the Origin run (docs/spec/statistics.md 21.4): z, Fisher's exact p and the power rows.
        assert!((cell(&p, &r, "Z")[0] + 3.1380014488579).abs() < 1e-9);
        assert!((cell(&p, &r, "Exact P")[0] - 0.0051489221075077).abs() < 1e-9);
        assert_eq!(cell(&p, &r, "Sample Size"), [40.0, 35.0, 50.0, 100.0, 200.0]);
        let power = cell(&p, &r, "Power");
        assert!((power[0] - 0.87616102837545).abs() < 1e-9 && (power[4] - 0.999999877623331).abs() < 1e-9);
        assert_eq!(cell(&p, &r, "Alpha"), [0.05; 5]);
        let ws = &p.workbook(r.book).unwrap().sheets[r.new_sheet.unwrap()];
        // The Fisher section repeats the sample summary and the interval tables.
        assert_eq!(ws.columns.iter().filter(|c| c.long_name == "Sample P").count(), 2);
        assert_eq!(ws.columns.iter().filter(|c| c.long_name == "Lower Limits").count(), 2);
        // Equal sizes give one actual-power row; with only the hypothetical ones ticked there are three rows.
        let equal = PropTestSpec {
            kind: PropTestKind::TwoSample { first: (30, 50), second: (20, 50), difference: 0.0 },
            power: PropPowerSpec { actual: true, ..Default::default() },
            ..Default::default()
        };
        let r = run(&mut p, &ids[..1], Tool::PropTest(equal.clone()));
        assert!((cell(&p, &r, "Power")[0] - 0.516323421027342).abs() < 1e-9);
        assert_eq!(cell(&p, &r, "Power").len(), 1);
        let hyp = PropTestSpec { power: PropPowerSpec { hypothetical: true, ..Default::default() }, ..equal.clone() };
        let r = run(&mut p, &ids[..1], Tool::PropTest(hyp));
        assert_eq!(cell(&p, &r, "Sample Size"), [50.0, 100.0, 200.0]);
        // Both samples all successes: no z and no p (Origin prints --).
        let flat = PropTestSpec {
            kind: PropTestKind::TwoSample { first: (10, 10), second: (8, 8), difference: 0.0 },
            ..Default::default()
        };
        let r = run(&mut p, &ids[..1], Tool::PropTest(flat));
        assert!(cell(&p, &r, "Z")[0].is_nan() && cell(&p, &r, "P")[0].is_nan());
        let bad_power =
            PropTestSpec { power: PropPowerSpec { actual: true, alpha: 1.5, ..Default::default() }, ..equal };
        assert!(p.analyze(&request(&ids[..1], Tool::PropTest(bad_power))).is_err());
    }

    #[test]
    fn huge_proportion_samples_use_the_normal_approximation_and_refuse_the_exact_test() {
        let (mut p, _, ids) = project();
        let big = PropTestSpec {
            kind: PropTestKind::OneSample { successes: 5, size: u64::MAX, proportion: 0.5 },
            ..Default::default()
        };
        let r = run(&mut p, &ids[..1], Tool::PropTest(big.clone()));
        assert_eq!(cell(&p, &r, "P").len(), 1);
        let exact = PropTestSpec { exact: true, ..big };
        assert!(p.analyze(&request(&ids[..1], Tool::PropTest(exact))).is_err());
        let two = PropTestSpec {
            kind: PropTestKind::TwoSample { first: (5, u64::MAX), second: (3, u64::MAX), difference: 0.0 },
            ..Default::default()
        };
        let r = run(&mut p, &ids[..1], Tool::PropTest(two.clone()));
        assert_eq!(cell(&p, &r, "P").len(), 1);
        let two_exact = PropTestSpec { exact: true, ..two };
        assert!(p.analyze(&request(&ids[..1], Tool::PropTest(two_exact))).is_err());
    }

    fn grid() -> Bin2dOptions {
        let axis = |w| AxisBins { min: Some(0.0), max: Some(10.0), size: BinSize::Width(w), ..Default::default() };
        Bin2dOptions { x: axis(2.0), y: axis(5.0), stat: Bin2dStat::Count }
    }

    #[test]
    fn two_d_binning_writes_origin_s_worksheet_form() {
        let (mut p, a, b) = binning_project();
        let r = run(&mut p, &[b], Tool::Binning2D(grid()));
        let ws = &p.workbook(r.book).unwrap().sheets[r.new_sheet.unwrap()];
        assert!(ws.name.starts_with("Binning2D"), "{}", ws.name);
        let names: Vec<&str> = ws.columns.iter().map(|c| c.long_name.as_str()).collect();
        assert_eq!(names, ["Bin Ends of A", "Count", "Count"]);
        assert_eq!(ws.columns[0].designation, Designation::X);
        assert_eq!(ws.columns[0].numeric().unwrap(), [2.0, 4.0, 6.0, 8.0, 10.0]);
        assert_eq!(ws.columns[1].comments, "0 - 5");
        assert_eq!(ws.columns[2].comments, "5 - 10");
        assert_eq!(ws.columns[1].numeric().unwrap(), [5.0, 2.0, 0.0, 0.0, 2.0]);
        assert_eq!(ws.columns[2].numeric().unwrap(), [0.0, 1.0, 2.0, 2.0, 1.0]);
        assert!(r.notes[0].contains("17 points") && r.notes[0].contains("2 outside"), "{:?}", r.notes);

        // The X and the Y column chosen together give the same grid; a long name is quoted as in Origin,
        // the mean leaves empty bins missing, and percentages are of the binned points with the unit %.
        p.column_mut(a).unwrap().long_name = "Potential".to_string();
        let mean = run(&mut p, &[a, b], Tool::Binning2D(Bin2dOptions { stat: Bin2dStat::Mean, ..grid() }));
        let ws = &p.workbook(mean.book).unwrap().sheets[mean.new_sheet.unwrap()];
        assert_eq!(ws.columns[0].long_name, "Bin Ends of \"Potential\"");
        assert_eq!(ws.columns[1].long_name, "Mean");
        let m = ws.columns[1].numeric().unwrap();
        assert!((m[0] - 1.58).abs() < 1e-12 && m[2].is_nan(), "{m:?}");
        let pct = run(&mut p, &[b], Tool::Binning2D(Bin2dOptions { stat: Bin2dStat::PercentFrequency, ..grid() }));
        let ws = &p.workbook(pct.book).unwrap().sheets[pct.new_sheet.unwrap()];
        assert_eq!(ws.columns[1].units, "%");
        assert!((ws.columns[1].numeric().unwrap()[0] - 100.0 / 3.0).abs() < 1e-12);
    }

    #[test]
    fn two_d_binning_with_centres_and_a_bad_range() {
        let (mut p, _, b) = binning_project();
        let mut spec = grid();
        spec.x.centers = true;
        let r = run(&mut p, &[b], Tool::Binning2D(spec));
        let ws = &p.workbook(r.book).unwrap().sheets[r.new_sheet.unwrap()];
        assert_eq!(ws.columns[0].long_name, "Bin Centers of A");
        assert_eq!(ws.columns[0].numeric().unwrap(), [0.0, 2.0, 4.0, 6.0, 8.0, 10.0]);
        spec.x.max = Some(-1.0);
        let err = p.analyze(&request(&[b], Tool::Binning2D(spec))).unwrap_err();
        assert!(err.to_string().contains("no 2D frequency counts"), "{err}");
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
    fn anova_dunnett_comparison() {
        let (mut p, _, ids) = project();
        let dunnett = run(
            &mut p,
            &ids,
            Tool::Anova(AnovaSpec { comparison: Some(MeansComparison::Dunnett), ..Default::default() }),
        );
        let ws = &p.workbook(dunnett.book).unwrap().sheets[dunnett.new_sheet.unwrap()];
        let labels = ws.columns.iter().find(|c| c.long_name == "Dunnett comparison").unwrap().text().unwrap().to_vec();
        assert_eq!(labels, ["B - A", "C - A"]);
        let (lcl, ucl, d) = (cell(&p, &dunnett, "LCL"), cell(&p, &dunnett, "UCL"), cell(&p, &dunnett, "Difference"));
        assert!(lcl[0] < d[0] && d[0] < ucl[0]);
        assert!(dunnett.notes.iter().any(|n| n.contains("against the control A")), "{:?}", dunnett.notes);
        // Another control group: C against A and B.
        let c3 = run(
            &mut p,
            &ids,
            Tool::Anova(AnovaSpec { comparison: Some(MeansComparison::Dunnett), control: 2, ..Default::default() }),
        );
        let ws = &p.workbook(c3.book).unwrap().sheets[c3.new_sheet.unwrap()];
        let labels = ws.columns.iter().find(|c| c.long_name == "Dunnett comparison").unwrap().text().unwrap().to_vec();
        assert_eq!(labels, ["A - C", "B - C"]);
        let bad = AnovaSpec { comparison: Some(MeansComparison::Dunnett), control: 3, ..Default::default() };
        assert!(p.analyze(&request(&ids, Tool::Anova(bad))).is_err());
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
    fn holm_comparison_has_alpha_column_and_no_interval_and_is_refused_by_the_two_way_anova() {
        let (mut p, _, ids) = project();
        let spec = AnovaSpec { comparison: Some(MeansComparison::HolmBonferroni), ..Default::default() };
        let r = run(&mut p, &ids, Tool::Anova(spec));
        let alpha = cell(&p, &r, "Alpha");
        assert!((alpha[0] - 0.05 / 3.0).abs() < 1e-12 && (alpha[2] - 0.05).abs() < 1e-12, "{alpha:?}");
        let ws = &p.workbook(r.book).unwrap().sheets[r.new_sheet.unwrap()];
        assert!(!ws.columns.iter().any(|c| c.long_name == "LCL"));
        let (mut p2, ids2) = two_way_project();
        let bad = TwoWayAnovaSpec { comparison: Some(MeansComparison::HolmSidak), ..Default::default() };
        assert!(p2.analyze(&request(&ids2, Tool::TwoWayAnova(bad))).is_err());
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

    // ----- two-way ANOVA, rank tests and correlation ---------------------------------------------------

    /// The unbalanced two-way design of the Origin reference runs (`tools/origin-ref/stats2-anova2.lines.txt`,
    /// case "unb"): factor A with levels 1 and 2, factor B with levels 1 to 3, 15 numbers, cells of 3, 2, 4, 1, 3, 2.
    const UNB_A: [i32; 15] = [1, 2, 2, 2, 1, 1, 1, 1, 2, 2, 2, 1, 1, 1, 1];
    const UNB_B: [i32; 15] = [1, 3, 2, 1, 3, 3, 2, 1, 3, 2, 2, 3, 3, 2, 1];
    const UNB_Y: [f64; 15] = [
        14.214, 14.2532, 16.6664, 16.5478, 12.8569, 12.067, 14.8418, 14.8812, 13.4589, 15.412, 17.199, 14.2347,
        11.9925, 13.4436, 14.5733,
    ];

    /// A project with the unbalanced design: A as text ("low", "high"), B as numbers, then the data.
    fn two_way_project() -> (Project, Vec<ColumnId>) {
        let mut p = Project::new();
        let book = p.add_workbook("design");
        let a: Vec<String> = UNB_A.iter().map(|&l| if l == 1 { "low" } else { "high" }.to_string()).collect();
        let b: Vec<f64> = UNB_B.iter().map(|&l| f64::from(l)).collect();
        let ids = vec![
            p.add_column(book, 0, Designation::Y, ColumnData::text(a)).unwrap(),
            p.add_column(book, 0, Designation::Y, ColumnData::numeric(b)).unwrap(),
            p.add_column(book, 0, Designation::Y, ColumnData::numeric(UNB_Y.to_vec())).unwrap(),
        ];
        (p, ids)
    }

    fn text_cells(p: &Project, r: &Report, name: &str) -> Vec<String> {
        let ws = &p.workbook(r.book).unwrap().sheets[r.new_sheet.unwrap()];
        let c = ws.columns.iter().find(|c| c.long_name == name).unwrap_or_else(|| panic!("no column {name}"));
        c.text().unwrap().to_vec()
    }

    #[test]
    fn two_way_anova_with_text_factors_gives_origins_type_three_table() {
        let (mut p, ids) = two_way_project();
        let r = run(&mut p, &ids, Tool::TwoWayAnova(TwoWayAnovaSpec::default()));
        assert!(r.message.contains("Two-way ANOVA") && r.message.contains("ANOVA2Way1"), "{}", r.message);
        assert_eq!(text_cells(&p, &r, "Source"), ["A", "B", "A*B", "Model", "Error", "Total"]);
        let ss = cell(&p, &r, "Sum of Squares");
        for (got, want) in ss.iter().zip([
            9.787795902880951,
            13.643401203574452,
            1.002130293422164,
            28.091550144833338,
            6.449822299166665,
            34.541372444000004,
        ]) {
            assert!((got - want).abs() < 1e-9, "{got} vs {want}");
        }
        assert_eq!(cell(&p, &r, "DF"), vec![1.0, 2.0, 2.0, 5.0, 9.0, 14.0]);
        let prob = cell(&p, &r, "Prob>F");
        assert!((prob[0] - 0.00495389138184954).abs() < 1e-7 && (prob[1] - 0.006015082727291787).abs() < 1e-7);
        // Level names come in the order they first appear; Tukey comparisons for A, B and the cells follow.
        let groups = text_cells(&p, &r, "Group");
        assert_eq!(&groups[..3], ["A: low", "A: high", "B: 1"]);
        let comparisons = text_cells(&p, &r, "Tukey comparison");
        assert_eq!(comparisons.len(), 1 + 3 + 15);
        assert_eq!(comparisons[0], "A: high - low");
        assert_eq!(cell(&p, &r, "Significant").len(), 19);
        assert!(r.notes.iter().any(|n| n.contains("type III")), "{:?}", r.notes);
        // One undo step per run is the app's job (Document::edit); here the sheet is new and the input untouched.
        assert_eq!(p.workbook(r.book).unwrap().sheets[0].columns.len(), 3);
    }

    #[test]
    fn two_way_anova_refuses_dunnett_and_an_x_range() {
        let (mut p, ids) = two_way_project();
        let err = p
            .analyze(&request(
                &ids,
                Tool::TwoWayAnova(TwoWayAnovaSpec { comparison: Some(MeansComparison::Dunnett), ..Default::default() }),
            ))
            .unwrap_err();
        assert!(err.to_string().contains("one-way ANOVA only"), "{err}");
        let limited = Request {
            columns: ids.clone(),
            rows: RowRange::XBetween(0.0, 1.0),
            tool: Tool::TwoWayAnova(TwoWayAnovaSpec::default()),
        };
        assert!(p.analyze(&limited).unwrap_err().to_string().contains("no X column to limit"));
    }

    #[test]
    fn two_way_anova_without_interaction_and_with_raw_layout() {
        let (mut p, ids) = two_way_project();
        let spec = TwoWayAnovaSpec { interaction: false, comparison: None, ..Default::default() };
        let r = run(&mut p, &ids, Tool::TwoWayAnova(spec));
        assert_eq!(text_cells(&p, &r, "Source"), ["A", "B", "Model", "Error", "Total"]);
        assert!((cell(&p, &r, "Sum of Squares")[0] - 9.749492240244503).abs() < 1e-9);
        assert!(
            !p.workbook(r.book).unwrap().sheets[r.new_sheet.unwrap()]
                .columns
                .iter()
                .any(|c| c.long_name == "Significant")
        );

        // Raw layout: one column per cell of a balanced 2 x 2 design, two numbers each.
        let mut q = Project::new();
        let book = q.add_workbook("raw");
        let cells = [[1.0, 2.0], [4.0, 5.0], [3.0, 5.0], [8.0, 9.0]];
        let columns: Vec<ColumnId> = cells
            .iter()
            .map(|c| q.add_column(book, 0, Designation::Y, ColumnData::numeric(c.to_vec())).unwrap())
            .collect();
        let levels = [("x", "u"), ("x", "v"), ("y", "u"), ("y", "v")].map(|(a, b)| (a.to_string(), b.to_string()));
        let spec = TwoWayAnovaSpec { layout: TwoWayLayout::Raw { levels: levels.to_vec() }, ..Default::default() };
        let r = run(&mut q, &columns, Tool::TwoWayAnova(spec.clone()));
        assert_eq!(
            text_cells(&q, &r, "Source"),
            ["Factor A", "Factor B", "Factor A*Factor B", "Model", "Error", "Total"]
        );
        assert_eq!(cell(&q, &r, "DF"), vec![1.0, 1.0, 1.0, 3.0, 4.0, 7.0]);
        // Wrong number of levels, and an X range, are refused.
        let bad = TwoWayAnovaSpec { layout: TwoWayLayout::Raw { levels: levels[..3].to_vec() }, ..Default::default() };
        assert!(q.analyze(&request(&columns, Tool::TwoWayAnova(bad))).is_err());
        let xr =
            Request { columns: columns.clone(), rows: RowRange::XBetween(0.0, 1.0), tool: Tool::TwoWayAnova(spec) };
        assert!(q.analyze(&xr).unwrap_err().to_string().contains("X column"));
    }

    #[test]
    fn two_way_anova_edge_cases_are_reported_not_panicked() {
        let (mut p, ids) = two_way_project();
        // Indexed data needs exactly three columns.
        let err = p.analyze(&request(&ids[..2], Tool::TwoWayAnova(TwoWayAnovaSpec::default()))).unwrap_err();
        assert!(err.to_string().contains("three columns"), "{err}");
        // A factor column in the data position (text) is not numeric.
        let swapped = [ids[2], ids[1], ids[0]];
        assert!(matches!(
            p.analyze(&request(&swapped, Tool::TwoWayAnova(TwoWayAnovaSpec::default()))),
            Err(AnalysisError::NotNumeric(_))
        ));
        // A row range that leaves one level of a factor with no data is refused with an explanation.
        let rows = Request {
            columns: ids.clone(),
            rows: RowRange::Rows(1, 3),
            tool: Tool::TwoWayAnova(TwoWayAnovaSpec::default()),
        };
        assert!(p.analyze(&rows).is_err());
        // Missing data and missing factor cells are skipped and counted.
        let mut q = Project::new();
        let book = q.add_workbook("gaps");
        let a = vec!["p", "p", "q", "q", "p", "q", "", "q"].into_iter().map(String::from).collect();
        let b = vec![1.0, 2.0, 1.0, 2.0, 1.0, 2.0, 1.0, 1.0];
        let y = vec![1.0, 3.0, 2.0, 5.0, 2.0, f64::NAN, 4.0, 3.0];
        let cols = vec![
            q.add_column(book, 0, Designation::Y, ColumnData::text(a)).unwrap(),
            q.add_column(book, 0, Designation::Y, ColumnData::numeric(b)).unwrap(),
            q.add_column(book, 0, Designation::Y, ColumnData::numeric(y)).unwrap(),
        ];
        let spec = TwoWayAnovaSpec { interaction: false, ..Default::default() };
        let r = run(&mut q, &cols, Tool::TwoWayAnova(spec));
        assert!(r.notes.iter().any(|n| n.contains("2 rows")), "{:?}", r.notes);
        // Constant data: sums of squares are zero and there are no F tests.
        let mut c = Project::new();
        let book = c.add_workbook("flat");
        let cols = vec![
            c.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![1.0, 1.0, 2.0, 2.0, 1.0, 2.0])).unwrap(),
            c.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![1.0, 2.0, 1.0, 2.0, 2.0, 1.0])).unwrap(),
            c.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![5.0; 6])).unwrap(),
        ];
        let r = run(&mut c, &cols, Tool::TwoWayAnova(TwoWayAnovaSpec { interaction: false, ..Default::default() }));
        assert!(cell(&c, &r, "F").iter().all(|f| f.is_nan()));
    }

    /// Samples of the Origin runs `mw1_*`, `sr1a_*` and `kw1`: 9 and 7 numbers with ties, unequal group sizes.
    fn tied_samples() -> (Project, Vec<ColumnId>) {
        let a = vec![1.1, 2.3, 2.3, 4.0, 5.5, 6.1, 7.2, 7.2, 9.9];
        let b = vec![2.2, 3.1, 4.0, 4.0, 8.8, 10.5, 11.0];
        let c = vec![3.3, 1.1, 5.0, 2.2, 8.0, 7.5, 1.0, 9.1, 4.4, 6.0, 12.5];
        let mut p = Project::new();
        let book = p.add_workbook("ranks");
        let ids = [a, b, c]
            .into_iter()
            .map(|v| p.add_column(book, 0, Designation::Y, ColumnData::numeric(v)).unwrap())
            .collect();
        (p, ids)
    }

    #[test]
    fn mann_whitney_tool_gives_origins_numbers_exact_and_asymptotic() {
        let (mut p, ids) = tied_samples();
        let spec = MannWhitneySpec { exact: true, ..Default::default() };
        let r = run(&mut p, &ids[..2], Tool::MannWhitney(spec));
        assert!(r.message.contains("Mann-Whitney") && r.message.contains("MannWhitney1"), "{}", r.message);
        assert_eq!(cell(&p, &r, "U"), vec![26.0]);
        assert_eq!(cell(&p, &r, "Rank sum 1"), vec![71.0]);
        assert!((cell(&p, &r, "Prob (exact)")[0] - 0.586188811188811).abs() < 1e-9);
        assert!((cell(&p, &r, "Prob (asymptotic)")[0] - 0.594998462238957).abs() < 1e-9);
        assert!((cell(&p, &r, "Z")[0] + 0.531606643912503).abs() < 1e-9);
        assert_eq!(cell(&p, &r, "Reject"), vec![0.0]);
        assert!(r.notes[0].contains("exact p"), "{:?}", r.notes);
        let asym = run(&mut p, &ids[..2], Tool::MannWhitney(MannWhitneySpec::default()));
        assert!(cell(&p, &asym, "Prob (exact)")[0].is_nan());
        let lower = MannWhitneySpec { tail: Tail::Lower, ..Default::default() };
        let l = run(&mut p, &ids[..2], Tool::MannWhitney(lower));
        assert!((cell(&p, &l, "Prob (asymptotic)")[0] - 0.297499231).abs() < 1e-8);
        let err = p.analyze(&request(&ids, Tool::MannWhitney(MannWhitneySpec::default()))).unwrap_err();
        assert!(err.to_string().contains("exactly two columns"), "{err}");
    }

    #[test]
    fn signed_rank_tool_one_sample_and_paired() {
        let (mut p, ids) = tied_samples();
        let one = SignedRankSpec { kind: SignedRankKind::OneSample { median: 3.0 }, ..Default::default() };
        let r = run(&mut p, &ids[..1], Tool::SignedRank(one));
        assert!(r.message.contains("One-sample Wilcoxon"), "{}", r.message);
        assert_eq!(cell(&p, &r, "Sum of positive ranks (T+)"), vec![38.0]);
        assert!((cell(&p, &r, "Prob (exact)")[0] - 0.06640625).abs() < 1e-12);
        assert!((cell(&p, &r, "Prob (asymptotic)")[0] - 0.0750477365274089).abs() < 1e-9);
        assert!((cell(&p, &r, "Z")[0] - 1.78017248729078).abs() < 1e-9);
        let paired = SignedRankSpec { kind: SignedRankKind::Paired, ..Default::default() };
        let two = run(&mut p, &ids[..2], Tool::SignedRank(paired));
        // Seven pairs by row position, one of them equal (4 and 4): six ranked differences.
        assert_eq!(cell(&p, &two, "N"), vec![6.0]);
        assert_eq!(cell(&p, &two, "Zero differences"), vec![1.0]);
        let err = p.analyze(&request(&ids, Tool::SignedRank(paired))).unwrap_err();
        assert!(err.to_string().contains("exactly two columns"), "{err}");
        // Every value equal to the median: nothing to rank, p = 1, and a sentence says so.
        let flat = p.add_column(r.book, 0, Designation::Y, ColumnData::numeric(vec![3.0; 4])).unwrap();
        let f = run(&mut p, &[flat], Tool::SignedRank(one));
        assert!(cell(&p, &f, "Z")[0].is_nan());
        assert!(f.notes.iter().any(|n| n.contains("nothing to rank")), "{:?}", f.notes);
    }

    #[test]
    fn kruskal_wallis_tool_gives_origins_numbers() {
        let (mut p, ids) = tied_samples();
        let r = run(&mut p, &ids, Tool::KruskalWallis(KruskalWallisSpec::default()));
        let value = cell(&p, &r, "Value");
        assert!((value[0] - 0.292865709022375).abs() < 1e-9);
        assert_eq!(value[1], 2.0);
        assert!((value[2] - 0.863783741648186).abs() < 1e-5);
        assert_eq!(cell(&p, &r, "Rank sum"), vec![119.5, 107.5, 151.0]);
        assert!(r.notes[0].contains("do not differ"), "{:?}", r.notes);
        assert!(p.analyze(&request(&ids[..1], Tool::KruskalWallis(KruskalWallisSpec::default()))).is_err());
    }

    #[test]
    fn sign_ks_median_and_friedman_tools_give_origins_numbers() {
        let (mut p, ids) = tied_samples();
        // Seven pairs by row position, one tie: the first number is smaller in the six others.
        let r = run(&mut p, &ids[..2], Tool::SignTest(SignTestSpec::default()));
        assert!(r.message.contains("Sign test"), "{}", r.message);
        assert_eq!(cell(&p, &r, "Pairs"), vec![7.0]);
        assert_eq!(cell(&p, &r, "N"), vec![6.0]);
        assert!(cell(&p, &r, "Z")[0].is_nan());
        let err = p.analyze(&request(&ids, Tool::SignTest(SignTestSpec::default()))).unwrap_err();
        assert!(err.to_string().contains("exactly two columns"), "{err}");
        // Origin kstest2: D = 20/63, Z = 0.62994, exact p = 8164/11440 (both columns have ties).
        let k = run(&mut p, &ids[..2], Tool::KsTwoSample(KsTwoSampleSpec::default()));
        assert!((cell(&p, &k, "D")[0] - 20.0 / 63.0).abs() < 1e-12);
        assert!((cell(&p, &k, "Prob")[0] - 0.713_636_363_636_363_6).abs() < 1e-9);
        assert!(k.notes[0].contains("exact p"), "{:?}", k.notes);
        // Origin friedman needs equal columns: the three tied samples have 9, 7 and 11 numbers.
        let err = p.analyze(&request(&ids, Tool::Friedman(FriedmanSpec::default()))).unwrap_err();
        assert!(err.to_string().contains("every column"), "{err}");
        let book = r.book;
        let cols: Vec<_> = [
            vec![1.1, 2.3, 2.3, 4.0, 5.5, 6.1, 7.2, 7.2, 9.9],
            vec![2.2, 3.1, 4.0, 4.0, 8.8, 10.5, 11.0, 12.5, 12.9],
            vec![3.3, 1.1, 5.0, 2.2, 8.0, 7.5, 1.0, 9.1, 4.4],
        ]
        .into_iter()
        .map(|v| p.add_column(book, 0, Designation::Y, ColumnData::numeric(v)).unwrap())
        .collect();
        // Origin mediantest of three columns: overall median 5, chi-square 1.18681, 2 df, p 0.55244.
        let m = run(&mut p, &cols, Tool::MedianTest(MedianTestSpec::default()));
        let value = cell(&p, &m, "Value");
        assert_eq!(value[0], 5.0);
        assert!((value[1] - 1.186_813_186_813_186_8).abs() < 1e-9);
        assert_eq!(value[2], 2.0);
        assert!((value[3] - 0.552_442_144_623_162_9).abs() < 1e-5);
        let f = run(&mut p, &cols, Tool::Friedman(FriedmanSpec::default()));
        assert_eq!(cell(&p, &f, "Rank sum"), vec![13.5, 24.5, 16.0]);
        let value = cell(&p, &f, "Value");
        assert!((value[0] - 7.388_888_888_888_889).abs() < 1e-9);
        assert!((value[2] - 0.024_861_261_638_607).abs() < 1e-6);
        assert!(f.notes[0].contains("differ significantly"), "{:?}", f.notes);
    }

    #[test]
    fn grubbs_and_dixon_tools_report_the_suspect_value_and_its_row() {
        let mut p = Project::new();
        let book = p.add_workbook("outliers");
        let data = vec![2.1, f64::NAN, 2.2, 2.6, 2.3, 2.5, 9.8, 2.2];
        let col = p.add_column(book, 0, Designation::Y, ColumnData::numeric(data)).unwrap();
        // Origin grubbs: G = 2.26335, critical value 2.01997, approximate p 2.2727e-6, the row of the 9.8 is 7.
        let g = run(&mut p, &[col], Tool::Grubbs(OutlierSpec::default()));
        assert!((cell(&p, &g, "G")[0] - 2.263_350_952_498_796_7).abs() < 1e-9);
        assert!((cell(&p, &g, "Critical value")[0] - 2.019_969_409_424_984).abs() < 2e-5);
        assert_eq!(cell(&p, &g, "Row"), vec![7.0]);
        assert_eq!(cell(&p, &g, "Outlier"), vec![1.0]);
        assert!(g.notes[0].contains("highest value 9.8 (row 7) is a significant outlier"), "{:?}", g.notes);
        // Seven numbers: inside Dixon's table. Origin: Q = 0.9 against 0.568 at 0.05 for [1, 1.1, 1.2, 1.15, 1.17, 1.12, 3].
        let q = p
            .add_column(book, 0, Designation::Y, ColumnData::numeric(vec![1.0, 1.1, 1.2, 1.15, 1.17, 1.12, 3.0]))
            .unwrap();
        let d = run(&mut p, &[q], Tool::Dixon(OutlierSpec::default()));
        assert!((cell(&p, &d, "Q")[0] - 0.9).abs() < 1e-12);
        assert_eq!(cell(&p, &d, "Critical Q"), vec![0.568]);
        assert_eq!(cell(&p, &d, "Row"), vec![7.0]);
        // Dixon refuses other levels and more than ten numbers.
        let err = p.analyze(&request(&[col], Tool::Dixon(OutlierSpec { alpha: 0.2 }))).unwrap_err();
        assert!(err.to_string().contains("0.1, 0.05 and 0.01"), "{err}");
        let long =
            p.add_column(book, 0, Designation::Y, ColumnData::numeric((0..12).map(f64::from).collect())).unwrap();
        let err = p.analyze(&request(&[long], Tool::Dixon(OutlierSpec::default()))).unwrap_err();
        assert!(err.to_string().contains("at most 10"), "{err}");
    }

    #[test]
    fn correlation_tool_writes_matrices_and_a_pair_table() {
        let (mut p, ids) = tied_samples();
        // Three columns of the same length, as the Origin run cc7 had; here simply the first 7 rows of each.
        let spec = CorrelationSpec { spearman: true, kendall: true, ..Default::default() };
        let r = p
            .analyze(&Request { columns: ids.clone(), rows: RowRange::Rows(1, 7), tool: Tool::Correlation(spec) })
            .unwrap();
        let t = r.table.as_ref().unwrap();
        assert_eq!(t.rows.len(), 3, "pairs of three columns");
        assert_eq!(t.headers[0], "N");
        assert!(t.headers.contains(&"Kendall tau".to_string()));
        assert_eq!(t.rows[0].1[0], 7.0);
        let ws = &p.workbook(r.book).unwrap().sheets[r.new_sheet.unwrap()];
        // The matrix of Pearson r: diagonal 1, symmetric.
        let col = |name: &str| {
            ws.columns.iter().filter(|c| c.long_name == name).map(|c| c.numeric().unwrap().to_vec()).collect::<Vec<_>>()
        };
        let (a, b) = (&col("A")[0], &col("B")[0]);
        assert_eq!(a[0], 1.0);
        assert!((a[1] - b[0]).abs() < 1e-15);
        let direct = sheaf_calc::correlation(
            &p.column(ids[0]).unwrap().numeric().unwrap()[..7],
            &p.column(ids[1]).unwrap().numeric().unwrap()[..7],
            sheaf_calc::CorrKind::Pearson,
        );
        assert!((a[1] - direct.r).abs() < 1e-15);
        assert!(r.notes.iter().any(|n| n.contains("pairwise")), "{:?}", r.notes);
        // Needs two columns and one coefficient.
        assert!(p.analyze(&request(&ids[..1], Tool::Correlation(CorrelationSpec::default()))).is_err());
        let none = CorrelationSpec { pearson: false, ..Default::default() };
        assert!(p.analyze(&request(&ids, Tool::Correlation(none))).is_err());
        // A constant column gives no coefficient and no panic.
        let flat = p.add_column(r.book, 0, Designation::Y, ColumnData::numeric(vec![2.0; 9])).unwrap();
        let f = run(
            &mut p,
            &[ids[0], flat],
            Tool::Correlation(CorrelationSpec { kendall: true, spearman: true, ..Default::default() }),
        );
        assert!(f.table.unwrap().rows[0].1[1].is_nan());
    }
}

#[cfg(test)]
mod disc_tests {
    use super::*;

    #[test]
    fn discrete_frequency_writes_origin_s_columns() {
        let mut p = Project::new();
        let book = p.add_workbook("cats");
        let texts = ["b", "a", "B", "c", "a", "", "d", ""].iter().map(|s| s.to_string()).collect();
        let t = p.add_column(book, 0, Designation::Y, ColumnData::text(texts)).unwrap();
        let n = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![3.0, f64::NAN, 3.0, 1.5])).unwrap();
        let spec = DiscreteFreqSpec { relative: true, cumulative: true, percent: true, ..Default::default() };
        let r = p
            .analyze(&Request { columns: vec![t, n], rows: RowRange::All, tool: Tool::DiscreteFrequency(spec) })
            .unwrap();
        let ws = &p.workbook(r.book).unwrap().sheets[r.new_sheet.unwrap()];
        assert!(ws.name.starts_with("DiscretFreq"), "{}", ws.name);
        let names: Vec<&str> = ws.columns.iter().map(|c| c.long_name.as_str()).collect();
        assert_eq!(names, ["Data", "Count", "Relative Frequency", "Cumulative Frequency"].repeat(2));
        assert_eq!(ws.columns[0].designation, Designation::X);
        assert_eq!(ws.columns[0].comments, "A");
        assert_eq!(ws.columns[4].comments, "B");
        assert_eq!(ws.columns[2].units, "%");
        // The trailing empty cell is not counted, the inner one is; "B" counts with "b".
        assert_eq!(ws.columns[0].text().unwrap(), ["a", "b", "", "c", "d"]);
        assert_eq!(ws.columns[1].numeric().unwrap(), [2.0, 2.0, 1.0, 1.0, 1.0]);
        assert!((ws.columns[2].numeric().unwrap()[0] - 2.0 / 7.0 * 100.0).abs() < 1e-12);
        assert_eq!(*ws.columns[3].numeric().unwrap().last().unwrap(), 100.0);
        let d = ws.columns[4].numeric().unwrap();
        assert_eq!(d[0], 3.0);
        assert!(d[1].is_nan() && d[2] == 1.5, "missing sorts before the numbers among equal counts: {d:?}");
        let none = DiscreteFreqSpec { count: false, ..Default::default() };
        let req = Request { columns: vec![t], rows: RowRange::All, tool: Tool::DiscreteFrequency(none) };
        assert!(p.analyze(&req).is_err());
    }
}
