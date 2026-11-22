//! Statistics > Process Capability as a tool of `Project::analyze` (T365; Origin 2027's Quality improvement apps
//! add nonnormal capability, and Sheaf has no Origin 2016 capability window to copy, so the layout is Sheaf's own).
//! The numerics are in `sheaf_calc::capability`; `docs/spec/statistics.md` section 36 says what is computed and
//! written.
//!
//! Every chosen column is a process of its own, in row order with missing values left out.

use super::dist::{GraphPlot, Output, XyGraph};
use super::statistics::num;
use super::*;
use crate::graph::{PlotKind, ScaleKind};
pub use calc::capability::CapModel;
use calc::capability::{capability, fitted_cdf};

/// What [`Tool::Capability`] does.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CapabilitySpec {
    pub lsl: Option<f64>,
    pub usl: Option<f64>,
    /// 1 for individual values (moving ranges), else the subgroup size (2 to 10).
    pub subgroup: usize,
    pub model: CapModel,
    pub graph: bool,
}

impl Default for CapabilitySpec {
    fn default() -> Self {
        CapabilitySpec { lsl: None, usl: None, subgroup: 1, model: CapModel::Normal, graph: true }
    }
}

const HEADERS: [&str; 21] = [
    "N",
    "Mean",
    "StdDev (overall)",
    "StdDev (within)",
    "Cp",
    "CPL",
    "CPU",
    "Cpk",
    "Pp",
    "PPL",
    "PPU",
    "Ppk",
    "PPM < LSL (observed)",
    "PPM > USL (observed)",
    "PPM total (observed)",
    "PPM < LSL (expected overall)",
    "PPM > USL (expected overall)",
    "PPM total (expected overall)",
    "PPM total (expected within)",
    "Fitted parameter 1",
    "Fitted parameter 2",
];

fn column_of(name: &str, comments: &str, designation: Designation, values: Vec<f64>) -> SheetColumn {
    SheetColumn {
        designation,
        long_name: name.to_string(),
        units: String::new(),
        comments: comments.to_string(),
        data: ColumnData::numeric(values),
    }
}

pub(super) fn run(spec: &CapabilitySpec, inputs: &[Input<'_>]) -> Result<Output, AnalysisError> {
    let mut table = ResultTable::new(&HEADERS);
    let mut notes = Vec::new();
    let mut on_sheet = Vec::new();
    let mut graphs = Vec::new();
    for input in inputs {
        let c = capability(input.ys(), spec.lsl, spec.usl, spec.subgroup, spec.model)
            .map_err(|e| AnalysisError::BadOption(format!("{}: {e}", input.label)))?;
        let values = vec![
            c.n as f64,
            c.mean,
            c.sd_overall,
            c.sd_within,
            c.cp,
            c.cpl,
            c.cpu,
            c.cpk,
            c.pp,
            c.ppl,
            c.ppu,
            c.ppk,
            c.observed.below,
            c.observed.above,
            c.observed.total,
            c.expected_overall.below,
            c.expected_overall.above,
            c.expected_overall.total,
            c.expected_within.total,
            c.params[0],
            c.params[1],
        ];
        let mut units = vec![String::new(); values.len()];
        for u in &mut units[1..4] {
            u.clone_from(&input.units);
        }
        table.push_row(input.label.clone(), values, units);
        notes.push(match spec.model {
            CapModel::Normal => format!(
                "{}: {} values, mean {}, within sigma {} ({}), overall sigma {}; Cpk {}, Ppk {}; expected {} PPM outside (overall).",
                input.label,
                c.n,
                num(c.mean),
                num(c.sd_within),
                if spec.subgroup == 1 {
                    "moving range of 2".to_string()
                } else {
                    format!("R bar / d2 of subgroups of {}", spec.subgroup)
                },
                num(c.sd_overall),
                num(c.cpk),
                num(c.ppk),
                num(c.expected_overall.total)
            ),
            model => format!(
                "{}: {} values, {} fit ({}, {}); percentile method Pp {}, Ppk {}; expected {} PPM outside.",
                input.label,
                c.n,
                model.name(),
                num(c.params[0]),
                num(c.params[1]),
                num(c.pp),
                num(c.ppk),
                num(c.expected_overall.total)
            ),
        });
        if spec.graph {
            let label = format!("Capability of {}", input.name);
            let x: Vec<f64> = input.ys().iter().copied().filter(|v| v.is_finite()).collect();
            let (lo, hi) = x.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), v| (a.min(*v), b.max(*v)));
            // Sturges' bins over the data.
            let bins = ((x.len() as f64).log2().ceil() as usize + 1).max(1);
            let w = if hi > lo { (hi - lo) / bins as f64 } else { 1.0 };
            let mut counts = vec![0.0; bins];
            for v in &x {
                counts[(((v - lo) / w) as usize).min(bins - 1)] += 1.0;
            }
            let centers: Vec<f64> = (0..bins).map(|i| lo + (i as f64 + 0.5) * w).collect();
            let top = counts.iter().copied().fold(0.0, f64::max);
            let first = on_sheet.len();
            on_sheet.push(column_of("Bin center", &label, Designation::X, centers));
            on_sheet.push(column_of("Count", &label, Designation::Y, counts));
            let mut plots = vec![GraphPlot {
                x: first,
                y: first + 1,
                kind: PlotKind::Column,
                label: input.label.clone(),
                line_of: None,
            }];
            // The fitted density scaled to counts, n w f(x), with f from a central difference of the CDF.
            let a = spec.lsl.map_or(lo, |l| l.min(lo)) - w;
            let b = spec.usl.map_or(hi, |u| u.max(hi)) + w;
            let a = if spec.model == CapModel::Normal { a } else { a.max(0.0) };
            let h = w * 1e-3;
            let xs: Vec<f64> = (0..=200).map(|i| a + (b - a) * f64::from(i) / 200.0).collect();
            let ys: Vec<f64> = xs
                .iter()
                .map(|t| {
                    let f = (fitted_cdf(spec.model, &c.params, t + h) - fitted_cdf(spec.model, &c.params, t - h))
                        / (2.0 * h);
                    x.len() as f64 * w * f
                })
                .collect();
            let x2 = on_sheet.len();
            on_sheet.push(column_of("Curve X", &label, Designation::X, xs));
            on_sheet.push(column_of(&format!("Fitted {}", spec.model.name()), &label, Designation::Y, ys));
            plots.push(GraphPlot {
                x: x2,
                y: x2 + 1,
                kind: PlotKind::Line,
                label: format!("Fitted {}", spec.model.name()),
                line_of: Some(0),
            });
            for (name, limit) in [("LSL", spec.lsl), ("USL", spec.usl)] {
                let Some(l) = limit else { continue };
                let x3 = on_sheet.len();
                on_sheet.push(column_of(&format!("{name} X"), &label, Designation::X, vec![l, l]));
                on_sheet.push(column_of(name, &label, Designation::Y, vec![0.0, top * 1.1]));
                plots.push(GraphPlot {
                    x: x3,
                    y: x3 + 1,
                    kind: PlotKind::Line,
                    label: format!("{name} = {}", num(l)),
                    line_of: Some(0),
                });
            }
            graphs.push(XyGraph {
                plots,
                x_scale: ScaleKind::Linear,
                y_scale: ScaleKind::Linear,
                x_title: input.name.clone(),
                y_title: "Count".into(),
                look: Default::default(),
            });
        }
    }
    notes.push(match spec.model {
        CapModel::Normal => {
            "Cp = (USL - LSL) / 6 s, CPL = (mean - LSL) / 3 s, CPU = (USL - mean) / 3 s with the within s; Pp, PPL, PPU and Ppk the same with the overall s.".to_string()
        }
        _ => "Percentile method: Pp = (USL - LSL) / (x99.865 - x0.135), PPL = (median - LSL) / (median - x0.135), PPU = (USL - median) / (x99.865 - median); within indices are not defined.".to_string(),
    });
    let mut sheet = vec![SheetColumn {
        designation: Designation::Label,
        long_name: "Data".into(),
        units: String::new(),
        comments: String::new(),
        data: ColumnData::text(table.rows.iter().map(|(l, _)| l.clone()).collect()),
    }];
    for (i, h) in table.headers.iter().enumerate() {
        sheet.push(column_of(h, "", Designation::Y, table.rows.iter().map(|(_, v)| v[i]).collect()));
    }
    sheet.extend(on_sheet);
    let (graph, more_graphs) = if graphs.is_empty() {
        (None, Vec::new())
    } else {
        let first = graphs.remove(0);
        (Some(first), graphs)
    };
    Ok(Output { table: Some(table), on_sheet: sheet, notes, graph, more_graphs })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(columns: Vec<ColumnId>, spec: CapabilitySpec) -> Request {
        Request { columns, rows: RowRange::All, tool: Tool::Capability(spec) }
    }

    #[test]
    fn normal_and_weibull_capability_with_a_histogram() {
        let mut p = Project::new();
        let book = p.add_workbook("cap");
        let y = p
            .add_column(
                book,
                0,
                Designation::Y,
                ColumnData::numeric(vec![10.0, 12.0, 11.0, f64::NAN, 13.0, 12.0, 14.0]),
            )
            .unwrap();
        let spec = CapabilitySpec { lsl: Some(5.0), usl: Some(20.0), ..Default::default() };
        let out = p.analyze(&request(vec![y], spec)).unwrap();
        let t = out.table.as_ref().unwrap();
        assert_eq!(t.rows[0].1[0], 6.0, "the missing value is left out");
        let sw = t.rows[0].1[3];
        assert!((sw - 1.6 / std::f64::consts::FRAC_2_SQRT_PI).abs() < 1e-6);
        assert!((t.rows[0].1[7] - 7.0 / (3.0 * sw)).abs() < 1e-12);
        assert!(out.new_graph.is_some());

        let spec = CapabilitySpec { usl: Some(30.0), model: CapModel::Weibull, graph: false, ..Default::default() };
        let out = p.analyze(&request(vec![y], spec)).unwrap();
        let t = out.table.as_ref().unwrap();
        assert!(t.rows[0].1[7].is_nan() && t.rows[0].1[11] > 1.0, "{:?}", t.rows[0].1);
        assert!(out.new_graph.is_none());
        assert!(p.analyze(&request(vec![y], CapabilitySpec::default())).is_err(), "no limits");
    }
}
