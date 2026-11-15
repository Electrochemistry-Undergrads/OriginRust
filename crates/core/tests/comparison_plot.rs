//! Multiple Comparisons Plot (T58): labels, p-values read from the report, filtering, error bars.

use sheaf_core::analysis::{AnovaSpec, MeansComparison, OutputTarget, Request, RowRange, TTestKind, TTestSpec, Tool};
use sheaf_core::compplot::*;
use sheaf_core::graph::PlotKind;
use sheaf_core::{ColumnData, Designation, Project, WindowId};

const GROUPS: [&[f64]; 4] = [
    &[9.1, 10.2, 9.7, 10.9, 9.4],
    &[10.0, 11.1, 10.4, 11.6, 10.3],
    &[13.2, 14.1, 13.8, 15.0, 14.4],
    &[17.9, 18.8, 19.4, 18.1, 17.2],
];

fn project(groups: &[&[f64]]) -> (Project, WindowId, Vec<sheaf_core::ColumnId>) {
    let mut p = Project::new();
    let b = p.add_workbook("data");
    let mut ids = Vec::new();
    for (i, g) in groups.iter().enumerate() {
        let id = p.add_column(b, 0, Designation::Y, ColumnData::numeric(g.to_vec())).unwrap();
        p.column_mut(id).unwrap().long_name = ["Control", "Low", "Medium", "High"][i].to_string();
        ids.push(id);
    }
    (p, b, ids)
}

fn anova(method: MeansComparison) -> (Project, WindowId, usize) {
    let (mut p, b, ids) = project(&GROUPS);
    let spec = AnovaSpec { comparison: Some(method), ..Default::default() };
    let r = p
        .analyze_to(&Request { columns: ids, rows: RowRange::All, tool: Tool::Anova(spec) }, &OutputTarget::NewSheet)
        .unwrap();
    (p, b, r.new_sheet.unwrap())
}

fn probs(p: &Project, b: WindowId, sheet: usize) -> Vec<f64> {
    let ws = &p.workbook(b).unwrap().sheets[sheet];
    ws.columns.iter().find(|c| c.long_name == "Prob").unwrap().numeric().unwrap().to_vec()
}

#[test]
fn labels_at_the_thresholds() {
    use BracketLabels::{Asterisks as A, PValues as P};
    for (p, want) in [
        (0.0499999, "*"),
        (0.05, "ns"),
        (0.01, "*"),
        (0.0099999, "**"),
        (0.001, "**"),
        (0.0009999, "***"),
        (0.0001, "***"),
        (0.00009999, "****"),
        (0.0, "****"),
        (0.5, "ns"),
        (1.0, "ns"),
    ] {
        assert_eq!(bracket_label(p, A), want, "p = {p}");
    }
    assert_eq!(bracket_label(0.0123, P), "p = 0.0123");
    assert_eq!(bracket_label(0.5, P), "p = 0.50");
    assert_eq!(bracket_label(0.0001, P), "p = 0.0001");
    assert_eq!(bracket_label(0.00009, P), "p < 0.0001");
    assert_eq!(bracket_label(1.0, P), "p = 1.00");
}

#[test]
fn the_bracket_p_values_are_the_report_rows_for_tukey_and_bonferroni() {
    for method in [MeansComparison::Tukey, MeansComparison::Bonferroni] {
        let (mut p, b, sheet) = anova(method);
        let want = probs(&p, b, sheet);
        assert_eq!(want.len(), 6);
        let o = ComparisonPlotOptions { significant_only: false, ..Default::default() };
        let g = p.comparison_plot(b, sheet, &o).unwrap();
        let l = &p.graph(g).unwrap().layers[0];
        assert_eq!(l.brackets.len(), 6);
        // Report order: Low-Control, Medium-Control, Medium-Low, High-Control, High-Low, High-Medium.
        let pairs = [(0, 1), (0, 2), (1, 2), (0, 3), (1, 3), (2, 3)];
        for ((br, w), pair) in l.brackets.iter().zip(&want).zip(pairs) {
            assert_eq!(br.p, *w, "{method:?}");
            assert_eq!((br.group_a, br.group_b), pair);
        }
    }
}

#[test]
fn significant_only_and_the_threshold_filter_the_pairs() {
    let (mut p, b, sheet) = anova(MeansComparison::Tukey);
    let all = probs(&p, b, sheet);
    let g = p.comparison_plot(b, sheet, &ComparisonPlotOptions::default()).unwrap();
    let shown = p.graph(g).unwrap().layers[0].brackets.len();
    assert_eq!(shown, all.iter().filter(|v| **v < 0.05).count());
    assert!(shown < 6, "the Control-Low pair is not significant");
    // A stricter threshold shows fewer; the stored settings keep every pair.
    let o = ComparisonPlotOptions { threshold: 1e-6, ..Default::default() };
    let g = p.comparison_plot(b, sheet, &o).unwrap();
    let l = &p.graph(g).unwrap().layers[0];
    assert_eq!(l.brackets.len(), all.iter().filter(|v| **v < 1e-6).count());
    assert_eq!(l.comparison.as_ref().unwrap().pairs.len(), 6);
    // Re-styling from the settings: all pairs, p-value labels.
    let mut s = l.comparison.clone().unwrap();
    s.significant_only = false;
    s.labels = BracketLabels::PValues;
    let br = s.brackets();
    assert_eq!(br.len(), 6);
    assert!(br.iter().all(|b| b.label.starts_with("p ")));
}

#[test]
fn error_bars_are_sd_sem_and_the_t_based_ci() {
    let v = [2.0, 4.0, 4.0, 6.0];
    // mean 4, variance 8/3, sd 1.632993, sem 0.8164966, t(0.975, 3) = 3.182446
    assert!((error_value(&v, ComparisonErrorBar::Sd) - 1.632_993_161_855_452).abs() < 1e-12);
    assert!((error_value(&v, ComparisonErrorBar::Sem) - 0.816_496_580_927_726).abs() < 1e-12);
    assert!((error_value(&v, ComparisonErrorBar::Ci95) - 3.182_446_305_284_263 * 0.816_496_580_927_726).abs() < 1e-6);
    // The graph's error column holds them, one per group, in report order.
    let (mut p, b, sheet) = anova(MeansComparison::Tukey);
    let o = ComparisonPlotOptions { error_bar: ComparisonErrorBar::Sem, ..Default::default() };
    let g = p.comparison_plot(b, sheet, &o).unwrap();
    let plot = &p.graph(g).unwrap().layers[0].plots[0];
    assert_eq!(plot.kind, PlotKind::Column);
    let err = p.column(plot.y_error.unwrap()).unwrap().numeric().unwrap().to_vec();
    for (e, grp) in err.iter().zip(GROUPS) {
        assert!((e - error_value(grp, ComparisonErrorBar::Sem)).abs() < 1e-12);
    }
    // The report's own SEM column agrees.
    let ws = &p.workbook(b).unwrap().sheets[sheet];
    let sem = ws.columns.iter().find(|c| c.long_name == "SEM").unwrap().numeric().unwrap();
    for (e, s) in err.iter().zip(sem) {
        assert!((e - s).abs() < 1e-9);
    }
}

#[test]
fn a_two_sample_t_test_gives_one_bracket_with_the_reports_p() {
    let (mut p, b, ids) = project(&GROUPS[2..]);
    let spec = TTestSpec { kind: TTestKind::TwoSample { equal_variance: true, difference: 0.0 }, ..Default::default() };
    let r = p
        .analyze_to(&Request { columns: ids, rows: RowRange::All, tool: Tool::TTest(spec) }, &OutputTarget::NextToInput)
        .unwrap();
    let sheet = r.new_sheet.unwrap();
    let want = probs(&p, b, sheet)[0];
    assert!(p.can_comparison_plot(b, sheet));
    let g = p.comparison_plot(b, sheet, &ComparisonPlotOptions::default()).unwrap();
    let l = &p.graph(g).unwrap().layers[0];
    assert_eq!(l.brackets.len(), 1);
    assert_eq!((l.brackets[0].group_a, l.brackets[0].group_b, l.brackets[0].p), (0, 1, want));
    assert_eq!(l.brackets[0].label, "****");
    // A one-sample t-test report is not a source.
    assert!(!p.can_comparison_plot(b, 0));
}

#[test]
fn box_violin_and_scatter_plots_have_one_group_per_position() {
    for (t, plots) in [(ComparisonPlotType::Box, 4), (ComparisonPlotType::Violin, 4), (ComparisonPlotType::Scatter, 2)]
    {
        let (mut p, b, sheet) = anova(MeansComparison::Tukey);
        let o = ComparisonPlotOptions { plot_type: t, ..Default::default() };
        let g = p.comparison_plot(b, sheet, &o).unwrap();
        assert_eq!(p.graph(g).unwrap().layers[0].plots.len(), plots, "{t:?}");
    }
    // The jitter is deterministic and stays within 0.18 of the group position.
    for g in 0..4 {
        for k in 0..50 {
            let x = jittered(g, k);
            assert!((x - (g + 1) as f64).abs() <= 0.18 + 1e-12);
            assert_eq!(x, jittered(g, k));
        }
    }
}

#[test]
fn settings_round_trip_and_old_layers_load_without_them() {
    let (mut p, b, sheet) = anova(MeansComparison::Tukey);
    let g = p.comparison_plot(b, sheet, &ComparisonPlotOptions::default()).unwrap();
    let layer = p.graph(g).unwrap().layers[0].clone();
    let json = serde_json::to_string(&layer).unwrap();
    let back: sheaf_core::Layer = serde_json::from_str(&json).unwrap();
    // (JSON text keeps a float to within one unit in the last place.)
    assert_eq!(back.brackets.len(), layer.brackets.len());
    for (a, b) in back.brackets.iter().zip(&layer.brackets) {
        assert_eq!((a.group_a, a.group_b, &a.label), (b.group_a, b.group_b, &b.label));
        assert!((a.p - b.p).abs() <= 1e-15 * b.p);
    }
    let (ca, cb) = (back.comparison.unwrap(), layer.comparison.clone().unwrap());
    assert_eq!((ca.plot_type, ca.error_bar, ca.labels, ca.pairs.len()), (cb.plot_type, cb.error_bar, cb.labels, 6));
    let mut plain = layer.clone();
    plain.brackets.clear();
    plain.comparison = None;
    let json = serde_json::to_string(&plain).unwrap();
    assert!(!json.contains("brackets") && !json.contains("comparison"));
    let opts: ComparisonPlotOptions = serde_json::from_str("{}").unwrap();
    assert_eq!(opts, ComparisonPlotOptions::default());
}

// ----- Holm adjusted p-values and decisions that agree with the report -----

fn named(names: &[&str], groups: &[&[f64]], spec: AnovaSpec) -> (Project, WindowId, usize) {
    let mut p = Project::new();
    let b = p.add_workbook("data");
    let mut ids = Vec::new();
    for (name, g) in names.iter().zip(groups) {
        let id = p.add_column(b, 0, Designation::Y, ColumnData::numeric(g.to_vec())).unwrap();
        p.column_mut(id).unwrap().long_name = name.to_string();
        ids.push(id);
    }
    let r = p
        .analyze_to(&Request { columns: ids, rows: RowRange::All, tool: Tool::Anova(spec) }, &OutputTarget::NewSheet)
        .unwrap();
    (p, b, r.new_sheet.unwrap())
}

fn set_column(p: &mut Project, b: WindowId, sheet: usize, name: &str, values: Vec<f64>) {
    let ws = &mut p.workbook_mut(b).unwrap().sheets[sheet];
    ws.columns.iter_mut().find(|c| c.long_name == name).unwrap().data = ColumnData::numeric(values);
}

fn column(p: &Project, b: WindowId, sheet: usize, name: &str) -> Vec<f64> {
    let ws = &p.workbook(b).unwrap().sheets[sheet];
    ws.columns.iter().find(|c| c.long_name == name).unwrap().numeric().unwrap().to_vec()
}

#[test]
fn holm_adjusted_p_values_match_the_hand_computation() {
    // Sorted: 0.01 (x4), 0.03 (x3), 0.04 (x2, but not below the one before), 0.2 (x1).
    let raw = [0.01, 0.04, 0.03, 0.2];
    let bonf = holm_adjust(&raw, StepDown::Bonferroni);
    for (got, want) in bonf.iter().zip([0.04, 0.09, 0.09, 0.2]) {
        assert!((got - want).abs() < 1e-12, "{bonf:?}");
    }
    let sidak = holm_adjust(&raw, StepDown::Sidak);
    let want = [1.0 - 0.99f64.powi(4), 1.0 - 0.97f64.powi(3), 1.0 - 0.97f64.powi(3), 0.2];
    for (got, want) in sidak.iter().zip(want) {
        assert!((got - want).abs() < 1e-12, "{sidak:?}");
    }
    // Capped at 1, and monotone in the sorted order.
    assert_eq!(holm_adjust(&[0.6, 0.7], StepDown::Bonferroni), vec![1.0, 1.0]);
}

#[test]
fn a_holm_pair_the_report_calls_not_significant_gets_no_star_and_no_bracket() {
    for (method, step) in
        [(MeansComparison::HolmBonferroni, StepDown::Bonferroni), (MeansComparison::HolmSidak, StepDown::Sidak)]
    {
        let spec = AnovaSpec { comparison: Some(method), ..Default::default() };
        let (mut p, b, sheet) =
            named(&["A", "B", "C"], &[&[1.0, 2.0, 1.5, 1.2], &[2.1, 2.4, 2.9, 2.2], &[3.0, 3.5, 3.1, 3.2]], spec);
        // The three raw p-values of the example, sorted as the report lists them; none is significant by the
        // step-down (0.04 > 0.05 / 3).
        set_column(&mut p, b, sheet, "Prob", vec![0.04, 0.2, 0.3]);
        set_column(&mut p, b, sheet, "Significant", vec![0.0, 0.0, 0.0]);
        let report = p.workbook(b).unwrap().sheets[sheet].clone();
        let pairs = read_pairs(&report).unwrap();
        let adj: Vec<f64> = pairs.iter().map(|q| q.p).collect();
        let want = holm_adjust(&[0.04, 0.2, 0.3], step);
        assert_eq!(adj, want);
        assert!(adj[0] > 0.05, "the adjusted p of the 0.04 pair is {}", adj[0]);
        // Significant only: nothing to show.
        let g = p.comparison_plot(b, sheet, &ComparisonPlotOptions::default()).unwrap();
        assert!(p.graph(g).unwrap().layers[0].brackets.is_empty(), "{method:?}");
        // Every pair: all "ns", also with the 0.04 pair whose raw p is below 0.05.
        let all = ComparisonPlotOptions { significant_only: false, ..Default::default() };
        let g = p.comparison_plot(b, sheet, &all).unwrap();
        let l = &p.graph(g).unwrap().layers[0];
        assert_eq!(l.brackets.len(), 3);
        assert!(l.brackets.iter().all(|br| br.label == "ns"), "{method:?}");
        // P-value labels show the adjusted p.
        let pv =
            ComparisonPlotOptions { significant_only: false, labels: BracketLabels::PValues, ..Default::default() };
        let g = p.comparison_plot(b, sheet, &pv).unwrap();
        let l = &p.graph(g).unwrap().layers[0];
        assert_eq!(l.brackets[0].label, bracket_label(want[0], BracketLabels::PValues));
    }
}

#[test]
fn the_brackets_agree_with_the_significant_column_for_every_method() {
    // Moderately separated groups, so that the methods disagree on some pairs.
    let fixtures: [Vec<&[f64]>; 2] = [
        GROUPS.to_vec(),
        vec![
            &[5.0, 6.1, 5.5, 6.4, 5.2, 5.9],
            &[5.8, 6.9, 6.2, 7.1, 6.0, 6.6],
            &[6.3, 7.4, 6.6, 7.8, 6.5, 7.0],
            &[6.9, 8.0, 7.2, 8.4, 7.1, 7.7],
            &[5.1, 6.0, 5.4, 6.6, 5.3, 6.1],
        ],
    ];
    let methods = [
        MeansComparison::Tukey,
        MeansComparison::Bonferroni,
        MeansComparison::HolmBonferroni,
        MeansComparison::HolmSidak,
        MeansComparison::Dunnett,
        MeansComparison::Sidak,
        MeansComparison::FisherLsd,
        MeansComparison::Scheffe,
    ];
    let mut holm_differs_from_raw = false;
    for groups in &fixtures {
        let names: Vec<String> = (0..groups.len()).map(|i| format!("G{i}")).collect();
        let names: Vec<&str> = names.iter().map(String::as_str).collect();
        for alpha in [0.05, 0.1] {
            for method in methods {
                let spec = AnovaSpec { comparison: Some(method), alpha, ..Default::default() };
                let (mut p, b, sheet) = named(&names, groups, spec);
                let sig = column(&p, b, sheet, "Significant");
                let raw = column(&p, b, sheet, "Prob");
                holm_differs_from_raw |= method.is_holm() && raw.iter().zip(&sig).any(|(p, s)| *p < alpha && *s == 0.0);
                let opts = |only: bool, labels| ComparisonPlotOptions {
                    significant_only: only,
                    threshold: alpha,
                    labels,
                    ..Default::default()
                };
                let g = p.comparison_plot(b, sheet, &opts(false, BracketLabels::Asterisks)).unwrap();
                let all = p.graph(g).unwrap().layers[0].brackets.clone();
                assert_eq!(all.len(), sig.len());
                for (br, s) in all.iter().zip(&sig) {
                    assert_eq!(br.label != "ns", *s != 0.0, "{method:?} alpha {alpha}: {br:?}");
                }
                let g = p.comparison_plot(b, sheet, &opts(true, BracketLabels::PValues)).unwrap();
                let shown = &p.graph(g).unwrap().layers[0].brackets;
                let want: Vec<(usize, usize)> =
                    all.iter().zip(&sig).filter(|(_, s)| **s != 0.0).map(|(b, _)| (b.group_a, b.group_b)).collect();
                let got: Vec<(usize, usize)> = shown.iter().map(|b| (b.group_a, b.group_b)).collect();
                assert_eq!(got, want, "{method:?} alpha {alpha}");
            }
        }
    }
    assert!(holm_differs_from_raw, "a fixture has a Holm pair whose raw p is below alpha but is not significant");
}

// ----- pairs by index -----

#[test]
fn groups_with_the_same_name_or_a_dash_in_the_name_map_to_the_right_pairs() {
    let data: [&[f64]; 3] = [&[1.0, 2.0, 1.5, 1.2], &[2.1, 2.4, 2.9, 2.2], &[5.0, 5.5, 6.1, 5.2]];
    let want = [(0, 1), (0, 2), (1, 2)];
    for names in [["Value", "Value", "Other"], ["x - y", "x", "y - x"], ["Value", "Value", "Value"]] {
        let (mut p, b, sheet) = named(&names, &data, AnovaSpec::default());
        let report = p.workbook(b).unwrap().sheets[sheet].clone();
        let pairs = read_pairs(&report).unwrap();
        assert_eq!(pairs.iter().map(|q| (q.a, q.b)).collect::<Vec<_>>(), want, "{names:?}");
        assert_eq!(pairs.iter().map(|q| q.p).collect::<Vec<_>>(), column(&p, b, sheet, "Prob"));
        let o = ComparisonPlotOptions { significant_only: false, ..Default::default() };
        let g = p.comparison_plot(b, sheet, &o).unwrap();
        let l = &p.graph(g).unwrap().layers[0];
        assert_eq!(l.brackets.iter().map(|q| (q.group_a, q.group_b)).collect::<Vec<_>>(), want);
    }
}

#[test]
fn a_report_without_stored_pairs_is_read_by_name_and_refused_when_the_names_are_ambiguous() {
    let data: [&[f64]; 3] = [&[1.0, 2.0, 1.5, 1.2], &[2.1, 2.4, 2.9, 2.2], &[5.0, 5.5, 6.1, 5.2]];
    let strip = |p: &mut Project, b: WindowId, sheet: usize| {
        p.workbook_mut(b).unwrap().sheets[sheet].comparison_source.as_mut().unwrap().pairs.clear();
        p.workbook(b).unwrap().sheets[sheet].clone()
    };
    // Distinct names: the fallback finds the pairs.
    let (mut p, b, sheet) = named(&["A", "B", "C"], &data, AnovaSpec::default());
    let stripped = strip(&mut p, b, sheet);
    let pairs = read_pairs(&stripped).unwrap();
    assert_eq!(pairs.iter().map(|q| (q.a, q.b)).collect::<Vec<_>>(), [(0, 1), (0, 2), (1, 2)]);
    // Two groups called Value.
    let (mut p, b, sheet) = named(&["Value", "Value", "C"], &data, AnovaSpec::default());
    let stripped = strip(&mut p, b, sheet);
    let err = read_pairs(&stripped).unwrap_err();
    assert!(err.contains("more than one pair"), "{err}");
    // "a - b" minus "c" and "a" minus "b - c" are both written "a - b - c".
    let four: [&[f64]; 4] =
        [&[1.0, 2.0, 1.5, 1.2], &[2.1, 2.4, 2.9, 2.2], &[5.0, 5.5, 6.1, 5.2], &[8.0, 8.4, 9.1, 8.8]];
    let (mut p, b, sheet) = named(&["c", "a - b", "b - c", "a"], &four, AnovaSpec::default());
    let stripped = strip(&mut p, b, sheet);
    assert!(read_pairs(&stripped).is_err());
}
