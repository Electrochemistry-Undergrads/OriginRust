//! What `sheaf analyze` means: the command line's parsed options turned into an analysis [`Job`]
//! and [`Target`] for [`crate::analyze::run_job`]. Nothing here prints.
//! `sheaf-cli` and `sheaf-mcp` (which builds the same options from JSON) both call it.

use crate::analyze::{Job, OperandSpec, parse_pair};
use crate::args::*;
use anyhow::{Result, anyhow, bail};
use sheaf_core::analysis::Wavelet;
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

fn filter_response(kind: FilterTypeArg) -> analysis::FilterResponse {
    match kind {
        FilterTypeArg::LowPass => analysis::FilterResponse::LowPass,
        FilterTypeArg::HighPass => analysis::FilterResponse::HighPass,
        FilterTypeArg::BandPass => analysis::FilterResponse::BandPass,
        FilterTypeArg::BandStop => analysis::FilterResponse::BandStop,
    }
}

fn freq_unit(unit: FreqUnitArg) -> analysis::FreqUnit {
    match unit {
        FreqUnitArg::Hz => analysis::FreqUnit::Hz,
        FreqUnitArg::Normalized => analysis::FreqUnit::Normalized,
    }
}

/// The multiple comparisons plot a `sheaf analyze anova` or `ttest` call asked for (`--comparison-plot`), checked.
pub fn comparison_plot_request(tool: &AnalyzeTool) -> Result<Option<crate::analyze::ComparisonPlotRequest>> {
    use sheaf_core::compplot::{BracketLabels, ComparisonErrorBar, ComparisonPlotOptions, ComparisonPlotType};
    let (plot, what) = match tool {
        AnalyzeTool::Anova { plot, comparison, .. } => {
            if plot.comparison_plot.is_some() && comparison_of(*comparison).is_none() {
                bail!("--comparison-plot needs a means comparison: choose --comparison other than none");
            }
            (plot, "anova")
        }
        AnalyzeTool::Ttest { plot, test, .. } => {
            if plot.comparison_plot.is_some() && *test != TTestKindArg::TwoSample {
                bail!("--comparison-plot is made for the two-sample t-test (--test two-sample)");
            }
            (plot, "t-test")
        }
        _ => return Ok(None),
    };
    let Some(path) = &plot.comparison_plot else { return Ok(None) };
    if !(plot.threshold > 0.0 && plot.threshold <= 1.0) {
        bail!("--threshold must lie between 0 and 1, not {}", plot.threshold);
    }
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    if !matches!(ext.as_str(), "png" | "svg" | "pdf") {
        bail!("--comparison-plot writes .png, .svg or .pdf, not '{}' (for the {what} plot)", path.display());
    }
    let options = ComparisonPlotOptions {
        plot_type: match plot.plot_type {
            ComparisonPlotTypeArg::Bar => ComparisonPlotType::Bar,
            ComparisonPlotTypeArg::Box => ComparisonPlotType::Box,
            ComparisonPlotTypeArg::Violin => ComparisonPlotType::Violin,
            ComparisonPlotTypeArg::Scatter => ComparisonPlotType::Scatter,
        },
        error_bar: match plot.error_bar {
            ErrorBarArg::Sd => ComparisonErrorBar::Sd,
            ErrorBarArg::Sem => ComparisonErrorBar::Sem,
            ErrorBarArg::Ci95 => ComparisonErrorBar::Ci95,
        },
        labels: match plot.labels {
            BracketLabelsArg::Asterisks => BracketLabels::Asterisks,
            BracketLabelsArg::PValues => BracketLabels::PValues,
        },
        significant_only: !plot.all_pairs,
        threshold: plot.threshold,
        alpha: 0.05,
        pairs: Vec::new(),
    };
    Ok(Some(crate::analyze::ComparisonPlotRequest { path: path.clone(), options }))
}

pub fn job_from(tool: AnalyzeTool) -> Result<(TargetArgs, Job)> {
    Ok(match tool {
        AnalyzeTool::CvCharge { target, scan_rate, baseline } => {
            (target, Job::Tool(Tool::CvCharge(analysis::CvChargeSpec { scan_rate, end_point_baseline: baseline })))
        }
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
        AnalyzeTool::WaveletDecompose { target, wavelet, ext } => {
            let spec = analysis::WaveletDecomposeSpec { wavelet: wavelet_arg(&wavelet)?, extension: wavelet_ext(ext) };
            (target, Job::Tool(Tool::WaveletDecompose(spec)))
        }
        AnalyzeTool::WaveletReconstruct { target, detail, wavelet, ext } => {
            (target, Job::WaveletReconstruct { detail, wavelet: wavelet_arg(&wavelet)?, extension: wavelet_ext(ext) })
        }
        AnalyzeTool::ContinuousWavelet { target, scales, wavelet, param } => {
            let wavelet = match wavelet {
                CwtWaveletArg::Morlet => analysis::CwtWavelet::Morlet,
                CwtWaveletArg::Dgauss => analysis::CwtWavelet::DGauss,
                CwtWaveletArg::Mexhat => analysis::CwtWavelet::MexHat,
            };
            (target, Job::ContinuousWavelet { scales, wavelet, param })
        }
        AnalyzeTool::WaveletDenoise { target, wavelet, ext, threshold, level, percent } => {
            let threshold = match threshold {
                WaveletThresholdArg::Custom => analysis::WaveletThresholdKind::Custom,
                WaveletThresholdArg::Sqtwolog => analysis::WaveletThresholdKind::SqtwoLog,
            };
            let spec = analysis::WaveletDenoiseSpec {
                wavelet: wavelet_arg(&wavelet)?,
                extension: wavelet_ext(ext),
                threshold,
                level,
                percent,
            };
            (target, Job::Tool(Tool::WaveletDenoise(spec)))
        }
        AnalyzeTool::WaveletSmooth { target, wavelet, ext, cutoff } => {
            let spec =
                analysis::WaveletSmoothSpec { wavelet: wavelet_arg(&wavelet)?, extension: wavelet_ext(ext), cutoff };
            (target, Job::Tool(Tool::WaveletSmooth(spec)))
        }
        AnalyzeTool::IirFilter {
            target,
            kind,
            method,
            order,
            minimum,
            unit,
            fs,
            cutoff,
            cutoff2,
            pass,
            pass2,
            stop,
            stop2,
            ripple,
            attenuation,
            forward_backward,
        } => {
            let spec = analysis::IirSpec {
                response: filter_response(kind),
                method: match method {
                    IirMethodArg::Butterworth => analysis::IirMethod::Butterworth,
                    IirMethodArg::Chebyshev1 => analysis::IirMethod::Chebyshev1,
                    IirMethodArg::Chebyshev2 => analysis::IirMethod::Chebyshev2,
                    IirMethodArg::Elliptic => analysis::IirMethod::Elliptic,
                },
                order: if minimum { None } else { Some(order) },
                unit: freq_unit(unit),
                sample_frequency: fs,
                cutoff,
                cutoff2,
                pass,
                pass2,
                stop,
                stop2,
                passband_ripple: ripple,
                stopband_attenuation: attenuation,
                forward_backward,
            };
            (target, Job::Tool(Tool::IirFilter(spec)))
        }
        AnalyzeTool::FirFilter {
            target,
            kind,
            method,
            order,
            minimum,
            window,
            window_param,
            no_scale,
            unit,
            fs,
            cutoff,
            cutoff2,
            pass,
            pass2,
            stop,
            stop2,
            pass_weight,
            stop_weight,
            magnitude_unit,
            ripple,
            attenuation,
            forward_backward,
        } => {
            use analysis::FirWindow as W;
            let window = match window {
                FirWindowArg::Bartlett => W::Bartlett,
                FirWindowArg::BartlettHanning => W::BartlettHanning,
                FirWindowArg::Blackman => W::Blackman,
                FirWindowArg::BlackmanHarris => W::BlackmanHarris,
                FirWindowArg::Bohman => W::Bohman,
                FirWindowArg::Chebyshev => W::Chebyshev,
                FirWindowArg::FlatTop => W::FlatTop,
                FirWindowArg::Gaussian => W::Gaussian,
                FirWindowArg::Hamming => W::Hamming,
                FirWindowArg::Hanning => W::Hanning,
                FirWindowArg::Kaiser => W::Kaiser,
                FirWindowArg::Nuttall => W::Nuttall,
                FirWindowArg::Parzen => W::Parzen,
                FirWindowArg::Rectangle => W::Rectangle,
                FirWindowArg::Taylor => W::Taylor,
                FirWindowArg::Triangular => W::Triangular,
                FirWindowArg::Tukey => W::Tukey,
            };
            let spec = analysis::FirSpec {
                response: filter_response(kind),
                method: match method {
                    FirMethodArg::LeastSquares => analysis::FirMethod::LeastSquares,
                    FirMethodArg::Window => analysis::FirMethod::Window,
                },
                order: if minimum { None } else { Some(order) },
                window,
                window_param,
                scale_passband: !no_scale,
                unit: freq_unit(unit),
                sample_frequency: fs,
                cutoff,
                cutoff2,
                pass,
                pass2,
                stop,
                stop2,
                pass_weight,
                stop_weight,
                magnitude_unit: match magnitude_unit {
                    MagnitudeUnitArg::Db => analysis::MagnitudeUnit::Db,
                    MagnitudeUnitArg::Linear => analysis::MagnitudeUnit::Linear,
                },
                passband_ripple: ripple,
                stopband_attenuation: attenuation,
                forward_backward,
            };
            (target, Job::Tool(Tool::FirFilter(spec)))
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
                window: fft_window(window),
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
        AnalyzeTool::Deconvolution { target, response, normalize, circular, interval } => {
            (target, Job::Deconvolution { response, normalize, circular, interval })
        }
        AnalyzeTool::Stft { target, interval, window, window_len, fft_len, overlap, output } => {
            let options = analysis::StftOptions {
                interval: interval.unwrap_or(1.0),
                window: fft_window(window),
                window_len,
                fft_len,
                overlap,
                output: match output {
                    StftOutputArg::Amplitude => analysis::StftOutput::Amplitude,
                    StftOutputArg::Db => analysis::StftOutput::Db,
                },
            };
            (target, Job::Tool(Tool::Stft(analysis::StftSpec { options, auto_interval: interval.is_none() })))
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
        AnalyzeTool::Normalize {
            target,
            method: kind @ (NormalizeKind::Ref | NormalizeKind::Cell),
            reference,
            stat,
            row,
            ..
        } => {
            // T260: Origin's ref and cell methods.
            let reference =
                reference.ok_or_else(|| anyhow!("--method ref and cell need the reference column: --ref COLUMN"))?;
            if row == 0 {
                bail!("--row counts from 1");
            }
            let stat = (kind == NormalizeKind::Ref).then_some(match stat {
                RefStatArg::Min => analysis::RefStat::Min,
                RefStatArg::Max => analysis::RefStat::Max,
                RefStatArg::Mean => analysis::RefStat::Mean,
                RefStatArg::Median => analysis::RefStat::Median,
                RefStatArg::Sum => analysis::RefStat::Sum,
            });
            (target, Job::NormalizeRef { reference, stat, row: row - 1 })
        }
        AnalyzeTool::Normalize { target, method, value, .. } => {
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
                NormalizeKind::Ref | NormalizeKind::Cell => unreachable!("handled above"),
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
        AnalyzeTool::XyzArea { target } => (target, Job::Tool(Tool::XyzArea)),
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
        AnalyzeTool::Fsd { target, fwhm, k, apodization } => {
            let apodization = match apodization {
                ApodizationKind::Boxcar => analysis::Apodization::Boxcar,
                ApodizationKind::Triangle => analysis::Apodization::Triangle,
                ApodizationKind::TriangleSquared => analysis::Apodization::TriangleSquared,
            };
            (target, Job::Tool(Tool::Fsd(analysis::FsdOptions { fwhm, enhancement: k, apodization })))
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
            baseline_goal,
            anchors,
            regions,
            fsd_fwhm,
            fsd_k,
            als_asymmetric,
            als_threshold,
            als_smoothing,
            als_iterations,
            limits,
            integrate,
            fit,
            fit_baseline,
            fit_shapes,
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
            // Origin's baseline goals start on User Defined.
            let baseline = match (baseline_goal, baseline, &anchors) {
                (Some(_), None, None) => Some(PeakBaselineKind::SecondDerivative),
                (_, b, _) => b,
            };
            let chain = integrate
                || baseline_goal.is_some()
                || fit.is_some()
                || method.is_some()
                || smooth > 0
                || min_width > 0.0
                || baseline.is_some()
                || anchors.is_some()
                || fsd_fwhm.is_some()
                || regions.is_some()
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
                    (Some(PeakBaselineKind::Minima), _) => {
                        analysis::ChainBaseline::AutoMinima(baseline_points.unwrap_or(5))
                    }
                    (Some(PeakBaselineKind::SecondDerivative), _) => analysis::ChainBaseline::SecondDerivative {
                        points: baseline_points.unwrap_or(8),
                        method: analysis::InterpMethod::Linear,
                    },
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
                    (Some(PeakBaselineKind::Als), _) => {
                        let o = analysis::AlsOptions {
                            asymmetric: als_asymmetric,
                            threshold: als_threshold,
                            smoothing: als_smoothing,
                            iterations: als_iterations,
                        };
                        o.check().map_err(anyhow::Error::msg)?;
                        analysis::ChainBaseline::Als(o)
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
                let peak_shape = |k: &PeakFitKind| match k {
                    PeakFitKind::Gauss => analysis::PeakShape::Gauss,
                    PeakFitKind::Lorentz => analysis::PeakShape::Lorentz,
                    PeakFitKind::Voigt => analysis::PeakShape::Voigt,
                };
                let fit = fit.map(|shape| analysis::PeakFitSpec {
                    shape: peak_shape(&shape),
                    baseline: match fit_baseline {
                        PeakFitBaselineKind::Constant => analysis::FitBaseline::Constant,
                        PeakFitBaselineKind::Linear => analysis::FitBaseline::Linear,
                    },
                    shapes: fit_shapes.iter().map(peak_shape).collect(),
                    peak_curves: false,
                });
                let find = analysis::FindOptions {
                    method,
                    npts: points,
                    smooth_points: smooth,
                    direction,
                    filter,
                    min_fwhm: min_width,
                    fsd: fsd_fwhm.map(|fwhm| analysis::FsdOptions { fwhm, enhancement: fsd_k, ..Default::default() }),
                };
                if let Some(goal) = baseline_goal {
                    if integrate || fit.is_some() {
                        bail!("--baseline-goal only makes the baseline; leave out --integrate and --fit");
                    }
                    let subtract = goal == BaselineGoalKind::Subtract;
                    return Ok((
                        target,
                        Job::Tool(Tool::PeakBaseline(analysis::PeakBaselineSpec { baseline, subtract })),
                    ));
                }
                (
                    target,
                    Job::Tool(Tool::PeakChain(analysis::PeakChainSpec {
                        centers: Vec::new(),
                        regions: match &regions {
                            None => Vec::new(),
                            Some(text) => {
                                let mut out = Vec::new();
                                for part in text.split(',') {
                                    out.push(parse_pair::<f64>(part, "region", "2:8")?);
                                }
                                out
                            }
                        },
                        chain: analysis::ChainOptions { baseline, find, limits },
                        integrate,
                        fit,
                    })),
                )
            }
        }
        AnalyzeTool::Stats { target, all, add, level, percentiles } => {
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
            spec.percentiles = percentiles;
            (target, Job::Tool(Tool::Statistics(spec)))
        }
        AnalyzeTool::Ttest { target, test, mean, difference, unequal, tail, alpha, .. } => {
            let kind = match test {
                TTestKindArg::OneSample => analysis::TTestKind::OneSample { mean },
                TTestKindArg::TwoSample => analysis::TTestKind::TwoSample { equal_variance: !unequal, difference },
                TTestKindArg::Paired => analysis::TTestKind::Paired { difference },
            };
            (target, Job::Tool(Tool::TTest(analysis::TTestSpec { kind, tail: tail_of(tail), alpha })))
        }
        AnalyzeTool::Vartest { target, variance, ratio, tail, alpha, levels } => {
            let two = ratio.is_some() || (variance.is_none() && target.y.len() == 2);
            let kind = if two {
                analysis::VarTestKind::TwoSample { ratio: ratio.unwrap_or(1.0) }
            } else {
                analysis::VarTestKind::OneSample { variance: variance.unwrap_or(1.0) }
            };
            let spec =
                analysis::VarTestSpec { kind, tail: tail_of(tail), alpha, intervals: !levels.is_empty(), levels };
            (target, Job::Tool(Tool::VarTest(spec)))
        }
        AnalyzeTool::Proptest {
            target,
            successes,
            size,
            proportion,
            difference,
            tail,
            alpha,
            exact,
            exact_only,
            pooled,
            levels,
            power,
            power_sizes,
            power_alpha,
        } => {
            if successes.len() != size.len() || !(1..=2).contains(&successes.len()) {
                bail!(
                    "give one --successes and one --size for the one-sample test, two of each for the two-sample test"
                );
            }
            let kind = if successes.len() == 1 {
                analysis::PropTestKind::OneSample { successes: successes[0], size: size[0], proportion }
            } else {
                analysis::PropTestKind::TwoSample {
                    first: (successes[0], size[0]),
                    second: (successes[1], size[1]),
                    difference,
                }
            };
            let spec = analysis::PropTestSpec {
                kind,
                tail: tail_of(tail),
                alpha,
                intervals: !levels.is_empty(),
                levels,
                normal: !exact_only,
                exact,
                pooled,
                power: analysis::PropPowerSpec {
                    actual: power,
                    hypothetical: !power_sizes.is_empty(),
                    alpha: power_alpha,
                    sizes: if power_sizes.is_empty() { analysis::PropPowerSpec::default().sizes } else { power_sizes },
                },
            };
            (target, Job::Tool(Tool::PropTest(spec)))
        }
        AnalyzeTool::Poissontest {
            target,
            total,
            size,
            exposure,
            rate,
            difference,
            tail,
            alpha,
            no_exact,
            no_normal,
            unpooled,
            levels,
        } => {
            if total.len() != size.len() || !(1..=2).contains(&total.len()) {
                bail!("give one --total and one --size for the one-sample test, two of each for the two-sample test");
            }
            if exposure.len() > total.len() {
                bail!("give at most one --exposure per sample");
            }
            let exposure_of = |i: usize| exposure.get(i).or(exposure.first()).copied().unwrap_or(1.0);
            let kind = if total.len() == 1 {
                analysis::PoissonTestKind::OneSample { total: total[0], size: size[0], exposure: exposure_of(0), rate }
            } else {
                analysis::PoissonTestKind::TwoSample {
                    first: (total[0], size[0]),
                    second: (total[1], size[1]),
                    exposures: (exposure_of(0), exposure_of(1)),
                    difference,
                }
            };
            let spec = analysis::PoissonTestSpec {
                kind,
                tail: tail_of(tail),
                alpha,
                intervals: !levels.is_empty(),
                levels,
                normal: !no_normal,
                exact: !no_exact,
                pooled: !unpooled,
            };
            (target, Job::Tool(Tool::PoissonTest(spec)))
        }
        AnalyzeTool::Anova { target, comparison, control, alpha, .. } => {
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
        AnalyzeTool::Power { .. } | AnalyzeTool::Acceptance { .. } => {
            unreachable!("handled before the data file is read")
        }
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
        AnalyzeTool::Anova3 {
            target,
            no_ab,
            no_ac,
            no_bc,
            no_abc,
            comparison,
            comparison_alpha,
            levene,
            power,
            power_sizes,
            power_alpha,
            no_descriptive,
            alpha,
        } => {
            no_x_range(&target, "the three-way ANOVA")?;
            if target.y.len() != 4 {
                bail!("the three-way ANOVA needs --y FACTOR_A,FACTOR_B,FACTOR_C,DATA (four columns)");
            }
            let comparisons: Vec<analysis::MeansComparison> =
                comparison.into_iter().filter_map(comparison_of).collect();
            if comparisons.contains(&analysis::MeansComparison::Dunnett) {
                bail!("Dunnett's test is not part of the three-way ANOVA");
            }
            let defaults = analysis::ThreeWayAnovaSpec::default();
            (
                target,
                Job::Tool(Tool::ThreeWayAnova(analysis::ThreeWayAnovaSpec {
                    terms: analysis::ThreeWayTerms { ab: !no_ab, ac: !no_ac, bc: !no_bc, abc: !no_abc },
                    levene,
                    descriptive: if no_descriptive { Vec::new() } else { defaults.descriptive },
                    descriptive_overall: !no_descriptive,
                    comparisons,
                    comparison_alpha,
                    power_actual: power,
                    power_sizes,
                    power_alpha,
                    alpha,
                })),
            )
        }
        AnalyzeTool::Rmanova { target, two_way, interaction, comparison, comparison_alpha, no_descriptive, alpha } => {
            no_x_range(&target, "the repeated measures ANOVA")?;
            let wanted = if two_way { 4 } else { 3 };
            if target.y.len() != wanted {
                bail!(
                    "the {} repeated measures ANOVA needs --y {} ({wanted} columns)",
                    if two_way { "two-way" } else { "one-way" },
                    if two_way { "FACTOR_A,FACTOR_B,DATA,SUBJECT" } else { "FACTOR,DATA,SUBJECT" }
                );
            }
            let given = !comparison.is_empty();
            let comparisons: Vec<analysis::MeansComparison> =
                comparison.into_iter().filter_map(comparison_of).collect();
            if comparisons.contains(&analysis::MeansComparison::Dunnett) {
                bail!("Dunnett's test is not part of the repeated measures ANOVA");
            }
            let mut spec = analysis::RmAnovaSpec::new(two_way);
            if two_way {
                if !comparisons.is_empty() {
                    bail!("the two-way repeated measures ANOVA has no means comparisons (not measured in Origin)");
                }
            } else if given {
                spec.comparisons = comparisons;
            }
            if interaction && !two_way {
                bail!("--interaction belongs to the two-way design (--two-way)");
            }
            spec.interaction = interaction;
            if no_descriptive {
                spec.descriptive = Vec::new();
            }
            // As in the window, the A*B cells are not listed unless asked for: the CLI has no flag for them yet.
            spec.comparison_alpha = comparison_alpha;
            spec.alpha = alpha;
            (target, Job::Tool(Tool::RmAnova(spec)))
        }
        AnalyzeTool::PartialCorr { target, control, listwise, one_tailed, show_significance, flag_significant } => {
            no_x_range(&target, "the partial correlation")?;
            (
                target,
                Job::PartialCorr {
                    controls: control,
                    spec: analysis::PartialCorrSpec {
                        controls: Vec::new(),
                        listwise,
                        one_tailed,
                        show_significance,
                        flag_significant,
                    },
                },
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
        AnalyzeTool::Stemleaf { target, leaf_unit, split, max_leaves, trim_outliers } => (
            target,
            Job::Tool(Tool::StemLeaf(analysis::StemLeafOptions {
                leaf_unit,
                auto_split: split,
                max_leaves,
                trim_outliers,
            })),
        ),
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
        AnalyzeTool::Probplot { target, family, no_graph, scores, band } => (
            target,
            Job::Tool(Tool::ProbPlot(analysis::ProbPlotSpec {
                kind: analysis::ProbPlotKind::Probability,
                family: family_of(family),
                graph: !no_graph,
                scores: scores_of(scores),
                band,
                data_on_x: false,
            })),
        ),
        AnalyzeTool::Qqplot { target, family, no_graph, scores, data_on_x } => (
            target,
            Job::Tool(Tool::ProbPlot(analysis::ProbPlotSpec {
                kind: analysis::ProbPlotKind::QQ,
                family: family_of(family),
                graph: !no_graph,
                scores: scores_of(scores),
                band: None,
                data_on_x,
            })),
        ),
        AnalyzeTool::Pca {
            target,
            covariance,
            pairwise,
            components,
            standardize_scores,
            no_scores,
            no_graph,
            no_loading_plot,
            no_biplot,
            score_plot,
            x_component,
            y_component,
        } => (
            target,
            Job::Tool(Tool::Pca(analysis::PcaSpec {
                correlation: !covariance,
                pairwise,
                components,
                standardize_scores,
                scores: !no_scores,
                scree: !no_graph,
                loading_plot: !no_graph && !no_loading_plot,
                score_plot: score_plot && !no_graph,
                biplot: !no_graph && !no_biplot,
                x_component,
                y_component,
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
        AnalyzeTool::Qpcr { target, control, e_target, e_reference, confidence, no_graph } => (
            target,
            Job::Tool(Tool::Qpcr(analysis::QpcrSpec {
                control,
                e_target,
                e_reference,
                confidence: confidence / 100.0,
                graph: !no_graph,
            })),
        ),
        AnalyzeTool::QpcrCurve { target, no_graph } => {
            (target, Job::Tool(Tool::QpcrCurve(analysis::QpcrCurveSpec { graph: !no_graph })))
        }
        AnalyzeTool::Cox { target, censor, confidence, ties, strata, at, residuals, ph_test, frequency, no_graph } => {
            let efron = match ties.to_ascii_lowercase().as_str() {
                "breslow" => false,
                "efron" => true,
                other => bail!("--ties takes breslow or efron, not {other:?}"),
            };
            (
                target,
                Job::Tool(Tool::Cox(analysis::CoxSpec {
                    censor,
                    confidence: confidence / 100.0,
                    graph: !no_graph,
                    frequency,
                    efron,
                    strata,
                    at,
                    residuals,
                    ph_test,
                })),
            )
        }
        AnalyzeTool::Lifereg { target, distribution, censor, frequency, percentiles, at, confidence } => {
            let dist = match distribution.to_ascii_lowercase().as_str() {
                "weibull" => analysis::LifeDist::Weibull,
                "lognormal" => analysis::LifeDist::Lognormal,
                "exponential" => analysis::LifeDist::Exponential,
                other => bail!("--distribution takes weibull, lognormal or exponential, not {other:?}"),
            };
            (
                target,
                Job::Tool(Tool::LifeRegression(analysis::LifeRegressionSpec {
                    dist,
                    censor,
                    frequency,
                    percentiles: percentiles.iter().map(|p| p / 100.0).collect(),
                    at,
                    confidence: confidence / 100.0,
                })),
            )
        }
        AnalyzeTool::Turnbull { target, no_graph, frequency } => {
            (target, Job::Tool(Tool::Turnbull(analysis::TurnbullSpec { graph: !no_graph, frequency })))
        }
        AnalyzeTool::LifedistInterval { target, distribution, no_graph, frequency } => {
            let dist = match distribution.to_ascii_lowercase().as_str() {
                "weibull" => analysis::LifeDist::Weibull,
                "lognormal" => analysis::LifeDist::Lognormal,
                "exponential" => analysis::LifeDist::Exponential,
                "normal" => analysis::LifeDist::Normal,
                other => bail!("--distribution takes weibull, lognormal, exponential or normal, not {other:?}"),
            };
            (
                target,
                Job::Tool(Tool::LifeDistArbitrary(analysis::LifeDistArbitrarySpec {
                    dist,
                    graph: !no_graph,
                    frequency,
                })),
            )
        }
        AnalyzeTool::Lifedist { target, distribution, censor, no_graph, frequency } => {
            let dist = match distribution.to_ascii_lowercase().as_str() {
                "weibull" => analysis::LifeDist::Weibull,
                "lognormal" => analysis::LifeDist::Lognormal,
                "exponential" => analysis::LifeDist::Exponential,
                "normal" => analysis::LifeDist::Normal,
                other => bail!("--distribution takes weibull, lognormal, exponential or normal, not {other:?}"),
            };
            (target, Job::Tool(Tool::LifeDist(analysis::LifeDistSpec { dist, censor, graph: !no_graph, frequency })))
        }
        AnalyzeTool::Stability { target, lsl, usl, confidence, alpha, no_graph } => (
            target,
            Job::Tool(Tool::Stability(analysis::StabilitySpec {
                lower: lsl,
                upper: usl,
                confidence: confidence / 100.0,
                alpha,
                graph: !no_graph,
            })),
        ),
        AnalyzeTool::Capability { target, lsl, usl, subgroup, model, lambda, cpm_target, confidence, no_graph } => {
            let model = match model.to_ascii_lowercase().as_str() {
                "normal" => analysis::CapModel::Normal,
                "lognormal" => analysis::CapModel::Lognormal,
                "weibull" => analysis::CapModel::Weibull,
                "boxcox" | "box-cox" => analysis::CapModel::BoxCox,
                other => bail!("--model takes normal, lognormal, weibull or boxcox, not {other:?}"),
            };
            if lambda.is_some() && model != analysis::CapModel::BoxCox {
                bail!("--lambda goes with --model boxcox");
            }
            (
                target,
                Job::Tool(Tool::Capability(analysis::CapabilitySpec {
                    lsl,
                    usl,
                    subgroup,
                    model,
                    graph: !no_graph,
                    lambda,
                    target: cpm_target,
                    confidence: confidence / 100.0,
                })),
            )
        }
        AnalyzeTool::Rarechart { target, chart, limits, distribution, sigma, no_graph } => {
            use analysis::{GLimits, RareChartKind, TDistribution};
            let kind = match chart.to_ascii_lowercase().as_str() {
                "g" => RareChartKind::G,
                "t" => RareChartKind::T,
                other => bail!("--chart takes g or t, not {other:?}"),
            };
            let g_limits = match limits.to_ascii_lowercase().as_str() {
                "probability" => GLimits::Probability,
                "sigma" => GLimits::Sigma,
                other => bail!("--limits takes probability or sigma, not {other:?}"),
            };
            let t_distribution = match distribution.to_ascii_lowercase().as_str() {
                "weibull" => TDistribution::Weibull,
                "exponential" => TDistribution::Exponential,
                other => bail!("--distribution takes weibull or exponential, not {other:?}"),
            };
            (
                target,
                Job::Tool(Tool::RareChart(analysis::RareChartSpec {
                    kind,
                    g_limits,
                    t_distribution,
                    sigma,
                    graph: !no_graph,
                })),
            )
        }
        AnalyzeTool::MonteCarlo { target, inputs, formula, samples, seed, lsl, usl, optimize, aim } => {
            let mut parsed = Vec::new();
            for text in &inputs {
                parsed.push(analysis::parse_monte_carlo_input(text).map_err(anyhow::Error::msg)?);
            }
            let mut controls = Vec::new();
            for text in &optimize {
                controls.extend(analysis::parse_monte_carlo_controls(text).map_err(anyhow::Error::msg)?);
            }
            (
                target,
                Job::Tool(Tool::MonteCarlo(analysis::MonteCarloSpec {
                    inputs: parsed,
                    formula,
                    samples,
                    seed,
                    lsl,
                    usl,
                    optimize: controls,
                    goal: aim.map_or(analysis::McGoal::Capability, analysis::McGoal::Target),
                })),
            )
        }
        AnalyzeTool::MixtureDesign {
            target,
            components,
            centroid_design,
            degree,
            axial,
            centroid,
            replicates,
            totals,
            lower,
            no_randomize,
            seed,
        } => (
            target,
            Job::Tool(Tool::MixtureDesign(analysis::MixtureDesignSpec {
                components,
                kind: if centroid_design {
                    analysis::MixtureKind::SimplexCentroid
                } else {
                    analysis::MixtureKind::SimplexLattice
                },
                degree,
                axial,
                centroid,
                replicates,
                totals,
                lower,
                randomize: !no_randomize,
                seed,
            })),
        ),
        AnalyzeTool::FactorialDesign { target, factors, fraction, replicates, center_points, no_randomize, seed } => (
            target,
            Job::Tool(Tool::FactorialDesign(analysis::FactorialDesignSpec {
                factors,
                fraction,
                replicates,
                center_points,
                randomize: !no_randomize,
                seed,
            })),
        ),
        AnalyzeTool::SplitPlotDesign {
            target,
            whole_plot_factors,
            subplot_factors,
            replicates,
            no_randomize,
            seed,
        } => (
            target,
            Job::Tool(Tool::SplitPlotDesign(analysis::SplitPlotDesignSpec {
                whole_plot_factors,
                subplot_factors,
                replicates,
                randomize: !no_randomize,
                seed,
            })),
        ),
        AnalyzeTool::SplitPlotEffects { target, whole_plot_factors, no_graph } => (
            target,
            Job::Tool(Tool::SplitPlotEffects(analysis::SplitPlotEffectsSpec { whole_plot_factors, graph: !no_graph })),
        ),
        AnalyzeTool::FactorialEffects { target, no_graph } => {
            (target, Job::Tool(Tool::FactorialEffects(analysis::FactorialEffectsSpec { graph: !no_graph })))
        }
        AnalyzeTool::Randomforest { target, task, trees, mtry, min_leaf, max_depth, seed, no_graph } => {
            let task = match task.to_ascii_lowercase().as_str() {
                "auto" => analysis::ForestTask::Auto,
                "classification" => analysis::ForestTask::Classification,
                "regression" => analysis::ForestTask::Regression,
                other => bail!("--task takes auto, classification or regression, not {other:?}"),
            };
            (
                target,
                Job::Tool(Tool::RandomForest(analysis::RandomForestSpec {
                    task,
                    trees,
                    mtry,
                    min_leaf,
                    max_depth,
                    seed,
                    graph: !no_graph,
                })),
            )
        }
        AnalyzeTool::Arima {
            target,
            p,
            d,
            q,
            no_constant,
            horizon,
            confidence,
            auto,
            max_p,
            max_q,
            sp,
            sd,
            sq,
            period,
            no_graph,
        } => (
            target,
            Job::Tool(Tool::Arima(analysis::ArimaSpec {
                p,
                d,
                q,
                constant: !no_constant,
                horizon,
                confidence: confidence / 100.0,
                graph: !no_graph,
                auto: auto.then_some((max_p, max_q)),
                seasonal: analysis::ArimaSeasonal { p: sp, d: sd, q: sq, period },
            })),
        ),
        AnalyzeTool::Forecast { target, model, period, constants, horizon, damped, phi, confidence, no_graph } => {
            let model = match model.to_ascii_lowercase().as_str() {
                "simple" => analysis::SmoothModel::Simple,
                "holt" => analysis::SmoothModel::Holt,
                "additive" => analysis::SmoothModel::WintersAdditive,
                "multiplicative" => analysis::SmoothModel::WintersMultiplicative,
                other => bail!("--model takes simple, holt, additive or multiplicative, not {other:?}"),
            };
            let constants = match constants {
                None => None,
                Some(text) => {
                    let v: Vec<f64> = text
                        .split(',')
                        .map(|t| t.trim().parse::<f64>())
                        .collect::<std::result::Result<_, _>>()
                        .map_err(|_| anyhow::anyhow!("--constants takes numbers separated by commas (got {text:?})"))?;
                    if v.len() != model.constants() {
                        bail!("--constants: the {} model takes {} constant(s)", model.name(), model.constants());
                    }
                    let mut c = [0.0; 3];
                    c[..v.len()].copy_from_slice(&v);
                    Some(c)
                }
            };
            (
                target,
                Job::Tool(Tool::ExpSmooth(analysis::ExpSmoothSpec {
                    model,
                    period,
                    constants,
                    horizon,
                    graph: !no_graph,
                    damping: match (damped, phi) {
                        (_, Some(p)) => analysis::Damping::Given(p),
                        (true, None) => analysis::Damping::Fitted,
                        (false, None) => analysis::Damping::None,
                    },
                    confidence: confidence / 100.0,
                })),
            )
        }
        AnalyzeTool::Stationarity { target, adf_terms, adf_lags, kpss_null, kpss_lags } => {
            let adf_terms = match adf_terms.to_ascii_lowercase().as_str() {
                "constant" => analysis::AdfTerms::Constant,
                "trend" => analysis::AdfTerms::Trend,
                "none" => analysis::AdfTerms::None,
                other => bail!("--adf-terms takes constant, trend or none, not {other:?}"),
            };
            let kpss_null = match kpss_null.to_ascii_lowercase().as_str() {
                "level" => analysis::KpssNull::Level,
                "trend" => analysis::KpssNull::Trend,
                other => bail!("--kpss-null takes level or trend, not {other:?}"),
            };
            (
                target,
                Job::Tool(Tool::Stationarity(analysis::StationaritySpec { adf_terms, adf_lags, kpss_null, kpss_lags })),
            )
        }
        AnalyzeTool::Decompose { target, period, model, no_graph } => {
            let model = match model.to_ascii_lowercase().as_str() {
                "additive" => analysis::DecomposeModel::Additive,
                "multiplicative" => analysis::DecomposeModel::Multiplicative,
                other => bail!("--model takes additive or multiplicative, not {other:?}"),
            };
            (target, Job::Tool(Tool::Decompose(analysis::DecomposeSpec { period, model, graph: !no_graph })))
        }
        AnalyzeTool::Acf { target, lags, confidence, no_graph } => (
            target,
            Job::Tool(Tool::Acf(analysis::AcfSpec {
                lags: lags.unwrap_or(0),
                confidence: confidence / 100.0,
                graph: !no_graph,
            })),
        ),
        AnalyzeTool::Manhattan { target, threshold, no_graph } => {
            (target, Job::Tool(Tool::Manhattan(analysis::ManhattanSpec { threshold, graph: !no_graph })))
        }
        AnalyzeTool::Timepivot { target, period, stat, every, fiscal_start, by } => {
            let by = match by.as_deref().map(|b| b.trim().to_ascii_lowercase()) {
                None => None,
                Some(b) if b == "hour" => Some(analysis::PivotBy::HourOfDay),
                Some(b) if b == "weekday" || b == "day" => Some(analysis::PivotBy::DayOfWeek),
                Some(b) if b == "month" => Some(analysis::PivotBy::MonthOfYear),
                Some(other) => bail!("--by must be hour, weekday or month (got \"{other}\")"),
            };
            if every == 0 {
                bail!("--every must be at least 1");
            }
            if !(1..=12).contains(&fiscal_start) {
                bail!("--fiscal-start must be a month from 1 to 12 (got {fiscal_start})");
            }
            let period = match analysis::PivotPeriod::ALL
                .into_iter()
                .find(|p| p.name().eq_ignore_ascii_case(period.trim()))
            {
                Some(p) => p,
                None => bail!("--period must be minute, hour, day, week, month, quarter or year (got \"{period}\")"),
            };
            let stat = match stat.trim().to_ascii_lowercase().as_str() {
                "sum" => analysis::PivotStat::Sum,
                "mean" => analysis::PivotStat::Mean,
                "count" => analysis::PivotStat::Count,
                "min" | "minimum" => analysis::PivotStat::Min,
                "max" | "maximum" => analysis::PivotStat::Max,
                "median" => analysis::PivotStat::Median,
                other => bail!("--stat must be sum, mean, count, min, max or median (got \"{other}\")"),
            };
            (target, Job::Tool(Tool::TimePivot(analysis::TimePivotSpec { period, stat, every, fiscal_start, by })))
        }
        AnalyzeTool::Binscatter { target, bins, equal_width, no_graph } => {
            if bins == 0 {
                bail!("--bins must be at least 1");
            }
            let rule =
                if equal_width { analysis::BinRule::EqualWidth(bins) } else { analysis::BinRule::EqualCount(bins) };
            (target, Job::Tool(Tool::BinScatter(analysis::BinScatterSpec { rule, graph: !no_graph })))
        }
        AnalyzeTool::Changepoint { target, change, penalty, min_segment, no_graph } => {
            let kind = match change.trim().to_ascii_lowercase().as_str() {
                "mean" => analysis::ChangeKind::Mean,
                "variance" | "var" => analysis::ChangeKind::Variance,
                "meanvar" | "meanvariance" | "both" => analysis::ChangeKind::MeanVariance,
                other => bail!("--change must be mean, variance or meanvar (got \"{other}\")"),
            };
            let penalty = match penalty.trim().to_ascii_lowercase().as_str() {
                "mbic" => analysis::ChangePenalty::Mbic,
                "bic" | "sic" => analysis::ChangePenalty::Bic,
                "aic" => analysis::ChangePenalty::Aic,
                "hq" | "hannan-quinn" => analysis::ChangePenalty::HannanQuinn,
                other => match other.parse::<f64>() {
                    Ok(v) if v.is_finite() && v >= 0.0 => analysis::ChangePenalty::Manual(v),
                    _ => bail!("--penalty must be mbic, bic, aic, hq or a number of at least 0 (got \"{other}\")"),
                },
            };
            (
                target,
                Job::Tool(Tool::ChangePoint(analysis::ChangePointSpec {
                    kind,
                    penalty,
                    min_segment,
                    graph: !no_graph,
                })),
            )
        }
        AnalyzeTool::Quantreg { target, quantiles, no_intercept, no_graph, se } => (
            target,
            Job::Tool(Tool::QuantReg(analysis::QuantRegSpec {
                quantiles,
                intercept: !no_intercept,
                graph: !no_graph,
                se: match se.as_str() {
                    "iid" => Some(analysis::QuantRegSe::Iid),
                    "none" => None,
                    _ => Some(analysis::QuantRegSe::Robust),
                },
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
            frequency,
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
                frequency,
            })),
        ),
        AnalyzeTool::GraphSummary { target, groups, level, arrangement } => {
            no_x_range(&target, "the graphical summary")?;
            let Some(arrangement) = analysis::GraphArrangement::from_word(&arrangement) else {
                bail!("--arrangement is separate, per-variable, per-group or all-in-one; got \"{arrangement}\"");
            };
            (target, Job::GraphSummary { groups, level, arrangement })
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
            PowerTestArg::OneProportion => analysis::PowerKind::OneProportion,
            PowerTestArg::TwoProportions => analysis::PowerKind::TwoProportions,
            PowerTestArg::OneVariance => analysis::PowerKind::OneVariance,
            PowerTestArg::TwoVariances => analysis::PowerKind::TwoVariances,
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

fn wavelet_ext(ext: WaveletExtArg) -> analysis::WaveletExtension {
    match ext {
        WaveletExtArg::Periodic => analysis::WaveletExtension::Periodic,
        WaveletExtArg::Pad0 => analysis::WaveletExtension::ZeroPadded,
    }
}

fn wavelet_arg(text: &str) -> Result<Wavelet> {
    Wavelet::parse(text).ok_or_else(|| {
        anyhow!("{text:?} is not a wavelet: haar, db2..db10, b11, b13, b15, b22, b24, b26, b28, b31, b33, b35 or b37")
    })
}

fn fft_window(w: FftWindowArg) -> analysis::FftWindow {
    match w {
        FftWindowArg::Rectangle => analysis::FftWindow::Rectangle,
        FftWindowArg::Welch => analysis::FftWindow::Welch,
        FftWindowArg::Triangular => analysis::FftWindow::Triangular,
        FftWindowArg::Bartlett => analysis::FftWindow::Bartlett,
        FftWindowArg::Hanning => analysis::FftWindow::Hanning,
        FftWindowArg::Hamming => analysis::FftWindow::Hamming,
        FftWindowArg::Blackman => analysis::FftWindow::Blackman,
    }
}

fn scores_of(s: ScoresArg) -> analysis::PlotScores {
    match s {
        ScoresArg::Filliben => analysis::PlotScores::Filliben,
        ScoresArg::Blom => analysis::PlotScores::Blom,
        ScoresArg::Benard => analysis::PlotScores::Benard,
        ScoresArg::Hazen => analysis::PlotScores::Hazen,
        ScoresArg::VanDerWaerden => analysis::PlotScores::VanDerWaerden,
        ScoresArg::KaplanMeier => analysis::PlotScores::KaplanMeier,
    }
}
