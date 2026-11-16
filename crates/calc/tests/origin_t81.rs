//! Origin 2016 runs of the T81 tools: partial correlation (`pcorrcoef`), three-way ANOVA (`ANOVAThreeWay`) and the one- and two-way repeated measures ANOVA.
//! The data and printed numbers of the runs are in `fixtures/origin_t81_data.rs` (generated from
//! `tools/origin-ref/stats6/t81-origin.compact.json`); the means comparison, descriptive, Levene and power
//! values of the balanced three-way run are copied from the report sheet cell by cell (scratch dump
//! `a3-bal.raw.txt`, local only). Origin prints 5 decimals or 6 significant digits, so the tolerances are
//! half a unit of the last printed digit plus a little.

#[path = "fixtures/origin_t81_data.rs"]
mod data;

use sheaf_calc::{
    MeansComparison, RmTerm, ThreeWayEffect as Effect, ThreeWayTerms, anova_three_way, partial_correlation, rm_anova,
};

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

/// The one-way repeated measures run (8 subjects, 4 levels): table rows, Mauchly, epsilons, multivariate tests,
/// between-subjects rows, level statistics and the means comparison blocks, as printed by Origin.
#[test]
fn one_way_repeated_measures_reproduces_the_origin_run() {
    let r = rm_anova(&data::rm1_data(), &[4], false).unwrap();
    let e = r.effect(RmTerm::A).unwrap();
    close(e.ss, 64.39032, 0.0, "SS");
    close(e.ss_error, 28.76007, 0.0, "SS error");
    // Sphericity assumed, Greenhouse-Geisser, Huynh-Feldt, lower bound: DF, MS, p; error DF and MS.
    let want = [
        (3.0, 21.46344, 1.40272e-5, 21.0, 1.36953),
        (2.89097, 22.27288, 1.93022e-5, 20.23682, 1.42118),
        (3.0, 21.46344, 1.40272e-5, 21.0, 1.36953),
        (1.0, 64.39032, 0.00547, 7.0, 4.10858),
    ];
    for (row, w) in e.rows.iter().zip(want) {
        close(row.df, w.0, 0.0, "df");
        close(row.mean_sq, w.1, 0.0, "ms");
        close(row.f, 15.67215, 0.0, "F");
        close(row.p, w.2, 2e-4, "p");
        close(row.df_error, w.3, 0.0, "error df");
        close(row.mean_sq_error, w.4, 0.0, "error ms");
    }
    let s = e.sphericity.unwrap();
    close(s.w, 0.94848, 0.0, "W");
    close(s.chi_sq, 0.30267, 0.0, "chi-square");
    assert_eq!(s.df, 5.0);
    close(s.p, 0.99759, 0.0, "Mauchly p");
    close(s.greenhouse_geisser, 0.96366, 0.0, "GG");
    assert_eq!(s.huynh_feldt, 1.0);
    close(s.lower_bound, 0.33333, 0.0, "LB");
    let m = e.multivariate.unwrap();
    close(m.pillai, 0.85546, 0.0, "Pillai");
    close(m.wilks, 0.14454, 0.0, "Wilks");
    close(m.hotelling, 5.91835, 0.0, "Hotelling");
    close(m.roy, 5.91835, 0.0, "Roy");
    close(m.f, 9.86391, 0.0, "multivariate F");
    assert_eq!((m.df, m.df_error), (3.0, 5.0));
    close(m.p, 0.01532, 0.0, "multivariate p");
    close(r.intercept.ss, 15121.48321, 0.0, "intercept SS");
    close(r.intercept.f, 804.75036, 0.0, "intercept F");
    close(r.intercept.p, 1.73883e-8, 2e-4, "intercept p");
    close(r.subjects.ss, 131.53195, 0.0, "subjects SS");
    assert_eq!(r.subjects.df, 7.0);
    close(r.subjects.mean_sq, 18.79028, 0.0, "subjects MS");

    let levels = r.level_stats(RmTerm::A, 0.05);
    let want = [
        (19.75074, 0.96025, 17.4801, 22.02137),
        (21.21604, 0.59138, 19.81765, 22.61442),
        (22.41076, 0.94481, 20.17663, 24.64489),
        (23.57499, 0.83539, 21.59961, 25.55037),
    ];
    for (l, w) in levels.iter().zip(want) {
        close(l.mean, w.0, 0.0, "mean");
        close(l.se, w.1, 0.0, "se");
        close(l.lcl, w.2, 1e-6, "lcl");
        close(l.ucl, w.3, 1e-6, "ucl");
    }

    // Means comparisons: pair order (1,2) (1,3) (1,4) (2,3) (2,4) (3,4), diff = mean(i) - mean(j), stat unsigned.
    let diff = [-1.4653, -2.66003, -3.82425, -1.19473, -2.35895, -1.16423];
    type Block = (MeansComparison, [f64; 6], [f64; 6], [f64; 6], [f64; 6]);
    let blocks: [Block; 5] = [
        (
            MeansComparison::Tukey,
            [3.54149, 6.42903, 9.24285, 2.88754, 5.70136, 2.81382],
            [0.08833, 9.39239e-4, 9.98939e-6, 0.20491, 0.00312, 0.22344],
            [-3.09624, -4.29097, -5.45519, -2.82567, -3.98989, -2.79517],
            [0.16564, -1.02908, -2.19331, 0.43622, -0.72801, 0.46672],
        ),
        (
            MeansComparison::Bonferroni,
            [2.50421, 4.54601, 6.53568, 2.0418, 4.03147, 1.98967],
            [0.12354, 0.00106, 1.07282e-5, 0.3236, 0.00362, 0.35884],
            [-3.16926, -4.36399, -5.52821, -2.89869, -4.06291, -2.86819],
            [0.23866, -0.95606, -2.12029, 0.50924, -0.65499, 0.53974],
        ),
        (
            MeansComparison::Sidak,
            [2.50421, 4.54601, 6.53568, 2.0418, 4.03147, 1.98967],
            [0.11735, 0.00106, 1.07282e-5, 0.28298, 0.00361, 0.30928],
            [-3.16377, -4.35849, -5.52272, -2.89319, -4.05742, -2.86269],
            [0.23317, -0.96156, -2.12578, 0.50374, -0.66048, 0.53424],
        ),
        (
            MeansComparison::FisherLsd,
            [2.50421, 4.54601, 6.53568, 2.0418, 4.03147, 1.98967],
            [0.02059, 1.76174e-4, 1.78804e-6, 0.05393, 6.0265e-4, 0.05981],
            [-2.68215, -3.87688, -5.0411, -2.41158, -3.5758, -2.38108],
            [-0.24845, -1.44317, -2.6074, 0.02213, -1.1421, 0.05263],
        ),
        (
            MeansComparison::Scheffe,
            [2.50421, 4.54601, 6.53568, 2.0418, 4.03147, 1.98967],
            [0.13205, 0.00209, 2.74805e-5, 0.27356, 0.00641, 0.2945],
            [-3.24178, -4.4365, -5.60073, -2.9712, -4.13543, -2.9407],
            [0.31118, -0.88355, -2.04777, 0.58175, -0.58247, 0.61225],
        ),
    ];
    for (method, stat, p, lcl, ucl) in blocks {
        let list = r.compare(method, 0.05);
        assert_eq!(list.len(), 6);
        for (k, c) in list.iter().enumerate() {
            let what = format!("{} pair {k}", method.label());
            close(c.diff, diff[k], 0.0, &format!("{what} diff"));
            close(c.se, 0.58513, 0.0, &format!("{what} se"));
            close(c.stat, stat[k], 0.0, &format!("{what} stat"));
            close(c.p, p[k], 3e-4, &format!("{what} p"));
            close(c.lcl, lcl[k], 3e-4, &format!("{what} lcl"));
            close(c.ucl, ucl[k], 3e-4, &format!("{what} ucl"));
        }
    }
    // Holm: the pairs by increasing p with thresholds 0.05 / 6, 0.05 / 5, ...
    let pairs = [(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)];
    let holm = r.compare(MeansComparison::HolmBonferroni, 0.05);
    let order: Vec<usize> = holm.iter().map(|c| pairs.iter().position(|p| *p == (c.i, c.j)).unwrap()).collect();
    assert_eq!(order, vec![2, 1, 4, 0, 3, 5]);
    for (c, a) in holm.iter().zip([0.05 / 6.0, 0.01, 0.0125, 0.05 / 3.0]) {
        close(c.alpha, a, 1e-4, "holm alpha");
    }
    let sidak = r.compare(MeansComparison::HolmSidak, 0.05);
    close(sidak[0].alpha, 0.00851, 1e-3, "holm-sidak alpha");
    close(sidak[3].alpha, 0.01695, 1e-3, "holm-sidak alpha 4");
}

/// The two-way repeated measures run (6 subjects, 2 x 3 cells, interaction on).
#[test]
fn two_way_repeated_measures_reproduces_the_origin_run() {
    let r = rm_anova(&data::rm2_data(), &[2, 3], true).unwrap();
    // (term, SS, DF, error SS, error DF, F, p) of the sphericity-assumed row.
    let want = [
        (RmTerm::A, 29.07798, 1.0, 9.07567e-5, 5.0, 1.60197e6, 5.84335e-15),
        (RmTerm::B, 0.05025, 2.0, 7.64333, 10.0, 0.03287, 0.96777),
        (RmTerm::Ab, 0.15591, 2.0, 8.33368, 10.0, 0.09354, 0.91149),
    ];
    for (term, ss, df, sse, dfe, f, p) in want {
        let e = r.effect(term).unwrap();
        close(e.ss, ss, 0.0, "SS");
        close(e.ss_error, sse, 1e-5, "SS error");
        close(e.rows[0].df, df, 0.0, "df");
        close(e.rows[0].df_error, dfe, 0.0, "error df");
        close(e.rows[0].f, f, 1e-5, "F");
        close(e.rows[0].p, p, 1e-4, "p");
        // The lower bound row.
        close(e.rows[3].df, 1.0, 0.0, "lower bound df");
        close(e.rows[3].df_error, dfe / df, 0.0, "lower bound error df");
    }
    let b = r.effect(RmTerm::B).unwrap();
    close(b.rows[1].df, 1.93858, 0.0, "B GG df");
    close(b.rows[1].df_error, 9.6929, 0.0, "B GG error df");
    close(b.rows[1].p, 0.9649, 0.0, "B GG p");
    close(b.rows[3].p, 0.86324, 0.0, "B LB p");
    let s = b.sphericity.unwrap();
    for (got, want, what) in [
        (s.w, 0.96832, "W"),
        (s.chi_sq, 0.12878, "chi-square"),
        (s.df, 2.0, "df"),
        (s.p, 0.93764, "p"),
        (s.greenhouse_geisser, 0.96929, "GG"),
        (s.huynh_feldt, 1.0, "HF"),
        (s.lower_bound, 0.5, "LB"),
    ] {
        close(got, want, 0.0, &format!("B Mauchly {what}"));
    }
    let ab = r.effect(RmTerm::Ab).unwrap();
    let s = ab.sphericity.unwrap();
    close(s.w, 0.98814, 0.0, "AB W");
    close(s.chi_sq, 0.04771, 0.0, "AB chi-square");
    close(s.p, 0.97643, 0.0, "AB p");
    close(s.greenhouse_geisser, 0.98828, 0.0, "AB GG");
    close(ab.rows[1].df, 1.97656, 0.0, "AB GG df");
    close(ab.rows[1].p, 0.90962, 0.0, "AB GG p");
    assert!(r.effect(RmTerm::A).unwrap().sphericity.is_none(), "one degree of freedom: no sphericity");
    close(r.intercept.ss, 39236.00333, 0.0, "intercept SS");
    close(r.intercept.f, 2181.60088, 0.0, "intercept F");
    close(r.intercept.p, 8.4964e-8, 2e-4, "intercept p");
    close(r.subjects.ss, 89.9248, 0.0, "subjects SS");
    close(r.subjects.mean_sq, 17.98496, 0.0, "subjects MS");

    // Descriptive statistics of A, B and the cells: mean, SE, limits.
    let a = r.level_stats(RmTerm::A, 0.05);
    for (l, w) in a.iter().zip([(32.11473, 0.7067, 30.2981, 33.93137), (33.9122, 0.70692, 32.095, 35.7294)]) {
        close(l.mean, w.0, 0.0, "A mean");
        close(l.se, w.1, 0.0, "A se");
        close(l.lcl, w.2, 1e-5, "A lcl");
        close(l.ucl, w.3, 1e-5, "A ucl");
    }
    let bl = r.level_stats(RmTerm::B, 0.05);
    let want = [(33.02463, 0.68089, 31.27435), (33.05261, 0.72852, 31.17988), (32.96316, 0.79485, 30.91994)];
    for (l, w) in bl.iter().zip(want) {
        close(l.mean, w.0, 0.0, "B mean");
        close(l.se, w.1, 0.0, "B se");
        close(l.lcl, w.2, 1e-5, "B lcl");
    }
    let cells = r.level_stats(RmTerm::Ab, 0.05);
    close(cells[0].mean, 32.21895, 0.0, "cell mean");
    close(cells[0].se, 0.67432, 0.0, "cell se");
    close(cells[5].se, 0.88034, 0.0, "last cell se");
    close(cells[5].ucl, 36.17298, 1e-6, "last cell ucl");
}
