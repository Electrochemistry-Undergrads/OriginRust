//! Hypothesis tests, ANOVA, frequency counts and normality tests against numbers from Origin.
//!
//! Reference scripts in `tools/origin-ref/`: `stats-ttest.lines.txt` (X-Functions `ttest1`, `ttest2`,
//! `ttestpair`), `stats-anova.lines.txt` and `stats-norm.lines.txt` (Origin's statistics library through
//! `stats-ref.c`), `stats-freq.lines.txt` and `stats-freq2.lines.txt` (`freqcounts`), `stats-bins.lines.txt`
//! (the automatic bin range). The inputs are closed-form columns that we generated; the formulas are
//! repeated below. Tolerances are written next to each group of numbers.

// The numbers are copied as Origin printed them, with all their digits.
#![allow(clippy::excessive_precision)]

use sheaf_calc::*;

/// Sample A of the scripts: 8 numbers.
fn sample_a() -> Vec<f64> {
    (1..=8).map(|i| 10.0 + 2.0 * (3.0 * i as f64).sin() + i as f64 / 4.0).collect()
}
/// Sample B: 10 numbers.
fn sample_b() -> Vec<f64> {
    (1..=10).map(|i| 11.5 + 2.5 * (2.0 * i as f64).cos() + i as f64 / 5.0).collect()
}
/// Sample P, the pair of A: 8 numbers.
fn sample_p() -> Vec<f64> {
    let a = sample_a();
    (1..=8).map(|i| a[i - 1] + 0.6 + 0.5 * (5.0 * i as f64).cos() + i as f64 / 10.0).collect()
}
/// Sample G, a third group for the ANOVA: 7 numbers.
fn sample_g() -> Vec<f64> {
    (1..=7).map(|i| 12.2 + 1.8 * (7.0 * i as f64).sin() + i as f64 / 8.0).collect()
}
/// Skewed sample: 20 numbers.
fn sample_skewed() -> Vec<f64> {
    (1..=20).map(|i| (1.1 * (2.3 * i as f64).sin()).exp() * 3.0).collect()
}
/// Mixed sample: 40 numbers.
fn sample_mixed() -> Vec<f64> {
    (1..=40).map(|i| 5.0 + 3.0 * (1.7 * i as f64).sin() + 2.0 * (0.37 * (i as f64).powf(1.2)).cos()).collect()
}
/// Squares 1, 4, 9, 16, 25.
fn sample_squares() -> Vec<f64> {
    (1..=5).map(|i| (i * i) as f64).collect()
}

fn close(name: &str, got: f64, want: f64, rel: f64) {
    let ok = (got - want).abs() <= rel * want.abs().max(1e-3);
    assert!(ok, "{name}: got {got:?}, Origin gave {want:?}");
}

// ----- t-tests -------------------------------------------------------------------------------------------------
// The statistic, the degrees of freedom and the standard errors agree to 1e-10. Origin's t distribution is
// less exact than ours: its p-values are off by up to 1e-5 (relative) and its confidence limits, which use the
// t quantile, by up to 3e-6 (the quantile for 7 degrees of freedom is 2.3646215 in Origin, 2.3646242 exactly),
// so those are compared with these tolerances.
const P_TOL: f64 = 2e-5;
const CI_TOL: f64 = 3e-6;

#[test]
fn one_sample_t_test_matches_ttest1() {
    let a = sample_a();
    // (hypothesized mean, tail, alpha, t, df, p, lcl, ucl)
    let cases = [
        (10.0, Tail::Two, 0.05, 2.1305388295862757, 7.0, 0.070613990459291051, 9.888272231144235, 12.145541742364333),
        (
            10.0,
            Tail::Upper,
            0.05,
            2.1305388295862757,
            7.0,
            0.035306995229645526,
            10.112624246903264,
            11.921189726605304,
        ),
        (10.0, Tail::Lower, 0.05, 2.1305388295862757, 7.0, 0.96469300477035447, 10.112624246903264, 11.921189726605304),
        (10.0, Tail::Two, 0.1, 2.1305388295862757, 7.0, 0.070613990459291051, 10.112624246903263, 11.921189726605306),
    ];
    for (m0, tail, alpha, t, df, p, lcl, ucl) in cases {
        let r = ttest_one(&a, m0, tail, alpha).unwrap();
        let tag = format!("{tail:?} alpha {alpha}");
        close(&format!("{tag} t"), r.t, t, 1e-10);
        close(&format!("{tag} df"), r.df, df, 1e-12);
        close(&format!("{tag} p"), r.p, p, P_TOL);
        close(&format!("{tag} lcl"), r.lcl, lcl, CI_TOL);
        close(&format!("{tag} ucl"), r.ucl, ucl, CI_TOL);
    }
}

#[test]
fn two_sample_t_test_matches_ttest2() {
    let (a, b) = (sample_a(), sample_b());
    // (equal variance, tail, mean difference, alpha, t, df, p, lcl, ucl); Origin's interval for the unequal
    // variance test is the pooled one, so only the pooled intervals are compared.
    let pooled = [
        (Tail::Two, 0.0, 0.05, -1.9072589691306436, 16.0, 0.07460412595719583, -3.3411796677281207, 0.1764242747222089),
        (
            Tail::Lower,
            -1.0,
            0.01,
            -0.70194687875824191,
            16.0,
            0.24639894664538048,
            -3.7257952675418076,
            0.56103987453589577,
        ),
        (
            Tail::Upper,
            0.0,
            0.05,
            -1.9072589691306436,
            16.0,
            0.96269793702140216,
            -3.0308686599152175,
            -0.13388673309069432,
        ),
    ];
    for (tail, md, alpha, t, df, p, lcl, ucl) in pooled {
        let r = ttest_two(&a, &b, md, tail, alpha, true).unwrap();
        let tag = format!("pooled {tail:?}");
        close(&format!("{tag} t"), r.t, t, 1e-10);
        close(&format!("{tag} df"), r.df, df, 1e-12);
        close(&format!("{tag} p"), r.p, p, P_TOL);
        close(&format!("{tag} lcl"), r.lcl, lcl, CI_TOL);
        close(&format!("{tag} ucl"), r.ucl, ucl, CI_TOL);
    }
    let welch = [
        (Tail::Two, -1.9937117971415532, 15.634490446322753, 0.063929845467027319),
        (Tail::Upper, -1.9937117971415532, 15.634490446322753, 0.9680350772664863),
    ];
    for (tail, t, df, p) in welch {
        let r = ttest_two(&a, &b, 0.0, tail, 0.05, false).unwrap();
        close("welch t", r.t, t, 1e-10);
        close("welch df", r.df, df, 1e-10);
        close("welch p", r.p, p, P_TOL);
    }
}

#[test]
fn paired_t_test_matches_ttestpair() {
    let (a, p) = (sample_a(), sample_p());
    let cases = [
        (Tail::Two, 0.0, 0.05, -6.8028551869795368, 0.00025255053248005217, -1.3027689605255226, -0.630707139518055),
        (Tail::Lower, -0.5, 0.1, -3.2843968068494638, 0.0067027957017920187, -1.1678096054128397, -0.765666494630738),
        (Tail::Upper, 0.0, 0.05, -6.8028551869795368, 0.99987372473376, -1.2359721409573488, -0.697503959086229),
    ];
    for (tail, md, alpha, t, pv, lcl, ucl) in cases {
        let r = ttest_paired(&a, &p, md, tail, alpha).unwrap();
        close("paired t", r.t, t, 1e-10);
        close("paired df", r.df, 7.0, 1e-12);
        close("paired p", r.p, pv, P_TOL);
        close("paired lcl", r.lcl, lcl, CI_TOL);
        close("paired ucl", r.ucl, ucl, CI_TOL);
    }
}

// ----- ANOVA: the summary table agrees to 1e-11; Origin's p-values and critical values come from the same less
// exact distribution routines as its t-test (see above), the studentized range quantile to about 1e-5, so the
// comparisons use 2e-5 and 1e-4. ------------------------------------------------------------------------

#[test]
fn one_way_anova_matches_origin() {
    let (a, b, g) = (sample_a(), sample_b(), sample_g());
    let r = anova_one_way(&[&a, &b, &g]).unwrap();
    let want_groups = [
        (8.0, 11.016906986754284, 1.3500093331968346, 0.4773003770843055),
        (10.0, 12.59928468325724, 2.0052906867165827, 0.63412859407475575),
        (7.0, 12.817436759993626, 1.2401239029079218, 0.46872277742873875),
    ];
    for (s, (n, mean, sd, sem)) in r.groups.iter().zip(want_groups) {
        close("n", s.n as f64, n, 0.0);
        close("mean", s.mean, mean, 1e-12);
        close("sd", s.sd, sd, 1e-11);
        close("sem", s.sem, sem, 1e-11);
    }
    close("model df", r.model.df, 2.0, 0.0);
    close("model ss", r.model.sum_sq, 15.407666998164066, 1e-11);
    close("model ms", r.model.mean_sq, 7.703833499082033, 1e-11);
    close("error df", r.error.df, 22.0, 0.0);
    close("error ss", r.error.sum_sq, 58.17583680950176, 1e-11);
    close("error ms", r.error.mean_sq, 2.6443562186137162, 1e-11);
    close("total df", r.total.df, 24.0, 0.0);
    close("total ss", r.total.sum_sq, 73.583503807665821, 1e-11);
    close("F", r.f, 2.9133115443579345, 1e-11);
    close("p", r.p, 0.075436916468437087, 1e-6);
    close("R squared", r.r_squared, 0.20939023287661002, 1e-11);
    close("COV", r.cov, 0.13379519544653162, 1e-11);
    close("root MSE", r.root_mse, 1.6261476619955877, 1e-11);
    close("grand mean", r.grand_mean, 12.154006401862484, 1e-11);
}

/// One row per pair (1,2), (1,3), (2,3): difference, SE, statistic, p, lcl, ucl, significant.
type Pairs = [(f64, f64, f64, f64, f64, f64, bool); 3];

#[test]
fn means_comparisons_match_origin() {
    let (a, b, g) = (sample_a(), sample_b(), sample_g());
    let r = anova_one_way(&[&a, &b, &g]).unwrap();
    let se = [0.7713495635495532, 0.84161137196118507, 0.801374317001141];
    let diff = [1.5823776965029559, 1.8005297732393419, 0.21815207673638604];
    let cases: [(MeansComparison, f64, Pairs, f64); 6] = [
        (
            MeansComparison::Tukey,
            0.05,
            [
                (
                    diff[0],
                    se[0],
                    2.9011749081613565,
                    0.12360316469188037,
                    -0.35528418836909603,
                    3.5200395813750078,
                    false,
                ),
                (
                    diff[1],
                    se[1],
                    3.0255456492204664,
                    0.10486068907071555,
                    -0.31363266929803535,
                    3.9146922157767192,
                    false,
                ),
                (diff[2], se[2], 0.3849806751169117, 0.9600462097946395, -1.794933225889483, 2.2312373793622551, false),
            ],
            1e-4,
        ),
        (
            MeansComparison::Bonferroni,
            0.05,
            [
                (
                    diff[0],
                    se[0],
                    2.0514404509691544,
                    0.1569508793104544,
                    -0.41635219525535017,
                    3.5811075882612622,
                    false,
                ),
                (
                    diff[1],
                    se[1],
                    2.139383845353247,
                    0.13125391996980387,
                    -0.3802633274162126,
                    3.9813228738948965,
                    false,
                ),
                (diff[2], se[2], 0.27222244600094342, 1.0, -1.8583783027025054, 2.2946824561752774, false),
            ],
            2e-5,
        ),
        (
            MeansComparison::Sidak,
            0.05,
            [
                (
                    diff[0],
                    se[0],
                    2.0514404509691544,
                    0.14888288135348327,
                    -0.41042209641024585,
                    3.5751774894161574,
                    false,
                ),
                (
                    diff[1],
                    se[1],
                    2.139383845353247,
                    0.12559513720499038,
                    -0.37379305912186478,
                    3.9748526056005486,
                    false,
                ),
                (
                    diff[2],
                    se[2],
                    0.27222244600094342,
                    0.99047046255191162,
                    -1.8522173749774984,
                    2.2885215284502705,
                    false,
                ),
            ],
            2e-5,
        ),
        (
            MeansComparison::FisherLsd,
            0.05,
            [
                (
                    diff[0],
                    se[0],
                    2.0514404509691544,
                    0.052316959770151467,
                    -0.017303386992033598,
                    3.1820587799979454,
                    false,
                ),
                (
                    diff[1],
                    se[1],
                    2.139383845353247,
                    0.043751306656601291,
                    0.055134617785827178,
                    3.5459249286928567,
                    true,
                ),
                (
                    diff[2],
                    se[2],
                    0.27222244600094342,
                    0.78798954715055358,
                    -1.4437965342231736,
                    1.8801006876959456,
                    false,
                ),
            ],
            2e-5,
        ),
        (
            MeansComparison::Scheffe,
            0.05,
            [
                (
                    diff[0],
                    se[0],
                    2.1042039619362636,
                    0.14581508376615987,
                    -0.44183913464383417,
                    3.606594527649746,
                    false,
                ),
                (
                    diff[1],
                    se[1],
                    2.288481618879223,
                    0.12505198119778801,
                    -0.40807185812585089,
                    4.0091314046045348,
                    false,
                ),
                (
                    diff[2],
                    se[2],
                    0.037052530053368275,
                    0.96368551625741639,
                    -1.8848573202712808,
                    2.3211614737440529,
                    false,
                ),
            ],
            2e-5,
        ),
        (
            MeansComparison::Tukey,
            0.01,
            [
                (
                    diff[0],
                    se[0],
                    2.9011749081613565,
                    0.12360316469188037,
                    -0.91991887583788134,
                    4.0846742688437931,
                    false,
                ),
                (
                    diff[1],
                    se[1],
                    3.0255456492204664,
                    0.10486068907071555,
                    -0.92969962056552191,
                    4.5307591670442058,
                    false,
                ),
                (
                    diff[2],
                    se[2],
                    0.3849806751169117,
                    0.9600462097946395,
                    -2.3815462978451292,
                    2.8178504513179012,
                    false,
                ),
            ],
            1e-4,
        ),
    ];
    for (method, alpha, want, tol) in cases {
        let got = r.compare(method, alpha);
        assert_eq!(got.len(), 3);
        for (c, (d, s, stat, p, lcl, ucl, sig)) in got.iter().zip(want) {
            let tag = format!("{} alpha {alpha} pair {}-{}", method.label(), c.i + 1, c.j + 1);
            close(&format!("{tag} diff"), c.diff, d, 1e-11);
            close(&format!("{tag} se"), c.se, s, 1e-11);
            close(&format!("{tag} stat"), c.stat, stat, 1e-9);
            close(&format!("{tag} p"), c.p, p, tol);
            close(&format!("{tag} lcl"), c.lcl, lcl, tol);
            close(&format!("{tag} ucl"), c.ucl, ucl, tol);
            assert_eq!(c.significant, sig, "{tag} significant");
        }
    }
}

// ----- normality tests ---------------------------------------------------------------------------------------

#[test]
fn shapiro_wilk_matches_origin() {
    // (sample, W, p); tolerance 1e-9 relative on W and 1e-6 on p (the p-value is a published
    // polynomial approximation, so it agrees to the digits Origin prints).
    let cases: [(Vec<f64>, f64, f64); 5] = [
        (sample_a(), 0.82098700430290972, 0.047791419072342911),
        (sample_b(), 0.83082949164470321, 0.034239319391649148),
        (sample_skewed(), 0.86345808756525178, 0.0090348937212556546),
        (sample_mixed(), 0.96900682935252569, 0.33460558729769774),
        (sample_squares(), 0.94982453477088336, 0.73593478170802706),
    ];
    for (i, (x, w, p)) in cases.iter().enumerate() {
        let r = shapiro_wilk(x).unwrap();
        close(&format!("case {i} W"), r.statistic, *w, 1e-9);
        close(&format!("case {i} p"), r.p, *p, 1e-6);
    }
}

#[test]
fn lilliefors_and_ks_distance_match_origin() {
    // (sample, D, Lilliefors p, relative tolerance of p). Origin reports at most 0.2. Below 0.1 the
    // Dallal-Wilkinson formula reproduces Origin to six digits; above 0.1 Origin interpolates in a table that we
    // do not have, and the published refinement is within about 6 % of it.
    let cases: [(Vec<f64>, f64, f64, f64); 5] = [
        (sample_a(), 0.26520622903367552, 0.10199037923925106, 0.03),
        (sample_b(), 0.22845119800136515, 0.13763297189629781, 0.08),
        (sample_skewed(), 0.19682280960920651, 0.040993496361269546, 1e-6),
        (sample_mixed(), 0.091097346511491317, 0.2, 0.0),
        (sample_squares(), 0.18193066912947942, 0.2, 0.0),
    ];
    for (i, (x, d, p, tol)) in cases.iter().enumerate() {
        close(&format!("case {i} D"), ks_distance(x).unwrap(), *d, 1e-9);
        let got = lilliefors(x).unwrap().p.min(0.2);
        close(&format!("case {i} Lilliefors p"), got, *p, *tol);
    }
}

#[test]
fn kolmogorov_smirnov_p_is_near_origin_for_small_samples() {
    // Origin's p for samples of 8, 10 and 20 numbers is 0.551, 0.612 and 0.374; the exact distribution of the
    // statistic (which we use) gives 0.542, 0.597 and 0.372, within 3 %. For 40 and 5 numbers Origin gives
    // 0.971 and 1, far from the exact 0.864 and 0.986: not copied, see docs/spec/statistics.md.
    let cases: [(Vec<f64>, f64); 3] =
        [(sample_a(), 0.55098618750604145), (sample_b(), 0.61193513624329965), (sample_skewed(), 0.37363482908593459)];
    for (i, (x, p)) in cases.iter().enumerate() {
        close(&format!("case {i} KS p"), kolmogorov_smirnov(x).unwrap().p, *p, 0.03);
    }
}

#[test]
fn anderson_darling_matches_origin() {
    // (sample, A squared, p)
    let cases: [(Vec<f64>, f64, f64); 4] = [
        (sample_a(), 0.64981155376987054, 0.055895134649805805),
        (sample_b(), 0.70135911691383868, 0.045514039972628449),
        (sample_skewed(), 0.980203182570218, 0.01084380682774861),
        (sample_mixed(), 0.27600095867126839, 0.63939353380942066),
    ];
    for (i, (x, a2, p)) in cases.iter().enumerate() {
        let r = anderson_darling(x).unwrap();
        close(&format!("case {i} A2"), r.statistic, *a2, 1e-9);
        close(&format!("case {i} p"), r.p, *p, 1e-6);
    }
}

#[test]
fn dagostino_matches_origin() {
    // (sample, skewness z, p, kurtosis z, p, K squared, p)
    let cases: [(Vec<f64>, [f64; 6]); 4] = [
        (
            sample_a(),
            [
                1.391114620220141,
                0.16419067212706162,
                -0.13091517751362558,
                0.895842408098424,
                1.9523386702936512,
                0.37675154717220205,
            ],
        ),
        (
            sample_b(),
            [
                0.0088613376413357869,
                0.99292976803770117,
                -2.5164027122659514,
                0.011855962727250091,
                6.33236113360423,
                0.042164334605854115,
            ],
        ),
        (
            sample_skewed(),
            [
                0.92918639469204667,
                0.35279249702933657,
                -2.3586550863619813,
                0.01834129415722896,
                6.42664117250205,
                0.040222827902720804,
            ],
        ),
        (
            sample_mixed(),
            [
                0.5866984769849708,
                0.5574062308333021,
                -0.99313540808061829,
                0.32064397062880245,
                1.3305330416799406,
                0.51413652008158706,
            ],
        ),
    ];
    for (i, (x, w)) in cases.iter().enumerate() {
        let r = dagostino(x);
        let (s, k, o) = (r.skewness.unwrap(), r.kurtosis.unwrap(), r.omnibus.unwrap());
        for (name, got, want) in [
            ("skew z", s.statistic, w[0]),
            ("skew p", s.p, w[1]),
            ("kurt z", k.statistic, w[2]),
            ("kurt p", k.p, w[3]),
            ("K2", o.statistic, w[4]),
            ("K2 p", o.p, w[5]),
        ] {
            close(&format!("case {i} {name}"), got, want, 1e-6);
        }
    }
    // Five numbers: only the kurtosis test (z = -0.27998236282012334, p = 0.77949103638817).
    let r = dagostino(&sample_squares());
    assert!(r.skewness.is_none() && r.omnibus.is_none());
    let k = r.kurtosis.unwrap();
    close("squares kurt z", k.statistic, -0.27998236282012334, 1e-6);
    close("squares kurt p", k.p, 0.77949103638817, 1e-6);
}

// ----- frequency counts -----------------------------------------------------------------------------------------

fn counts_and_ends(f: &Freq) -> (Vec<usize>, Vec<f64>) {
    (f.rows.iter().map(|r| r.count).collect(), f.rows.iter().map(|r| r.end).collect())
}

#[test]
fn freqcounts_with_given_ranges_matches_origin() {
    let d = sample_mixed();
    let o = |min, max, size| FreqOptions { min, max, size, ..Default::default() };
    // inc 1, -2 .. 11
    let f = freq_counts(&d, &o(Some(-2.0), Some(11.0), BinSize::Width(1.0))).unwrap();
    assert_eq!(counts_and_ends(&f).0, vec![0, 0, 3, 3, 4, 3, 7, 6, 5, 4, 0, 5, 0]);
    // inc 2, 0 .. 8: values above the range are not counted
    let f = freq_counts(&d, &o(Some(0.0), Some(8.0), BinSize::Width(2.0))).unwrap();
    assert_eq!(counts_and_ends(&f).0, vec![6, 7, 13, 9]);
    assert_eq!(f.cumulative(), vec![6, 13, 26, 35]);
    // inc 2, 1 .. 9
    let f = freq_counts(&d, &o(Some(1.0), Some(9.0), BinSize::Width(2.0))).unwrap();
    assert_eq!(counts_and_ends(&f).0, vec![7, 10, 11, 4]);
    // inc 2.5, from 0, last edge automatic (9.84 is within a tenth of a bin of 10, so the range reaches 12.5)
    let f = freq_counts(&d, &o(Some(0.0), None, BinSize::Width(2.5))).unwrap();
    assert_eq!(counts_and_ends(&f).0, vec![7, 13, 14, 6, 0]);
    // outliers as extra rows
    let f = freq_counts(&d, &FreqOptions { below: true, above: true, ..o(Some(0.0), Some(8.0), BinSize::Width(2.0)) })
        .unwrap();
    assert_eq!(counts_and_ends(&f).0, vec![0, 6, 7, 13, 9, 5]);
    assert!((f.rows[0].center + 1.0).abs() < 1e-12 && (f.rows[5].center - 9.0).abs() < 1e-12 && f.rows[5].end.is_nan());
    // relative frequencies: counts over the 40 values, in percent they are 15, 17.5, 32.5, 22.5, 12.5
    let f = freq_counts(&d, &o(Some(-2.0), Some(12.0), BinSize::Width(2.0))).unwrap();
    let rel: Vec<f64> = f.relative().iter().map(|r| r * 100.0).collect();
    for (g, w) in rel.iter().zip([0.0, 15.0, 17.5, 32.5, 22.5, 12.5, 0.0]) {
        close("percent", *g, w, 1e-12);
    }
}

#[test]
fn freqcounts_edge_semantics_match_origin() {
    // The value v appears 2^v times (v = 0 .. 10), so a count shows which values a bin holds.
    let x: Vec<f64> = (0..=10).flat_map(|v| std::iter::repeat_n(v as f64, 1 << v)).collect();
    let o = |min, max, size| FreqOptions { min, max, size, ..Default::default() };
    let counts = |f: &Freq| f.rows.iter().map(|r| r.count).collect::<Vec<_>>();
    // Bins hold their lower edge and not their upper edge: [0,2) has 1 + 2, and the 10s are in no bin.
    let f = freq_counts(&x, &o(Some(0.0), Some(10.0), BinSize::Width(2.0))).unwrap();
    assert_eq!(counts(&f), vec![3, 12, 48, 192, 768]);
    assert_eq!(f.cumulative(), vec![3, 15, 63, 255, 1023]);
    let ends: Vec<f64> = f.rows.iter().map(|r| r.end).collect();
    let centers: Vec<f64> = f.rows.iter().map(|r| r.center).collect();
    assert_eq!((ends, centers), (vec![2.0, 4.0, 6.0, 8.0, 10.0], vec![1.0, 3.0, 5.0, 7.0, 9.0]));
    // 2 .. 8: three bins; the row above the range holds the values >= 8. Origin's rows differ in one detail
    // when both outlier rows are on (it moves the 1024 tens from the row above to the row below), which is
    // not copied; with only the row above, Origin gives the same numbers as we do.
    let both = FreqOptions { below: true, above: true, ..o(Some(2.0), Some(8.0), BinSize::Width(2.0)) };
    assert_eq!(counts(&freq_counts(&x, &both).unwrap()), vec![3, 12, 48, 192, 1792]);
    let above_only = FreqOptions { above: true, ..o(Some(2.0), Some(8.0), BinSize::Width(2.0)) };
    assert_eq!(counts(&freq_counts(&x, &above_only).unwrap()), vec![12, 48, 192, 1792]);
    // The number of bins is the range over the size, rounded: 10/3 gives 3 bins, 7/2 gives 4.
    assert_eq!(freq_counts(&x, &o(Some(0.0), Some(10.0), BinSize::Width(3.0))).unwrap().rows.len(), 3);
    assert_eq!(freq_counts(&x, &o(Some(1.0), Some(8.0), BinSize::Width(2.0))).unwrap().rows.len(), 4);
    // A given number of bins.
    let f = freq_counts(&x, &o(Some(2.0), Some(8.0), BinSize::Count(3))).unwrap();
    assert_eq!(counts(&f), vec![12, 48, 192]);
    let small: Vec<f64> = (0..=10).map(f64::from).collect();
    let f = freq_counts(&small, &o(Some(0.0), Some(10.0), BinSize::Count(4))).unwrap();
    assert_eq!(counts(&f), vec![3, 2, 3, 2]);
    assert_eq!(f.cumulative(), vec![3, 5, 8, 10]);
}

/// Cases of Origin's automatic bin range: (n, min, max) and the first edge, last edge and size it chose.
/// `tools/origin-ref/stats-bins.lines.txt`. The size rule is a fit (see the module text), so the sizes are
/// checked against a hit rate; the edges are checked exactly for the sizes Origin chose.
const BIN_CASES: [(usize, f64, f64, f64, f64, f64); 24] = [
    (40, 0.48466053657571817, 9.8396491225694742, 0.0, 10.0, 2.0),
    (8, 9.92685416399913, 13.423311277072113, 9.0, 14.4, 0.9),
    (10, 9.92685416399913, 13.423311277072113, 9.0, 14.4, 0.9),
    (20, 9.92685416399913, 13.423311277072113, 9.5, 13.5, 0.5),
    (5, 1.0, 25.0, 0.0, 27.0, 9.0),
    (8, 1.0, 25.0, 0.0, 30.0, 6.0),
    (20, 1.0, 25.0, 0.0, 30.0, 5.0),
    (100, 1.0, 25.0, 0.0, 26.0, 2.0),
    (20, 1.0002677907874664, 8.87086771485441, 0.0, 10.0, 2.0),
    (20, 0.0, 1.0, -0.2, 1.2, 0.2),
    (100, 0.0, 1.0, -0.1, 1.1, 0.1),
    (5, 100.0, 100000.0, -40000.0, 120000.0, 40000.0),
    (20, 100.0, 100000.0, -20000.0, 120000.0, 20000.0),
    (8, -5.0, 5.0, -6.0, 6.0, 3.0),
    (20, -5.0, 5.0, -6.0, 6.0, 2.0),
    (100, -5.0, 5.0, -6.0, 6.0, 1.0),
    (8, -2.3, 17.1, -5.0, 20.0, 5.0),
    (40, -2.3, 17.1, -4.0, 18.0, 2.0),
    (5, 12.5, 47.0, 10.0, 50.0, 10.0),
    (8, 12.5, 47.0, 9.0, 54.0, 9.0),
    (20, 12.5, 47.0, 10.0, 50.0, 5.0),
    (8, 0.0, 100.0, -30.0, 120.0, 30.0),
    (100, 0.0, 100.0, -10.0, 110.0, 10.0),
    (8, 0.0, 7.0, -2.0, 8.0, 2.0),
];

#[test]
fn automatic_bin_range_agrees_with_origin_for_the_size_origin_chose() {
    // The one-tenth-of-a-bin margin is a fit: Origin's decision is not a clean threshold (a gap of 0.0802 bins
    // did not add a bin at the end for the 40-number sample, a gap of 0.0855 did for the 8-number one).
    // On these cases the fit hits all but two; on 600 random ones it hits 98 % of the starts and 98 % of the ends.
    let misses: Vec<String> = BIN_CASES
        .iter()
        .filter_map(|&(n, lo, hi, first, last, inc)| {
            let (s, e) = auto_bin_range(lo, hi, inc);
            let tol = 1e-9 * inc.max(1.0);
            ((s - first).abs() > tol || (e - last).abs() > tol)
                .then(|| format!("n {n} {lo}..{hi}: got {s}..{e}, Origin {first}..{last}"))
        })
        .collect();
    assert!(misses.len() <= 2, "{misses:#?}");
}

#[test]
fn automatic_bin_size_agrees_with_origin_in_most_cases() {
    let hits = BIN_CASES
        .iter()
        .filter(|(n, lo, hi, _, _, inc)| (auto_bin_size(*n, *lo, *hi) - inc).abs() < 1e-9 * inc)
        .count();
    // Measured on 600 random cases the rule agrees in 57 % of them; on these hand-picked ones it does at least half.
    assert!(hits * 2 >= BIN_CASES.len(), "{hits} of {}", BIN_CASES.len());
}

#[test]
fn default_bins_of_the_freqcounts_runs() {
    // Default settings on four of the samples: Origin's bin ends and counts.
    let d = sample_mixed();
    let f = freq_counts(&d, &FreqOptions::default()).unwrap();
    assert_eq!(counts_and_ends(&f), (vec![6, 7, 13, 9, 5], vec![2.0, 4.0, 6.0, 8.0, 10.0]));
    assert_eq!(f.cumulative(), vec![6, 13, 26, 35, 40]);
    let f = freq_counts(&sample_skewed(), &FreqOptions::default()).unwrap();
    assert_eq!(counts_and_ends(&f), (vec![8, 4, 2, 2, 4], vec![2.0, 4.0, 6.0, 8.0, 10.0]));
    let f = freq_counts(&sample_squares(), &FreqOptions::default()).unwrap();
    assert_eq!(counts_and_ends(&f), (vec![2, 2, 1], vec![9.0, 18.0, 27.0]));
    // The 8-number sample: Origin chose bins of 0.9 from 9.0 to 14.4 (our size rule gives 1 and our end margin stops at 13.5, so the range is given here).
    let f = freq_counts(
        &sample_a(),
        &FreqOptions { min: Some(9.0), max: Some(14.4), size: BinSize::Width(0.9), ..Default::default() },
    )
    .unwrap();
    let ends: Vec<f64> = f.rows.iter().map(|r| r.end).collect();
    for (g, w) in ends.iter().zip([9.9, 10.8, 11.7, 12.6, 13.5, 14.4]) {
        assert!((g - w).abs() < 1e-9, "{ends:?}");
    }
    assert_eq!(f.rows.iter().map(|r| r.count).collect::<Vec<_>>(), vec![0, 5, 1, 1, 1, 0]);
}
