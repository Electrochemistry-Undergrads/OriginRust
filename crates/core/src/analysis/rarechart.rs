//! Statistics > Quality > Rare Event Chart (G/T) as a tool of `Project::analyze` (T364; Origin 2027's Quality
//! improvement apps, not in Origin 2016, so the layout is Sheaf's own). The numerics are in
//! `sheaf_calc::rarechart`; `docs/spec/statistics.md` section 35 says what is computed and written.
//!
//! Every chosen column is a chart of its own, in row order with missing values left out.

use super::dist::{GraphPlot, Output, XyGraph};
use super::statistics::num;
use super::*;
use crate::graph::{PlotKind, ScaleKind};
pub use calc::rarechart::{GLimits, TDistribution};
use calc::rarechart::{RareChart, g_chart, t_chart, tail};

/// Which chart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RareChartKind {
    /// Opportunities between events (counts, 0 or more).
    #[default]
    G,
    /// Time between events (above 0).
    T,
}

/// What [`Tool::RareChart`] does.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RareChartSpec {
    pub kind: RareChartKind,
    /// The limits of the G chart.
    pub g_limits: GLimits,
    /// The distribution of the T chart.
    pub t_distribution: TDistribution,
    /// The limits are at this many sigmas (or the matching normal tail probability; default 3).
    pub sigma: f64,
    pub graph: bool,
}

impl Default for RareChartSpec {
    fn default() -> Self {
        RareChartSpec {
            kind: RareChartKind::G,
            g_limits: GLimits::Probability,
            t_distribution: TDistribution::Weibull,
            sigma: 3.0,
            graph: true,
        }
    }
}

fn column_of(name: &str, comments: &str, designation: Designation, values: Vec<f64>) -> SheetColumn {
    SheetColumn {
        designation,
        long_name: name.to_string(),
        units: String::new(),
        comments: comments.to_string(),
        data: ColumnData::numeric(values),
    }
}

fn chart_of(spec: &RareChartSpec, values: &[f64]) -> Result<RareChart, calc::rarechart::RareChartError> {
    match spec.kind {
        RareChartKind::G => g_chart(values, spec.g_limits, spec.sigma),
        RareChartKind::T => t_chart(values, spec.t_distribution, spec.sigma),
    }
}

pub(super) fn run(spec: &RareChartSpec, inputs: &[Input<'_>]) -> Result<Output, AnalysisError> {
    let what = match spec.kind {
        RareChartKind::G => "G",
        RareChartKind::T => "T",
    };
    let mut table = ResultTable::new(&[
        "N",
        "Mean",
        "Parameter 1",
        "Parameter 2",
        "Center line",
        "Upper control limit",
        "Lower control limit",
        "Points above",
        "Points below",
    ]);
    let mut notes = Vec::new();
    let mut on_sheet = Vec::new();
    let mut graphs = Vec::new();
    for input in inputs {
        let c = chart_of(spec, input.ys()).map_err(|e| AnalysisError::BadOption(format!("{}: {e}", input.label)))?;
        let n = c.values.len();
        table.push_row(
            input.label.clone(),
            vec![
                n as f64,
                c.mean,
                c.params[0],
                c.params.get(1).copied().unwrap_or(f64::NAN),
                c.center,
                c.ucl,
                c.lcl,
                c.above.len() as f64,
                c.below.len() as f64,
            ],
            vec![String::new(), input.units.clone(), String::new(), String::new()]
                .into_iter()
                .chain(std::iter::repeat_n(input.units.clone(), 3))
                .chain([String::new(), String::new()])
                .collect(),
        );
        let fitted = match (spec.kind, spec.t_distribution) {
            (RareChartKind::G, _) => format!("geometric p = {}", num(c.params[0])),
            (RareChartKind::T, TDistribution::Exponential) => format!("exponential mean {}", num(c.params[0])),
            (RareChartKind::T, TDistribution::Weibull) => {
                format!("Weibull shape {}, scale {}", num(c.params[0]), num(c.params[1]))
            }
        };
        let flagged: Vec<String> = c.above.iter().chain(&c.below).map(|i| (i + 1).to_string()).collect();
        notes.push(format!(
            "{}: {what} chart of {n} values ({fitted}); CL {}, UCL {}, LCL {}; {}.",
            input.label,
            num(c.center),
            num(c.ucl),
            num(c.lcl),
            if flagged.is_empty() {
                "every point within the limits".to_string()
            } else {
                format!("point(s) {} beyond the limits", flagged.join(", "))
            }
        ));
        let label = format!("{what} chart of {}", input.name);
        let x = on_sheet.len();
        let points: Vec<f64> = (1..=n).map(|i| i as f64).collect();
        on_sheet.push(column_of("Point", &label, Designation::X, points));
        on_sheet.push(column_of(&input.name, &label, Designation::Y, c.values.clone()));
        let mut plots =
            vec![GraphPlot { x, y: x + 1, kind: PlotKind::LineSymbol, label: input.label.clone(), line_of: None }];
        for (name, v) in [("Center line", c.center), ("UCL", c.ucl), ("LCL", c.lcl)] {
            on_sheet.push(column_of(name, &label, Designation::Y, vec![v; n]));
            plots.push(GraphPlot {
                x,
                y: on_sheet.len() - 1,
                kind: PlotKind::Line,
                label: format!("{name} = {}", num(v)),
                line_of: Some(0),
            });
        }
        graphs.push(XyGraph {
            plots,
            x_scale: ScaleKind::Linear,
            y_scale: ScaleKind::Linear,
            x_title: "Point".into(),
            y_title: input.name.clone(),
            look: Default::default(),
        });
    }
    let limits = match (spec.kind, spec.g_limits) {
        (RareChartKind::G, GLimits::Sigma) => format!("gbar +- {} sqrt(gbar (gbar + 1))", num(spec.sigma)),
        _ => format!(
            "quantiles of the fitted distribution at {}, 0.5 and {}",
            num(tail(spec.sigma)),
            num(1.0 - tail(spec.sigma))
        ),
    };
    notes.push(format!("Limits: {limits}."));
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
    let shift = sheet.len();
    sheet.extend(on_sheet);
    let (graph, more_graphs) = if spec.graph && !graphs.is_empty() {
        let first = graphs.remove(0);
        (Some(first), graphs)
    } else {
        (None, Vec::new())
    };
    Ok(Output { table: Some(table), on_sheet: sheet, notes, graph, more_graphs }.shift_graphs(shift))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(columns: Vec<ColumnId>, spec: RareChartSpec) -> Request {
        Request { columns, rows: RowRange::All, tool: Tool::RareChart(spec) }
    }

    #[test]
    fn g_and_t_charts_flag_points_and_make_graphs() {
        let mut p = Project::new();
        let book = p.add_workbook("rare");
        let counts = vec![10.0, 25.0, 3.0, 40.0, 18.0, f64::NAN, 300.0, 12.0];
        let g = p.add_column(book, 0, Designation::Y, ColumnData::numeric(counts)).unwrap();
        let spec = RareChartSpec { g_limits: GLimits::Sigma, ..Default::default() };
        let out = p.analyze(&request(vec![g], spec)).unwrap();
        let t = out.table.as_ref().unwrap();
        assert_eq!(t.rows[0].1[0], 7.0, "the missing value is left out");
        assert!((t.rows[0].1[4] - 408.0 / 7.0).abs() < 1e-12);
        assert_eq!(t.rows[0].1[7], 1.0, "300 is above the limit");
        assert!(out.new_graph.is_some() && out.notes[0].contains("point(s) 6 beyond"), "{:?}", out.notes);
        let cols = super::super::dist::plot_columns(&p, out.new_graph.unwrap());
        assert_eq!(
            (cols[0].0.as_str(), cols[1].1.as_str(), cols[2].1.as_str()),
            ("Point", "Center line", "UCL"),
            "{cols:?}"
        );

        let times = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![2.0, 4.0, 6.0, 8.0])).unwrap();
        let spec = RareChartSpec {
            kind: RareChartKind::T,
            t_distribution: TDistribution::Exponential,
            graph: false,
            ..Default::default()
        };
        let out = p.analyze(&request(vec![times], spec)).unwrap();
        let t = out.table.as_ref().unwrap();
        assert!((t.rows[0].1[4] - 5.0 * 2f64.ln()).abs() < 1e-12);
        assert!(out.new_graph.is_none());
        // A count of 0 is no time between events.
        let zero = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![0.0, 4.0])).unwrap();
        assert!(
            p.analyze(&request(vec![zero], RareChartSpec { kind: RareChartKind::T, ..Default::default() })).is_err()
        );
        assert!(p.analyze(&request(vec![zero], RareChartSpec::default())).is_ok());
    }
}
