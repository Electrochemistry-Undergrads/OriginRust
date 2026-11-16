//! Origin 2016 runs of the T81 tools: partial correlation (`pcorrcoef`) and three-way ANOVA (`ANOVAThreeWay`).
//! The data and printed numbers of the runs are in `fixtures/origin_t81_data.rs` (generated from
//! `tools/origin-ref/stats6/t81-origin.compact.json`); the means comparison, descriptive, Levene and power
//! values of the balanced three-way run are copied from the report sheet cell by cell (scratch dump
//! `a3-bal.raw.txt`, local only). Origin prints 5 decimals or 6 significant digits, so the tolerances are
//! half a unit of the last printed digit plus a little.

#[path = "fixtures/origin_t81_data.rs"]
mod data;

use sheaf_calc::{MeansComparison, ThreeWayEffect as Effect, ThreeWayTerms, anova_three_way, partial_correlation};

fn close(got: f64, want: f64, rel: f64, what: &str) {
    if want.is_nan() {
        assert!(got.is_nan(), "{what}: got {got}, want NaN");
        return;
    }
    assert!((got - want).abs() <= rel * want.abs() + 6e-6, "{what}: got {got}, want {want}");
}

#[test]
fn partial_correlation_reproduces_the_origin_runs() {
    for run in data::pc_runs() {
        let vars: Vec<&[f64]> = run.vars.iter().map(|&i| run.columns[i].as_slice()).collect();
        let controls: Vec<&[f64]> = run.controls.iter().map(|&i| run.columns[i].as_slice()).collect();
        let r = partial_correlation(&vars, &controls, run.listwise, run.one_tailed).unwrap();
        let m = vars.len();
        for i in 0..m {
            for j in 0..m {
                let what = format!("run {} ({i},{j})", run.name);
                close(r.r(i, j), run.r[i * m + j], 0.0, &format!("{what} r"));
                close(r.p(i, j), run.p[i * m + j], 2e-4, &format!("{what} p"));
                assert_eq!(r.df(i, j), run.df[i * m + j], "{what} df");
            }
        }
    }
}

#[test]
fn partial_correlation_refuses_what_origin_refuses() {
    let runs = data::pc_runs();
    let c = &runs[0].columns;
    let (x, y, z) = (c[0].as_slice(), c[1].as_slice(), c[2].as_slice());
    assert!(partial_correlation(&[x, y], &[], false, false).is_err());
    assert!(partial_correlation(&[x], &[z], false, false).is_err());
    assert!(partial_correlation(&[x, y], &[&[1.0; 20]], false, false).is_err());
    assert!(partial_correlation(&[x, y], &[z, z], false, false).is_err());
    assert!(partial_correlation(&[x, y], &[x], false, false).is_err());
    let y2: Vec<f64> = x.iter().map(|v| 2.0 * v + 1.0).collect();
    assert!(partial_correlation(&[x, &y2], &[z], false, false).is_err());
}

fn check_table(run: &data::A3Run) -> sheaf_calc::ThreeWayAnova {
    let r = anova_three_way(&run.a, &run.b, &run.c, &run.y, ThreeWayTerms::default()).unwrap();
    for &(term, df, ss, ms, f, p) in &run.table {
        let got = match term {
            "A" => r.row_of(Effect::A),
            "B" => r.row_of(Effect::B),
            "C" => r.row_of(Effect::C),
            "A*B" => r.row_of(Effect::Ab),
            "A*C" => r.row_of(Effect::Ac),
            "B*C" => r.row_of(Effect::Bc),
            "A*B*C" => r.row_of(Effect::Abc),
            "Model" => Some(&r.model),
            "Error" => {
                assert_eq!(r.error.df, df);
                close(r.error.sum_sq, ss, 1e-5, "Error SS");
                close(r.error.mean_sq, ms, 1e-4, "Error MS");
                continue;
            }
            "Total" => {
                assert_eq!(r.total.df, df);
                close(r.total.sum_sq, ss, 1e-5, "Total SS");
                continue;
            }
            other => panic!("unexpected row {other}"),
        };
        let got = got.unwrap_or_else(|| panic!("no row {term}"));
        assert_eq!(got.df, df, "{term} df");
        close(got.sum_sq, ss, 1e-5, &format!("{term} SS"));
        close(got.mean_sq, ms, 1e-4, &format!("{term} MS"));
        close(got.f, f, 1e-4, &format!("{term} F"));
        close(got.p, p, 2e-4, &format!("{term} p"));
    }
    r
}

#[test]
fn three_way_table_is_origin_s_type_three_for_balanced_and_unbalanced_data() {
    let bal = check_table(&data::a3_bal());
    assert_eq!(bal.total.df, 23.0);
    let unb = check_table(&data::a3_unb());
    assert_eq!(unb.total.df, 25.0);
}

fn balanced() -> sheaf_calc::ThreeWayAnova {
    let run = data::a3_bal();
    anova_three_way(&run.a, &run.b, &run.c, &run.y, ThreeWayTerms::default()).unwrap()
}

#[test]
fn three_way_descriptive_statistics_of_the_balanced_run() {
    let r = balanced();
    let a = r.stats_of(Effect::A);
    assert_eq!((a[0].n, a[1].n), (12, 12));
    close(a[0].mean, 15.63485, 1e-6, "A1 mean");
    close(a[0].sd, 1.02591, 1e-5, "A1 SD");
    close(a[0].sem, 0.29615, 1e-4, "A1 SEM");
    close(a[1].sd.powi(2), 2.48217, 1e-5, "A2 variance");
    let ab = r.stats_of(Effect::Ab);
    assert_eq!(ab.len(), 6);
    close(ab[1].mean, 14.69, 1e-6, "A1B2 mean");
    close(ab[4].sd, 2.04164, 1e-5, "A2B2 SD");
    let abc = r.stats_of(Effect::Abc);
    assert_eq!((abc.len(), abc[0].n), (12, 2));
    close(abc[1].mean, 16.90755, 1e-6, "A1B1C2 mean");
    assert_eq!(r.overall.n, 24);
    close(r.overall.mean, 16.55305, 1e-6, "overall mean");
    close(r.overall.sd, 1.60319, 1e-5, "overall SD");
}

#[test]
fn three_way_means_comparisons_match_the_balanced_run() {
    let r = balanced();
    // Origin prints mean(i) - mean(j) and the signed statistic; Sheaf's Comparison has mean(j) - mean(i).
    let tukey_a = &r.compare(Effect::A, MeansComparison::Tukey, 0.05)[0];
    close(-tukey_a.diff, -1.83639, 1e-5, "Tukey A diff");
    close(tukey_a.se, 0.19737, 1e-4, "Tukey A SE");
    close(tukey_a.stat, 13.15797, 1e-5, "Tukey A q");
    close(tukey_a.p, 9.23288e-7, 1e-4, "Tukey A p");
    close(-tukey_a.ucl, -2.26643, 1e-5, "Tukey A LCL");
    close(-tukey_a.lcl, -1.40635, 1e-5, "Tukey A UCL");
    let bonf_a = &r.compare(Effect::A, MeansComparison::Bonferroni, 0.05)[0];
    close(bonf_a.p, 7.76269e-7, 1e-4, "Bonferroni A p");
    let tukey_b = r.compare(Effect::B, MeansComparison::Tukey, 0.05);
    let want = [(1.81271, 0.43131, -0.33506), (5.7123, 0.00432, 0.3315), (3.89959, 0.04269, 0.02165)];
    for (c, (stat, p, lcl)) in tukey_b.iter().zip(want) {
        close(c.stat, stat, 1e-4, "Tukey B q");
        close(c.p, p, 1e-3, "Tukey B p");
        close(-c.ucl, lcl, 1e-4, "Tukey B LCL");
    }
    let lsd_b = r.compare(Effect::B, MeansComparison::FisherLsd, 0.05);
    for (c, (p, sig)) in lsd_b.iter().zip([(0.22413, false), (0.00164, true), (0.01736, true)]) {
        close(c.p, p, 1e-3, "LSD B p");
        assert_eq!(c.significant, sig);
    }
    let bonf_b = r.compare(Effect::B, MeansComparison::Bonferroni, 0.05);
    for (c, p) in bonf_b.iter().zip([0.67239, 0.00493, 0.05209]) {
        close(c.p, p, 1e-3, "Bonferroni B p");
    }
    let sch_b = r.compare(Effect::B, MeansComparison::Scheffe, 0.05);
    for (c, p) in sch_b.iter().zip([0.46306, 0.00579, 0.05261]) {
        close(c.p, p, 1e-3, "Scheffe B p");
    }
    // A*B: the first eight of the fifteen pairs, in Origin's order (cell 1 against 2, 3, 4, 5, 6, then 2 against 3, 4, 5).
    let ab = r.compare(Effect::Ab, MeansComparison::Tukey, 0.05);
    assert_eq!(ab.len(), 15);
    let want_q = [7.60165, 3.47738, -3.75806, -8.79615, 0.84297, -4.12427, -11.35971, -16.3978];
    let want_p = [0.00178, 0.21098, 0.15622, 4.91705e-4, 0.98933, 0.10371, 4.14648e-5, 7.62343e-7];
    let want_diff = [1.83757, 0.8406, -0.90845, -2.12633, 0.20377, -0.99697, -2.74603, -3.9639];
    for k in 0..8 {
        // The printed statistic is signed like the difference mean(i) - mean(j).
        close(-ab[k].diff, want_diff[k], 1e-4, "A*B diff");
        close(ab[k].stat * (-ab[k].diff).signum(), want_q[k], 1e-4, "A*B q");
        close(ab[k].p, want_p[k], 1e-3, "A*B p");
        close(ab[k].se, 0.34186, 1e-4, "A*B SE");
    }
}

#[test]
fn three_way_levene_and_power_of_the_balanced_run() {
    let r = balanced();
    // Two numbers per cell make the deviations within a cell equal: the error sum of squares is rounding noise.
    let l = r.levene().unwrap();
    assert_eq!((l.model.df, l.error.df), (11.0, 12.0));
    close(l.model.sum_sq, 1.04913, 1e-4, "Levene SS");
    close(l.model.mean_sq, 0.09538, 1e-3, "Levene MS");
    assert!(l.error.sum_sq < 1e-9);
    let power = r.power(0.05, &[50.0, 100.0, 200.0]);
    let get = |e: Effect| &power.iter().find(|p| p.effect == e).unwrap().powers;
    let expect: [(Effect, f64, [f64; 4]); 7] = [
        (Effect::A, 24.0, [0.99997, 1.0, 1.0, 1.0]),
        (Effect::B, 24.0, [0.62766, 0.96071, 0.9998, 1.0]),
        (Effect::C, 24.0, [0.82334, 0.99316, 0.99999, 1.0]),
        (Effect::Ab, 48.0, [1.0, 1.0, 1.0, 1.0]),
        (Effect::Ac, 48.0, [0.0541, 0.05429, 0.05885, 0.068]),
        (Effect::Bc, 48.0, [0.97374, 0.97902, 0.99996, 1.0]),
        (Effect::Abc, 72.0, [0.99996, 0.99771, 1.0, 1.0]),
    ];
    for (e, n, powers) in expect {
        let got = get(e);
        assert_eq!(got[0].0, n, "{e:?} actual N");
        assert_eq!(got.iter().map(|p| p.0).skip(1).collect::<Vec<_>>(), [50.0, 100.0, 200.0]);
        for (g, w) in got.iter().zip(powers) {
            assert!((g.1 - w).abs() < 6e-5, "{e:?} N={}: got {}, want {w}", g.0, g.1);
        }
    }
}
