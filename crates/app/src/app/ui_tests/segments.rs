use super::*;

/// Plot > Line + Symbol > 2 Point Segment and 3 Point Segment are live (T71); Line Series is live too (T82).
#[test]
fn the_segment_plots_are_in_the_line_and_symbol_menu() {
    let mut h = start(vec![data_file("segments-menu")]);
    click(&mut h, "Title Book1 - sample.csv");
    menu_path(&mut h, &["Plot", "Line + Symbol"]);
    assert!(entry_enabled(&h, "2 Point Segment") && entry_enabled(&h, "3 Point Segment"));
    assert!(entry_enabled(&h, "Line Series"));
    close_menus(&mut h);
}

#[test]
fn line_series_makes_one_plot_and_a_helper_sheet_in_one_undo_step() {
    use sheaf_core::graph::Connect;
    let mut h = start(vec![data_file("line-series")]);
    select_b_and_c(&mut h);
    let graphs = project(&h).graphs().count();
    let sheets = |h: &Harness<'static, SheafApp>| project(h).workbooks().next().unwrap().sheets.len();
    let before = sheets(&h);
    menu_path(&mut h, &["Plot", "Line + Symbol", "Line Series"]);
    let g: Vec<_> = project(&h).graphs().collect();
    assert_eq!(g.len(), graphs + 1);
    let plots = &g[graphs].layers[0].plots;
    assert_eq!(plots.len(), 1);
    assert_eq!(plots[0].line.connect, Connect::Segment2);
    assert_eq!(sheets(&h), before + 1);
    assert!(project(&h).workbooks().next().unwrap().sheets.iter().any(|s| s.name.starts_with("LineSeries")));
    ctrl(&mut h, egui::Key::Z);
    assert_eq!(project(&h).graphs().count(), graphs);
    assert_eq!(sheets(&h), before);
}

/// Plot > Symbol > Scatter Central (T75) and Column Scatter (T79) are live.
#[test]
fn scatter_central_is_in_the_symbol_menu() {
    let mut h = start(vec![data_file("scatter-central-menu")]);
    click(&mut h, "Title Book1 - sample.csv");
    menu_path(&mut h, &["Plot", "Symbol"]);
    assert!(entry_enabled(&h, "Scatter Central"));
    assert!(entry_enabled(&h, "Column Scatter"));
    close_menus(&mut h);
}
