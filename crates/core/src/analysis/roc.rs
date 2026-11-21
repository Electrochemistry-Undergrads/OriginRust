//! Statistics > ROC Curve as a tool of `Project::analyze`. The numerics are in `sheaf_calc` (`roc`); what the tool
//! computes and reports is written down in `docs/spec/statistics.md`, section 19.
//!
//! The chosen columns are the test variables followed, as the last one, by the state column (Origin's dialog also
//! takes the last column of the selection as the state). The state column may hold numbers or text.

use super::dist::{GraphPlot, Output, XyGraph};
use super::statistics::{cell_left_out, num};
use super::*;
use crate::graph::{PlotKind, ScaleKind};

/// What [`Tool::Roc`] does.
#[derive(Debug, Clone, PartialEq)]
pub struct RocSpec {
    /// The value of the state column that marks a positive case: a number (`1`) or a text (`yes`). Cells are
    /// compared by value, so a text cell "1" matches a numeric 1.
    pub positive: String,
    /// Larger test values point to the positive state (Origin's "Positive v.s. High", the default); false is
    /// "Positive v.s. Low".
    pub higher_is_positive: bool,
    /// Confidence level of the interval of the area, in percent.
    pub confidence: f64,
    /// Make a graph of the curves, with the diagonal.
    pub curve: bool,
}

impl Default for RocSpec {
    fn default() -> Self {
        RocSpec { positive: "1".into(), higher_is_positive: true, confidence: 95.0, curve: true }
    }
}

fn label_column(name: &str, values: Vec<String>) -> SheetColumn {
    SheetColumn {
        designation: Designation::Label,
        long_name: name.to_string(),
        units: String::new(),
        comments: String::new(),
        data: ColumnData::text(values),
    }
}

fn number_column(name: &str, comments: &str, designation: Designation, values: Vec<f64>) -> SheetColumn {
    SheetColumn {
        designation,
        long_name: name.to_string(),
        units: String::new(),
        comments: comments.to_string(),
        data: ColumnData::numeric(values),
    }
}

/// Whether a cell of the state column is the positive value; `None` for a missing cell.
pub(super) fn is_positive(c: &Column, row: usize, positive: &str, positive_number: Option<f64>) -> Option<bool> {
    if let Some(t) = c.data.text_cell(row) {
        let t = t.trim();
        if t == positive {
            return Some(true);
        }
        return Some(positive_number.is_some_and(|p| t.parse::<f64>().is_ok_and(|v| v == p)));
    }
    let v = c.numeric()?.get(row).copied().filter(|v| !v.is_nan())?;
    Some(positive_number == Some(v))
}

pub(super) fn run(
    spec: &RocSpec,
    sheet: &Worksheet,
    columns: &[ColumnId],
    rows: RowRange,
) -> Result<Output, AnalysisError> {
    let positive = spec.positive.trim();
    if positive.is_empty() {
        return Err(AnalysisError::BadOption("give the value of the state column that is the positive state".into()));
    }
    if !(spec.confidence > 0.0 && spec.confidence < 100.0) {
        return Err(AnalysisError::BadOption("the confidence level must lie between 0 and 100 percent".into()));
    }
    if columns.len() < 2 {
        return Err(AnalysisError::BadOption(
            "the ROC curve needs at least two columns: the test variable(s), then the state column last".into(),
        ));
    }
    if matches!(rows, RowRange::XBetween(..)) {
        return Err(AnalysisError::BadOption(
            "the ROC curve has no X range to limit; choose rows with a row range".into(),
        ));
    }
    let column = |id: ColumnId| sheet.column(id).ok_or(AnalysisError::NoSuchColumn(id));
    let (state_id, test_ids) = columns.split_last().expect("two columns");
    let state = column(*state_id)?;
    let positive_number = positive.parse::<f64>().ok();
    let n = test_ids
        .iter()
        .map(|id| column(*id).map(|c| c.data.len()))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .chain([state.data.len()])
        .max()
        .unwrap_or(0);
    if n == 0 {
        return Err(AnalysisError::Empty(state.short_name.clone()));
    }
    let (first, last) = resolve_rows(rows, &state.short_name, &[], n)?;
    // A masked or filtered state cell is no state (T255).
    let flags: Vec<Option<bool>> = (first..=last)
        .map(|r| if cell_left_out(sheet, state, r) { None } else { is_positive(state, r, positive, positive_number) })
        .collect();

    let mut table = ResultTable::new(&[
        "Positive",
        "Negative",
        "Area",
        "Standard error",
        "Lower limit",
        "Upper limit",
        "Z",
        "Prob>|Z|",
        "Best cut-off",
        "Best sensitivity",
        "Best specificity",
    ]);
    let mut notes = Vec::new();
    let mut curves: Vec<(String, calc::Roc)> = Vec::new();
    for id in test_ids {
        let c = column(*id)?;
        let values = sheet.values_of(c).ok_or_else(|| AnalysisError::NotNumeric(c.short_name.clone()))?;
        // A row counts when it has a test value and a state; the state of a missing row is not guessed.
        let mut v = Vec::with_capacity(flags.len());
        let mut s = Vec::with_capacity(flags.len());
        for (k, flag) in flags.iter().enumerate() {
            let x = values.get(first + k).copied().unwrap_or(f64::NAN);
            match flag {
                Some(f) => {
                    v.push(x);
                    s.push(*f);
                }
                None => {
                    v.push(f64::NAN);
                    s.push(false);
                }
            }
        }
        let options =
            calc::RocOptions { higher_is_positive: spec.higher_is_positive, alpha: 1.0 - spec.confidence / 100.0 };
        let r =
            calc::roc(&v, &s, &options).map_err(|e| AnalysisError::BadOption(format!("{}: {e}", column_label(c))))?;
        let best = r.points[r.best];
        table.push_row(
            column_label(c),
            vec![
                r.n_positive as f64,
                r.n_negative as f64,
                r.auc,
                r.se,
                r.lcl,
                r.ucl,
                r.z,
                r.p,
                best.cutoff,
                best.sensitivity,
                best.specificity,
            ],
            {
                let mut u = vec![String::new(); 11];
                u[8] = c.units.clone();
                u
            },
        );
        notes.push(format!(
            "{}: area under the curve {} (standard error {}, {}% confidence interval {} to {}), {} positive and {} negative cases; \
             best cut-off {} (sensitivity {}, specificity {}).",
            column_label(c),
            num(r.auc),
            num(r.se),
            num(spec.confidence),
            num(r.lcl),
            num(r.ucl),
            r.n_positive,
            r.n_negative,
            num(best.cutoff),
            num(best.sensitivity),
            num(best.specificity),
        ));
        curves.push((column_label(c), r));
    }
    let direction = if spec.higher_is_positive { "higher" } else { "lower" };
    notes.push(format!(
        "Positive state: {positive}; {direction} test values point to it. Standard error, interval and p-value as Origin's; \
         the best cut-off (Sheaf's own) has the largest sensitivity + specificity - 1."
    ));

    let mut on_sheet = vec![label_column("Data", table.rows.iter().map(|(l, _)| l.clone()).collect())];
    for (i, h) in table.headers.iter().enumerate() {
        on_sheet.push(number_column(h, "", Designation::Y, table.rows.iter().map(|(_, v)| v[i]).collect()));
    }
    let mut plots = Vec::new();
    for (label, r) in &curves {
        let cut: Vec<f64> = r.points.iter().map(|p| p.cutoff).collect();
        let fpr: Vec<f64> = r.points.iter().map(|p| 1.0 - p.specificity).collect();
        let tpr: Vec<f64> = r.points.iter().map(|p| p.sensitivity).collect();
        on_sheet.push(number_column("Cut-off", label, Designation::Y, cut));
        let x = on_sheet.len();
        on_sheet.push(number_column("1 - Specificity", label, Designation::X, fpr));
        on_sheet.push(number_column("Sensitivity", label, Designation::Y, tpr));
        plots.push(GraphPlot { x, y: x + 1, kind: PlotKind::Line, label: label.clone(), line_of: None });
    }
    let x = on_sheet.len();
    on_sheet.push(number_column("Reference X", "Diagonal", Designation::X, vec![0.0, 1.0]));
    on_sheet.push(number_column("Reference Y", "Diagonal", Designation::Y, vec![0.0, 1.0]));
    plots.push(GraphPlot { x, y: x + 1, kind: PlotKind::Line, label: "Reference".into(), line_of: None });

    let graph = spec.curve.then(|| XyGraph {
        plots,
        x_scale: ScaleKind::Linear,
        y_scale: ScaleKind::Linear,
        x_title: "1 - Specificity".into(),
        y_title: "Sensitivity".into(),
        look: Default::default(),
    });
    Ok(Output { table: Some(table), on_sheet, notes, graph, more_graphs: Vec::new() })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A (test value) and B (state) as in `calc::roc`'s hand-worked case, and C a text state.
    fn project() -> (Project, Vec<ColumnId>) {
        let value = vec![3.0, 4.0, 5.0, 1.0, 2.0, 4.0];
        let state = vec![1.0, 1.0, 1.0, 0.0, 0.0, 0.0];
        let text: Vec<String> = ["yes", "yes", "yes", "no", "no", ""].iter().map(|s| s.to_string()).collect();
        let mut p = Project::new();
        let book = p.add_workbook("roc");
        let a = p.add_column(book, 0, Designation::Y, ColumnData::numeric(value)).unwrap();
        let b = p.add_column(book, 0, Designation::Y, ColumnData::numeric(state)).unwrap();
        let c = p.add_column(book, 0, Designation::Y, ColumnData::text(text)).unwrap();
        (p, vec![a, b, c])
    }

    fn request(columns: Vec<ColumnId>, spec: RocSpec) -> Request {
        Request { columns, rows: RowRange::All, tool: Tool::Roc(spec) }
    }

    #[test]
    fn numeric_state_gives_the_hand_worked_area_and_a_graph() {
        let (mut p, ids) = project();
        let r = p.analyze(&request(ids[..2].to_vec(), RocSpec::default())).unwrap();
        assert!(r.message.contains("ROC curve") && r.message.contains("ROC1"), "{}", r.message);
        let t = r.table.unwrap();
        assert_eq!(t.rows.len(), 1);
        let v = &t.rows[0].1;
        assert_eq!((v[0], v[1]), (3.0, 3.0));
        assert!((v[2] - 5.0 / 6.0).abs() < 1e-12, "{}", v[2]);
        // Origin's standard error (statistics.md 19.5): Q1 = 61/81 and Q2 = 58/81 along the curve.
        let (a, q1, q2) = (5.0 / 6.0, 61.0 / 81.0, 58.0 / 81.0);
        let se = ((a * (1.0 - a) + 2.0 * (q1 - a * a) + 2.0 * (q2 - a * a)) / 9.0f64).sqrt();
        assert!((v[3] - se).abs() < 1e-12, "{} against {se}", v[3]);
        // Origin's interpolated cut-off: 3 and above positive is "above 2.5".
        assert_eq!(v[8], 2.5, "best cut-off");
        let sheet = &p.workbook(r.book).unwrap().sheets[r.new_sheet.unwrap()];
        let long: Vec<&str> = sheet.columns.iter().map(|c| c.long_name.as_str()).collect();
        assert!(
            long.contains(&"1 - Specificity") && long.contains(&"Sensitivity") && long.contains(&"Cut-off"),
            "{long:?}"
        );
        let g = p.graph(r.new_graph.unwrap()).unwrap();
        assert_eq!(g.layers[0].plots.len(), 2, "the curve and the diagonal");
    }

    #[test]
    fn text_state_direction_and_errors() {
        let (mut p, ids) = project();
        let spec = RocSpec { positive: "yes".into(), curve: false, ..Default::default() };
        // The sixth row has an empty state and is left out: positives 3, 4, 5 against negatives 1, 2.
        let r = p.analyze(&request(vec![ids[0], ids[2]], spec.clone())).unwrap();
        assert!(r.new_graph.is_none());
        let t = r.table.unwrap();
        assert_eq!((t.rows[0].1[0], t.rows[0].1[1]), (3.0, 2.0));
        assert_eq!(t.rows[0].1[2], 1.0);
        let low = p.analyze(&request(vec![ids[0], ids[2]], RocSpec { higher_is_positive: false, ..spec })).unwrap();
        assert_eq!(low.table.unwrap().rows[0].1[2], 0.0);
        // A positive value that no cell has, a missing value, one column.
        let none = RocSpec { positive: "maybe".into(), ..Default::default() };
        assert!(p.analyze(&request(ids[..2].to_vec(), none)).is_err());
        assert!(
            p.analyze(&request(ids[..2].to_vec(), RocSpec { positive: " ".into(), ..Default::default() })).is_err()
        );
        assert!(p.analyze(&request(ids[..1].to_vec(), RocSpec::default())).is_err());
        assert!(p.analyze(&request(ids[..2].to_vec(), RocSpec { confidence: 100.0, ..Default::default() })).is_err());
    }

    #[test]
    fn a_masked_state_cell_leaves_the_case_out() {
        // T255: as a masked test value does.
        let (mut p, ids) = project();
        let table = |p: &mut Project| {
            format!("{:?}", p.analyze(&request(ids[..2].to_vec(), RocSpec::default())).unwrap().table)
        };
        let all = table(&mut p);
        let mut gone = p.clone();
        gone.column_mut(ids[0]).unwrap().mask.insert(0);
        let want = table(&mut gone);
        assert_ne!(all, want);
        p.column_mut(ids[1]).unwrap().mask.insert(0);
        assert_eq!(table(&mut p), want);
    }
}
