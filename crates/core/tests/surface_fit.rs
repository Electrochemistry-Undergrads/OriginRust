//! Surface fits on XYZ worksheet columns (`docs/spec/fitting.md`, section 15).

use sheaf_core::fitting::{FitModel, FitRequest, SurfaceColumns, surface_columns, surface_functions};
use sheaf_core::{ColumnData, ColumnId, Designation, Project};

/// An 11 x 11 grid with X, Y, Z columns: z = 1.5 + 0.8 x - 0.6 y plus a little fixed noise.
fn xyz_sheet() -> (Project, ColumnId, ColumnId, ColumnId) {
    let mut p = Project::new();
    let book = p.add_workbook("data");
    let (mut xs, mut ys, mut zs) = (Vec::new(), Vec::new(), Vec::new());
    for i in 0..121 {
        let (x, y) = (f64::from(i % 11) * 0.5, f64::from(i / 11) * 0.5);
        xs.push(x);
        ys.push(y);
        zs.push(1.5 + 0.8 * x - 0.6 * y + 0.03 * (37.0 * f64::from(i)).sin());
    }
    let x = p.add_column(book, 0, Designation::X, ColumnData::numeric(xs)).unwrap();
    let y = p.add_column(book, 0, Designation::Y, ColumnData::numeric(ys)).unwrap();
    let z = p.add_column(book, 0, Designation::Z, ColumnData::numeric(zs)).unwrap();
    (p, x, y, z)
}

fn request(z: ColumnId, function: &str) -> FitRequest {
    let mut r = FitRequest::new(z, FitModel::BuiltIn(function.into()));
    r.surface = Some(SurfaceColumns::default());
    r
}

#[test]
fn the_x_and_y_columns_of_a_z_column_follow_the_designations() {
    let (p, x, y, z) = xyz_sheet();
    assert_eq!(surface_columns(&p, z), Some((x, y)));
}

#[test]
fn a_plane_is_fitted_and_the_results_are_placed() {
    let (mut p, x, y, z) = xyz_sheet();
    let out = p.fit(&request(z, "Plane")).unwrap();
    let get = |n: &str| out.report.params.iter().find(|q| q.name == n).unwrap().value;
    assert!((get("z0") - 1.5).abs() < 0.05, "z0 = {}", get("z0"));
    assert!((get("a") - 0.8).abs() < 0.02);
    assert!((get("b") + 0.6).abs() < 0.02);
    assert_eq!(out.report.stats.n_points, 121);
    let book = p.workbook(out.book).unwrap();
    let names: Vec<&str> = book.sheets.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(&names[1..], ["FitSurface1", "FitSurface1Stats", "FitSurfaceCurve1"]);
    // The fitted and residual columns follow the Z column; the grid sheet has X, Y and Z.
    let sheet = &book.sheets[0];
    let at = sheet.column_index(z).unwrap();
    assert_eq!(sheet.columns[at + 1].id, out.fitted_column);
    assert_eq!(sheet.columns[at + 2].id, out.residual_column);
    assert!(sheet.columns[at + 1].fit_result);
    let grid = &book.sheets[3];
    let designations: Vec<Designation> = grid.columns.iter().map(|c| c.designation).collect();
    assert_eq!(designations, [Designation::X, Designation::Y, Designation::Z]);
    assert_eq!(grid.row_count(), 900);
    // X and Y, and the fit result columns, are not offered as Y columns again.
    assert_eq!(surface_columns(&p, z), Some((x, y)));
    // The saved project holds the results and reads back equal.
    let json = serde_json::to_string(&p).unwrap();
    let back: Project = serde_json::from_str(&json).unwrap();
    // (missing values are NaN, which never compares equal, so compare the saved text)
    assert_eq!(serde_json::to_string(&back).unwrap(), json);
}

#[test]
fn chosen_columns_rows_and_missing_values_work_like_curve_fits() {
    let (mut p, x, y, z) = xyz_sheet();
    let (book, _, _) = p.locate(z).map(|(b, s, c)| (b.id, s.name.clone(), c.id)).unwrap();
    // A missing z leaves its point out.
    if let Some(ColumnData::Numeric(v)) = p.column_mut(z).map(|c| &mut c.data) {
        std::sync::Arc::make_mut(v)[3] = f64::NAN;
    }
    let mut r = request(z, "Plane");
    r.surface = Some(SurfaceColumns { x: Some(y), y: Some(x) });
    let out = p.fit(&r).unwrap();
    assert_eq!(out.report.stats.n_points, 120);
    let get = |n: &str| out.report.params.iter().find(|q| q.name == n).unwrap().value;
    // X and Y exchanged: the slopes swap.
    assert!((get("a") + 0.6).abs() < 0.02 && (get("b") - 0.8).abs() < 0.02);
    assert!(p.workbook(book).is_some());
}

#[test]
fn start_values_and_a_fixed_parameter_apply() {
    let (mut p, _, _, z) = xyz_sheet();
    let mut r = request(z, "Plane");
    r.params = vec![sheaf_core::fitting::ParamSetting {
        name: "z0".into(),
        initial: Some(2.0),
        fixed: true,
        ..Default::default()
    }];
    let out = p.fit(&r).unwrap();
    let z0 = &out.report.params[0];
    assert!(z0.fixed && z0.value == 2.0);
}

#[test]
fn bad_requests_change_nothing() {
    let (mut p, _, _, z) = xyz_sheet();
    let before = p.clone();
    for model in [FitModel::Linear, FitModel::BuiltIn("Gauss".into()), FitModel::BuiltIn("Nope".into())] {
        let mut r = request(z, "Plane");
        r.model = model;
        assert!(p.fit(&r).is_err());
    }
    let mut r = request(z, "Plane");
    r.surface = Some(SurfaceColumns { x: Some(z), y: None });
    assert!(p.fit(&r).is_err());
    // A curve function of a surface request, and a surface function of a curve request, fail.
    assert!(p.fit(&FitRequest::new(z, FitModel::BuiltIn("Plane".into()))).is_err());
    assert_eq!(p, before);
}

#[test]
fn a_formula_in_x_and_y_is_a_surface_model() {
    let (mut p, _, _, z) = xyz_sheet();
    let mut r = request(z, "Plane");
    r.model = FitModel::Formula { formula: "c + p*x + q*y".into(), params: vec!["c".into(), "p".into(), "q".into()] };
    let out = p.fit(&r).unwrap();
    assert!((out.report.params[1].value - 0.8).abs() < 0.02);
}

#[test]
fn the_surface_function_list_has_origins_category() {
    let list = surface_functions();
    assert_eq!(list.len(), 22);
    assert!(list.iter().all(|f| f.category == "Surface Fitting" && f.formula.is_some()));
    assert!(list.iter().any(|f| f.name == "Gauss2D" && f.params.len() == 6));
}
