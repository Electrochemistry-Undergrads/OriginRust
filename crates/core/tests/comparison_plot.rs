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
