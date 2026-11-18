//! T127: row statistics uses each selected column's saved range at original worksheet rows.
use super::*;

#[test]
fn row_statistics_menu_uses_per_column_ranges_and_undo_redo_preserves_the_whole_project() {
    let mut h = start(vec![data_file("row-statistics-range")]);
    show_window(&mut h, "Book1 - sample.csv");
    click_text(&mut h, "4.5");
    menu_path(&mut h, &["Edit", "Set As Begin"]);
    click_text(&mut h, "10.5");
    menu_path(&mut h, &["Edit", "Set As End"]);
    select_b_and_c(&mut h);
    let source = sheet(&h).clone();
    let before = project(&h).clone();
    menu_path(&mut h, &["Statistics", "Descriptive Statistics", "Statistics on Rows..."]);
    click(&mut h, "OK");
    assert!(h.state().analysis.is_none());
    assert_eq!(h.state().doc.undo_label(), Some("Statistics on rows"));
    let result = sheet(&h);
    let mean = result.columns.iter().find(|c| c.long_name == "Mean").unwrap().numeric().unwrap();
    let sd = result.columns.iter().find(|c| c.long_name == "Standard Deviation").unwrap().numeric().unwrap();
    assert_eq!(mean, [30.5, 12.5, 8.5, 8.5]);
    assert!(sd[0].is_nan() && sd[3].is_nan());
    assert!((sd[1] - 16. / 2.0_f64.sqrt()).abs() < 1e-12);
    let mut expected = result.clone();
    expected.columns.truncate(source.columns.len());
    assert_eq!(expected, source);
    let after = serde_json::to_value(project(&h)).unwrap();
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::Undo, &ctx);
    settle(&mut h);
    assert_eq!(project(&h), &before);
    h.state_mut().apply(Action::Redo, &ctx);
    settle(&mut h);
    assert_eq!(serde_json::to_value(project(&h)).unwrap(), after);
}

#[test]
fn reset_full_range_restores_all_row_contributors() {
    let mut h = start(vec![data_file("row-statistics-range-reset")]);
    show_window(&mut h, "Book1 - sample.csv");
    click_text(&mut h, "6.5");
    menu_path(&mut h, &["Edit", "Set As Begin"]);
    menu_path(&mut h, &["Edit", "Reset to Full Range"]);
    select_b_and_c(&mut h);
    menu_path(&mut h, &["Statistics", "Descriptive Statistics", "Statistics on Rows..."]);
    click(&mut h, "OK");
    let mean = sheet(&h).columns.iter().find(|c| c.long_name == "Mean").unwrap().numeric().unwrap();
    assert_eq!(mean, [16.5, 12.5, 8.5, 4.5]);
}
