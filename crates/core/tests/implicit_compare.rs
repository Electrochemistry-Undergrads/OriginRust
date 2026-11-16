//! Nonlinear Implicit Curve Fit and Compare Datasets on worksheet columns (`docs/spec/fitting.md`, sections 17 and 18).

use sheaf_core::fitting::{CompareDatasetsOptions, FitModel, FitRequest, ImplicitRequest};
use sheaf_core::{ColumnData, ColumnId, Designation, Project};

const X: [f64; 24] = [
    6.8771, 6.7559, 5.9526, 5.2031, 3.9579, 2.8619, 1.6235, 0.2221, -0.7142, -1.9336, -2.5504, -2.9623, -2.9532,
    -2.5886, -2.2021, -1.2785, -0.1731, 1.1322, 2.7632, 3.8372, 4.8968, 5.8634, 6.5663, 6.9798,
];
const Y: [f64; 24] = [
    3.5294, 4.7287, 5.9146, 6.9587, 7.4546, 7.9726, 8.0536, 7.6544, 7.0358, 6.0738, 5.0668, 3.8005, 2.5562, 1.247,
    0.1363, -0.8845, -1.5579, -1.9176, -1.9789, -1.7822, -1.0059, -0.1353, 0.8643, 2.1296,
];

fn circle_project() -> (Project, ColumnId, ColumnId) {
    let mut p = Project::new();
    let book = p.add_workbook("Book1");
    let x = p.add_column(book, 0, Designation::X, ColumnData::numeric(X.to_vec())).unwrap();
    let y = p.add_column(book, 0, Designation::Y, ColumnData::numeric(Y.to_vec())).unwrap();
    (p, x, y)
}

#[test]
fn the_circle_report_is_placed_in_three_sheets_with_the_distances_next_to_y() {
    let (mut p, x, y) = circle_project();
    let out = p.fit_implicit(&ImplicitRequest::new(x, y, "circle")).unwrap();
    let get = |n: &str| out.report.params.iter().find(|q| q.name == n).unwrap().value;
    assert!((get("xc") - 2.0013316329).abs() < 1e-6 && (get("r") - 5.0035928649).abs() < 1e-6);
    let book = p.workbook(out.book).unwrap();
    let names: Vec<&str> = book.sheets.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, ["Sheet1", "FitImplicit1", "FitImplicit1Stats", "FitImplicitCurve1"]);
    assert_eq!(book.sheets[1].row_count(), 3);
    assert_eq!(book.sheets[3].row_count(), 361, "a closed curve of 360 segments");
    let sheet = &book.sheets[0];
    let at = sheet.column_index(y).unwrap();
    assert_eq!(sheet.columns[at + 1].id, out.distance_column);
    assert!(sheet.columns[at + 1].fit_result);
    let d = sheet.columns[at + 1].numeric().unwrap();
    let expected = (X[0] - get("xc")).hypot(Y[0] - get("yc")) - get("r");
    assert!((d[0] - expected).abs() < 1e-12);
    // A second run takes the next sheet names.
    let again = p.fit_implicit(&ImplicitRequest::new(x, y, "Circle")).unwrap();
    assert_eq!(p.workbook(again.book).unwrap().sheets[again.param_sheet].name, "FitImplicit2");
}

#[test]
fn a_bad_function_or_rows_change_nothing() {
    let (mut p, x, y) = circle_project();
    let before = serde_json::to_string(&p).unwrap();
    assert!(p.fit_implicit(&ImplicitRequest::new(x, y, "PlaneMod")).is_err());
    let mut r = ImplicitRequest::new(x, y, "Circle");
    r.rows = sheaf_core::analysis::RowRange::Rows(1, 3);
    assert!(p.fit_implicit(&r).is_err(), "three points cannot fit three parameters");
    assert_eq!(serde_json::to_string(&p).unwrap(), before);
}

fn two_datasets(second: impl Fn(f64) -> f64) -> (Project, ColumnId, ColumnId) {
    let mut p = Project::new();
    let book = p.add_workbook("Book1");
    let noise = [0.05, -0.04, 0.03, -0.02, 0.06, -0.05, 0.01, -0.03, 0.04, -0.06];
    let x: Vec<f64> = (0..10).map(f64::from).collect();
    let y1: Vec<f64> = x.iter().zip(&noise).map(|(x, e)| 1.0 + 2.0 * x + e).collect();
    let y2: Vec<f64> = x.iter().zip(noise.iter().rev()).map(|(x, e)| second(*x) + e).collect();
    p.add_column(book, 0, Designation::X, ColumnData::numeric(x.clone())).unwrap();
    let a = p.add_column(book, 0, Designation::Y, ColumnData::numeric(y1)).unwrap();
    p.add_column(book, 0, Designation::X, ColumnData::numeric(x)).unwrap();
    let b = p.add_column(book, 0, Designation::Y, ColumnData::numeric(y2)).unwrap();
    (p, a, b)
}

#[test]
fn compare_datasets_reports_the_f_test_and_the_parameters() {
    let (mut p, a, b) = two_datasets(|x| 3.0 + 2.5 * x);
    let out = p
        .compare_datasets(
            &FitRequest::new(a, FitModel::Linear),
            &FitRequest::new(b, FitModel::Linear),
            &CompareDatasetsOptions::default(),
        )
        .unwrap();
    assert!(out.comparison.prob < 1e-6);
    assert_eq!(out.comparison.different(), Some(true));
    let book = p.workbook(out.book).unwrap();
    let ws = &book.sheets[out.sheet];
    assert_eq!(ws.name, "FitCompareData1");
    let heads: Vec<&str> = ws.columns.iter().map(|c| c.long_name.as_str()).collect();
    assert_eq!(heads, ["Quantity", "Data1", "Data2", "Text"]);
    let names: Vec<String> = ws.columns[0].text().unwrap().to_vec();
    assert!(names.iter().any(|n| n == "Intercept") && names.iter().any(|n| n == "Prob>F"));
    assert!(names.iter().any(|n| n == "R-square (COD)"));
}

#[test]
fn the_two_tick_boxes_and_names_decide_the_blocks() {
    let (mut p, a, b) = two_datasets(|x| 1.0 + 2.0 * x);
    let opts = CompareDatasetsOptions { parameters: false, statistics: false, names: ["Old".into(), "New".into()] };
    let out = p
        .compare_datasets(&FitRequest::new(a, FitModel::Linear), &FitRequest::new(b, FitModel::Linear), &opts)
        .unwrap();
    assert_eq!(out.comparison.different(), Some(false));
    let ws = &p.workbook(out.book).unwrap().sheets[out.sheet];
    assert_eq!(ws.columns[1].long_name, "Old");
    let names: Vec<String> = ws.columns[0].text().unwrap().to_vec();
    assert!(!names.iter().any(|n| n == "Intercept" || n == "Number of points"));
    assert!(names.iter().any(|n| n == "F value"));
}

#[test]
fn different_models_or_the_same_dataset_are_refused_and_change_nothing() {
    let (mut p, a, b) = two_datasets(|x| 3.0 + x);
    let before = serde_json::to_string(&p).unwrap();
    let opts = CompareDatasetsOptions::default();
    let line = FitRequest::new(a, FitModel::Linear);
    let quad = FitRequest::new(b, FitModel::Polynomial { order: 2 });
    assert!(p.compare_datasets(&line, &quad, &opts).is_err());
    assert!(p.compare_datasets(&line, &line, &opts).is_err());
    assert_eq!(serde_json::to_string(&p).unwrap(), before);
}

#[test]
fn a_nonlinear_model_is_compared_too() {
    let mut p = Project::new();
    let book = p.add_workbook("Book1");
    let x: Vec<f64> = (0..20).map(|i| f64::from(i) * 0.5).collect();
    let wiggle = |i: usize| 0.01 * (7.0 * i as f64).sin();
    let y1: Vec<f64> = x.iter().enumerate().map(|(i, x)| 1.0 + 4.0 * (-x / 3.0).exp() + wiggle(i)).collect();
    let y2: Vec<f64> = x.iter().enumerate().map(|(i, x)| 1.0 + 4.0 * (-x / 3.0).exp() + wiggle(i + 5)).collect();
    p.add_column(book, 0, Designation::X, ColumnData::numeric(x.clone())).unwrap();
    let a = p.add_column(book, 0, Designation::Y, ColumnData::numeric(y1)).unwrap();
    p.add_column(book, 0, Designation::X, ColumnData::numeric(x)).unwrap();
    let b = p.add_column(book, 0, Designation::Y, ColumnData::numeric(y2)).unwrap();
    let model =
        FitModel::Formula { formula: "y0 + A*exp(-x/t)".into(), params: vec!["y0".into(), "A".into(), "t".into()] };
    let out = p
        .compare_datasets(
            &FitRequest::new(a, model.clone()),
            &FitRequest::new(b, model),
            &CompareDatasetsOptions::default(),
        )
        .unwrap();
    assert_eq!(out.comparison.params, 3);
    assert_eq!(out.comparison.different(), Some(false), "Prob>F {}", out.comparison.prob);
}
