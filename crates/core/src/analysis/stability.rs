//! Statistics > Stability Study as a tool of `Project::analyze` (T368; Origin 2027's Quality improvement app for
//! stability studies, not in Origin 2016, so the layout is Sheaf's own). The numerics are in
//! `sheaf_calc::stability`; `docs/spec/statistics.md` section 38 says what is computed and written.
//!
//! The chosen columns are the time, the response and, optionally, the batch (numbers or text).

use super::dist::{GraphPlot, Output, XyGraph};
use super::statistics::num;
use super::survival::group_cell;
use super::*;
use crate::graph::{PlotKind, ScaleKind};
use calc::stability::{HORIZON, StabilityOptions, stability};

/// What [`Tool::Stability`] does.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StabilitySpec {
    pub lower: Option<f64>,
    pub upper: Option<f64>,
    /// Confidence level of the bounds (0.95).
    pub confidence: f64,
    /// Significance level of the poolability tests (0.25, ICH Q1E).
    pub alpha: f64,
    pub graph: bool,
}

impl Default for StabilitySpec {
    fn default() -> Self {
        StabilitySpec { lower: None, upper: None, confidence: 0.95, alpha: 0.25, graph: true }
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

fn shelf(v: f64) -> String {
    if v.is_finite() { num(v) } else { format!("beyond {HORIZON} times the last time") }
}

pub(super) fn run(
    spec: &StabilitySpec,
    sheet: &Worksheet,
    columns: &[ColumnId],
    rows: RowRange,
) -> Result<Output, AnalysisError> {
    let column = |id: ColumnId| sheet.column(id).ok_or(AnalysisError::NoSuchColumn(id));
    let (tc, yc, bc) = match columns {
        [t, y] => (column(*t)?, column(*y)?, None),
        [t, y, b] => (column(*t)?, column(*y)?, Some(column(*b)?)),
        _ => {
            return Err(AnalysisError::BadOption(
                "the stability study takes two or three columns: time, response and, optionally, batch".into(),
            ));
        }
    };
    let ts = sheet.values_of(tc).ok_or_else(|| AnalysisError::NotNumeric(tc.short_name.clone()))?;
    let ys = sheet.values_of(yc).ok_or_else(|| AnalysisError::NotNumeric(yc.short_name.clone()))?;
    let n = ts.len().max(ys.len()).max(bc.map_or(0, |b| b.data.len()));
    if n == 0 {
        return Err(AnalysisError::Empty(yc.short_name.clone()));
    }
    let (first, last) = resolve_rows(rows, &yc.short_name, &[], n)?;
    let mut labels: Vec<String> = Vec::new();
    let mut points: Vec<(usize, f64, f64)> = Vec::new();
    for r in first..=last {
        let (Some(&t), Some(&y)) = (ts.get(r), ys.get(r)) else { continue };
        if !(t.is_finite() && y.is_finite()) {
            continue;
        }
        let label = match bc {
            Some(b) => match group_cell(b, r) {
                Some(l) => l,
                None => continue,
            },
            None => column_label(yc),
        };
        let k = match labels.iter().position(|l| *l == label) {
            Some(k) => k,
            None => {
                labels.push(label);
                labels.len() - 1
            }
        };
        points.push((k, t, y));
    }
    // Numeric batch labels in increasing order.
    if labels.iter().all(|l| l.parse::<f64>().is_ok()) {
        let mut order: Vec<usize> = (0..labels.len()).collect();
        order.sort_by(|a, b| {
            labels[*a].parse::<f64>().unwrap_or(0.0).total_cmp(&labels[*b].parse::<f64>().unwrap_or(0.0))
        });
        let mut new_index = vec![0; labels.len()];
        for (new, old) in order.iter().enumerate() {
            new_index[*old] = new;
        }
        for p in &mut points {
            p.0 = new_index[p.0];
        }
        labels = order.iter().map(|i| labels[*i].clone()).collect();
    }
    let opts =
        StabilityOptions { lower: spec.lower, upper: spec.upper, confidence: spec.confidence, alpha: spec.alpha };
    let s = stability(&points, opts).map_err(|e| AnalysisError::BadOption(e.to_string()))?;
    let mut table =
        ResultTable::new(&["N", "Intercept", "Slope", "Shelf life", "Prob (equal slopes)", "Prob (equal intercepts)"]);
    let blank = vec![String::new(); 6];
    for (label, b) in labels.iter().zip(&s.batches) {
        let mut units = blank.clone();
        units[1].clone_from(&yc.units);
        units[3].clone_from(&tc.units);
        let life = if b.shelf_life.is_finite() { b.shelf_life } else { f64::NAN };
        table.push_row(
            format!("Batch {label}"),
            vec![b.n as f64, b.intercept, b.slope, life, f64::NAN, f64::NAN],
            units,
        );
    }
    let mut units = blank.clone();
    units[3].clone_from(&tc.units);
    table.push_row(
        "All batches".into(),
        vec![
            points.len() as f64,
            f64::NAN,
            f64::NAN,
            if s.shelf_life.is_finite() { s.shelf_life } else { f64::NAN },
            s.p_slopes,
            s.p_intercepts,
        ],
        units,
    );
    let pct = num(100.0 * spec.confidence);
    let sided = if spec.lower.is_some() && spec.upper.is_some() { "two-sided" } else { "one-sided" };
    let mut notes = vec![format!(
        "{} batch(es), {} points; model: {} (residual MSE {}, {} df); shelf life {} (the first time a {sided} {pct}% confidence bound of a batch mean reaches a limit).",
        labels.len(),
        points.len(),
        s.model.name(),
        num(s.mse),
        s.df,
        shelf(s.shelf_life)
    )];
    if labels.len() > 1 {
        notes.push(format!(
            "Poolability at {}: equal slopes Prob > F = {}{}.",
            num(spec.alpha),
            num(s.p_slopes),
            if s.p_intercepts.is_nan() {
                " (rejected: separate lines)".to_string()
            } else {
                format!(", equal intercepts Prob > F = {}", num(s.p_intercepts))
            }
        ));
    }
    let mut on_sheet = Vec::new();
    let mut graph = None;
    if spec.graph {
        let last_t = points.iter().map(|p| p.1).fold(0.0, f64::max);
        let end = if s.shelf_life.is_finite() { s.shelf_life.max(last_t) } else { last_t } * 1.1;
        let curve: Vec<f64> = (0..=100).map(|i| end * f64::from(i) / 100.0).collect();
        let mut plots = Vec::new();
        for (b, label) in labels.iter().enumerate() {
            let comment = format!("Batch {label}");
            let (px, py): (Vec<f64>, Vec<f64>) = points.iter().filter(|p| p.0 == b).map(|p| (p.1, p.2)).unzip();
            let x = on_sheet.len();
            on_sheet.push(column_of(&column_label(tc), &comment, Designation::X, px));
            on_sheet.push(column_of(&column_label(yc), &comment, Designation::Y, py));
            let first = plots.len();
            plots.push(GraphPlot { x, y: x + 1, kind: PlotKind::Scatter, label: comment.clone(), line_of: None });
            let x2 = on_sheet.len();
            on_sheet.push(column_of("Time", &comment, Designation::X, curve.clone()));
            on_sheet.push(column_of(
                "Fitted mean",
                &comment,
                Designation::Y,
                curve.iter().map(|t| s.mean(b, *t)).collect(),
            ));
            plots.push(GraphPlot {
                x: x2,
                y: x2 + 1,
                kind: PlotKind::Line,
                label: format!("Fit, {comment}"),
                line_of: Some(first),
            });
            for (side, sign, wanted) in [("Lower", -1.0, spec.lower.is_some()), ("Upper", 1.0, spec.upper.is_some())] {
                if !wanted {
                    continue;
                }
                let name = format!("{side} {pct}% bound");
                on_sheet.push(column_of(
                    &name,
                    &comment,
                    Designation::Y,
                    curve.iter().map(|t| s.mean(b, *t) + sign * s.half_width(b, *t)).collect(),
                ));
                plots.push(GraphPlot {
                    x: x2,
                    y: on_sheet.len() - 1,
                    kind: PlotKind::Line,
                    label: format!("{name}, {comment}"),
                    line_of: Some(first),
                });
            }
        }
        for (name, limit) in [("Lower limit", spec.lower), ("Upper limit", spec.upper)] {
            let Some(l) = limit else { continue };
            let x = on_sheet.len();
            on_sheet.push(column_of(&format!("{name} X"), "Specification", Designation::X, vec![0.0, end]));
            on_sheet.push(column_of(name, "Specification", Designation::Y, vec![l, l]));
            plots.push(GraphPlot {
                x,
                y: x + 1,
                kind: PlotKind::Line,
                label: format!("{name} = {}", num(l)),
                line_of: None,
            });
        }
        graph = Some(XyGraph {
            plots,
            x_scale: ScaleKind::Linear,
            y_scale: ScaleKind::Linear,
            x_title: column_label(tc),
            y_title: column_label(yc),
            look: Default::default(),
        });
    }
    let mut sheet_cols = vec![SheetColumn {
        designation: Designation::Label,
        long_name: "Batch".into(),
        units: String::new(),
        comments: String::new(),
        data: ColumnData::text(table.rows.iter().map(|(l, _)| l.clone()).collect()),
    }];
    for (i, h) in table.headers.iter().enumerate() {
        sheet_cols.push(column_of(h, "", Designation::Y, table.rows.iter().map(|(_, v)| v[i]).collect()));
    }
    let shift = sheet_cols.len();
    sheet_cols.extend(on_sheet);
    Ok(Output { table: Some(table), on_sheet: sheet_cols, notes, graph, more_graphs: Vec::new() }.shift_graphs(shift))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(columns: Vec<ColumnId>, spec: StabilitySpec) -> Request {
        Request { columns, rows: RowRange::All, tool: Tool::Stability(spec) }
    }

    #[test]
    fn batches_pool_and_the_graph_draws_its_columns() {
        let mut p = Project::new();
        let book = p.add_workbook("stab");
        let times = [0.0, 3.0, 6.0, 9.0, 12.0];
        let mut t = Vec::new();
        let mut y = Vec::new();
        let mut batch = Vec::new();
        for (b, start) in [(2.0, 100.0), (1.0, 100.0)] {
            for (i, ti) in times.iter().enumerate() {
                t.push(*ti);
                y.push(start - 0.5 * ti + if i % 2 == 0 { 0.1 } else { -0.1 } * b);
                batch.push(b);
            }
        }
        let tc = p.add_column(book, 0, Designation::X, ColumnData::numeric(t)).unwrap();
        let yc = p.add_column(book, 0, Designation::Y, ColumnData::numeric(y)).unwrap();
        let bc = p.add_column(book, 0, Designation::Y, ColumnData::numeric(batch)).unwrap();
        let spec = StabilitySpec { lower: Some(90.0), ..Default::default() };
        let out = p.analyze(&request(vec![tc, yc, bc], spec)).unwrap();
        let t = out.table.as_ref().unwrap();
        assert_eq!(t.rows.iter().map(|r| r.0.as_str()).collect::<Vec<_>>(), ["Batch 1", "Batch 2", "All batches"]);
        assert!((t.rows[0].1[2] + 0.5).abs() < 0.05, "{:?}", t.rows[0]);
        let life = t.rows[2].1[3];
        assert!(life > 15.0 && life < 20.0, "the line meets 90 at 20: {life}");
        assert!(out.notes[0].contains("one line for all batches"), "{:?}", out.notes);
        let cols = super::super::dist::plot_columns(&p, out.new_graph.unwrap());
        assert_eq!(
            (cols[1].0.as_str(), cols[1].1.as_str(), cols[2].1.as_str()),
            ("Time", "Fitted mean", "Lower 95% bound")
        );
        assert_eq!(cols.last().unwrap().1, "Lower limit");
        assert!(p.analyze(&request(vec![tc, yc, bc], StabilitySpec::default())).is_err(), "no limit");
        assert!(p.analyze(&request(vec![tc], spec)).is_err());
    }
}
