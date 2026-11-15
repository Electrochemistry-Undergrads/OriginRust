//! The IIR Filter and FIR Filter tools of `Project::analyze` (Analysis > Signal Processing; T55,
//! analysis-tools.md 17). The filters are designed and applied by `sheaf_calc::{iir, fir}`; this
//! file reads the input, takes the sample frequency from the X column when the frequencies are in
//! Hz and writes the filtered column next to the input, with the filter in its comment.

use super::*;
use sheaf_calc::{FirMethod, FirSpec, FreqUnit, IirSpec};

fn error(e: calc::FilterError, name: &str) -> AnalysisError {
    AnalysisError::BadOption(format!("{name}: {e}"))
}

/// `1.5`, `20`, `0.123457`: six decimals at most, no trailing zeros.
fn num(v: f64) -> String {
    let s = format!("{v:.6}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// Frequencies (normalized, Nyquist = 1) as the user gave them: Hz with the sample frequency, or normalized.
fn freqs(wn: &[f64], unit: FreqUnit, fs: f64) -> String {
    let conv = |w: f64| if unit == FreqUnit::Hz { w * fs / 2.0 } else { w };
    let list = wn.iter().map(|w| num(conv(*w))).collect::<Vec<_>>().join(" to ");
    if unit == FreqUnit::Hz { format!("{list} Hz") } else { list }
}

fn word(r: calc::FilterResponse) -> &'static str {
    r.label()
}

pub(super) fn run_iir(spec: &IirSpec, inputs: &[Input], beside: &mut Vec<NewColumn>) -> Result<(), AnalysisError> {
    for inp in inputs {
        let y = signal_values(inp.ys(), &inp.short)?;
        let fs = 1.0 / auto_interval(inp.xs());
        let filter = spec.design(Some(fs)).map_err(|e| error(e, "IIR filter"))?;
        let out = filter.apply(&y, spec.forward_backward).map_err(|e| error(e, &inp.short))?;
        let fs_used = spec.sample_frequency.unwrap_or(fs);
        let mut how = format!("order {}, Fc {}", filter.order, freqs(&filter.wn, spec.unit, fs_used));
        if spec.order.is_none() {
            how = format!("minimum {how}");
        }
        if spec.forward_backward {
            how.push_str(", forward-backward");
        }
        beside.push(NewColumn {
            after: inp.id,
            long_name: format!("{} IIR filtered", inp.name),
            units: inp.units.clone(),
            comments: format!(
                "{} {} IIR filter of {} ({how}){}",
                spec.method.label(),
                word(spec.response),
                inp.short,
                inp.range_note()
            ),
            data: inp.aligned(&out),
        });
    }
    Ok(())
}

pub(super) fn run_fir(spec: &FirSpec, inputs: &[Input], beside: &mut Vec<NewColumn>) -> Result<(), AnalysisError> {
    for inp in inputs {
        let y = signal_values(inp.ys(), &inp.short)?;
        let fs = 1.0 / auto_interval(inp.xs());
        let filter = spec.design(Some(fs)).map_err(|e| error(e, "FIR filter"))?;
        let out = filter.apply(&y, spec.forward_backward).map_err(|e| error(e, &inp.short))?;
        let fs_used = spec.sample_frequency.unwrap_or(fs);
        let design = match (spec.method, spec.order) {
            (FirMethod::LeastSquares, _) => "least squares".to_string(),
            _ => format!("{} window", spec.window.label()),
        };
        let mut how = format!("order {}", filter.order());
        if spec.order.is_none() {
            how = format!("minimum {how}");
        }
        match (spec.method, spec.order) {
            (FirMethod::LeastSquares, _) => {
                let k = filter.edges.len() / 2;
                how.push_str(&format!(
                    ", pass {}, stop {}",
                    freqs(&filter.edges[..k], spec.unit, fs_used),
                    freqs(&filter.edges[k..], spec.unit, fs_used)
                ));
            }
            _ => how.push_str(&format!(", Fc {}", freqs(&filter.edges, spec.unit, fs_used))),
        }
        if spec.forward_backward {
            how.push_str(", forward-backward");
        }
        beside.push(NewColumn {
            after: inp.id,
            long_name: format!("{} FIR filtered", inp.name),
            units: inp.units.clone(),
            comments: format!(
                "{} {} FIR filter of {} ({how}){}",
                design,
                word(spec.response),
                inp.short,
                inp.range_note()
            ),
            data: inp.aligned(&out),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::column::RecalcMode;

    fn setup() -> (Project, ColumnId) {
        let mut p = Project::new();
        let book = p.add_workbook("Book1");
        let x: Vec<f64> = (0..20).map(|i| i as f64).collect();
        p.add_column(book, 0, Designation::X, ColumnData::numeric(x)).unwrap();
        let y = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![1.0; 20])).unwrap();
        (p, y)
    }

    fn req(y: ColumnId, tool: Tool) -> Request {
        Request { columns: vec![y], rows: RowRange::All, tool }
    }

    #[test]
    fn iir_and_fir_runs_follow_their_input_and_survive_json() {
        for tool in [
            Tool::IirFilter(IirSpec { order: Some(2), cutoff: Some(0.2), ..IirSpec::default() }),
            Tool::FirFilter(FirSpec { order: Some(4), cutoff: Some(0.2), ..FirSpec::default() }),
        ] {
            let (mut p, y) = setup();
            assert!(Project::can_link(&tool));
            let r = p.analyze_linked(&req(y, tool), &OutputTarget::NextToInput, RecalcMode::Auto).unwrap();
            let out = r.new_columns[0];
            let before = p.clone();
            p.column_mut(y).unwrap().data = ColumnData::numeric(vec![3.0; 20]);
            assert!(p.update_analysis_links(&before).is_empty());
            let got = p.numeric(out).unwrap().to_vec();
            if matches!(p.analysis_links[0].tool, crate::analysis_link::LinkTool::FirFilter(_)) {
                assert!(got.iter().all(|v| (v - 3.0).abs() < 1e-9), "{got:?}");
            } else {
                // one way starts from zero state: the end has settled at the level, the start has not
                assert!((got[19] - 3.0).abs() < 1e-3 && (got[0] - 3.0).abs() > 1.0, "{got:?}");
            }
            let text = serde_json::to_string(&p).unwrap();
            let back: Project = serde_json::from_str(&text).unwrap();
            assert_eq!(back.analysis_links, p.analysis_links);
        }
    }
}
