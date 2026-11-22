//! Statistics > Seasonal Decomposition as a tool of `Project::analyze` (T370; Origin 2024b's Time Series Analysis
//! app, not in Origin 2016, so the layout is Sheaf's own). The numerics are in `sheaf_calc::decompose`;
//! `docs/spec/statistics.md` section 39 says what is computed and written.
//!
//! Every chosen column is a series of its own, in row order; it must be complete in the chosen rows.

use super::dist::{GraphPlot, Output, XyGraph};
use super::statistics::num;
use super::*;
use crate::graph::{PlotKind, ScaleKind};
pub use calc::decompose::DecomposeModel;
use calc::decompose::decompose;

/// What [`Tool::Decompose`] does.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DecomposeSpec {
    /// Points per season (12 for monthly data with a yearly pattern).
    pub period: usize,
    pub model: DecomposeModel,
    /// Make the graphs: data with trend, seasonal and residual.
    pub graph: bool,
}

impl Default for DecomposeSpec {
    fn default() -> Self {
        DecomposeSpec { period: 12, model: DecomposeModel::Additive, graph: true }
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

pub(super) fn run(spec: &DecomposeSpec, inputs: &[Input<'_>]) -> Result<Output, AnalysisError> {
    let model = match spec.model {
        DecomposeModel::Additive => "additive",
        DecomposeModel::Multiplicative => "multiplicative",
    };
    let mut table = ResultTable::new(&["Position", "Seasonal index"]);
    let mut notes = Vec::new();
    let mut on_sheet = Vec::new();
    let mut graphs = Vec::new();
    for input in inputs {
        let d = decompose(input.ys(), spec.period, spec.model)
            .map_err(|e| AnalysisError::BadOption(format!("{}: {e}", input.label)))?;
        for (k, v) in d.indices.iter().enumerate() {
            let unit = if spec.model == DecomposeModel::Additive { input.units.clone() } else { String::new() };
            table.push_row(
                format!("{}, position {}", input.label, k + 1),
                vec![(k + 1) as f64, *v],
                vec![String::new(), unit],
            );
        }
        let resid: Vec<f64> = d.residual.iter().copied().filter(|v| v.is_finite()).collect();
        let centre = if spec.model == DecomposeModel::Additive { 0.0 } else { 1.0 };
        let rms = (resid.iter().map(|v| (v - centre).powi(2)).sum::<f64>() / resid.len() as f64).sqrt();
        let (lo, hi) = d.indices.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), v| (a.min(*v), b.max(*v)));
        notes.push(format!(
            "{}: {model} decomposition with period {}; seasonal indices from {} to {}; residual RMS about {} is {} ({} points without a trend at the ends).",
            input.label,
            spec.period,
            num(lo),
            num(hi),
            num(centre),
            num(rms),
            2 * (spec.period / 2)
        ));
        let comment = format!("Decomposition of {}", input.name);
        let x = on_sheet.len();
        on_sheet.push(column_of(&input.x_name, &comment, Designation::X, input.xs().to_vec()));
        on_sheet.push(column_of(&input.name, &comment, Designation::Y, input.ys().to_vec()));
        on_sheet.push(column_of("Trend", &comment, Designation::Y, d.trend));
        on_sheet.push(column_of("Seasonal", &comment, Designation::Y, d.seasonal));
        on_sheet.push(column_of("Residual", &comment, Designation::Y, d.residual));
        if spec.graph {
            let graph = |plots: Vec<GraphPlot>, y_title: &str| XyGraph {
                plots,
                x_scale: ScaleKind::Linear,
                y_scale: ScaleKind::Linear,
                x_title: input.x_name.clone(),
                y_title: y_title.to_string(),
                look: Default::default(),
            };
            graphs.push(graph(
                vec![
                    GraphPlot { x, y: x + 1, kind: PlotKind::Line, label: input.label.clone(), line_of: None },
                    GraphPlot { x, y: x + 2, kind: PlotKind::Line, label: "Trend".into(), line_of: None },
                ],
                &input.name,
            ));
            graphs.push(graph(
                vec![GraphPlot { x, y: x + 3, kind: PlotKind::Line, label: "Seasonal".into(), line_of: None }],
                "Seasonal",
            ));
            graphs.push(graph(
                vec![GraphPlot { x, y: x + 4, kind: PlotKind::Scatter, label: "Residual".into(), line_of: None }],
                "Residual",
            ));
        }
    }
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
    let (graph, more_graphs) = if graphs.is_empty() {
        (None, Vec::new())
    } else {
        let first = graphs.remove(0);
        (Some(first), graphs)
    };
    Ok(Output { table: Some(table), on_sheet: sheet, notes, graph, more_graphs }.shift_graphs(shift))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(columns: Vec<ColumnId>, spec: DecomposeSpec) -> Request {
        Request { columns, rows: RowRange::All, tool: Tool::Decompose(spec) }
    }

    #[test]
    fn indices_rows_and_three_graphs_of_their_own_columns() {
        let mut p = Project::new();
        let book = p.add_workbook("ts");
        let pattern = [1.0, -2.0, 0.5, 0.5];
        let y: Vec<f64> = (0..16).map(|i| 3.0 + 0.5 * f64::from(i) + pattern[i as usize % 4]).collect();
        let yc = p.add_column(book, 0, Designation::Y, ColumnData::numeric(y)).unwrap();
        let out = p.analyze(&request(vec![yc], DecomposeSpec { period: 4, ..Default::default() })).unwrap();
        let t = out.table.as_ref().unwrap();
        assert_eq!(t.rows.len(), 4);
        assert!((t.rows[1].1[1] + 2.0).abs() < 1e-12, "{:?}", t.rows[1]);
        assert_eq!(out.more_graphs.len(), 2);
        let cols = super::super::dist::plot_columns(&p, out.new_graph.unwrap());
        assert_eq!(cols[1].1, "Trend", "{cols:?}");
        let cols = super::super::dist::plot_columns(&p, out.more_graphs[1]);
        assert_eq!(cols[0].1, "Residual");
        assert!(p.analyze(&request(vec![yc], DecomposeSpec { period: 9, ..Default::default() })).is_err());
    }
}
