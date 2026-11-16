use super::*;

/// Plot > Line + Symbol > 2 Point Segment and 3 Point Segment are live (T71); Line Series is not built.
#[test]
fn the_segment_plots_are_in_the_line_and_symbol_menu() {
    let mut h = start(vec![data_file("segments-menu")]);
    click(&mut h, "Title Book1 - sample.csv");
    menu_path(&mut h, &["Plot", "Line + Symbol"]);
    assert!(entry_enabled(&h, "2 Point Segment") && entry_enabled(&h, "3 Point Segment"));
    assert!(!entry_enabled(&h, "Line Series"), "Line Series stays grey");
    close_menus(&mut h);
}
