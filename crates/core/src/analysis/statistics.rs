//! The statistics tools of `Project::analyze`: t-tests, one-way and two-way ANOVA, nonparametric tests,
//! correlation coefficients, normality tests and the histogram (frequency counts). The numerics are in `sheaf_calc`; this module arranges the chosen columns, writes the
//! result tables and the plain-English lines about them. What each tool reports follows Origin, see
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

// ----- two-way ANOVA ---------------------------------------------------------------------------------

/// A factor cell as text: the cell itself, or a number written without a trailing ".0". `None` when empty.
fn level_text(col: &Column, row: usize) -> Option<String> {
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
    Ok(Output { table: Some(table), on_sheet, notes, plots: Vec::new() })
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
    Ok(Output { table: Some(table), on_sheet: Vec::new(), notes, plots: Vec::new() })
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
    Ok(Output { table: Some(table), on_sheet: Vec::new(), notes, plots: Vec::new() })
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
    Ok(Output { table: Some(table), on_sheet, notes, plots: Vec::new() })
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
    Ok(Output { table: Some(table), on_sheet: Vec::new(), notes, plots: Vec::new() })
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
    Ok(Output { table: Some(table), on_sheet: Vec::new(), notes, plots: Vec::new() })
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
    Ok(Output { table: Some(table), on_sheet, notes, plots: Vec::new() })
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
    Ok(Output { table: Some(table), on_sheet, notes, plots: Vec::new() })
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
    Ok(Output { table: Some(table), on_sheet: Vec::new(), notes, plots: Vec::new() })
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
    Ok(Output { table: Some(table), on_sheet: Vec::new(), notes, plots: Vec::new() })
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
    Ok(Output { table: Some(table), on_sheet, notes, plots: Vec::new() })
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
