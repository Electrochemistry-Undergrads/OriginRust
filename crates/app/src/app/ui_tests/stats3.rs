//! Statistics > Descriptive Statistics > Partial Correlation Coefficient... and Statistics > ANOVA > Three-Way
//! ANOVA... (T81): the windows, their tabs and what OK writes.

use super::*;

/// The partial correlation window needs a controlling variable; with two controls it reproduces the Origin run.
#[test]
fn the_descriptive_menu_runs_a_partial_correlation_with_two_controls() {
    let mut h = start(vec![csv_file(
        "partcorr",
        "pc",
        "a,b,c,d
0.632,4.903,-1.224,0.095
2.929,7.068,0.378,0.145
2.723,8.643,0.995,-0.345
1.709,5.173,-0.513,-0.062
0.907,4.698,-1.329,0.273
1.646,5.831,-0.066,0.692
3.531,6.826,0.478,1.097
2.656,7.268,1.098,0.21
1.02,5.578,-1.216,-1.79
0.775,3.524,-1.293,0.702
2.334,6.63,0.706,-1.42
2.373,6.334,0.474,-0.171
3.316,9.095,1.534,-1.474
2.562,8.275,0.982,0.002
2.13,7.155,-0.102,-0.779
1.767,6.4,-0.274,0.214
7.435,9.746,2.571,3.3
2.119,6.412,-0.289,-0.049
1.819,5.007,-0.785,0.842
-0.479,5.129,-1.138,-1.323
",
    )]);
    show_window(&mut h, "Book1 - pc.csv");
    open_analysis_on(&mut h, AnalysisKind::PartialCorr, &["A", "B"]);
    assert!(h.state().analysis.is_some(), "the partial correlation window is open");
    for label in [
        "Controlling Variables",
        "Pairwise",
        "Listwise",
        "Two-tailed",
        "One-tailed",
        "Show Significance in Result Table",
        "Flag Significant Correlations",
    ] {
        assert!(has_label(&h, label), "{label} is missing");
    }
    // No controlling variable yet: OK keeps the window open and says why.
    click(&mut h, "OK");
    assert!(h.state().analysis.is_some(), "the window stays open without a control");
    h.get(By::new().role(egui::accesskit::Role::ComboBox).value("(none)")).click();
    settle(&mut h);
    h.query_all_by_label("C  c").last().expect("C in the list").click();
    settle(&mut h);
    // The list closes after each choice: open it again for the second control.
    h.get(By::new().role(egui::accesskit::Role::ComboBox).value("C")).click();
    settle(&mut h);
    h.query_all_by_label("D  d").last().expect("D in the list").click();
    settle(&mut h);
    click(&mut h, "OK");
    assert!(h.state().analysis.is_none(), "OK closes the window");
    let (sheet, b) = result_column(&h, "b");
    assert!(sheet.starts_with("PartialCorr"), "{sheet}");
    // Origin: r = 0.3195 for a and b controlling for c and d.
    assert!((b[0] - 0.3195).abs() < 6e-5 && b[1] == 1.0, "{b:?}");
    assert_eq!(h.state().doc.undo_label(), Some("Partial correlation"));
}
