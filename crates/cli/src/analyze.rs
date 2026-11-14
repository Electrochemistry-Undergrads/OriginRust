//! `sheaf analyze`: run an analysis tool on worksheet columns without a window.

use crate::args::*;
use crate::commands::{
    is_project, load_project, named_columns, pick_book, pick_sheet, read_data_file, save_project, write_sheet,
};
use anyhow::{Result, anyhow, bail};
use sheaf_core::Project;
use sheaf_core::analysis::{self, BaselineSpec, IntegrateSpec, InterpSpec, Tool};
use sheaf_core::template::Template;

/// Reads "FROM:TO" as two numbers.
pub(crate) fn parse_pair<T: std::str::FromStr>(text: &str, what: &str, example: &str) -> Result<(T, T)> {
    let parsed = text.split_once(':').and_then(|(a, b)| Some((a.trim().parse().ok()?, b.trim().parse().ok()?)));
    parsed.ok_or_else(|| anyhow!("'{text}' is not a valid {what}; write it as FROM:TO, for example {example}"))
}

/// A number with six significant digits, without trailing zeros.
pub(crate) fn short_number(v: f64) -> String {
    if v.is_nan() {
        return "--".to_string();
    }
    if v == 0.0 || !v.is_finite() {
        return format!("{v}");
    }
    let magnitude = v.abs().log10().floor() as i32;
    if !(-5..=8).contains(&magnitude) {
        return format!("{v:.5e}");
    }
    let text = format!("{:.*}", (5 - magnitude).max(0) as usize, v);
    if text.contains('.') { text.trim_end_matches('0').trim_end_matches('.').to_string() } else { text }
}

fn print_table(t: &analysis::ResultTable) {
    let headers: Vec<String> = t
        .headers
        .iter()
        .enumerate()
        .map(|(i, h)| match t.units.get(i).map(String::as_str) {
            Some(u) if !u.is_empty() => format!("{h} ({u})"),
            _ => h.clone(),
        })
        .collect();
    // Where the rows have different units (A and mA columns), each number carries its own unit.
    let cells: Vec<Vec<String>> = t
        .rows
        .iter()
        .enumerate()
        .map(|(r, (_, v))| {
            v.iter()
                .enumerate()
                .map(|(c, x)| match t.unit(r, c) {
                    u if t.mixed_units(c) && !u.is_empty() => format!("{} {u}", short_number(*x)),
                    _ => short_number(*x),
                })
                .collect()
        })
        .collect();
    let label_width = t.rows.iter().map(|(l, _)| l.chars().count()).max().unwrap_or(0).max(4);
    let widths: Vec<usize> = headers
        .iter()
        .enumerate()
        .map(|(i, h)| cells.iter().map(|r| r[i].chars().count()).max().unwrap_or(0).max(h.chars().count()))
        .collect();
    let mut line = format!("{:<label_width$}", "Data");
    for (h, w) in headers.iter().zip(&widths) {
        line.push_str(&format!("  {h:>w$}"));
    }
    println!("{line}");
    for ((label, _), row) in t.rows.iter().zip(&cells) {
        let mut line = format!("{label:<label_width$}");
        for (c, w) in row.iter().zip(&widths) {
            line.push_str(&format!("  {c:>w$}"));
        }
        println!("{line}");
    }
}

/// What was asked for. Sorting and filtering change the sheet as a whole; every other tool
/// works on Y columns.
enum Job {
    Tool(Tool),
    /// Curve math; the reference column is looked up once the project is loaded.
    Math {
        op: MathKind,
        operand: OperandArgs,
        /// Simple Column Math: row by row, no X.
        column: bool,
    },
    /// Convolution or correlation; the second column is looked up once the project is loaded.
    Signal {
        conv: bool,
        other: String,
        circular: bool,
        normalize: bool,
        interval: Option<f64>,
    },
    Sort {
        by: String,
        descending: bool,
    },
    Filter {
        condition: String,
        in_place: bool,
    },
}

fn tail_of(tail: TailArg) -> analysis::Tail {
    match tail {
        TailArg::Two => analysis::Tail::Two,
        TailArg::Upper => analysis::Tail::Upper,
        TailArg::Lower => analysis::Tail::Lower,
    }
}

fn comparison_of(comparison: ComparisonArg) -> Option<analysis::MeansComparison> {
    use analysis::MeansComparison as M;
    match comparison {
        ComparisonArg::Tukey => Some(M::Tukey),
        ComparisonArg::Bonferroni => Some(M::Bonferroni),
        ComparisonArg::Sidak => Some(M::Sidak),
        ComparisonArg::Lsd => Some(M::FisherLsd),
        ComparisonArg::Scheffe => Some(M::Scheffe),
        ComparisonArg::HolmBonferroni => Some(M::HolmBonferroni),
        ComparisonArg::HolmSidak => Some(M::HolmSidak),
        ComparisonArg::None => None,
    }
}

fn job_from(tool: AnalyzeTool) -> Result<(TargetArgs, Job)> {
    Ok(match tool {
        AnalyzeTool::Integrate { target, baseline, absolute, no_cumulative, ranges, row_ranges } => {
            if ranges.is_empty() && row_ranges.is_empty() {
                (
                    target,
                    Job::Tool(Tool::Integrate(IntegrateSpec {
                        area_type: if absolute {
                            analysis::AreaType::Absolute
                        } else {
                            analysis::AreaType::Mathematical
                        },
                        end_point_baseline: baseline,
                        cumulative: !no_cumulative,
                    })),
                )
            } else {
                if absolute || no_cumulative {
                    bail!(
                        "--absolute and --no-cumulative do not apply to --ranges: the result table always \
                         has both the signed and the absolute area and no running integral"
                    );
                }
                let mut list = Vec::new();
                for text in &ranges {
                    let (a, b) = parse_pair::<f64>(text, "X range", "0.2:0.6")?;
                    list.push(analysis::RowRange::XBetween(a, b));
                }
                for text in &row_ranges {
                    let (a, b) = parse_pair::<usize>(text, "row range", "120:480")?;
                    list.push(analysis::RowRange::Rows(a, b));
                }
                (
                    target,
                    Job::Tool(Tool::IntegrateRanges(analysis::IntegrateRangesSpec {
                        ranges: list,
                        end_point_baseline: baseline,
                    })),
                )
            }
        }
        AnalyzeTool::SubtractRef { target, operand } => {
            if operand.reference.is_none() {
                bail!("subtract-ref needs the reference curve: --ref COLUMN (use 'math --value' for a constant)");
            }
            (target, Job::Math { op: MathKind::Subtract, operand, column: false })
        }
        AnalyzeTool::Math { target, op, operand } => (target, Job::Math { op, operand, column: false }),
        AnalyzeTool::ColumnMath { target, op, operand } => (target, Job::Math { op, operand, column: true }),
        AnalyzeTool::Hilbert { target } => (target, Job::Tool(Tool::Hilbert)),
        AnalyzeTool::Fft {
            target,
            interval,
            window,
            correction,
            normalize,
            two_sided,
            no_shift,
            no_unwrap,
            science,
            power,
        } => {
            let options = analysis::FftOptions {
                interval: interval.unwrap_or(1.0),
                window: match window {
                    FftWindowArg::Rectangle => analysis::FftWindow::Rectangle,
                    FftWindowArg::Welch => analysis::FftWindow::Welch,
                    FftWindowArg::Triangular => analysis::FftWindow::Triangular,
                    FftWindowArg::Bartlett => analysis::FftWindow::Bartlett,
                    FftWindowArg::Hanning => analysis::FftWindow::Hanning,
                    FftWindowArg::Hamming => analysis::FftWindow::Hamming,
                    FftWindowArg::Blackman => analysis::FftWindow::Blackman,
                },
                correction: match correction {
                    FftCorrectionArg::None => analysis::WindowCorrection::None,
                    FftCorrectionArg::Amplitude => analysis::WindowCorrection::Amplitude,
                    FftCorrectionArg::Power => analysis::WindowCorrection::Power,
                },
                normalize,
                two_sided,
                shift: !no_shift,
                unwrap: !no_unwrap,
                science,
                power: match power {
                    FftPowerArg::Msa => analysis::PowerNorm::Msa,
                    FftPowerArg::Ssa => analysis::PowerNorm::Ssa,
                    FftPowerArg::Tisa => analysis::PowerNorm::Tisa,
                },
            };
            (target, Job::Tool(Tool::Fft(analysis::FftSpec { options, auto_interval: interval.is_none() })))
        }
        AnalyzeTool::LevelCrossing { target, level, row, min_gap, details } => {
            let index = match row {
                CrossingRow::Any => analysis::CrossingIndex::Nearest,
                CrossingRow::Left => analysis::CrossingIndex::Left,
                CrossingRow::Right => analysis::CrossingIndex::Right,
            };
            (target, Job::Tool(Tool::LevelCrossing(analysis::LevelCrossingSpec { level, index, min_gap, details })))
        }
        AnalyzeTool::Convolution { target, response, circular, normalize, interval } => {
            (target, Job::Signal { conv: true, other: response, circular, normalize, interval })
        }
        AnalyzeTool::SignalCorrelation { target, other, circular, normalize, interval } => {
            (target, Job::Signal { conv: false, other, circular, normalize, interval: Some(interval) })
        }
        AnalyzeTool::Differentiate { target, order, smooth, poly, points } => {
            (target, Job::Tool(Tool::Differentiate(analysis::DiffOptions { order, smooth, poly, npts: points })))
        }
        AnalyzeTool::Smooth { target, method, points, poly, percent, span, order, boundary } => {
            let method = match method {
                SmoothKind::SavitzkyGolay => analysis::SmoothMethod::SavitzkyGolay { poly_order: poly },
                SmoothKind::Average => analysis::SmoothMethod::AdjacentAveraging { weighted: false },
                SmoothKind::WeightedAverage => analysis::SmoothMethod::AdjacentAveraging { weighted: true },
                SmoothKind::Percentile => analysis::SmoothMethod::Percentile { percent },
                SmoothKind::Fft => analysis::SmoothMethod::FftFilter,
                SmoothKind::Lowess => analysis::SmoothMethod::Lowess { span: local_span(span) },
                SmoothKind::Loess => analysis::SmoothMethod::Loess { span: local_span(span) },
                SmoothKind::Binomial => analysis::SmoothMethod::Binomial { order },
            };
            let boundary = match boundary {
                BoundaryKind::None => analysis::Boundary::None,
                BoundaryKind::Reflect => analysis::Boundary::Reflect,
                BoundaryKind::Repeat => analysis::Boundary::Repeat,
                BoundaryKind::Periodic => analysis::Boundary::Periodic,
                BoundaryKind::Extrapolate => analysis::Boundary::Extrapolate,
            };
            (target, Job::Tool(Tool::Smooth(analysis::SmoothOptions { method, npts: points, boundary })))
        }
        AnalyzeTool::Normalize { target, method, value } => {
            use analysis::NormalizeMethod as M;
            let method = match method {
                NormalizeKind::Max => M::Max,
                NormalizeKind::Range => M::ZeroToOne,
                NormalizeKind::Range100 => M::ZeroToHundred,
                NormalizeKind::ZScore => M::ZScore,
                NormalizeKind::Min => M::Min,
                NormalizeKind::Mean => M::Mean,
                NormalizeKind::Median => M::Median,
                NormalizeKind::Sd => M::Sd,
                NormalizeKind::Norm => M::Norm,
                NormalizeKind::Mode => M::Mode,
                NormalizeKind::Sum => M::Sum,
                NormalizeKind::Divide => {
                    M::Divide(value.ok_or_else(|| anyhow!("--method divide needs the divisor: --value NUMBER"))?)
                }
            };
            (target, Job::Tool(Tool::Normalize(method)))
        }
        AnalyzeTool::Baseline { target, constant, end_points, line } => {
            let spec = match (constant, end_points, line.as_slice()) {
                (Some(level), _, _) => BaselineSpec::Constant(level),
                (None, true, _) => BaselineSpec::EndPoints,
                (None, false, [x1, y1, x2, y2]) => BaselineSpec::Line((*x1, *y1), (*x2, *y2)),
                (None, false, []) => BaselineSpec::Minimum,
                _ => bail!("--line needs four numbers: X1,Y1,X2,Y2"),
            };
            (target, Job::Tool(Tool::SubtractBaseline(spec)))
        }
        AnalyzeTool::Interpolate { target, method, points, x_min, x_max, smoothing } => {
            let method = match method {
                InterpKind::Linear => analysis::InterpMethod::Linear,
                InterpKind::Spline => analysis::InterpMethod::CubicSpline,
                InterpKind::Akima => analysis::InterpMethod::Akima,
                InterpKind::BSpline => analysis::InterpMethod::BSpline,
            };
            let range = match (x_min, x_max) {
                (None, None) => None,
                (Some(lo), Some(hi)) => Some((lo, hi)),
                _ => bail!("give both --x-min and --x-max, or neither"),
            };
            (
                target,
                Job::Tool(Tool::Interpolate(InterpSpec {
                    method,
                    npts: points,
                    range,
                    smoothing,
                    ..InterpSpec::default()
                })),
            )
        }
        AnalyzeTool::Average { target, x: which, points, reference_curve, method, sd, se, count, min, max } => {
            let evenly = points.map_or(analysis::CommonX::Auto, analysis::CommonX::Evenly);
            let (x, range) = match (which, reference_curve) {
                (_, Some(0)) => bail!("--reference-curve counts from 1"),
                (_, Some(r)) => (analysis::CommonX::Reference(r - 1), analysis::XRange::Intersection),
                (AverageX::Same, None) => (analysis::CommonX::Points, analysis::XRange::Intersection),
                (AverageX::Common, None) => (evenly, analysis::XRange::Intersection),
                (AverageX::Full, None) => (evenly, analysis::XRange::Full),
                (AverageX::Any, None) => (evenly, analysis::XRange::Union),
            };
            let options = analysis::AverageOptions {
                x,
                range,
                method: match method {
                    AverageInterp::Linear => analysis::AverageMethod::Linear,
                    AverageInterp::Spline => analysis::AverageMethod::CubicSpline,
                    AverageInterp::Akima => analysis::AverageMethod::Akima,
                },
            };
            (target, Job::Tool(Tool::AverageCurves(analysis::AverageSpec { options, sd, count, se, min, max })))
        }
        AnalyzeTool::Intersect { target, touching } => {
            (target, Job::Tool(Tool::Intersect(analysis::IntersectSpec { include_touching: touching })))
        }
        AnalyzeTool::PolygonArea { target } => (target, Job::Tool(Tool::PolygonArea)),
        AnalyzeTool::SubtractLine { target, x1, x2, y1, y2 } => (
            target,
            Job::Tool(Tool::SubtractLine(analysis::SubtractLineSpec {
                p1: analysis::LinePoint { x: x1, y: y1 },
                p2: analysis::LinePoint { x: x2, y: y2 },
            })),
        ),
        AnalyzeTool::ReduceDup { target, method, tol, count } => {
            use analysis::DupMethod as M;
            let method = match method {
                DupKind::Mean => M::Mean,
                DupKind::Median => M::Median,
                DupKind::Min => M::Min,
                DupKind::Max => M::Max,
                DupKind::Sum => M::Sum,
                DupKind::Sd => M::Sd,
                DupKind::Se => M::Se,
                DupKind::Rms => M::Rms,
            };
            (target, Job::Tool(Tool::ReduceDup(analysis::ReduceDupSpec { method, tol, count })))
        }
        AnalyzeTool::ReduceX { target, every, step, intervals, frequency, method, x_min, x_max, tolerance } => {
            let range = match (x_min, x_max) {
                (None, None) => None,
                (Some(lo), Some(hi)) => Some((lo, hi)),
                _ => bail!("give both --x-min and --x-max, or neither"),
            };
            let group = match (every, step, intervals, frequency) {
                (_, Some(s), _, _) => analysis::ReduceGroup::Interval(s),
                (_, _, Some(n), _) => analysis::ReduceGroup::Intervals(n),
                (_, _, _, Some(f)) => analysis::ReduceGroup::Frequency(f),
                (n, ..) => analysis::ReduceGroup::EveryN(n.unwrap_or(analysis::REDUCE_AUTO_FACTOR)),
            };
            let method = match method {
                ReduceKind::Mean => analysis::ReduceMethod::Mean,
                ReduceKind::First => analysis::ReduceMethod::First,
            };
            let opts = analysis::ReduceOptions { group, method, range, tolerance_pct: tolerance };
            (target, Job::Tool(Tool::ReduceX(opts)))
        }
        AnalyzeTool::Translate { target, axis, by, at, to } => {
            let shift = match (by, at, to) {
                (Some(d), _, _) => analysis::Shift::By(d),
                (None, Some(at), Some(to)) => analysis::Shift::PointTo { at, to },
                _ => bail!("give --by AMOUNT, or --at X together with --to VALUE"),
            };
            let axis = match axis {
                AxisKind::X => analysis::TranslateAxis::X,
                AxisKind::Y => analysis::TranslateAxis::Y,
            };
            (target, Job::Tool(Tool::Translate(analysis::TranslateSpec { axis, shift })))
        }
        AnalyzeTool::MedianFilter { target, points, edge } => {
            let edge = match edge {
                MedianEdgeKind::Shrink => analysis::MedianEdge::Shrink,
                MedianEdgeKind::Repeat => analysis::MedianEdge::Repeat,
                MedianEdgeKind::Reflect => analysis::MedianEdge::Reflect,
            };
            (target, Job::Tool(Tool::MedianFilter(analysis::MedianFilterSpec { npts: points, edge })))
        }
        AnalyzeTool::Peaks {
            target,
            direction,
            points,
            min_height,
            count,
            all,
            method,
            smooth,
            min_width,
            baseline,
            baseline_points,
            anchors,
            limits,
            integrate,
            fit,
            fit_baseline,
        } => {
            let direction = match direction {
                PeakKind::Both => analysis::PeakDirection::Both,
                PeakKind::Positive => analysis::PeakDirection::Positive,
                PeakKind::Negative => analysis::PeakDirection::Negative,
            };
            let filter = match (min_height, count, all) {
                (Some(h), _, _) => analysis::PeakFilter::Height(h),
                (None, Some(n), _) => analysis::PeakFilter::Number(n),
                (None, None, true) => analysis::PeakFilter::None,
                (None, None, false) => analysis::PeakFilter::Height(0.0),
            };
            let chain = integrate
                || fit.is_some()
                || method.is_some()
                || smooth > 0
                || min_width > 0.0
                || baseline.is_some()
                || anchors.is_some()
                || limits.is_some();
            if !chain {
                (
                    target,
                    Job::Tool(Tool::FindPeaks(analysis::PeakOptions {
                        npts: points,
                        direction,
                        filter,
                        ..Default::default()
                    })),
                )
            } else {
                let baseline = match (baseline, &anchors) {
                    (Some(PeakBaselineKind::Anchors), None) => bail!("--baseline anchors needs --anchors X1:Y1,X2:Y2"),
                    (None | Some(PeakBaselineKind::Anchors), Some(text)) => {
                        let mut pts = Vec::new();
                        for part in text.split(',') {
                            pts.push(parse_pair::<f64>(part, "anchor point", "1.5:0.02")?);
                        }
                        if pts.len() < 2 {
                            bail!("--anchors needs at least two points, for example 0:0.1,20:0.3");
                        }
                        analysis::ChainBaseline::Anchors(pts, analysis::InterpMethod::Linear)
                    }
                    (Some(PeakBaselineKind::None), Some(_)) => {
                        bail!("--anchors cannot be combined with --baseline none")
                    }
                    (Some(PeakBaselineKind::EndPoints), _) => analysis::ChainBaseline::EndPoints,
                    (Some(PeakBaselineKind::Minima), _) => analysis::ChainBaseline::AutoMinima(baseline_points),
                    (Some(PeakBaselineKind::Minimum), _) => {
                        analysis::ChainBaseline::ConstantStat(analysis::BaselineStat::Minimum)
                    }
                    (Some(PeakBaselineKind::Maximum), _) => {
                        analysis::ChainBaseline::ConstantStat(analysis::BaselineStat::Maximum)
                    }
                    (Some(PeakBaselineKind::Mean), _) => {
                        analysis::ChainBaseline::ConstantStat(analysis::BaselineStat::Mean)
                    }
                    (Some(PeakBaselineKind::Median), _) => {
                        analysis::ChainBaseline::ConstantStat(analysis::BaselineStat::Median)
                    }
                    (Some(PeakBaselineKind::None) | None, None) => analysis::ChainBaseline::None,
                };
                let method = match method.unwrap_or(PeakMethodKind::LocalMax) {
                    PeakMethodKind::LocalMax => analysis::FindMethod::LocalMax,
                    PeakMethodKind::Window => analysis::FindMethod::Window,
                    PeakMethodKind::FirstDerivative => analysis::FindMethod::FirstDerivative,
                    PeakMethodKind::SecondDerivative => analysis::FindMethod::SecondDerivative,
                };
                let limits = match limits.unwrap_or(PeakLimitKind::Baseline) {
                    PeakLimitKind::Baseline => analysis::LimitMode::Baseline,
                    PeakLimitKind::HalfHeight => analysis::LimitMode::HalfHeight,
                };
                let fit = fit.map(|shape| analysis::PeakFitSpec {
                    shape: match shape {
                        PeakFitKind::Gauss => analysis::PeakShape::Gauss,
                        PeakFitKind::Lorentz => analysis::PeakShape::Lorentz,
                    },
                    baseline: match fit_baseline {
                        PeakFitBaselineKind::Constant => analysis::FitBaseline::Constant,
                        PeakFitBaselineKind::Linear => analysis::FitBaseline::Linear,
                    },
                });
                let find = analysis::FindOptions {
                    method,
                    npts: points,
                    smooth_points: smooth,
                    direction,
                    filter,
                    min_fwhm: min_width,
                };
                (
                    target,
                    Job::Tool(Tool::PeakChain(analysis::PeakChainSpec {
                        chain: analysis::ChainOptions { baseline, find, limits },
                        integrate,
                        fit,
                    })),
                )
            }
        }
        AnalyzeTool::Stats { target, all, add, level } => {
            let mut spec = if all { analysis::StatisticsSpec::all() } else { analysis::StatisticsSpec::default() };
            for name in &add {
                match name.trim().to_ascii_lowercase().as_str() {
                    "iqr" => spec.iqr = true,
                    "variance" | "var" => spec.variance = true,
                    "skewness" | "skew" => spec.skewness = true,
                    "kurtosis" | "kurt" => spec.kurtosis = true,
                    "cv" => spec.cv = true,
                    "limits" | "cl" | "ci" => spec.confidence_limits = true,
                    other => {
                        bail!("unknown quantity {other:?}; use iqr, variance, skewness, kurtosis, cv or limits")
                    }
                }
            }
            spec.level = level;
            (target, Job::Tool(Tool::Statistics(spec)))
        }
        AnalyzeTool::Ttest { target, test, mean, difference, unequal, tail, alpha } => {
            let kind = match test {
                TTestKindArg::OneSample => analysis::TTestKind::OneSample { mean },
                TTestKindArg::TwoSample => analysis::TTestKind::TwoSample { equal_variance: !unequal, difference },
                TTestKindArg::Paired => analysis::TTestKind::Paired { difference },
            };
            (target, Job::Tool(Tool::TTest(analysis::TTestSpec { kind, tail: tail_of(tail), alpha })))
        }
        AnalyzeTool::Anova { target, comparison, alpha } => {
            (target, Job::Tool(Tool::Anova(analysis::AnovaSpec { alpha, comparison: comparison_of(comparison) })))
        }
        AnalyzeTool::Power { .. } => unreachable!("handled before the data file is read"),
        AnalyzeTool::Anova2 { target, raw, levels_a, levels_b, no_interaction, comparison, alpha } => {
            let layout = if raw {
                if levels_a.len() != target.y.len() || levels_b.len() != target.y.len() {
                    bail!(
                        "--raw needs one level of factor A (--levels-a) and one of factor B (--levels-b) for each \
                         of the {} --y columns; {} and {} were given",
                        target.y.len(),
                        levels_a.len(),
                        levels_b.len()
                    );
                }
                analysis::TwoWayLayout::Raw { levels: levels_a.into_iter().zip(levels_b).collect() }
            } else {
                if target.y.len() != 3 {
                    bail!(
                        "the two-way ANOVA with indexed data needs --y FACTOR_A,FACTOR_B,DATA (three columns), \
                         or --raw with --levels-a and --levels-b"
                    );
                }
                analysis::TwoWayLayout::Indexed
            };
            (
                target,
                Job::Tool(Tool::TwoWayAnova(analysis::TwoWayAnovaSpec {
                    layout,
                    interaction: !no_interaction,
                    comparison: comparison_of(comparison),
                    alpha,
                })),
            )
        }
        AnalyzeTool::Mannwhitney { target, tail, exact, alpha } => {
            (target, Job::Tool(Tool::MannWhitney(analysis::MannWhitneySpec { tail: tail_of(tail), exact, alpha })))
        }
        AnalyzeTool::Signrank { target, test, median, tail, asymptotic, alpha } => {
            let kind = match test {
                SignRankKindArg::OneSample => analysis::SignedRankKind::OneSample { median },
                SignRankKindArg::Paired => analysis::SignedRankKind::Paired,
            };
            (
                target,
                Job::Tool(Tool::SignedRank(analysis::SignedRankSpec {
                    kind,
                    tail: tail_of(tail),
                    exact: !asymptotic,
                    alpha,
                })),
            )
        }
        AnalyzeTool::Rowstats { target, add, no_mean_sd } => {
            let mut spec = analysis::RowStatsSpec { mean: !no_mean_sd, sd: !no_mean_sd, ..Default::default() };
            for name in &add {
                match name.trim().to_ascii_lowercase().as_str() {
                    "sem" => spec.sem = true,
                    "min" => spec.min = true,
                    "max" => spec.max = true,
                    "n" => spec.n = true,
                    "sum" => spec.sum = true,
                    other => bail!("unknown quantity {other:?}; use sem, min, max, n or sum"),
                }
            }
            (target, Job::Tool(Tool::RowStatistics(spec)))
        }
        AnalyzeTool::Grubbs { target, alpha } => (target, Job::Tool(Tool::Grubbs(analysis::OutlierSpec { alpha }))),
        AnalyzeTool::Dixon { target, alpha } => (target, Job::Tool(Tool::Dixon(analysis::OutlierSpec { alpha }))),
        AnalyzeTool::Sign { target, tail, alpha } => {
            (target, Job::Tool(Tool::SignTest(analysis::SignTestSpec { tail: tail_of(tail), alpha })))
        }
        AnalyzeTool::Ks2 { target, tail, alpha } => {
            (target, Job::Tool(Tool::KsTwoSample(analysis::KsTwoSampleSpec { tail: tail_of(tail), alpha })))
        }
        AnalyzeTool::Mediantest { target, alpha } => {
            (target, Job::Tool(Tool::MedianTest(analysis::MedianTestSpec { alpha })))
        }
        AnalyzeTool::Friedman { target, alpha } => {
            (target, Job::Tool(Tool::Friedman(analysis::FriedmanSpec { alpha })))
        }
        AnalyzeTool::Kruskal { target, alpha } => {
            (target, Job::Tool(Tool::KruskalWallis(analysis::KruskalWallisSpec { alpha })))
        }
        AnalyzeTool::Corr { target, spearman, kendall, no_pearson, listwise, alpha } => (
            target,
            Job::Tool(Tool::Correlation(analysis::CorrelationSpec {
                pearson: !no_pearson,
                spearman,
                kendall,
                listwise,
                alpha,
            })),
        ),
        AnalyzeTool::Histogram { target, bin_size, bins, min, max, below, above, no_graph } => {
            let size = match (bin_size, bins) {
                (Some(w), _) => analysis::BinSize::Width(w),
                (None, Some(n)) => analysis::BinSize::Count(n),
                (None, None) => analysis::BinSize::Auto,
            };
            let freq = analysis::FreqOptions { min, max, size, below, above };
            (target, Job::Tool(Tool::Histogram(analysis::HistogramSpec { freq, graph: !no_graph })))
        }
        AnalyzeTool::Discfreq { target, relative, cumulative, percent, no_count, case_sensitive, sort } => {
            let spec = analysis::DiscreteFreqSpec {
                count: !no_count,
                relative,
                cumulative,
                percent,
                case_sensitive,
                sort: analysis::DiscSort::from_code(&sort).expect("clap checks the value"),
            };
            (target, Job::Tool(Tool::DiscreteFrequency(spec)))
        }
        AnalyzeTool::Bin2d {
            target,
            stat,
            x_inc,
            x_bins,
            x_min,
            x_max,
            x_centers,
            x_below,
            x_above,
            y_inc,
            y_bins,
            y_min,
            y_max,
            y_centers,
            y_below,
            y_above,
        } => {
            let size = |inc: Option<f64>, bins: Option<usize>| match (inc, bins) {
                (Some(w), _) => analysis::BinSize::Width(w),
                (None, Some(n)) => analysis::BinSize::Count(n),
                (None, None) => analysis::BinSize::Auto,
            };
            let x = analysis::AxisBins {
                min: x_min,
                max: x_max,
                size: size(x_inc, x_bins),
                centers: x_centers,
                below: x_below,
                above: x_above,
            };
            let y = analysis::AxisBins {
                min: y_min,
                max: y_max,
                size: size(y_inc, y_bins),
                centers: y_centers,
                below: y_below,
                above: y_above,
            };
            let stat = analysis::Bin2dStat::from_code(&stat).expect("clap checks the value");
            (target, Job::Tool(Tool::Binning2D(analysis::Bin2dOptions { x, y, stat })))
        }
        AnalyzeTool::Normality { target, alpha } => {
            (target, Job::Tool(Tool::Normality(analysis::NormalitySpec { alpha })))
        }
        AnalyzeTool::Sort { target, by, descending } => {
            whole_sheet_only(&target, "sort")?;
            (target, Job::Sort { by, descending })
        }
        AnalyzeTool::Filter { target, condition, in_place } => {
            whole_sheet_only(&target, "filter")?;
            (target, Job::Filter { condition, in_place })
        }
    })
}

/// Sorting and filtering work on all rows and columns of a sheet; refuse the options that choose
/// rows or columns instead of silently ignoring them.
fn whole_sheet_only(target: &TargetArgs, tool: &str) -> Result<()> {
    let given: Vec<&str> =
        [(!target.y.is_empty(), "--y"), (target.rows.is_some(), "--rows"), (target.x_range.is_some(), "--x-range")]
            .into_iter()
            .filter_map(|(set, name)| set.then_some(name))
            .collect();
    if given.is_empty() {
        Ok(())
    } else {
        bail!("{tool} works on all rows and columns of the sheet; {} cannot be used with it", given.join(", "))
    }
}

/// Finds the reference column (loading its file if one was named) and builds the tool.
fn math_tool(
    project: &mut Project,
    book: sheaf_core::WindowId,
    sheet: usize,
    op: MathKind,
    operand: &OperandArgs,
    column: bool,
    read: &ReadOptions,
) -> Result<Tool> {
    let op = match op {
        MathKind::Add => analysis::CurveOp::Add,
        MathKind::Subtract => analysis::CurveOp::Subtract,
        MathKind::Multiply => analysis::CurveOp::Multiply,
        MathKind::Divide => analysis::CurveOp::Divide,
        MathKind::Power => analysis::CurveOp::Power,
    };
    let operand_value = match (&operand.reference, operand.value) {
        (None, Some(v)) => analysis::CurveOperand::Constant(v),
        (None, None) => bail!("give the second operand: --value NUMBER or --ref COLUMN"),
        (Some(name), _) => {
            let (rb, rs) = match &operand.ref_file {
                Some(file) => {
                    let loaded = if is_project(file) {
                        load_project(file)?
                    } else {
                        let mut p = Project::new();
                        p.adopt_workbook(read_data_file(file, read)?.workbook);
                        p
                    };
                    let from = pick_book(&loaded, operand.ref_book.as_deref())?;
                    let book = loaded.workbook(from).expect("picked above").clone();
                    let id = project.adopt_workbook(book);
                    (id, pick_sheet(project, id, operand.ref_sheet.as_deref())?)
                }
                None => {
                    let rb = match &operand.ref_book {
                        Some(n) => pick_book(project, Some(n))?,
                        None => book,
                    };
                    let rs = match (&operand.ref_sheet, rb == book) {
                        (Some(n), _) => pick_sheet(project, rb, Some(n))?,
                        (None, true) => sheet,
                        (None, false) => pick_sheet(project, rb, None)?,
                    };
                    (rb, rs)
                }
            };
            analysis::CurveOperand::Reference(named_columns(project, rb, rs, std::slice::from_ref(name))?[0])
        }
    };
    if column {
        if operand.common {
            bail!("--common only applies to curve math; column math goes row by row");
        }
        return Ok(Tool::ColumnMath(analysis::ColumnMathSpec { op, operand: operand_value }));
    }
    if operand.common && matches!(operand_value, analysis::CurveOperand::Constant(_)) {
        bail!("--common only applies to a reference curve");
    }
    Ok(Tool::CurveMath(analysis::CurveMathSpec { op, operand: operand_value, common_only: operand.common }))
}

/// `analyze power`: no data file, the table goes to the terminal (and `--export`).
fn power(a: AnalyzeTool) -> Result<()> {
    let AnalyzeTool::Power { test, sizes, powers, mean1, mean2, sd, levels, css, alpha, one_sided, export } = a else {
        unreachable!("called for the power tool only");
    };
    if sizes.is_empty() == powers.is_empty() {
        bail!("give either --sizes (to find powers) or --powers (to find sample sizes)");
    }
    let spec = analysis::PowerSpec {
        kind: match test {
            PowerTestArg::OneSample => analysis::PowerKind::OneSampleT,
            PowerTestArg::TwoSample => analysis::PowerKind::TwoSampleT,
            PowerTestArg::Paired => analysis::PowerKind::PairedT,
            PowerTestArg::Anova => analysis::PowerKind::Anova,
        },
        goal: if sizes.is_empty() {
            analysis::PowerGoal::SampleSize(powers)
        } else {
            analysis::PowerGoal::Power(sizes)
        },
        alpha,
        two_sided: !one_sided,
        mean1,
        mean2,
        sd,
        levels,
        css,
    };
    let mut project = Project::new();
    let report = project.power_analysis(&spec)?;
    let ws = &project.workbook(report.book).expect("just added").sheets[0];
    println!("{:>12}  {:>12}", "Sample Size", "Power");
    let (size, power) = (ws.columns[0].numeric().unwrap_or(&[]), ws.columns[1].numeric().unwrap_or(&[]));
    for (n, p) in size.iter().zip(power) {
        println!("{:>12}  {:>12}", short_number(*n), short_number(*p));
    }
    for note in &report.notes {
        println!("{note}");
    }
    if let Some(path) = export {
        write_sheet(ws, &path)?;
    }
    Ok(())
}

pub fn analyze(a: AnalyzeArgs) -> Result<()> {
    if matches!(a.tool, AnalyzeTool::Power { .. }) {
        return power(a.tool);
    }
    let (target, job) = job_from(a.tool)?;
    let mut project = if is_project(&target.input) {
        load_project(&target.input)?
    } else {
        let mut p = Project::new();
        p.adopt_workbook(read_data_file(&target.input, &target.read)?.workbook);
        p
    };
    let book = pick_book(&project, target.book.as_deref())?;
    let sheet = pick_sheet(&project, book, target.sheet.as_deref())?;
    let job = match job {
        Job::Math { op, operand, column } => {
            Job::Tool(math_tool(&mut project, book, sheet, op, &operand, column, &target.read)?)
        }
        Job::Signal { conv, other, circular, normalize, interval } => {
            let other = named_columns(&project, book, sheet, std::slice::from_ref(&other))?[0];
            Job::Tool(if conv {
                Tool::Convolution(analysis::ConvolutionSpec { response: other, circular, normalize, interval })
            } else {
                let interval = interval.unwrap_or(1.0);
                Tool::SignalCorrelation(analysis::SignalCorrelationSpec { other, circular, normalize, interval })
            })
        }
        other => other,
    };

    // The sheet that holds the results, for --export.
    let result_sheet = match job {
        Job::Tool(tool) => {
            let columns = if target.y.is_empty() {
                project.default_plot_columns(book, sheet, Template::Line)
            } else {
                named_columns(&project, book, sheet, &target.y)?
            };
            if columns.is_empty() {
                bail!("there are no numeric Y columns to work on; name them with --y");
            }
            let rows = match (&target.rows, &target.x_range) {
                (Some(r), _) => {
                    let (from, to) = parse_pair::<usize>(r, "row range", "120:480")?;
                    analysis::RowRange::Rows(from, to)
                }
                (None, Some(r)) => {
                    let (from, to) = parse_pair::<f64>(r, "X range", "0.2:0.6")?;
                    analysis::RowRange::XBetween(from, to)
                }
                (None, None) => analysis::RowRange::All,
            };
            let report = project.analyze(&analysis::Request { columns, rows, tool })?;
            if let Some(t) = &report.table {
                print_table(t);
            }
            for note in &report.notes {
                println!("{note}");
            }
            println!("{}", report.message);
            report.new_sheet.unwrap_or(sheet)
        }
        Job::Math { .. } | Job::Signal { .. } => unreachable!("turned into a tool above"),
        Job::Sort { by, descending } => {
            let key = named_columns(&project, book, sheet, std::slice::from_ref(&by))?[0];
            project.sort_sheet(book, sheet, key, descending)?;
            println!("sorted by column {by}");
            sheet
        }
        Job::Filter { condition, in_place } => {
            let before = project.workbook(book).expect("picked above").sheets[sheet].row_count();
            let (kept, at) = project.filter_sheet(book, sheet, &condition, !in_place)?;
            let name = &project.workbook(book).expect("picked above").sheets[at].name;
            println!("kept {kept} of {before} rows in sheet {name}");
            at
        }
    };

    if let Some(path) = &target.export {
        write_sheet(&project.workbook(book).expect("picked above").sheets[result_sheet], path)?;
    }
    if let Some(path) = &target.save {
        save_project(&project, path)?;
    }
    Ok(())
}

fn local_span(span: f64) -> analysis::LocalSpan {
    if span > 0.0 { analysis::LocalSpan::Proportion(span) } else { analysis::LocalSpan::Points }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_print_with_six_significant_digits() {
        assert_eq!(short_number(20.0), "20");
        assert_eq!(short_number(0.000123456789), "0.000123457");
        assert_eq!(short_number(-1234.5678), "-1234.57");
        assert_eq!(short_number(1.5e-9), "1.50000e-9");
        assert_eq!(short_number(2.5e12), "2.50000e12");
        assert_eq!(short_number(0.0), "0");
        assert_eq!(short_number(f64::NAN), "--");
        assert_eq!(short_number(123456789.0), "123456789");
    }

    #[test]
    fn ranges_are_read_as_from_colon_to() {
        assert_eq!(parse_pair::<usize>("120:480", "row range", "120:480").unwrap(), (120, 480));
        assert_eq!(parse_pair::<f64>("-0.5: 0.1", "X range", "0.2:0.6").unwrap(), (-0.5, 0.1));
        assert!(parse_pair::<usize>("12-48", "row range", "120:480").is_err());
        assert!(parse_pair::<usize>("a:3", "row range", "120:480").is_err());
    }
}
