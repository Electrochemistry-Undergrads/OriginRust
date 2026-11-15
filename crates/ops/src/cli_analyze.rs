//! What `sheaf analyze` means: the command line's parsed options turned into an analysis [`Job`]
//! and [`Target`] for [`crate::analyze::run_job`]. Nothing here prints.
//! `sheaf-cli` and `sheaf-mcp` (which builds the same options from JSON) both call it.

use crate::analyze::{Job, OperandSpec, parse_pair};
use crate::args::*;
use anyhow::{Result, anyhow, bail};
use sheaf_core::analysis::{self, BaselineSpec, IntegrateSpec, InterpSpec, Tool};
use std::path::PathBuf;

pub(crate) fn tail_of(tail: TailArg) -> analysis::Tail {
    match tail {
        TailArg::Two => analysis::Tail::Two,
        TailArg::Upper => analysis::Tail::Upper,
        TailArg::Lower => analysis::Tail::Lower,
    }
}

fn family_of(family: FamilyArg) -> analysis::DistFamily {
    use analysis::DistFamily as F;
    match family {
        FamilyArg::Normal => F::Normal,
        FamilyArg::Lognormal => F::Lognormal,
        FamilyArg::Exponential => F::Exponential,
        FamilyArg::Gamma => F::Gamma,
        FamilyArg::Weibull => F::Weibull,
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
        ComparisonArg::Dunnett => Some(M::Dunnett),
        ComparisonArg::None => None,
    }
}

pub fn job_from(tool: AnalyzeTool) -> Result<(TargetArgs, Job)> {
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
            (target, Job::Math { op: analysis::CurveOp::Subtract, operand: operand_spec(operand), column: false })
        }
        AnalyzeTool::Math { target, op, operand } => {
            (target, Job::Math { op: curve_op(op), operand: operand_spec(operand), column: false })
        }
        AnalyzeTool::ColumnMath { target, op, operand } => {
            (target, Job::Math { op: curve_op(op), operand: operand_spec(operand), column: true })
        }
        AnalyzeTool::Envelope { target, kind, npts } => {
            let kind = match kind {
                EnvelopeArg::Upper => analysis::EnvelopeType::Upper,
                EnvelopeArg::Lower => analysis::EnvelopeType::Lower,
                EnvelopeArg::Both => analysis::EnvelopeType::Both,
            };
            let options = analysis::EnvelopeOptions { kind, npts };
            (target, Job::Tool(Tool::Envelope(analysis::EnvelopeSpec { options })))
        }
        AnalyzeTool::Hilbert { target } => (target, Job::Tool(Tool::Hilbert)),
        AnalyzeTool::Coherence { target, other, fft_len, win_len, overlap, window, alpha, beta, interval } => {
            let window = match window {
                CoherenceWindowArg::Rectangle => analysis::CoherenceWindow::Rectangle,
                CoherenceWindowArg::Welch => analysis::CoherenceWindow::Welch,
                CoherenceWindowArg::Triangular => analysis::CoherenceWindow::Triangular,
                CoherenceWindowArg::Bartlett => analysis::CoherenceWindow::Bartlett,
                CoherenceWindowArg::Hanning => analysis::CoherenceWindow::Hanning,
                CoherenceWindowArg::Hamming => analysis::CoherenceWindow::Hamming,
                CoherenceWindowArg::Blackman => analysis::CoherenceWindow::Blackman,
                CoherenceWindowArg::Gaussian => analysis::CoherenceWindow::Gaussian { alpha },
                CoherenceWindowArg::Kaiser => analysis::CoherenceWindow::Kaiser { beta },
            };
            let options = analysis::CoherenceOptions { fft_len, win_len, overlap, window, interval };
            (target, Job::Coherence { other, options })
        }
        AnalyzeTool::Decimate { target, factor, filter, order } => {
            let filter = match filter {
                DecimateFilterArg::None => analysis::DecimateFilter::None,
                DecimateFilterArg::Ave => analysis::DecimateFilter::MovingAverage,
                DecimateFilterArg::Fir => analysis::DecimateFilter::Fir,
            };
            let options = analysis::DecimateOptions { factor, filter, order };
            (target, Job::Tool(Tool::Decimate(analysis::DecimateSpec { options })))
        }
        AnalyzeTool::FftFilter { target, kind, cutoff, freq1, freq2, threshold, pass, stop, no_offset, interval } => {
            let kind = match kind {
                FftFilterArg::LowPass => analysis::FftFilterType::LowPass,
                FftFilterArg::HighPass => analysis::FftFilterType::HighPass,
                FftFilterArg::BandPass => analysis::FftFilterType::BandPass,
                FftFilterArg::BandBlock => analysis::FftFilterType::BandBlock,
                FftFilterArg::Threshold => analysis::FftFilterType::Threshold,
                FftFilterArg::Parabolic => analysis::FftFilterType::LowPassParabolic,
            };
            let options = analysis::FftFilterOptions {
                kind,
                cutoff,
                freq1,
                freq2,
                threshold,
                pass,
                stop,
                keep_offset: !no_offset,
            };
            (target, Job::Tool(Tool::FftFilter(analysis::FftFilterSpec { options, interval })))
        }
        AnalyzeTool::TraceInterp { target, method, points } => {
            let method = match method {
                TraceMethodArg::Linear => analysis::TraceMethod::Linear,
                TraceMethodArg::Spline => analysis::TraceMethod::CubicSpline,
                TraceMethodArg::Bspline => analysis::TraceMethod::CubicBSpline,
            };
            (target, Job::Tool(Tool::TraceInterp(analysis::TraceInterpSpec { method, points })))
        }
        AnalyzeTool::Ifft { target, imaginary, interval, no_shift, no_unwrap, science } => {
            let options = analysis::IfftOptions {
                interval: interval.unwrap_or(1.0),
                shift: !no_shift,
                unwrap: !no_unwrap,
                science,
            };
            (target, Job::Ifft { imaginary, options, auto_interval: interval.is_none() })
        }
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
                        PeakFitKind::Voigt => analysis::PeakShape::Voigt,
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
        AnalyzeTool::Anova { target, comparison, control, alpha } => {
            if control == 0 {
                bail!("--control counts the groups from 1");
            }
            (
                target,
                Job::Tool(Tool::Anova(analysis::AnovaSpec {
                    alpha,
                    comparison: comparison_of(comparison),
                    control: control - 1,
                })),
            )
        }
        AnalyzeTool::Crosstab { target, no_row_percent, no_column_percent, no_total_percent, no_expected, alpha } => {
            no_x_range(&target, "the cross tabulation")?;
            (
                target,
                Job::Tool(Tool::Crosstab(analysis::CrosstabSpec {
                    row_percent: !no_row_percent,
                    column_percent: !no_column_percent,
                    total_percent: !no_total_percent,
                    expected: !no_expected,
                    alpha,
                })),
            )
        }
        AnalyzeTool::Power { .. } => unreachable!("handled before the data file is read"),
        AnalyzeTool::Anova2 { target, raw, levels_a, levels_b, no_interaction, comparison, alpha } => {
            no_x_range(&target, "the two-way ANOVA")?;
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
        AnalyzeTool::Distfit { target, family } => {
            let families = if family.is_empty() {
                analysis::DistFamily::ALL.to_vec()
            } else {
                family.into_iter().map(family_of).collect()
            };
            (target, Job::Tool(Tool::DistFit(analysis::DistFitSpec { families })))
        }
        AnalyzeTool::Probplot { target, family, no_graph } => (
            target,
            Job::Tool(Tool::ProbPlot(analysis::ProbPlotSpec {
                kind: analysis::ProbPlotKind::Probability,
                family: family_of(family),
                graph: !no_graph,
            })),
        ),
        AnalyzeTool::Qqplot { target, family, no_graph } => (
            target,
            Job::Tool(Tool::ProbPlot(analysis::ProbPlotSpec {
                kind: analysis::ProbPlotKind::QQ,
                family: family_of(family),
                graph: !no_graph,
            })),
        ),
        AnalyzeTool::Pca { target, covariance, pairwise, components, standardize_scores, no_scores, no_graph } => (
            target,
            Job::Tool(Tool::Pca(analysis::PcaSpec {
                correlation: !covariance,
                pairwise,
                components,
                standardize_scores,
                scores: !no_scores,
                scree: !no_graph,
            })),
        ),
        AnalyzeTool::Roc { target, positive, lower_is_positive, confidence, no_graph } => (
            target,
            Job::Tool(Tool::Roc(analysis::RocSpec {
                positive,
                higher_is_positive: !lower_is_positive,
                confidence,
                curve: !no_graph,
            })),
        ),
        AnalyzeTool::Kmeans { target, clusters, max_passes, no_anova, no_membership, no_graph } => (
            target,
            Job::Tool(Tool::Kmeans(analysis::KmeansSpec {
                clusters,
                max_passes,
                anova: !no_anova,
                membership: !no_membership,
                member_distance: !no_membership,
                graph: !no_graph,
            })),
        ),
        AnalyzeTool::KaplanMeier {
            target,
            censor,
            confidence,
            no_quartiles,
            no_mean,
            log_rank,
            breslow,
            tarone_ware,
            band,
            no_graph,
        } => (
            target,
            Job::Tool(Tool::KaplanMeier(analysis::KaplanMeierSpec {
                censor,
                confidence,
                quartiles: !no_quartiles,
                mean: !no_mean,
                log_rank,
                breslow,
                tarone_ware,
                curve: !no_graph,
                band,
            })),
        ),
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

/// The tools that read factor columns have no X column to limit; say so before running instead of failing in
/// the tool.
fn no_x_range(target: &TargetArgs, tool: &str) -> Result<()> {
    if target.x_range.is_some() {
        bail!("{tool} has no X column to limit, so --x-range cannot be used with it; choose rows with --rows");
    }
    Ok(())
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

/// The power tool's request (`analyze power`) and where `--export` writes the table. Needs no data.
pub fn power_spec(a: AnalyzeTool) -> Result<(analysis::PowerSpec, Option<PathBuf>)> {
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
    Ok((spec, export))
}

/// The operator of `math` and `subtract-ref`.
pub(crate) fn curve_op(op: MathKind) -> analysis::CurveOp {
    match op {
        MathKind::Add => analysis::CurveOp::Add,
        MathKind::Subtract => analysis::CurveOp::Subtract,
        MathKind::Multiply => analysis::CurveOp::Multiply,
        MathKind::Divide => analysis::CurveOp::Divide,
        MathKind::Power => analysis::CurveOp::Power,
    }
}

pub(crate) fn operand_spec(o: OperandArgs) -> OperandSpec {
    OperandSpec {
        value: o.value,
        reference: o.reference,
        ref_file: o.ref_file,
        ref_book: o.ref_book,
        ref_sheet: o.ref_sheet,
        common: o.common,
    }
}

pub(crate) fn local_span(span: f64) -> analysis::LocalSpan {
    if span > 0.0 { analysis::LocalSpan::Proportion(span) } else { analysis::LocalSpan::Points }
}
