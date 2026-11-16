//! Nonlinear Implicit Curve Fit and Compare Datasets without a window: the shared specifications and runs used by
//! the command line (`sheaf fit-implicit`, `sheaf compare-datasets`). The numbers and the report sheets come from
//! `Project::fit_implicit` and `Project::compare_datasets` of `sheaf-core`, so the app and the command line agree.
//! What Origin does and what is Sheaf's choice is in `docs/spec/fitting.md`, sections 17 and 18.

use crate::analyze::row_range;
use crate::fit::{model_from, name_value};
use crate::pick::{named_columns, pick_book, pick_sheet};
use anyhow::{Result, anyhow, bail};
use serde::{Deserialize, Serialize};
use sheaf_core::Project;
use sheaf_core::fitting::{
    CompareDatasetsOptions, CompareDatasetsOutcome, FitRequest, ImplicitOutcome, ImplicitRequest, ParamSetting,
};
use sheaf_fit::implicit::ImplicitFunction;

/// A Nonlinear Implicit Curve Fit of two columns.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ImplicitOps {
    /// `Circle` or `Ellipse` (default Circle).
    pub function: String,
    /// The X and Y columns of the points, by name.
    pub x: String,
    pub y: String,
    /// Starting values, each `NAME=VALUE`; the others are estimated from the data.
    pub init: Vec<String>,
    pub rows: Option<String>,
    pub x_range: Option<String>,
    pub confidence: Option<f64>,
    pub max_iterations: Option<usize>,
    pub tolerance: Option<f64>,
    pub book: Option<String>,
    pub sheet: Option<String>,
}

/// Fits the implicit function in `project`; a failed run leaves the project as it was.
pub fn run_implicit(project: &mut Project, spec: &ImplicitOps) -> Result<ImplicitOutcome> {
    let name = if spec.function.trim().is_empty() { "Circle" } else { spec.function.trim() };
    let function = ImplicitFunction::by_name(name).ok_or_else(|| {
        anyhow!("there is no implicit function '{name}'; Circle and Ellipse are available (PlaneMod is not)")
    })?;
    let book = pick_book(project, spec.book.as_deref())?;
    let sheet = pick_sheet(project, book, spec.sheet.as_deref())?;
    if spec.x.trim().is_empty() || spec.y.trim().is_empty() {
        bail!("name the X and Y columns of the points with --x and --y");
    }
    let x = named_columns(project, book, sheet, std::slice::from_ref(&spec.x))?[0];
    let y = named_columns(project, book, sheet, std::slice::from_ref(&spec.y))?[0];
    let mut request = ImplicitRequest::new(x, y, function.name());
    let values: Vec<(String, f64)> =
        spec.init.iter().flat_map(|t| t.split(',')).map(|t| name_value(t, "--init")).collect::<Result<_>>()?;
    if let Some((bad, _)) = values.iter().find(|(n, _)| !function.params().contains(&n.as_str())) {
        bail!("{} has no parameter '{bad}' (its parameters are {})", function.name(), function.params().join(", "));
    }
    if !values.is_empty() {
        let numbers =
            |id| project.locate(id).and_then(|(_, _, c)| c.numeric().map(<[f64]>::to_vec)).unwrap_or_default();
        let start = function.initial(&numbers(x), &numbers(y));
        request.initial = function
            .params()
            .iter()
            .zip(start)
            .map(|(p, default)| values.iter().find(|(n, _)| n == p).map_or(default, |(_, v)| *v))
            .collect();
    }
    request.rows = row_range(spec.rows.as_deref(), spec.x_range.as_deref())?;
    if let Some(c) = spec.confidence {
        request.confidence_level = c;
    }
    if let Some(n) = spec.max_iterations {
        request.max_iterations = n;
    }
    if let Some(t) = spec.tolerance {
        request.tolerance = t;
    }
    Ok(project.fit_implicit(&request)?)
}

/// Compare Datasets of two Y columns of one sheet, each with its own X column.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CompareDatasetsOps {
    /// The model: `linear`, `poly:N` or a built-in function (default linear).
    pub model: String,
    /// The Y columns of the first and the second dataset.
    pub y1: String,
    pub y2: String,
    /// Starting values for the fit of both, each `NAME=VALUE`.
    pub init: Vec<String>,
    pub name1: Option<String>,
    pub name2: Option<String>,
    /// Leave out the parameter and statistics blocks (Origin's two boxes, both on by default).
    pub no_parameters: bool,
    pub no_statistics: bool,
    pub book: Option<String>,
    pub sheet: Option<String>,
}

/// Compares the datasets in `project`; a failed run leaves the project as it was.
pub fn run_compare_datasets(project: &mut Project, spec: &CompareDatasetsOps) -> Result<CompareDatasetsOutcome> {
    let book = pick_book(project, spec.book.as_deref())?;
    let sheet = pick_sheet(project, book, spec.sheet.as_deref())?;
    if spec.y1.trim().is_empty() || spec.y2.trim().is_empty() {
        bail!("name the Y columns of the two datasets with --y1 and --y2");
    }
    let ids = named_columns(project, book, sheet, &[spec.y1.clone(), spec.y2.clone()])?;
    let model = model_from(if spec.model.trim().is_empty() { "linear" } else { &spec.model })?;
    let mut settings: Vec<ParamSetting> = Vec::new();
    for t in spec.init.iter().flat_map(|t| t.split(',')) {
        let (n, v) = name_value(t, "--init")?;
        settings.push(ParamSetting { name: n, initial: Some(v), ..Default::default() });
    }
    let mut a = FitRequest::new(ids[0], model);
    a.params = settings;
    let mut b = a.clone();
    b.y = ids[1];
    let options = CompareDatasetsOptions {
        parameters: !spec.no_parameters,
        statistics: !spec.no_statistics,
        names: [
            spec.name1.clone().unwrap_or_else(|| "Data1".into()),
            spec.name2.clone().unwrap_or_else(|| "Data2".into()),
        ],
    };
    Ok(project.compare_datasets(&a, &b, &options)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sheaf_core::column::{ColumnData, Designation};

    fn project(columns: &[(&str, Designation, Vec<f64>)]) -> Project {
        let mut p = Project::new();
        let book = p.add_workbook("Book1");
        for (name, d, v) in columns {
            let id = p.add_column(book, 0, *d, ColumnData::numeric(v.clone())).unwrap();
            p.column_mut(id).unwrap().short_name = (*name).to_string();
        }
        p
    }

    #[test]
    fn a_circle_is_fitted_from_named_columns_and_init_must_name_parameters() {
        let t: Vec<f64> = (0..12).map(|k| f64::from(k) * 0.5).collect();
        let x: Vec<f64> = t.iter().map(|t| 1.0 + 2.0 * t.cos()).collect();
        let y: Vec<f64> = t.iter().map(|t| -1.0 + 2.0 * t.sin()).collect();
        let mut p = project(&[("A", Designation::X, x), ("B", Designation::Y, y)]);
        let spec = ImplicitOps { x: "A".into(), y: "B".into(), ..Default::default() };
        let out = run_implicit(&mut p, &spec).unwrap();
        assert!((out.report.params[2].value - 2.0).abs() < 1e-6);
        let bad = ImplicitOps { init: vec!["q=1".into()], ..spec.clone() };
        assert!(run_implicit(&mut p, &bad).unwrap_err().to_string().contains("no parameter 'q'"));
        let named = ImplicitOps { function: "PlaneMod".into(), ..spec };
        assert!(run_implicit(&mut p, &named).unwrap_err().to_string().contains("no implicit function"));
    }

    #[test]
    fn two_columns_are_compared_with_the_default_names() {
        let x: Vec<f64> = (0..8).map(f64::from).collect();
        let a: Vec<f64> = x.iter().map(|v| 1.0 + 2.0 * v + 0.01 * (v * 3.0).sin()).collect();
        let b: Vec<f64> = x.iter().map(|v| 5.0 + 2.0 * v + 0.01 * (v * 5.0).sin()).collect();
        let mut p = project(&[
            ("A", Designation::X, x.clone()),
            ("B", Designation::Y, a),
            ("C", Designation::X, x),
            ("D", Designation::Y, b),
        ]);
        let spec = CompareDatasetsOps { y1: "B".into(), y2: "D".into(), ..Default::default() };
        let out = run_compare_datasets(&mut p, &spec).unwrap();
        assert_eq!(out.comparison.different(), Some(true));
        let ws = &p.workbook(out.book).unwrap().sheets[out.sheet];
        assert_eq!((ws.columns[1].long_name.as_str(), ws.columns[2].long_name.as_str()), ("Data1", "Data2"));
        assert!(run_compare_datasets(&mut p, &CompareDatasetsOps { y1: "B".into(), ..Default::default() }).is_err());
    }
}
