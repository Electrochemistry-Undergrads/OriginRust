//! Power and sample size (Origin's Statistics > Power and Sample Size): the power of a t test, the one-way ANOVA or
//! (T415, section 12.1) a proportion or variance test for a list of sample sizes, or the smallest sample sizes that reach a list of wanted powers. The
//! numerics are in `sheaf_calc::power`; the numbers were checked against Origin, see `docs/spec/statistics.md`
//! section 12. These tools need no worksheet data, so they do not go through `Project::analyze`.

use super::{AnalysisError, Report};
use crate::column::{ColumnData, Designation};
use crate::project::Project;
use serde::{Deserialize, Serialize};
use sheaf_calc as calc;

/// Which test the power belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PowerKind {
    OneSampleT,
    TwoSampleT,
    PairedT,
    /// One-way ANOVA.
    Anova,
    /// One proportion against a hypothesized one, normal approximation (T415).
    OneProportion,
    /// Two proportions, normal approximation (T415).
    TwoProportions,
    /// One variance against a hypothesized one, chi-square (T415).
    OneVariance,
    /// Two variances, F (T415).
    TwoVariances,
}

impl PowerKind {
    pub fn label(self) -> &'static str {
        match self {
            PowerKind::OneSampleT => "One-sample t-test",
            PowerKind::TwoSampleT => "Two-sample t-test",
            PowerKind::PairedT => "Paired t-test",
            PowerKind::Anova => "One-way ANOVA",
            PowerKind::OneProportion => "One-proportion test",
            PowerKind::TwoProportions => "Two-proportion test",
            PowerKind::OneVariance => "One-sample variance test",
            PowerKind::TwoVariances => "Two-sample variance test",
        }
    }

    /// The proportion or variance test of this kind, if it is one.
    pub fn rate_test(self) -> Option<calc::RateTest> {
        match self {
            PowerKind::OneProportion => Some(calc::RateTest::OneProportion),
            PowerKind::TwoProportions => Some(calc::RateTest::TwoProportions),
            PowerKind::OneVariance => Some(calc::RateTest::OneVariance),
            PowerKind::TwoVariances => Some(calc::RateTest::TwoVariances),
            _ => None,
        }
    }

    /// Whether the sample sizes are per group.
    pub fn per_group(self) -> bool {
        matches!(self, PowerKind::TwoSampleT | PowerKind::Anova | PowerKind::TwoProportions | PowerKind::TwoVariances)
    }
}

/// What to compute.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PowerGoal {
    /// The power for these sample sizes (per group for the two-sample test and the ANOVA).
    Power(Vec<f64>),
    /// The smallest sample sizes that reach these powers.
    SampleSize(Vec<f64>),
}

/// The inputs of a power analysis, as in Origin's dialog.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PowerSpec {
    pub kind: PowerKind,
    pub goal: PowerGoal,
    pub alpha: f64,
    /// t tests: two-sided (default) or one-sided (upper) test.
    pub two_sided: bool,
    /// One-sample: the mean; two-sample: the mean of the first group; paired: the mean difference; proportion and
    /// variance tests: the (first) proportion or variance.
    pub mean1: f64,
    /// One-sample: the hypothesized mean; two-sample: the mean of the second group; paired: 0; proportion and
    /// variance tests: the hypothesized or second proportion or variance.
    pub mean2: f64,
    /// Standard deviation (pooled for two samples, of the differences for paired samples, the root of the error
    /// mean square for the ANOVA).
    pub sd: f64,
    /// ANOVA: the number of groups.
    pub levels: f64,
    /// ANOVA: the corrected sum of squares of the group means.
    pub css: f64,
}

impl Default for PowerSpec {
    fn default() -> Self {
        PowerSpec {
            kind: PowerKind::TwoSampleT,
            goal: PowerGoal::Power(vec![50.0, 100.0, 200.0]),
            alpha: 0.05,
            two_sided: true,
            mean1: 0.0,
            mean2: 1.0,
            sd: 1.0,
            levels: 3.0,
            css: 1.0,
        }
    }
}

/// The result of a power analysis: sample sizes and powers in rows, and plain-English lines.
#[derive(Debug, Clone, PartialEq)]
pub struct PowerOutput {
    pub sizes: Vec<f64>,
    pub powers: Vec<f64>,
    pub notes: Vec<String>,
}

fn num(v: f64) -> String {
    if v.is_nan() {
        "--".to_string()
    } else {
        let t = format!("{v:.6}");
        t.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

fn error(e: calc::PowerError) -> AnalysisError {
    AnalysisError::BadOption(e.to_string())
}

/// A proportion or variance power analysis (T415).
fn rate_analysis(spec: &PowerSpec, test: calc::RateTest) -> Result<PowerOutput, AnalysisError> {
    let (a, b) = (spec.mean1, spec.mean2);
    let proportions = matches!(test, calc::RateTest::OneProportion | calc::RateTest::TwoProportions);
    if proportions && !(a > 0.0 && a < 1.0 && b > 0.0 && b < 1.0) {
        return Err(AnalysisError::BadOption("the proportions must lie between 0 and 1".into()));
    }
    if !proportions && !(a > 0.0 && b > 0.0 && a.is_finite() && b.is_finite()) {
        return Err(AnalysisError::BadOption("the variances must be above 0".into()));
    }
    let mut notes = vec![
        format!(
            "{}: significance level {}, {} test.",
            spec.kind.label(),
            num(spec.alpha),
            if spec.two_sided { "two-sided" } else { "one-sided" }
        ),
        match test {
            calc::RateTest::OneProportion => {
                format!("Proportion {} against {} (normal approximation).", num(a), num(b))
            }
            calc::RateTest::TwoProportions => {
                format!("Proportions {} and {} (normal approximation, unpooled).", num(a), num(b))
            }
            calc::RateTest::OneVariance => {
                format!("Variance {} against {} (ratio {}, chi-square test).", num(a), num(b), num(a / b))
            }
            calc::RateTest::TwoVariances => {
                format!("Variances {} and {} (ratio {}, F test).", num(a), num(b), num(a / b))
            }
        },
    ];
    let (sizes, powers) = match &spec.goal {
        PowerGoal::Power(sizes) => {
            let powers = calc::rate_powers(test, sizes, a, b, spec.alpha, spec.two_sided).map_err(error)?;
            (sizes.clone(), powers)
        }
        PowerGoal::SampleSize(powers) => {
            let found = calc::rate_sample_sizes(test, powers, a, b, spec.alpha, spec.two_sided).map_err(error)?;
            if let Some(l) = found.lowest {
                notes.push(format!("The power of the smallest sample (2 observations) is {}.", num(l)));
            }
            if found.below_lowest.iter().any(|b| *b) {
                notes.push("A wanted power that the smallest sample already reaches has no sample size.".to_string());
            }
            (found.sizes, powers.clone())
        }
    };
    notes.push(if spec.kind.per_group() {
        "Sample sizes are per group.".to_string()
    } else {
        "Sample sizes are the number of observations.".to_string()
    });
    Ok(PowerOutput { sizes, powers, notes })
}

/// Runs a power analysis without touching a project.
pub fn power_analysis(spec: &PowerSpec) -> Result<PowerOutput, AnalysisError> {
    if let Some(test) = spec.kind.rate_test() {
        return rate_analysis(spec, test);
    }
    let test = match spec.kind {
        PowerKind::OneSampleT => Some(calc::PowerTest::OneSample),
        PowerKind::TwoSampleT => Some(calc::PowerTest::TwoSample),
        PowerKind::PairedT => Some(calc::PowerTest::Paired),
        _ => None,
    };
    let effect = spec.mean2 - spec.mean1;
    let mut notes = vec![format!(
        "{}: significance level {}{}, standard deviation {}.",
        spec.kind.label(),
        num(spec.alpha),
        match (test, spec.two_sided) {
            (Some(_), true) => ", two-sided test",
            (Some(_), false) => ", one-sided test",
            (None, _) => "",
        },
        num(spec.sd)
    )];
    match test {
        Some(_) => {
            notes.push(format!("Means {} and {} (difference {}).", num(spec.mean1), num(spec.mean2), num(effect)))
        }
        None => {
            notes.push(format!("{} groups, corrected sum of squares of the means {}.", num(spec.levels), num(spec.css)))
        }
    }
    let (sizes, powers) = match &spec.goal {
        PowerGoal::Power(sizes) => {
            let powers = match test {
                Some(t) => calc::t_powers(t, sizes, effect, spec.sd, spec.alpha, spec.two_sided),
                None => calc::anova_powers(sizes, spec.levels, spec.css, spec.sd, spec.alpha),
            }
            .map_err(error)?;
            (sizes.clone(), powers)
        }
        PowerGoal::SampleSize(powers) => {
            let found = match test {
                Some(t) => calc::t_sample_sizes(t, powers, effect, spec.sd, spec.alpha, spec.two_sided),
                None => calc::anova_sample_sizes(powers, spec.levels, spec.css, spec.sd, spec.alpha),
            }
            .map_err(error)?;
            if let Some(l) = found.lowest {
                notes.push(format!("The power of the smallest sample (2 observations) is {}.", num(l)));
            }
            if found.below_lowest.iter().any(|b| *b) {
                notes.push(
                    "A wanted power that the smallest sample already reaches has no sample size (Origin reports an \
                     error and leaves the cell empty, or gives 2 for the two-sample test)."
                        .to_string(),
                );
            }
            (found.sizes, powers.clone())
        }
    };
    if matches!(spec.goal, PowerGoal::Power(_)) {
        notes.push(match test {
            Some(calc::PowerTest::TwoSample) | None => "Sample sizes are per group.".to_string(),
            _ => "Sample sizes are the number of observations (pairs).".to_string(),
        });
    }
    Ok(PowerOutput { sizes, powers, notes })
}

impl Project {
    /// Runs a power analysis and puts the result in a new workbook (`Power1`, ...).
    pub fn power_analysis(&mut self, spec: &PowerSpec) -> Result<Report, AnalysisError> {
        let out = power_analysis(spec)?;
        let book = self.add_workbook(&format!("Power and sample size: {}", spec.kind.label()));
        let name = match spec.goal {
            PowerGoal::Power(_) => "Power",
            PowerGoal::SampleSize(_) => "SampleSize",
        };
        for (title, values) in [("Sample Size", &out.sizes), ("Power", &out.powers)] {
            let id = self
                .add_column(book, 0, Designation::Y, ColumnData::numeric(values.clone()))
                .expect("the workbook was just added");
            if let Some(c) = self.column_mut(id) {
                c.long_name = title.to_string();
            }
        }
        if let Some(b) = self.workbook_mut(book) {
            b.sheets[0].name = name.to_string();
        }
        Ok(Report {
            book,
            sheet: 0,
            new_columns: Vec::new(),
            new_sheet: Some(0),
            new_graph: None,
            more_graphs: Vec::new(),
            table: None,
            message: format!(
                "{}: {} rows in workbook {}",
                spec.kind.label(),
                out.sizes.len(),
                self.workbook(book).map_or("", |b| &b.name)
            ),
            notes: out.notes,
        })
    }
}
