//! Analysis > Fitting > Compare Datasets... and Nonlinear Implicit Curve Fit...: the menu items are enabled, the
//! windows run the tools of `sheaf-core` and each OK is one undo step (fitting.md 17 and 18).

use super::*;

fn circle_file(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sheaf-app-ui-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("circle.csv");
    // Origin's measured data (tools/origin-ref/t91/circle-data.json): centre (2, 3), radius 5.
    let x = [
        6.8771, 6.7559, 5.9526, 5.2031, 3.9579, 2.8619, 1.6235, 0.2221, -0.7142, -1.9336, -2.5504, -2.9623, -2.9532,
        -2.5886, -2.2021, -1.2785, -0.1731, 1.1322, 2.7632, 3.8372, 4.8968, 5.8634, 6.5663, 6.9798,
    ];
    let y = [
        3.5294, 4.7287, 5.9146, 6.9587, 7.4546, 7.9726, 8.0536, 7.6544, 7.0358, 6.0738, 5.0668, 3.8005, 2.5562, 1.247,
        0.1363, -0.8845, -1.5579, -1.9176, -1.9789, -1.7822, -1.0059, -0.1353, 0.8643, 2.1296,
    ];
    let mut csv = String::from("x,y\n");
    for (a, b) in x.iter().zip(&y) {
        csv += &format!("{a},{b}\n");
    }
    std::fs::write(&path, csv).unwrap();
    path
}

#[test]
fn both_items_are_enabled_in_the_fitting_menu() {
    let mut h = start(vec![circle_file("fitds-menu")]);
    menu_path(&mut h, &["Analysis", "Fitting"]);
    assert!(entry_enabled(&h, "Nonlinear Implicit Curve Fit..."));
    assert!(entry_enabled(&h, "Compare Datasets..."));
}

#[test]
fn the_implicit_fit_window_fits_the_circle_and_is_one_undo_step() {
    let mut h = start(vec![circle_file("fitds-circle")]);
    let before = project(&h).clone();
    menu_path(&mut h, &["Analysis", "Fitting", "Nonlinear Implicit Curve Fit..."]);
    settle(&mut h);
    h.get_by_label("Nonlinear Implicit Curve Fit");
    assert!(h.state().fit_more.is_some());
    click(&mut h, "OK");
    assert!(h.state().fit_more.is_none(), "{:?}", h.state().fit_more.as_ref().map(|d| d.problem_for_test()));
    let names = sheet_names(&h);
    assert!(
        names.contains(&"FitImplicit1".to_string()) && names.contains(&"FitImplicitCurve1".to_string()),
        "{names:?}"
    );
    let book = project(&h).workbooks().next().unwrap();
    let params = &book.sheets[names.iter().position(|n| n == "FitImplicit1").unwrap()];
    let value = params.columns[1].numeric().unwrap();
    assert!((value[0] - 2.0013316).abs() < 1e-5 && (value[2] - 5.0035929).abs() < 1e-5, "{value:?}");
    assert!(h.state().status.contains("Circle"), "{}", h.state().status);
    menu(&mut h, "Edit", "Undo Nonlinear Implicit Curve Fit");
    assert_eq!(project(&h), &before);
}

#[test]
fn the_ellipse_can_be_chosen_in_the_window_and_is_fitted_too() {
    let mut h = start(vec![circle_file("fitds-ellipse")]);
    menu_path(&mut h, &["Analysis", "Fitting", "Nonlinear Implicit Curve Fit..."]);
    settle(&mut h);
    h.state_mut().fit_more.as_mut().unwrap().choose_implicit_for_test("Ellipse");
    click(&mut h, "OK");
    assert!(h.state().fit_more.is_none());
    assert!(sheet_names(&h).contains(&"FitImplicit1".to_string()));
}

#[test]
fn compare_datasets_writes_the_report_with_the_chosen_names_in_one_undo_step() {
    let mut h = start(vec![decays_file("fitds-compare")]);
    let before = project(&h).clone();
    menu_path(&mut h, &["Analysis", "Fitting", "Compare Datasets..."]);
    settle(&mut h);
    h.get_by_label("1st Dataset Name");
    h.get_by_label("Fit Parameters");
    h.state_mut().fit_more.as_mut().unwrap().choose_models_for_test("ExpDec1", "Linear");
    h.state_mut().fit_more.as_mut().unwrap().set_dataset_names_for_test("First", "Second");
    click(&mut h, "OK");
    assert!(h.state().fit_more.is_none(), "{:?}", h.state().fit_more.as_ref().map(|d| d.problem_for_test()));
    let names = sheet_names(&h);
    let at = names.iter().position(|n| n == "FitCompareData1").unwrap_or_else(|| panic!("{names:?}"));
    let ws = &project(&h).workbooks().next().unwrap().sheets[at];
    let heads: Vec<&str> = ws.columns.iter().map(|c| c.long_name.as_str()).collect();
    assert_eq!(heads, ["Quantity", "First", "Second", "Text"]);
    let rows = ws.columns[0].text().unwrap();
    assert!(rows.iter().any(|r| r == "Prob>F") && rows.iter().any(|r| r == "R-square (COD)"));
    menu(&mut h, "Edit", "Undo Compare Datasets");
    assert_eq!(project(&h), &before);
}
