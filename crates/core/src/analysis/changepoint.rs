//! Statistics > Change Point Analysis as a tool of `Project::analyze` (T346; the change-point app of Origin
//! 2019b, not in Origin 2016, so the layout is Sheaf's own). The numerics are in `sheaf_calc::changepoint`;
//! `docs/spec/statistics.md` section 30 says what is computed and written.
//!
//! Every chosen column is a series of its own, in row order; its X column (or the row number) only places
//! the segments.

use super::dist::{GraphPlot, Output, XyGraph};
use super::statistics::num;
use super::*;
use crate::graph::{PlotKind, ScaleKind};
use calc::changepoint::{ChangeKind, Penalty, change_points};

/// What [`Tool::ChangePoint`] does.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChangePointSpec {
    /// What changes between segments (default the mean).
    pub kind: ChangeKind,
    /// The penalty per change (default MBIC's).
    pub penalty: Penalty,
    /// The fewest points in a segment (default 2).
    pub min_segment: usize,
    /// Make a graph of the data with the segment means.
    pub graph: bool,
}

impl Default for ChangePointSpec {
    fn default() -> Self {
        ChangePointSpec { kind: ChangeKind::Mean, penalty: Penalty::Mbic, min_segment: 2, graph: true }
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

pub(super) fn run(spec: &ChangePointSpec, inputs: &[Input<'_>]) -> Result<Output, AnalysisError> {
    let mut table =
        ResultTable::new(&["Segment", "Start row", "End row", "Start X", "End X", "N", "Mean", "Standard deviation"]);
    let mut notes = Vec::new();
    let mut on_sheet = Vec::new();
    let mut plots = Vec::new();
    for input in inputs {
        // Missing values are left out; `rows[k]` is the row of the k-th value used.
        let (rows, values): (Vec<usize>, Vec<f64>) =
            input.ys().iter().enumerate().filter(|(_, v)| v.is_finite()).map(|(k, v)| (input.first + k, *v)).unzip();
        let r = change_points(&values, spec.kind, spec.penalty, spec.min_segment)
            .map_err(|e| AnalysisError::BadOption(format!("{}: {e}", input.label)))?;
        let x_at = |row: usize| input.x.get(row).copied().unwrap_or(f64::NAN);
        let mut steps_x = Vec::new();
        let mut steps_y = Vec::new();
        for (k, s) in r.segments.iter().enumerate() {
            let (a, b) = (rows[s.start], rows[s.end]);
            table.push_row(
                format!("{}, segment {}", input.label, k + 1),
                vec![
                    (k + 1) as f64,
                    (a + 1) as f64,
                    (b + 1) as f64,
                    x_at(a),
                    x_at(b),
                    (s.end + 1 - s.start) as f64,
                    s.mean,
                    s.sd,
                ],
                vec![
                    String::new(),
                    String::new(),
                    String::new(),
                    input.x_units.clone(),
                    input.x_units.clone(),
                    String::new(),
                    input.units.clone(),
                    input.units.clone(),
                ],
            );
            if k > 0 {
                steps_x.push(f64::NAN);
                steps_y.push(f64::NAN);
            }
            steps_x.extend([x_at(a), x_at(b)]);
            steps_y.extend([s.mean, s.mean]);
        }
        let at: Vec<String> = r.changes.iter().map(|&c| num(x_at(rows[c]))).collect();
        notes.push(if at.is_empty() {
            format!("{}: no change point (penalty {} per change).", input.label, num(r.penalty))
        } else {
            format!(
                "{}: {} change point(s), new segments starting at {} = {} (penalty {} per change).",
                input.label,
                at.len(),
                input.x_short,
                at.join(", "),
                num(r.penalty)
            )
        });
        if spec.graph {
            let x = on_sheet.len();
            on_sheet.push(column_of(&input.x_name, &input.label, Designation::X, input.xs().to_vec()));
            on_sheet.push(column_of(&input.name, &input.label, Designation::Y, input.ys().to_vec()));
            plots.push(GraphPlot { x, y: x + 1, kind: PlotKind::Line, label: input.label.clone(), line_of: None });
            let x2 = on_sheet.len();
            let label = format!("Segment means of {}", input.name);
            on_sheet.push(column_of(&input.x_name, &label, Designation::X, steps_x));
            on_sheet.push(column_of(&label, &label, Designation::Y, steps_y));
            plots.push(GraphPlot { x: x2, y: x2 + 1, kind: PlotKind::Line, label, line_of: Some(plots.len() - 1) });
        }
    }
    let what = match spec.kind {
        ChangeKind::Mean => "the mean (common standard deviation from the first differences)",
        ChangeKind::Variance => "the variance (about the mean of the whole series)",
        ChangeKind::MeanVariance => "the mean and the variance",
    };
    notes.push(format!(
        "Changes in {what}, exact optimum of the normal costs plus a penalty per change (PELT); segments of at least {} points.",
        spec.min_segment
    ));
    let graph = (spec.graph && !plots.is_empty()).then(|| XyGraph {
        plots,
        x_scale: ScaleKind::Linear,
        y_scale: ScaleKind::Linear,
        x_title: inputs[0].x_name.clone(),
        y_title: inputs[0].name.clone(),
        look: Default::default(),
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
    Ok(Output { table: Some(table), on_sheet: sheet, notes, graph, more_graphs: Vec::new() })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(columns: Vec<ColumnId>, spec: ChangePointSpec) -> Request {
        Request { columns, rows: RowRange::All, tool: Tool::ChangePoint(spec) }
    }

    /// X = 0.5, 1, ... and a series with levels 0, 4, 1 and a little noise, one missing value.
    fn project() -> (Project, ColumnId) {
        let mut p = Project::new();
        let book = p.add_workbook("cp");
        let xs: Vec<f64> = (1..=60).map(|i| f64::from(i) * 0.5).collect();
        let mut ys: Vec<f64> = (0..60)
            .map(|i| {
                let level = if i < 20 {
                    0.0
                } else if i < 40 {
                    4.0
                } else {
                    1.0
                };
                level + 0.2 * (f64::from(i * i) * 0.77).sin()
            })
            .collect();
        ys[10] = f64::NAN;
        p.add_column(book, 0, Designation::X, ColumnData::numeric(xs)).unwrap();
        let y = p.add_column(book, 0, Designation::Y, ColumnData::numeric(ys)).unwrap();
        (p, y)
    }

    #[test]
    fn segments_are_reported_at_their_rows_and_x_values() {
        let (mut p, y) = project();
        let out = p.analyze(&request(vec![y], ChangePointSpec::default())).unwrap();
        let t = out.table.as_ref().unwrap();
        assert_eq!(t.rows.len(), 3, "{:?}", t.rows);
        // Start row and X of segments 2 and 3: rows 21 and 41 (X 10.5 and 20.5).
        assert_eq!((t.rows[1].1[1], t.rows[1].1[3]), (21.0, 10.5));
        assert_eq!((t.rows[2].1[1], t.rows[2].1[3]), (41.0, 20.5));
        // The missing row is left out of the first segment's count.
        assert_eq!(t.rows[0].1[5], 19.0);
        assert!((t.rows[1].1[6] - 4.0).abs() < 0.2);
        assert!(out.new_graph.is_some());
    }

    #[test]
    fn bad_options_are_refused() {
        let (mut p, y) = project();
        let spec = ChangePointSpec { kind: ChangeKind::Variance, min_segment: 1, ..Default::default() };
        assert!(p.analyze(&request(vec![y], spec)).is_err());
        let spec = ChangePointSpec { penalty: Penalty::Manual(f64::NAN), ..Default::default() };
        assert!(p.analyze(&request(vec![y], spec)).is_err());
    }
}
