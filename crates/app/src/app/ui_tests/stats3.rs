//! Statistics > Descriptive Statistics > Partial Correlation Coefficient... and Statistics > ANOVA > Three-Way
//! ANOVA... (T81): the windows, their tabs and what OK writes.

use super::*;

/// The balanced 2 x 3 x 2 design of the Origin run: A has F = 43.28302 with 1 and 12 degrees of freedom.
const THREE_WAY: &str = "a,b,c,y
lo,b1,c1,15.814
lo,b2,c2,15.4569
hi,b1,c2,17.6589
hi,b3,c1,16.2762
lo,b2,c1,14.8436
lo,b3,c2,15.9664
hi,b2,c2,20.2379
lo,b1,c1,16.4812
lo,b3,c1,14.5347
hi,b1,c2,17.82
hi,b3,c2,15.9952
lo,b2,c1,13.867
hi,b1,c1,17.812
hi,b2,c2,20.4605
lo,b1,c2,17.4733
lo,b3,c1,15.7478
hi,b2,c1,16.2572
hi,b3,c2,15.4057
lo,b2,c2,14.5925
hi,b1,c1,16.4532
hi,b3,c1,17.6181
lo,b1,c2,16.3418
lo,b3,c2,16.499
hi,b2,c1,17.66
";

#[test]
fn the_anova_menu_runs_a_three_way_anova_with_text_factors() {
    let mut h = start(vec![csv_file("anova3", "design3", THREE_WAY)]);
    show_window(&mut h, "Book1 - design3.csv");
    menu_path(&mut h, &["Statistics", "ANOVA", "Three-Way ANOVA..."]);
    assert!(h.state().analysis.is_some(), "the three-way ANOVA window is open");
    for label in ["Input", "Model", "Descriptive Statistics", "Means Comparison", "Power Analysis", "Factor C"] {
        assert!(has_label(&h, label), "{label} is missing");
    }
    assert!(has_label(&h, "From row") && !has_label(&h, "X from"), "the factors have no X column to limit");
    click(&mut h, "OK");
    assert!(h.state().analysis.is_none(), "OK closes the window");
    let (sheet, df) = result_column(&h, "DF");
    assert!(sheet.starts_with("ANOVA3Way"), "{sheet}");
    // A, B, C, A*B, A*C, B*C, A*B*C, Model, Error, Total.
    assert_eq!(df, vec![1.0, 2.0, 1.0, 2.0, 1.0, 2.0, 2.0, 11.0, 12.0, 23.0]);
    let (_, ss) = result_column(&h, "Sum of Squares");
    assert!((ss[0] - 20.23401).abs() < 1e-5 && (ss[7] - 53.50542).abs() < 1e-5, "{ss:?}");
    let (_, f) = result_column(&h, "F Value");
    assert!((f[0] - 43.28302).abs() < 1e-4, "{f:?}");
    assert_eq!(h.state().doc.undo_label(), Some("Three-way ANOVA"));
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::Undo, &ctx);
    settle(&mut h);
    assert_eq!(project(&h).workbooks().next().unwrap().sheets.len(), 1);
}

#[test]
fn the_three_way_window_has_the_seven_comparison_methods_and_the_power_inputs() {
    let mut h = start(vec![csv_file("anova3-tabs", "design3", THREE_WAY)]);
    show_window(&mut h, "Book1 - design3.csv");
    menu_path(&mut h, &["Statistics", "ANOVA", "Three-Way ANOVA..."]);
    click(&mut h, "Means Comparison");
    for label in ["Tukey", "Bonferroni", "Sidak", "Fisher LSD", "Scheffe", "Bonferroni-Holm", "Sidak-Holm"] {
        assert!(has_label(&h, label), "{label} is missing");
    }
    assert!(!has_label(&h, "Dunnett"));
    click(&mut h, "Power Analysis");
    assert!(has_label(&h, "Actual Power") && has_label(&h, "Hypothetical Power"));
    click(&mut h, "Model");
    assert!(has_label(&h, "A*B*C"));
    // Switching the three-way interaction off leaves it out of the table.
    click(&mut h, "A*B*C");
    click(&mut h, "OK");
    let (_, df) = result_column(&h, "DF");
    assert_eq!(df.len(), 9, "{df:?}");
}

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

/// The one-way repeated measures run of Origin: 8 subjects by 4 levels as indexed rows `level,y,subject`.
fn rm_one_way_csv() -> String {
    let table = [
        [22.5244, 23.5244, 24.5244, 25.5244],
        [18.138, 19.677, 22.5219, 24.6265],
        [17.0564, 21.0059, 19.8137, 22.3672],
        [23.5919, 22.9043, 26.4075, 24.4996],
        [19.4001, 19.1614, 18.8032, 21.3784],
        [20.3396, 22.2064, 23.9391, 25.4933],
        [21.2722, 21.8676, 23.4055, 25.5475],
        [15.6833, 19.3813, 19.8708, 19.163],
    ];
    let mut text = String::from("level,y,subject\n");
    for (s, row) in table.iter().enumerate() {
        for (l, v) in row.iter().enumerate() {
            text.push_str(&format!("L{},{v},S{}\n", l + 1, s + 1));
        }
    }
    text
}

#[test]
fn the_anova_menu_runs_a_one_way_repeated_measures_anova() {
    let mut h = start(vec![csv_file("rm1", "rmdata", &rm_one_way_csv())]);
    show_window(&mut h, "Book1 - rmdata.csv");
    menu_path(&mut h, &["Statistics", "ANOVA", "One-Way Repeated Measures ANOVA..."]);
    assert!(h.state().analysis.is_some(), "the repeated measures window is open");
    for label in ["Input", "Means Comparison", "Factor", "Data", "Subject", "Descriptive Statistics"] {
        assert!(has_label(&h, label), "{label} is missing");
    }
    assert!(!has_label(&h, "Interactions"), "the interaction belongs to the two-way window");
    click(&mut h, "OK");
    assert!(h.state().analysis.is_none(), "OK closes the window");
    let (sheet, ms) = result_column(&h, "Mean Square");
    assert!(sheet.starts_with("ANOVAOneWayRM"), "{sheet}");
    assert!((ms[0] - 21.46344).abs() < 1e-5, "{ms:?}");
    // The multivariate block comes first: its F is 9.86391.
    let (_, f) = result_column(&h, "F Value");
    assert!((f[0] - 9.86391).abs() < 1e-4, "{f:?}");
    assert_eq!(h.state().doc.undo_label(), Some("One-way repeated measures ANOVA"));
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::Undo, &ctx);
    settle(&mut h);
    assert_eq!(project(&h).workbooks().next().unwrap().sheets.len(), 1);
}

#[test]
fn the_two_way_repeated_measures_window_has_interactions_and_no_comparison_methods() {
    let mut h = start(vec![csv_file(
        "rm2",
        "rm2data",
        "a,b,y,s
A1,B1,1.0,S1
A1,B2,2.5,S1
A2,B1,2.0,S1
A2,B2,3.0,S1
A1,B1,1.5,S2
A1,B2,2.0,S2
A2,B1,2.5,S2
A2,B2,4.5,S2
A1,B1,0.5,S3
A1,B2,2.2,S3
A2,B1,2.9,S3
A2,B2,3.1,S3
A1,B1,1.1,S4
A1,B2,1.9,S4
A2,B1,2.1,S4
A2,B2,3.8,S4
",
    )]);
    show_window(&mut h, "Book1 - rm2data.csv");
    menu_path(&mut h, &["Statistics", "ANOVA", "Two-Way Repeated Measures ANOVA..."]);
    for label in ["Input", "Descriptive Statistics", "Means Comparison", "Factor A", "Factor B", "Interactions"] {
        assert!(has_label(&h, label), "{label} is missing");
    }
    click(&mut h, "Interactions");
    click(&mut h, "Means Comparison");
    assert!(!has_label(&h, "Tukey"), "the two-way comparisons are not built");
    click(&mut h, "OK");
    let (sheet, df) = result_column(&h, "DF");
    assert!(sheet.starts_with("ANOVATwoWayRM"), "{sheet}");
    // Interactions on: A, B and A*B, each with its error term, in the first block of the sheet.
    assert!(df.len() >= 6, "{df:?}");
    assert_eq!(h.state().doc.undo_label(), Some("Two-way repeated measures ANOVA"));
}
