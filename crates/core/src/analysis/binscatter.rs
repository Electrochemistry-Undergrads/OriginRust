//! Statistics > Descriptive Statistics > Binned Scatter as a tool of `Project::analyze` (T356; Origin 2027's binned
//! scatter, not in Origin 2016, so the layout is Sheaf's own). The numerics are in `sheaf_calc::binscatter`;
//! `docs/spec/statistics.md` section 31 says what is computed and written.

use super::dist::{GraphPlot, Output, XyGraph};
use super::*;
use crate::graph::{PlotKind, ScaleKind};
use calc::binscatter::{BinRule, binned_scatter};

/// What [`Tool::BinScatter`] does.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BinScatterSpec {
    /// How X is cut (default 10 bins of equal counts).
    pub rule: BinRule,
    /// Make a graph of the data with the bin means.
    pub graph: bool,
}

impl Default for BinScatterSpec {
    fn default() -> Self {
        BinScatterSpec { rule: BinRule::default(), graph: true }
    }
}

fn column_of(name: &str, units: &str, comments: &str, designation: Designation, values: Vec<f64>) -> SheetColumn {
    SheetColumn {
        designation,
        long_name: name.to_string(),
        units: units.to_string(),
        comments: comments.to_string(),
        data: ColumnData::numeric(values),
    }
}

pub(super) fn run(spec: &BinScatterSpec, inputs: &[Input<'_>]) -> Result<Output, AnalysisError> {
    let mut table = ResultTable::new(&["X from", "X to", "X mean", "N", "Y mean", "Y SD", "Y SE"]);
    let mut notes = Vec::new();
    let mut extra = Vec::new();
    let mut plots = Vec::new();
    for input in inputs {
        let bins = binned_scatter(input.xs(), input.ys(), spec.rule)
            .map_err(|e| AnalysisError::BadOption(format!("{}: {e}", input.label)))?;
        let (xu, yu) = (input.x_units.clone(), input.units.clone());
        for (k, b) in bins.iter().enumerate() {
            table.push_row(
                format!("{}, bin {}", input.label, k + 1),
                vec![b.x_from, b.x_to, b.x_mean, b.n as f64, b.y_mean, b.y_sd, b.y_se],
                vec![xu.clone(), xu.clone(), xu.clone(), String::new(), yu.clone(), yu.clone(), yu.clone()],
            );
        }
        notes.push(format!(
            "{}: {} bins of {} points in all.",
            input.label,
            bins.len(),
            bins.iter().map(|b| b.n).sum::<usize>()
        ));
        if spec.graph {
            let x = extra.len();
            extra.push(column_of(&input.x_name, &xu, &input.label, Designation::X, input.xs().to_vec()));
            extra.push(column_of(&input.name, &yu, &input.label, Designation::Y, input.ys().to_vec()));
            plots.push(GraphPlot { x, y: x + 1, kind: PlotKind::Scatter, label: input.label.clone(), line_of: None });
            let label = format!("Bin means of {}", input.name);
            let x2 = extra.len();
            extra.push(column_of(&input.x_name, &xu, &label, Designation::X, bins.iter().map(|b| b.x_mean).collect()));
            extra.push(column_of(&label, &yu, &label, Designation::Y, bins.iter().map(|b| b.y_mean).collect()));
            plots.push(GraphPlot {
                x: x2,
                y: x2 + 1,
                kind: PlotKind::LineSymbol,
                label,
                line_of: Some(plots.len() - 1),
            });
        }
    }
    notes.push(match spec.rule {
        BinRule::EqualWidth(n) => format!("{n} bins of equal width; empty bins are left out."),
        BinRule::EqualCount(n) => format!("{n} bins of nearly equal counts in X order."),
    });
    let graph = (spec.graph && !plots.is_empty()).then(|| XyGraph {
        plots,
        x_scale: ScaleKind::Linear,
        y_scale: ScaleKind::Linear,
        x_title: inputs[0].x_name.clone(),
        y_title: inputs[0].name.clone(),
        look: Default::default(),
    });
    let mut on_sheet = vec![SheetColumn {
        designation: Designation::Label,
        long_name: "Bin".into(),
        units: String::new(),
        comments: String::new(),
        data: ColumnData::text(table.rows.iter().map(|(l, _)| l.clone()).collect()),
    }];
    for (i, h) in table.headers.iter().enumerate() {
        on_sheet.push(column_of(h, "", "", Designation::Y, table.rows.iter().map(|(_, v)| v[i]).collect()));
    }
    on_sheet.extend(extra);
    Ok(Output { table: Some(table), on_sheet, notes, graph, more_graphs: Vec::new() })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(columns: Vec<ColumnId>, spec: BinScatterSpec) -> Request {
        Request { columns, rows: RowRange::All, tool: Tool::BinScatter(spec) }
    }

    #[test]
    fn one_row_per_bin_and_a_graph() {
        let mut p = Project::new();
        let book = p.add_workbook("bs");
        let xs: Vec<f64> = (0..20).map(f64::from).collect();
        let ys: Vec<f64> = (0..20).map(|i| f64::from(i % 4)).collect();
        p.add_column(book, 0, Designation::X, ColumnData::numeric(xs)).unwrap();
        let y = p.add_column(book, 0, Designation::Y, ColumnData::numeric(ys)).unwrap();
        let spec = BinScatterSpec { rule: BinRule::EqualCount(5), graph: true };
        let out = p.analyze(&request(vec![y], spec)).unwrap();
        let t = out.table.as_ref().unwrap();
        assert_eq!(t.rows.len(), 5);
        // Each bin of 4 consecutive rows holds y = 0, 1, 2, 3: mean 1.5.
        assert!(t.rows.iter().all(|(_, v)| v[3] == 4.0 && v[4] == 1.5));
        assert_eq!((t.rows[1].1[0], t.rows[1].1[1], t.rows[1].1[2]), (4.0, 7.0, 5.5));
        assert!(out.new_graph.is_some());
        let none = BinScatterSpec { rule: BinRule::EqualCount(0), graph: false };
        assert!(p.analyze(&request(vec![y], none)).is_err());
    }
}
