use super::*;
use sheaf_core::analysis::{AnovaSpec, OutputTarget, Request, RowRange, Tool};

/// Runs a one-way ANOVA of the sample's `up` and `down` columns (a Tukey comparison, one pair) and shows its
/// report sheet. Returns the workbook.
fn anova_report(h: &mut Harness<'static, SheafApp>) -> sheaf_core::WindowId {
    let book = project(h).workbooks().next().unwrap().id;
    let cols: Vec<_> =
        ["B", "C"].iter().map(|n| project(h).workbook(book).unwrap().sheet().column_by_name(n).unwrap().id).collect();
    let sheet = h
        .state_mut()
        .doc
        .apply("ANOVA", |p| {
            p.analyze_to(
                &Request { columns: cols, rows: RowRange::All, tool: Tool::Anova(AnovaSpec::default()) },
                &OutputTarget::NewSheet,
            )
        })
        .unwrap()
        .new_sheet
        .unwrap();
    h.state_mut().doc.adjust_view(|p| p.workbook_mut(book).unwrap().active_sheet = sheet);
    show_window(h, "Title Book1 - sample.csv");
    settle(h);
    book
}

#[test]
fn the_report_sheet_offers_the_plot_and_ok_makes_one_graph_in_one_undo_step() {
    let mut h = start(vec![data_file("comparison-plot")]);
    let book = anova_report(&mut h);
    let graphs = project(&h).graphs().count();
    h.get_by_label("Multiple Comparisons Plot...").click();
    settle(&mut h);
    // The dialog: untick Significant only so that the (not significant) pair is bracketed.
    h.get_by_label("Significant only").click();
    settle(&mut h);
    h.get_by_label("OK").click();
    settle(&mut h);
    assert_eq!(project(&h).graphs().count(), graphs + 1);
    let g = project(&h).graphs().last().unwrap();
    let l = &g.layers[0];
    assert_eq!(l.brackets.len(), 1);
    // The p-value is the report's "Prob" of the pair.
    let ws = project(&h).workbook(book).unwrap().sheets.iter().find(|s| s.comparison_source.is_some()).unwrap();
    let report_p = ws.columns.iter().find(|c| c.long_name == "Prob").unwrap().numeric().unwrap()[0];
    assert_eq!(l.brackets[0].p, report_p);
    assert!(l.comparison.as_ref().unwrap().pairs.len() == 1 && !l.comparison.as_ref().unwrap().significant_only);
    assert_eq!(h.state().doc.undo_label(), Some("Multiple comparisons plot"));
    menu(&mut h, "Edit", "Undo Multiple comparisons plot");
    assert_eq!(project(&h).graphs().count(), graphs);
}

#[test]
fn the_statistics_menu_entry_needs_a_report_sheet() {
    let mut h = start(vec![data_file("comparison-menu")]);
    // On the plain data sheet there is nothing to plot: a message, no dialog.
    show_window(&mut h, "Title Book1 - sample.csv");
    menu_path(&mut h, &["Statistics", "Multiple Comparisons Plot..."]);
    assert!(h.state().status.contains("report sheet"), "{}", h.state().status);
    assert!(h.query_by_label("Plot Type").is_none());
    anova_report(&mut h);
    menu_path(&mut h, &["Statistics", "Multiple Comparisons Plot..."]);
    assert!(h.query_by_label("Plot Type").is_some());
}
