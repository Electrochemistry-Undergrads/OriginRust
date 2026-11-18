//! Tools > Import Filters Manager (T106, import-ascii.md "Dialog themes").

use super::*;

#[test]
fn the_tools_menu_opens_the_filter_manager() {
    let mut h = start(vec![data_file("filter-manager")]);
    show_window(&mut h, "Book1 - sample.csv");
    menu_path(&mut h, &["Tools", "Import Filters Manager..."]);
    assert!(h.state().filter_manager.is_some(), "the window opened");
    assert!(h.query_by_label("*.xls;*.xlsx;*.xlsm").is_some(), "the Excel filter is listed");
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    assert!(h.state().filter_manager.is_none());
}
