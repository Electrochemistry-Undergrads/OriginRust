//! T126: range menus feed Statistics on Columns without changing stored source data.
use super::*;

fn run_statistics(h: &mut Harness<'static, SheafApp>) {
    click_text(h, "B(Y1)");
    menu_path(h, &["Statistics", "Descriptive Statistics", "Statistics on Columns..."]);
    assert!(h.state().analysis.is_some());
    click(h, "OK");
}

#[test]
fn begin_end_statistics_and_history_keep_the_complete_source_worksheet() {
    let mut h = start(vec![data_file("stats-data-range")]);
    show_window(&mut h, "Book1 - sample.csv");
    let book = project(&h).workbooks().next().unwrap().id;
    let b = column_id(&h, "B");
    click_text(&mut h, "4.5");
    menu_path(&mut h, &["Edit", "Set As Begin"]);
    click_text(&mut h, "6.5");
    menu_path(&mut h, &["Edit", "Set As End"]);
    let source = sheet(&h).clone();
    let before = project(&h).clone();
    run_statistics(&mut h);
    let ws = project(&h).workbook(book).unwrap();
    assert_eq!(ws.sheets[0], source);
    let results = ws.sheets.last().unwrap();
    let result = |name| results.columns.iter().find(|c| c.long_name == name).unwrap().numeric().unwrap()[0];
    assert_eq!((result("N"), result("Mean"), result("Missing")), (2., 5.5, 0.));
    assert_eq!(project(&h).numeric(b).unwrap(), [2.5, 4.5, 6.5, 8.5]);
    let after = project(&h).clone();
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::Undo, &ctx);
    settle(&mut h);
    assert_eq!(project(&h), &before);
    h.state_mut().apply(Action::Redo, &ctx);
    settle(&mut h);
    assert_eq!(project(&h), &after);
}

#[test]
fn reset_full_range_changes_the_next_descriptive_result_without_rewriting_source() {
    let mut h = start(vec![data_file("stats-data-range-reset")]);
    show_window(&mut h, "Book1 - sample.csv");
    let b = column_id(&h, "B");
    click_text(&mut h, "6.5");
    menu_path(&mut h, &["Edit", "Set As Begin"]);
    menu_path(&mut h, &["Edit", "Reset to Full Range"]);
    assert_eq!(project(&h).column(b).unwrap().data_range, None);
    run_statistics(&mut h);
    let results = project(&h).workbooks().next().unwrap().sheets.last().unwrap();
    let result = |name| results.columns.iter().find(|c| c.long_name == name).unwrap().numeric().unwrap()[0];
    assert_eq!((result("N"), result("Mean"), result("Missing")), (4., 5.5, 0.));
    assert_eq!(project(&h).numeric(b).unwrap(), [2.5, 4.5, 6.5, 8.5]);
}
