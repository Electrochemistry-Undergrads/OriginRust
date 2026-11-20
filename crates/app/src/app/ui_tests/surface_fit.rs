use super::*;

/// An XYZ file on an 11 x 11 grid: z = 1.5 + 0.8 x - 0.6 y plus a little fixed noise.
fn xyz_file(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sheaf-app-ui-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut text = String::from("x,y,z\n");
    for i in 0..121 {
        let (x, y) = (f64::from(i % 11) * 0.5, f64::from(i / 11) * 0.5);
        text.push_str(&format!("{x},{y},{}\n", 1.5 + 0.8 * x - 0.6 * y + 0.03 * (37.0 * f64::from(i)).sin()));
    }
    let path = dir.join("xyz.csv");
    std::fs::write(&path, text).unwrap();
    path
}

/// Starts with the file and designates its third column as Z.
fn start_xyz(tag: &str) -> Harness<'static, SheafApp> {
    let mut h = start(vec![xyz_file(tag)]);
    let z = project(&h).workbooks().next().unwrap().sheet().column_by_name("C").unwrap().id;
    h.state_mut().doc.apply("Set Z", |p| p.column_mut(z).unwrap().designation = Designation::Z);
    settle(&mut h);
    h
}

#[test]
fn nonlinear_surface_fit_is_in_the_menu_and_opens_on_the_surface_functions() {
    let mut h = start_xyz("surface-menu");
    menu_path(&mut h, &["Analysis", "Fitting"]);
    assert!(entry_enabled(&h, "Nonlinear Surface Fit..."));
    click(&mut h, "Nonlinear Surface Fit...");
    assert_eq!(h.query_all_by_label("Find Z").count(), 1, "Origin names the page Find Z for a surface");
    assert_eq!(h.query_all_by_label("Find X/Y").count(), 0);
    let d = h.state().fit.as_ref().expect("the window opened");
    let (function, offered, _) = d.function_for_test();
    assert_eq!(function, "Gauss2D");
    assert_eq!(offered.len(), 22, "the category Surface Fitting holds the 22 surface functions");
    let request = d.request_for_test().unwrap();
    let columns = request.surface.expect("a surface request");
    let book = project(&h).workbooks().next().unwrap();
    let sheet = book.sheet();
    assert_eq!(request.y, sheet.column_by_name("C").unwrap().id, "Z is the fitted column");
    assert_eq!(columns.x, Some(sheet.column_by_name("A").unwrap().id));
    assert_eq!(columns.y, Some(sheet.column_by_name("B").unwrap().id));
}

#[test]
fn a_plane_is_fitted_from_the_window_and_the_results_are_one_undo_step() {
    let mut h = start_xyz("surface-fit");
    let before = project(&h).clone();
    menu_path(&mut h, &["Analysis", "Fitting", "Nonlinear Surface Fit..."]);
    h.state_mut().fit.as_mut().unwrap().choose_for_test("Plane");
    settle(&mut h);
    // Fit until converged tries the fit and keeps the window; the table shows the values.
    click(&mut h, "Fit until converged");
    let d = h.state().fit.as_ref().unwrap();
    assert!(d.problem_for_test().is_none(), "{:?}", d.problem_for_test());
    assert!(d.message_for_test().contains("converged"), "{}", d.message_for_test());
    let rows = d.rows_for_test();
    assert!((rows[1].2 - 0.8).abs() < 0.02 && (rows[2].2 + 0.6).abs() < 0.02, "{rows:?}");
    // Fit writes the results and closes the window.
    click(&mut h, "Fit");
    assert!(h.state().fit.is_none());
    let book = project(&h).workbooks().next().unwrap();
    let names: Vec<&str> = book.sheets.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(&names[1..], ["FitSurface1", "FitSurface1Stats", "FitSurfaceCurve1"]);
    assert_eq!(project(&h).graphs().count(), before.graphs().count(), "a surface fit draws no 2D graph");
    assert!(h.state().status.contains("Surface fit Plane"), "{}", h.state().status);
    menu(&mut h, "Edit", "Undo Fit");
    assert_eq!(project(&h), &before);
}

#[test]
fn a_matrix_is_surface_fitted_through_xyz_columns() {
    use sheaf_core::matrix_project::MatrixOp;
    let mut h = start(Vec::new());
    let ctx = h.ctx.clone();
    let mut id = sheaf_core::WindowId(0);
    h.state_mut().doc.apply("Matrix", |p| {
        id = p.add_matrix_book("");
        p.apply_matrix_op(id, 0, &MatrixOp::SetDimensions { rows: 6, cols: 6, ranges: Some((0.0, 5.0, 0.0, 5.0)) })
            .unwrap();
        p.apply_matrix_op(id, 0, &MatrixOp::SetValues("1 + 0.5*x - 0.2*y".into())).unwrap();
    });
    settle(&mut h);
    h.state_mut().apply(Action::MatrixCmd { id, cmd: menus::MatrixCmd::SurfaceFit }, &ctx);
    settle(&mut h);
    assert_eq!(h.state().doc.undo_label(), Some("Matrix to XYZ for surface fit"));
    let book = project(&h).workbooks().next().expect("the XYZ workbook");
    let designations: Vec<_> = book.sheet().columns.iter().map(|c| c.designation).collect();
    assert_eq!(designations, [Designation::X, Designation::Y, Designation::Z]);
    assert_eq!(book.sheet().columns[2].len(), 36);
    let d = h.state().fit.as_ref().expect("the surface-fit window opened");
    assert!(d.request_for_test().unwrap().surface.is_some(), "a surface request on the Z column");
}
