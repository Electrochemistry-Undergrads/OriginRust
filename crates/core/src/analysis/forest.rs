//! Statistics > Random Forest as a tool of `Project::analyze` (T377; Origin 2026b's Random Forest app, not in Origin
//! 2016, so the layout is Sheaf's own). The numerics are in `sheaf_calc::forest`; `docs/spec/statistics.md` section
//! 44 says what is computed and written.
//!
//! The chosen columns are the predictors (numbers) and, last, the response: classes (a text column, or numbers when
//! classification is asked for) or values.

use super::dist::{GraphPlot, Output, XyGraph};
use super::statistics::num;
use super::survival::group_cell;
use super::*;
use crate::graph::{PlotKind, ScaleKind};
use calc::forest::{ForestOptions, Target, forest};

/// Classification or regression.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ForestTask {
    /// Classification for a text response, regression for a numeric one.
    #[default]
    Auto,
    Classification,
    Regression,
}

/// What [`Tool::RandomForest`] does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RandomForestSpec {
    pub task: ForestTask,
    pub trees: usize,
    /// Predictors tried at each node (0: `floor(sqrt p)` for classes, `floor(p / 3)` for values).
    pub mtry: usize,
    /// Smallest leaf (0: 1 for classes, 5 for values).
    pub min_leaf: usize,
    /// Depth limit (0: none).
    pub max_depth: usize,
    pub seed: u64,
    pub graph: bool,
}

impl Default for RandomForestSpec {
    fn default() -> Self {
        RandomForestSpec {
            task: ForestTask::Auto,
            trees: 100,
            mtry: 0,
            min_leaf: 0,
            max_depth: 0,
            seed: 1,
            graph: true,
        }
    }
}

fn number_column(name: &str, designation: Designation, values: Vec<f64>) -> SheetColumn {
    SheetColumn {
        designation,
        long_name: name.to_string(),
        units: String::new(),
        comments: String::new(),
        data: ColumnData::numeric(values),
    }
}

fn text_column(name: &str, values: Vec<String>) -> SheetColumn {
    SheetColumn {
        designation: Designation::Y,
        long_name: name.to_string(),
        units: String::new(),
        comments: String::new(),
        data: ColumnData::text(values),
    }
}

pub(super) fn run(
    spec: &RandomForestSpec,
    sheet: &Worksheet,
    columns: &[ColumnId],
    rows: RowRange,
) -> Result<Output, AnalysisError> {
    if spec.trees == 0 {
        return Err(AnalysisError::BadOption("grow at least one tree".into()));
    }
    if columns.len() < 2 {
        return Err(AnalysisError::BadOption(
            "the random forest needs the predictor column(s) first and the response column last".into(),
        ));
    }
    let column = |id: ColumnId| sheet.column(id).ok_or(AnalysisError::NoSuchColumn(id));
    let (y_id, x_ids) = columns.split_last().expect("two columns");
    let yc = column(*y_id)?;
    let xcs: Vec<&Column> = x_ids.iter().map(|id| column(*id)).collect::<Result<_, _>>()?;
    let x_all: Vec<Vec<f64>> = xcs
        .iter()
        .map(|c| {
            sheet.values_of(c).map(|v| v.into_owned()).ok_or_else(|| AnalysisError::NotNumeric(c.short_name.clone()))
        })
        .collect::<Result<_, _>>()?;
    let y_numbers = sheet.values_of(yc).map(|v| v.into_owned());
    let classify = match spec.task {
        ForestTask::Classification => true,
        ForestTask::Regression => false,
        ForestTask::Auto => y_numbers.is_none(),
    };
    if !classify && y_numbers.is_none() {
        return Err(AnalysisError::NotNumeric(yc.short_name.clone()));
    }
    let n = x_all.iter().map(Vec::len).chain([yc.data.len()]).max().unwrap_or(0);
    if n == 0 {
        return Err(AnalysisError::Empty(yc.short_name.clone()));
    }
    let (first, last) = resolve_rows(rows, &yc.short_name, &[], n)?;
    // Complete rows only.
    let mut used = Vec::new();
    let mut x = Vec::new();
    let mut labels: Vec<String> = Vec::new();
    let mut classes: Vec<usize> = Vec::new();
    let mut values: Vec<f64> = Vec::new();
    for r in first..=last {
        let row: Vec<f64> = x_all.iter().map(|c| c.get(r).copied().unwrap_or(f64::NAN)).collect();
        if row.iter().any(|v| !v.is_finite()) {
            continue;
        }
        if classify {
            let Some(label) = group_cell(yc, r) else { continue };
            let k = labels.iter().position(|l| *l == label).unwrap_or_else(|| {
                labels.push(label);
                labels.len() - 1
            });
            classes.push(k);
        } else {
            let v = y_numbers.as_ref().and_then(|y| y.get(r)).copied().unwrap_or(f64::NAN);
            if !v.is_finite() {
                continue;
            }
            values.push(v);
        }
        used.push(r);
        x.push(row);
    }
    if x.len() < 2 {
        return Err(AnalysisError::BadOption("fewer than 2 complete rows".into()));
    }
    let target = if classify {
        Target::Class { y: classes.clone(), classes: labels.len() }
    } else {
        Target::Value(values.clone())
    };
    let opts = ForestOptions {
        trees: spec.trees,
        mtry: (spec.mtry > 0).then_some(spec.mtry),
        min_leaf: if spec.min_leaf > 0 {
            spec.min_leaf
        } else if classify {
            1
        } else {
            5
        },
        max_depth: (spec.max_depth > 0).then_some(spec.max_depth),
        seed: spec.seed,
        bootstrap: true,
    };
    let f = forest(&x, &target, opts).map_err(|e| AnalysisError::BadOption(e.to_string()))?;
    let names: Vec<String> = xcs.iter().map(|c| column_label(c)).collect();
    let mut table = ResultTable::new(&["Importance"]);
    for (name, v) in names.iter().zip(&f.importance) {
        table.push_row(name.clone(), vec![*v], vec![String::new()]);
    }
    let what = if classify { "out-of-bag error rate" } else { "out-of-bag mean squared error" };
    let top = names
        .iter()
        .zip(&f.importance)
        .max_by(|a, b| a.1.total_cmp(b.1))
        .map(|(n, v)| format!("{n} ({})", num(*v)))
        .unwrap_or_default();
    let mut notes = vec![format!(
        "{} forest of {} trees on {} complete rows ({}): {what} {}; most important predictor {top}.",
        if classify { "Classification" } else { "Regression" },
        spec.trees,
        x.len(),
        if classify { format!("{} classes", labels.len()) } else { "numeric response".to_string() },
        num(f.oob_error)
    )];
    let mut sheet_cols = vec![SheetColumn {
        designation: Designation::Label,
        long_name: "Predictor".into(),
        units: String::new(),
        comments: String::new(),
        data: ColumnData::text(names.clone()),
    }];
    sheet_cols.push(number_column("Importance", Designation::Y, f.importance.clone()));
    let shift = sheet_cols.len();
    let mut on_sheet = vec![number_column("Index", Designation::X, (1..=names.len()).map(|i| i as f64).collect())];
    on_sheet.push(number_column("Importance (plot)", Designation::Y, f.importance.clone()));
    on_sheet.push(number_column("Row", Designation::X, used.iter().map(|r| (r + 1) as f64).collect()));
    if classify {
        let name = |k: f64| if k.is_finite() { labels[k as usize].clone() } else { String::new() };
        on_sheet.push(text_column(&column_label(yc), classes.iter().map(|k| labels[*k].clone()).collect()));
        on_sheet.push(text_column("Out-of-bag prediction", f.oob.iter().map(|v| name(*v)).collect()));
        on_sheet.push(text_column("Forest prediction", f.fitted.iter().map(|v| name(*v)).collect()));
        // Confusion matrix of the out-of-bag predictions: one row per actual class.
        on_sheet.push(text_column("Actual \\ out-of-bag predicted", labels.clone()));
        for (j, lab) in labels.iter().enumerate() {
            let counts = (0..labels.len())
                .map(|i| {
                    classes.iter().zip(&f.oob).filter(|(a, p)| **a == i && p.is_finite() && **p as usize == j).count()
                        as f64
                })
                .collect();
            on_sheet.push(number_column(lab, Designation::Y, counts));
        }
        notes.push(
            "The confusion matrix counts rows by actual class (rows) and out-of-bag prediction (columns).".into(),
        );
    } else {
        on_sheet.push(number_column(&column_label(yc), Designation::Y, values));
        on_sheet.push(number_column("Out-of-bag prediction", Designation::Y, f.oob.clone()));
        on_sheet.push(number_column("Forest prediction", Designation::Y, f.fitted.clone()));
    }
    let graph = spec.graph.then(|| XyGraph {
        plots: vec![GraphPlot { x: 0, y: 1, kind: PlotKind::Column, label: "Importance".into(), line_of: None }],
        x_scale: ScaleKind::Linear,
        y_scale: ScaleKind::Linear,
        x_title: "Predictor (index)".into(),
        y_title: "Importance".into(),
        look: Default::default(),
    });
    sheet_cols.extend(on_sheet);
    Ok(Output { table: Some(table), on_sheet: sheet_cols, notes, graph, more_graphs: Vec::new() }.shift_graphs(shift))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(columns: Vec<ColumnId>, spec: RandomForestSpec) -> Request {
        Request { columns, rows: RowRange::All, tool: Tool::RandomForest(spec) }
    }

    #[test]
    fn a_text_response_is_classified_and_a_number_regressed() {
        let mut p = Project::new();
        let book = p.add_workbook("rf");
        let a: Vec<f64> = (0..40).map(|i| f64::from(i % 10)).collect();
        let b: Vec<f64> = (0..40).map(|i| f64::from((i * 7) % 5)).collect();
        let label: Vec<String> = (0..40).map(|i| if i % 10 >= 5 { "high".into() } else { "low".into() }).collect();
        let value: Vec<f64> = (0..40).map(|i| 2.0 * f64::from(i % 10)).collect();
        let ac = p.add_column(book, 0, Designation::Y, ColumnData::numeric(a)).unwrap();
        let bc = p.add_column(book, 0, Designation::Y, ColumnData::numeric(b)).unwrap();
        let lc = p.add_column(book, 0, Designation::Y, ColumnData::text(label)).unwrap();
        let vc = p.add_column(book, 0, Designation::Y, ColumnData::numeric(value)).unwrap();
        let spec = RandomForestSpec { trees: 20, ..Default::default() };
        let out = p.analyze(&request(vec![ac, bc, lc], spec)).unwrap();
        let t = out.table.as_ref().unwrap();
        assert!(t.rows[0].1[0] > 0.8, "the first predictor decides: {:?}", t.rows);
        assert!(
            out.notes[0].contains("Classification forest of 20 trees") && out.notes[0].contains("2 classes"),
            "{:?}",
            out.notes
        );
        let cols = super::super::dist::plot_columns(&p, out.new_graph.unwrap());
        assert_eq!(cols[0], ("Index".to_string(), "Importance (plot)".to_string()));
        let out = p.analyze(&request(vec![ac, bc, vc], RandomForestSpec { graph: false, ..spec })).unwrap();
        assert!(out.notes[0].contains("Regression forest"), "{:?}", out.notes);
        assert!(p.analyze(&request(vec![ac, lc], RandomForestSpec { task: ForestTask::Regression, ..spec })).is_err());
    }
}
