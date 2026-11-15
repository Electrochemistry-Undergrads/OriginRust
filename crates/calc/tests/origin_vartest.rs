//! Tests for variance and proportions against numbers from Origin (OriginPro 2016): the X-functions
//! `vartest1` and `vartest2` (LabTalk probes) and the report sheets of the One-Sample and Two-Sample Test for
//! Variance, One-Sample Proportion Test and Two-Sample Proportion Test dialogs (run through the menu with
//! `tools/origin-ref/origin-dialog.ps1`). docs/spec/statistics.md section 21 lists every case and what was not
//! measured. The inputs are closed-form columns that we generated. Tolerance 1e-6 relative everywhere (Origin
//! prints 15 digits; its exact binomial p-values carry errors near 1e-11).

// The numbers are copied as Origin printed them, with all their digits.
#![allow(clippy::excessive_precision)]

use sheaf_calc::*;

const TOL: f64 = 1e-6;

fn close(got: f64, want: f64) -> bool {
    (got - want).abs() <= TOL * want.abs().max(1e-300)
}

#[track_caller]
fn eq(what: &str, got: f64, want: f64) {
    assert!(close(got, want), "{what}: got {got:e}, Origin {want:e}");
}

/// Column A of the probes: 8 numbers.
fn a() -> Vec<f64> {
    (1..=8).map(|i| 10.0 + 2.0 * (3.0 * i as f64).sin() + i as f64 / 4.0).collect()
}
/// Column B: 10 numbers.
fn b() -> Vec<f64> {
    (1..=10).map(|i| 11.5 + 2.5 * (2.0 * i as f64).cos() + i as f64 / 5.0).collect()
}

/// `vartest1 irng:=<col> var:=.. tail:=.. alpha:=..`: statistic, p, then the two-sided interval of level
/// `1 - alpha` (two-sided test) or `1 - 2 alpha` (one-sided test).
#[test]
fn one_sample_variance_matches_vartest1() {
    use Tail::{Lower, Two, Upper};
    // (data, var, tail, alpha, chi-square, df, p, lcl, ucl)
    let c = [1.0, 4.0];
    #[rustfmt::skip]
    let cases: Vec<(Vec<f64>, f64, Tail, f64, f64, f64, f64, f64, f64)> = vec![
        (a(), 5.0, Two, 0.05, 2.5515352796059867, 7.0, 0.1536666591362128, 0.79671917636177081, 7.54950463751445),
        (a(), 5.0, Two, 0.1, 2.5515352796059867, 7.0, 0.1536666591362128, 0.9069132656161778, 5.8863018373309908),
        (a(), 5.0, Upper, 0.05, 2.5515352796059867, 7.0, 0.9231666704318936, 0.9069132656161778, 5.8863018373309908),
        (a(), 5.0, Upper, 0.1, 2.5515352796059867, 7.0, 0.9231666704318936, 1.0616324387314418, 4.5030689390488),
        (a(), 5.0, Lower, 0.05, 2.5515352796059867, 7.0, 0.0768333295681064, 0.9069132656161778, 5.8863018373309908),
        (a(), 5.0, Lower, 0.1, 2.5515352796059867, 7.0, 0.0768333295681064, 1.0616324387314418, 4.5030689390488),
        (a(), 2.0, Upper, 0.01, 6.3788381990149663, 7.0, 0.49627466522456387, 0.690525816643653, 10.296400864464324),
        (a(), 2.0, Lower, 0.01, 6.3788381990149663, 7.0, 0.50372533477543613, 0.690525816643653, 10.296400864464324),
        (a(), 0.5, Two, 0.2, 25.515352796059865, 7.0, 0.0012291602206175867, 1.0616324387314418, 4.5030689390488),
        (a(), 2.0, Two, 0.4, 6.3788381990149663, 7.0, 0.99254933044912774, 1.3013721162753586, 3.3376767663881264),
        (a(), 2.0, Two, 0.6, 6.3788381990149663, 7.0, 0.99254933044912774, 1.5217725436503311, 2.731058069624936),
        (b(), 8.0, Two, 0.05, 4.5238395805112965, 9.0, 0.25262502219237859, 1.9024947960908112, 13.402034642259519),
        (c.to_vec(), 2.0, Two, 0.05, 2.2499999999999996, 1.0, 0.26722880508204305, 0.89572117034192467, 4582.16221365583),
        (c.to_vec(), 2.0, Upper, 0.05, 2.2499999999999996, 1.0, 0.13361440254102153, 1.1714308124098258, 1144.414999151644),
    ];
    for (i, (data, var, tail, alpha, chi, df, p, lcl, ucl)) in cases.iter().enumerate() {
        let r = vartest_one(data, *var, *tail, *alpha).expect("case runs");
        eq(&format!("case {i} chi-square"), r.statistic, *chi);
        eq(&format!("case {i} df"), r.df.0, *df);
        eq(&format!("case {i} p"), r.p, *p);
        eq(&format!("case {i} lcl"), r.lcl, *lcl);
        eq(&format!("case {i} ucl"), r.ucl, *ucl);
    }
}

#[test]
fn one_sample_variance_report_matches_the_dialog() {
    let r = vartest_one(&a(), 1.0, Tail::Two, 0.05).unwrap();
    let s = r.samples[0];
    assert_eq!(s.n, 8);
    eq("mean", s.mean, 11.0169069867543);
    eq("sd", s.sd, 1.35000933319683);
    eq("variance", r.estimate, 1.82252519971856);
    eq("chi-square", r.statistic, 12.7576763980299);
    eq("p", r.p, 0.156474664426636);
    // Confidence levels 90, 95, 99 of the upper-tailed test of variance 5: lower limits only.
    let lower = [1.06163243873144, 0.906913265616178, 0.690525816643653];
    for (level, want) in [0.9, 0.95, 0.99].iter().zip(lower) {
        let (lo, hi) = variance_interval(8, r.estimate, Tail::Upper, *level);
        eq("upper-tailed lower limit", lo, want);
        assert!(hi.is_nan());
    }
    // Lower-tailed test: lower limit 0, upper limits.
    let upper = [4.5030689390488, 5.88630183733099, 10.2964008644643];
    for (level, want) in [0.9, 0.95, 0.99].iter().zip(upper) {
        let (lo, hi) = variance_interval(8, r.estimate, Tail::Lower, *level);
        assert_eq!(lo, 0.0);
        eq("lower-tailed upper limit", hi, want);
    }
    // Two-sided test, levels 80 and 97.5.
    let (lo80, hi80) = variance_interval(8, r.estimate, Tail::Two, 0.8);
    let (lo97, hi97) = variance_interval(8, r.estimate, Tail::Two, 0.975);
    eq("80 lower", lo80, 1.06163243873144);
    eq("80 upper", hi80, 4.5030689390488);
    eq("97.5 lower", lo97, 0.713319017255759);
    eq("97.5 upper", hi97, 9.56153776351299);
}

#[test]
fn two_sample_variance_matches_vartest2() {
    use Tail::{Lower, Two, Upper};
    let (a, b) = (a(), b());
    let c = [1.0, 4.0];
    // (first, second, tail, alpha, F, df1, df2, p, lcl, ucl)
    #[rustfmt::skip]
    let cases: Vec<(&[f64], &[f64], Tail, f64, f64, f64, f64, f64, f64, f64)> = vec![
        (&a, &b, Two, 0.05, 0.45323022914345851, 7.0, 9.0, 0.30934827784074181, 0.10798803078907984, 2.1860277807322057),
        (&a, &b, Two, 0.1, 0.45323022914345851, 7.0, 9.0, 0.30934827784074181, 0.13764506856862324, 1.6663801124937843),
        (&a, &b, Upper, 0.05, 0.45323022914345851, 7.0, 9.0, 0.845325861079629, 0.13764506856862324, 1.6663801124937843),
        (&a, &b, Upper, 0.1, 0.45323022914345851, 7.0, 9.0, 0.845325861079629, 0.18090833332278655, 1.2349046991296266),
        (&a, &b, Lower, 0.05, 0.45323022914345851, 7.0, 9.0, 0.15467413892037091, 0.13764506856862324, 1.6663801124937843),
        (&a, &b, Lower, 0.1, 0.45323022914345851, 7.0, 9.0, 0.15467413892037091, 0.18090833332278655, 1.2349046991296266),
        (&b, &a, Two, 0.05, 2.2063841634964634, 9.0, 7.0, 0.30934827784074165, 0.45745099905212766, 9.260296786026279),
        (&b, &a, Upper, 0.05, 2.2063841634964634, 9.0, 7.0, 0.15467413892037082, 0.600103179307219, 7.26502032547161),
        (&c, &a, Two, 0.05, 2.4691016621854658, 1.0, 7.0, 0.32019618587582288, 0.30585988127374536, 2341.2439338930417),
        (&c, &a, Upper, 0.05, 2.4691016621854658, 1.0, 7.0, 0.16009809293791144, 0.441585078763867, 584.60539871007109),
        (&a, &b, Two, 0.4, 0.45323022914345851, 7.0, 9.0, 0.30934827784074181, 0.25031077116768458, 0.87249913260653356),
    ];
    // Origin's F distribution is only good to about 4e-6 relative in the tails (its 2.5 % point of F(7, 9)
    // differs from scipy's and ours by 1.3e-6, its two-sided p of the first case by 7e-8, its 5 % point by 4e-6): 1e-5 here.
    let near = |what: String, got: f64, want: f64| {
        assert!((got - want).abs() <= 1e-5 * want.abs().max(1e-300), "{what}: got {got:e}, Origin {want:e}");
    };
    for (i, (x, y, tail, alpha, f, d1, d2, p, lcl, ucl)) in cases.iter().enumerate() {
        let r = vartest_two(x, y, 1.0, *tail, *alpha).expect("case runs");
        eq(&format!("case {i} F"), r.statistic, *f);
        eq(&format!("case {i} df1"), r.df.0, *d1);
        eq(&format!("case {i} df2"), r.df.1, *d2);
        near(format!("case {i} p"), r.p, *p);
        near(format!("case {i} lcl"), r.lcl, *lcl);
        near(format!("case {i} ucl"), r.ucl, *ucl);
    }
    // The report sheet of the dialog (columns A and B, hypothesized ratio 1).
    let r = vartest_two(&a, &b, 1.0, Two, 0.05).unwrap();
    eq("mean B", r.samples[1].mean, 12.5992846832572);
    eq("sd B", r.samples[1].sd, 2.00529068671658);
    eq("variance B", r.samples[1].sd * r.samples[1].sd, 4.02119073823226);
}

#[test]
fn flat_samples_of_the_variance_tests() {
    // Origin: a flat sample gives chi-square 0, df 0, p 0 and leaves the limits unset; as the second sample of
    // the F test the ratio is infinite and the limits Infinity, as the first sample 0 and 0.
    let flat = [3.0; 5];
    let r = vartest_one(&flat, 1.0, Tail::Two, 0.05).unwrap();
    assert_eq!((r.statistic, r.df.0, r.p), (0.0, 0.0, 0.0));
    assert!(r.lcl.is_nan() && r.ucl.is_nan());
    let r = vartest_two(&a(), &flat, 1.0, Tail::Two, 0.05).unwrap();
    assert_eq!((r.lcl, r.ucl), (f64::INFINITY, f64::INFINITY));
    let r = vartest_two(&flat, &a(), 1.0, Tail::Two, 0.05).unwrap();
    assert_eq!((r.lcl, r.ucl), (0.0, 0.0));
}

#[test]
fn missing_values_are_left_out_of_the_variance_test() {
    // Origin: a missing value in a column of 8 gave 7 numbers (df 6); the statistic equals the one of the
    // column without that row (the 7 remaining numbers of A are the ones below).
    let mut data = a();
    let kept: Vec<f64> = data.iter().enumerate().filter(|(i, _)| *i != 2).map(|(_, v)| *v).collect();
    data[2] = f64::NAN;
    let with = vartest_one(&data, 2.0, Tail::Two, 0.05).unwrap();
    let without = vartest_one(&kept, 2.0, Tail::Two, 0.05).unwrap();
    assert_eq!(with.df.0, 6.0);
    eq("statistic", with.statistic, without.statistic);
}

/// What one run of the One-Sample Proportion Test dialog printed.
struct One {
    x: u64,
    n: u64,
    p0: f64,
    tail: Tail,
    z: f64,
    p_normal: f64,
    p_exact: f64,
    /// Limits at the levels 90, 95, 99 %: Wald lower, upper, exact lower, upper (NaN = "--").
    wald: [[f64; 3]; 2],
    exact: [[f64; 3]; 2],
}

#[test]
fn one_proportion_matches_the_dialog() {
    use Tail::{Lower, Two, Upper};
    let nan = f64::NAN;
    #[rustfmt::skip]
    let cases = [
        // 20 of 20, two-sided: Wald 1 and 1; the Clopper-Pearson interval has no upper limit ("--").
        One { x: 20, n: 20, p0: 0.5, tail: Two, z: 4.4721359549996, p_normal: 7.744216431016E-06, p_exact: 1.9073486328125E-06,
              wald: [[1.0; 3], [1.0; 3]],
              exact: [[0.860891659331735, 0.831566529016915, 0.767270499010926], [nan; 3]] },
        One { x: 60, n: 100, p0: 0.5, tail: Two, z: 2.0, p_normal: 0.045500263896185, p_exact: 0.056887929303614,
              wald: [[0.519418958248053, 0.503981766472894, 0.473810650835933], [0.680581041751947, 0.696018233527106, 0.726189349164067]],
              exact: [[0.512975820253895, 0.497209150422335, 0.466472995652681], [0.6824739847635, 0.696705231297123, 0.723742341563696]] },
        // Below n p0 the z is 0 (Origin clips it), also for the two-sided test: p = 1.
        One { x: 45, n: 100, p0: 0.5, tail: Upper, z: 0.0, p_normal: 0.5, p_exact: 0.86437349591575,
              wald: [[0.38624361461394, 0.368169565265201, 0.334265654549483], [1.0; 3]],
              exact: [[0.382504782870829, 0.36516229019426, 0.333324563042116], [1.0; 3]] },
        One { x: 45, n: 100, p0: 0.5, tail: Lower, z: 0.0, p_normal: 0.5, p_exact: 0.18410080071068,
              wald: [[0.0; 3], [0.51375638538606, 0.531830434734799, 0.565734345450516]],
              exact: [[0.0; 3], [0.519038051203254, 0.53711039864948, 0.570760822154472]] },
        One { x: 1, n: 5, p0: 0.5, tail: Two, z: 0.0, p_normal: 1.0, p_exact: 0.375,
              wald: [[0.0; 3], [0.494240361832046, 0.550609016230633, 0.660778353693703]],
              exact: [[0.0102062183130115, 0.00505076337946806, 0.00100200602108012], [0.657408318001139, 0.716417936118089, 0.81490272787827]] },
        One { x: 30, n: 100, p0: 0.3, tail: Two, z: 0.0, p_normal: 1.0, p_exact: 1.0,
              wald: [[0.224623337473722, 0.210183166814579, 0.181960672392018], [0.375376662526278, 0.389816833185421, 0.418039327607982]],
              exact: [[0.224923224371387, 0.212406420489537, 0.189014761860124], [0.384220612794201, 0.399814676179804, 0.430613553616521]] },
        One { x: 700, n: 1000, p0: 0.65, tail: Two, z: 3.314967720659, p_normal: 0.00091653707611106, p_exact: 0.00090671247636319,
              wald: [[0.67616380639951, 0.671597423491067, 0.662672687128398], [0.72383619360049, 0.728402576508932, 0.737327312871602]],
              exact: [[0.675246387280651, 0.670538321302634, 0.661272793808857], [0.723877228840989, 0.728278887870856, 0.736794007752001]] },
        One { x: 20, n: 100, p0: 0.5, tail: Two, z: 0.0, p_normal: 1.0, p_exact: 6.9309254833213E-10,
              wald: [[0.134205854921941, 0.121601440618398, 0.096966827858044], [0.265794145078059, 0.278398559381602, 0.303033172141956]],
              exact: [[0.136661325245867, 0.126655552101956, 0.108410541466599], [0.277200249670914, 0.291842689088628, 0.321226412852798]] },
        One { x: 7, n: 20, p0: 0.4, tail: Upper, z: 0.0, p_normal: 0.5, p_exact: 0.74998932806218,
              wald: [[0.213317854229862, 0.174570365130816, 0.101886519605961], [1.0; 3]],
              exact: [[0.206664033282847, 0.177310917574449, 0.129178784296507], [1.0; 3]] },
        One { x: 60, n: 100, p0: 0.5, tail: Lower, z: 2.0, p_normal: 0.97724986805191, p_exact: 0.9823999004491,
              wald: [[0.0; 3], [0.662782948292984, 0.680581041751947, 0.71396730511217]],
              exact: [[0.0; 3], [0.665762600164827, 0.6824739847635, 0.712919389419604]] },
        One { x: 60, n: 100, p0: 0.5, tail: Upper, z: 2.0, p_normal: 0.022750131948092, p_exact: 0.028443964651807,
              wald: [[0.537217051707016, 0.519418958248053, 0.48603269488783], [1.0; 3]],
              exact: [[0.531147852190932, 0.512975820253895, 0.478903612797712], [1.0; 3]] },
    ];
    let levels = [0.9, 0.95, 0.99];
    for (i, c) in cases.iter().enumerate() {
        let r = proportion_one(c.x, c.n, c.p0, c.tail).expect("case runs");
        eq(&format!("case {i} estimate"), r.estimate, c.x as f64 / c.n as f64);
        assert!((r.z - c.z).abs() <= TOL * c.z.abs().max(1.0), "case {i} z: {} vs {}", r.z, c.z);
        eq(&format!("case {i} normal p"), r.p_normal, c.p_normal);
        // Origin's exact p is off by up to 4e-10 absolute for tiny values (6.93e-10 printed for 1.116e-9).
        assert!(
            (r.p_exact - c.p_exact).abs() <= (TOL * c.p_exact).max(5e-10),
            "case {i} exact p: {} vs {}",
            r.p_exact,
            c.p_exact
        );
        for (k, level) in levels.iter().enumerate() {
            let (wl, wu) = wald_interval(c.x, c.n, c.tail, *level);
            let (el, eu) = exact_interval(c.x, c.n, c.tail, *level);
            for (what, got, want) in [
                ("wald lower", wl, c.wald[0][k]),
                ("wald upper", wu, c.wald[1][k]),
                ("exact lower", el, c.exact[0][k]),
                ("exact upper", eu, c.exact[1][k]),
            ] {
                if want.is_nan() {
                    assert!(got.is_nan(), "case {i} {what} {level}: {got} but Origin printed --");
                } else {
                    assert!(
                        (got - want).abs() <= TOL * want.abs().max(1e-3),
                        "case {i} {what} at {level}: got {got:e}, Origin {want:e}"
                    );
                }
            }
        }
    }
}

#[test]
fn one_sided_alpha_changes_the_headline_interval_level() {
    // Origin, 7 of 20 against 0.4, upper tail, significance 0.1: the headline columns read "90% LCL", 0.2133...,
    // which is the 90 % one-sided Wald limit (1.2816 standard errors below the estimate).
    let (lo, hi) = wald_interval(7, 20, Tail::Upper, 0.9);
    eq("90% LCL", lo, 0.21331785422986);
    assert_eq!(hi, 1.0);
    let (lo, _) = exact_interval(7, 20, Tail::Upper, 0.9);
    eq("exact 90% LCL", lo, 0.20666403328285);
}

#[test]
fn two_proportions_match_the_dialog() {
    // 30 of 50 against 20 of 50, pooled z, two-sided, levels 90 95 99.
    let r = proportion_two((30, 50), (20, 50), 0.0, true, Tail::Two).unwrap();
    eq("difference", r.estimate, 0.2);
    eq("z", r.z, 2.0);
    eq("p", r.p_normal, 0.045500263896185);
    let (lo, hi) = wald_difference_interval((30, 50), (20, 50), Tail::Two, 0.95);
    eq("95% LCL", lo, 0.0079635329457877);
    eq("95% UCL", hi, 0.39203646705421);
    let want = [
        (0.9, 0.0388379164961064, 0.361162083503894),
        (0.95, 0.00796353294578767, 0.392036467054212),
        (0.99, -0.0523786983281347, 0.452378698328135),
    ];
    for (level, wl, wu) in want {
        let (lo, hi) = wald_difference_interval((30, 50), (20, 50), Tail::Two, level);
        assert!((lo - wl).abs() <= TOL * wl.abs(), "{level}: {lo} vs {wl}");
        assert!((hi - wu).abs() <= TOL * wu.abs(), "{level}: {hi} vs {wu}");
    }
}

/// Every run of the Two-Sample Proportion Test dialog of section 21.4 (summarized input, levels 90 95 99):
/// the unpooled and pooled z, the three tails, a negative difference (z keeps its sign here, unlike the
/// one-sample test), a test difference of 0.1, small samples and Fisher's exact p.
#[test]
fn two_proportions_cover_every_dialog_run() {
    use Tail::{Lower, Two, Upper};
    // (first, second, difference, pooled, tail, z, p of the normal approximation, Fisher's p)
    #[rustfmt::skip]
    let cases: Vec<((u64, u64), (u64, u64), f64, bool, Tail, f64, f64, f64)> = vec![
        ((30, 50), (20, 50), 0.0, false, Two,   2.0412414523193,  0.041226833337006,  0.071342403485151),
        ((30, 50), (20, 50), 0.0, true,  Upper, 2.0,              0.022750131948092,  0.035671201744004),
        ((30, 50), (20, 50), 0.0, false, Lower, 2.0412414523193,  0.9793865833315,    0.98634419171458),
        ((20, 50), (30, 50), 0.0, false, Two,  -2.0412414523193,  0.041226833337006,  0.071342403485151),
        ((20, 50), (30, 50), 0.0, false, Upper,-2.0412414523193,  0.9793865833315,    0.98634419171744),
        ((20, 50), (30, 50), 0.0, true,  Lower,-2.0,              0.022750131948092,  0.035671201741147),
        ((30, 50), (20, 50), 0.1, false, Two,   1.0206207261597,  0.30743416593004,   f64::NAN),
        ((3, 10),  (1, 8),   0.0, false, Two,   0.93983017460485, 0.34730467831052,   0.5882352938664),
        ((10, 10), (5, 10),  0.0, false, Two,   3.1622776601684,  0.0015654022579966, 0.032507739937749),
        ((15, 40), (25, 35), 0.0, false, Two,  -3.1380014488579,  0.0017010403614346, 0.0051489221075077),
    ];
    for (i, (first, second, d0, pooled, tail, z, p, fisher)) in cases.iter().enumerate() {
        let r = proportion_two(*first, *second, *d0, *pooled, *tail).expect("case runs");
        eq(&format!("case {i} z"), r.z, *z);
        eq(&format!("case {i} p"), r.p_normal, *p);
        match r.p_fisher {
            Some(got) if !fisher.is_nan() => {
                // Origin's exact p is good to about 5e-10 (its sums run in single steps of a few 1e-12).
                assert!((got - fisher).abs() <= 5e-10, "case {i} Fisher: got {got:e}, Origin {fisher:e}");
            }
            None => assert!(fisher.is_nan(), "case {i}: no Fisher p, Origin has {fisher}"),
            Some(got) => panic!("case {i}: Fisher p {got} for a test difference other than 0"),
        }
    }
}

/// One-sided and signed intervals of the difference (Origin's Confidence Intervals tables).
#[test]
fn two_proportion_intervals_follow_the_tail() {
    use Tail::{Lower, Two, Upper};
    let near = |what: &str, got: f64, want: f64| {
        assert!((got - want).abs() <= TOL * want.abs().max(1e-300), "{what}: got {got:e}, Origin {want:e}");
    };
    // 30 of 50 against 20 of 50, upper tail: lower limits only (the upper limit prints as 1).
    for (level, want) in [(0.9, 0.074434103414031), (0.95, 0.0388379164961065), (0.99, -0.0279346102243397)] {
        let (lo, hi) = wald_difference_interval((30, 50), (20, 50), Upper, level);
        near("upper-tailed lower limit", lo, want);
        assert_eq!(hi, 1.0);
    }
    // Lower tail: [-1, upper].
    for (level, want) in [(0.9, 0.325565896585969), (0.95, 0.361162083503893), (0.99, 0.42793461022434)] {
        let (lo, hi) = wald_difference_interval((30, 50), (20, 50), Lower, level);
        assert_eq!(lo, -1.0);
        near("lower-tailed upper limit", hi, want);
    }
    // The negative difference of 20 of 50 against 30 of 50.
    let (lo, hi) = wald_difference_interval((20, 50), (30, 50), Two, 0.95);
    near("negative lower", lo, -0.39203646705421);
    near("negative upper", hi, -0.0079635329457877);
    let (lo, hi) = wald_difference_interval((20, 50), (30, 50), Two, 0.99);
    near("negative 99 lower", lo, -0.452378698328135);
    near("negative 99 upper", hi, 0.0523786983281348);
    // Small samples: 3 of 10 against 1 of 8 (not cut: the limits stay inside [-1, 1]).
    let (lo, hi) = wald_difference_interval((3, 10), (1, 8), Two, 0.95);
    near("small lower", lo, -0.18995284633601);
    near("small upper", hi, 0.53995284633601);
    // 15 of 40 against 25 of 35.
    let (lo, hi) = wald_difference_interval((15, 40), (25, 35), Two, 0.95);
    near("unequal lower", lo, -0.55120014176661);
    near("unequal upper", hi, -0.12737128680482);
    // Both samples all successes: no spread, no z and no p, interval 0 to 0, Fisher's p 1.
    let r = proportion_two((10, 10), (8, 8), 0.0, false, Two).unwrap();
    assert!(r.z.is_nan() && r.p_normal.is_nan());
    assert_eq!(r.p_fisher, Some(1.0));
    assert_eq!(wald_difference_interval((10, 10), (8, 8), Two, 0.95), (0.0, 0.0));
}

fn a3() -> Vec<f64> {
    (1..=8).map(|i| 10.0 + (3.0 * i as f64).sin()).collect()
}
fn b3() -> Vec<f64> {
    (1..=10).map(|i| 10.0 + 6.0 * (2.0 * i as f64).sin()).collect()
}

/// Two-Sample Test for Variance dialog, a significant case (A: 10 + sin 3i, n 8; B: 10 + 6 sin 2i, n 10) with
/// the three tails and the intervals ticked (levels 90 95 99).
#[test]
fn two_sample_variance_dialog_reports_each_tail() {
    use Tail::{Lower, Two, Upper};
    let (a, b) = (a3(), b3());
    // Origin's F distribution is only good to about 4e-6 relative in the tails: 1e-5.
    let near = |what: &str, got: f64, want: f64| {
        assert!((got - want).abs() <= 1e-5 * want.abs().max(1e-300), "{what}: got {got:e}, Origin {want:e}");
    };
    let r = vartest_two(&a, &b, 1.0, Two, 0.05).unwrap();
    eq("F", r.statistic, 0.0198795620225935);
    near("two-sided p", r.p, 3.11295523751483e-5);
    assert!(r.rejected());
    let lo = [0.00603738122870874, 0.00473656569603998, 0.00288741737587226];
    let hi = [0.0730906825481206, 0.0958834430181464, 0.169251014777939];
    for (i, level) in [0.9, 0.95, 0.99].into_iter().enumerate() {
        let (l, u) = ratio_interval(a.len(), b.len(), r.estimate, Two, level);
        near("two-sided lower", l, lo[i]);
        near("two-sided upper", u, hi[i]);
    }
    let r = vartest_two(&a, &b, 1.0, Lower, 0.05).unwrap();
    near("lower p", r.p, 1.55647761875741e-5);
    assert!(r.rejected());
    let hi = [0.0541653291854217, 0.0730906825481206, 0.133565841332107];
    for (i, level) in [0.9, 0.95, 0.99].into_iter().enumerate() {
        let (l, u) = ratio_interval(a.len(), b.len(), r.estimate, Lower, level);
        assert_eq!(l, 0.0);
        near("lower-tailed upper", u, hi[i]);
    }
    let r = vartest_two(&a, &b, 1.0, Upper, 0.05).unwrap();
    near("upper p", r.p, 0.999984435223812);
    assert!(!r.rejected());
    let lo = [0.00793499241983704, 0.00603738122870874, 0.00354178496000994];
    for (i, level) in [0.9, 0.95, 0.99].into_iter().enumerate() {
        let (l, u) = ratio_interval(a.len(), b.len(), r.estimate, Upper, level);
        near("upper-tailed lower", l, lo[i]);
        assert!(u.is_nan());
    }
    // Original columns, alpha 0.1, levels 80 and 97.5 (two-sided) and the one-sided p-values.
    let (a, b) = (self::a(), self::b());
    let r = vartest_two(&a, &b, 1.0, Two, 0.1).unwrap();
    for (level, want_lo, want_hi) in
        [(0.8, 0.180908333322787, 1.23490469912963), (0.975, 0.0864579395572253, 2.81524108445531)]
    {
        let (l, u) = ratio_interval(a.len(), b.len(), r.estimate, Two, level);
        near("80/97.5 lower", l, want_lo);
        near("80/97.5 upper", u, want_hi);
    }
    near("upper p", vartest_two(&a, &b, 1.0, Upper, 0.05).unwrap().p, 0.845325861079629);
    near("lower p", vartest_two(&a, &b, 1.0, Lower, 0.05).unwrap().p, 0.154674138920371);
}

/// One-Sample Test for Variance dialog, significant cases (column A, n 8, variance 1.8225...).
#[test]
fn one_sample_variance_dialog_significant_cases() {
    use Tail::{Lower, Two, Upper};
    let a = a();
    let r = vartest_one(&a, 0.3, Two, 0.05).unwrap();
    eq("chi-square", r.statistic, 42.5255879934331);
    eq("two-sided p", r.p, 8.23668836691382e-7);
    assert!(r.rejected());
    let r = vartest_one(&a, 0.3, Upper, 0.05).unwrap();
    eq("upper p", r.p, 4.11834418345691e-7);
    assert!(r.rejected());
    let r = vartest_one(&a, 20.0, Lower, 0.05).unwrap();
    eq("chi-square", r.statistic, 0.637883819901497);
    eq("lower p", r.p, 0.00123117416379124);
    assert!(r.rejected());
    // Two-sided intervals at 90 95 99.
    let want = [
        (0.906913265616178, 5.88630183733099),
        (0.796719176361771, 7.54950463751445),
        (0.629146861198078, 12.8962370807767),
    ];
    for (level, (wl, wu)) in [0.9, 0.95, 0.99].into_iter().zip(want) {
        let (l, u) = variance_interval(a.len(), r.estimate, Two, level);
        eq("lower", l, wl);
        eq("upper", u, wu);
    }
}

/// The "Power Analysis" group of the Two-Sample Proportion Test dialog (Actual Power and Hypothetical Power).
#[test]
fn two_proportion_power_matches_the_dialog() {
    use Tail::{Lower, Two, Upper};
    // Two-sided, 30 of 50 against 20 of 50: significance 0.05 and 0.01 (the test difference does not matter).
    eq("power 0.05", power_two_proportions(0.6, 0.4, 50.0, 0.05, Two), 0.516323421027342);
    eq("power 0.01", power_two_proportions(0.6, 0.4, 50.0, 0.01, Two), 0.278367227355963);
    // 15 of 40 against 25 of 35: the actual sizes 40 and 35 (each taken for both samples), then 50 100 200.
    let (p1, p2) = (15.0 / 40.0, 25.0 / 35.0);
    let want = [
        (40.0, 0.87616102837545),
        (35.0, 0.828106373863776),
        (50.0, 0.938038580207161),
        (100.0, 0.998814261304273),
        (200.0, 0.999999877623331),
    ];
    for (n, w) in want {
        eq(&format!("power n {n}"), power_two_proportions(p1, p2, n, 0.05, Two), w);
    }
    // One-sided: Origin returns only one of the two terms, the one that looks like the wrong direction.
    eq("upper", power_two_proportions(0.6, 0.4, 50.0, 0.05, Upper), 9.96062005463871e-5);
    eq("lower 40", power_two_proportions(p1, p2, 40.0, 0.05, Lower), 3.01679803625835e-7);
    eq("lower 35", power_two_proportions(p1, p2, 35.0, 0.05, Lower), 8.72825092768114e-7);
    // Nothing to compute without spread.
    assert!(power_two_proportions(1.0, 1.0, 10.0, 0.05, Two).is_nan());
}

/// Warning under the normal approximation of the two-sample test: some sample with fewer than 10 successes or
/// fewer than 10 failures.
#[test]
fn two_sample_normal_approximation_warning() {
    for (a, b) in [
        ((3, 10), (1, 8)),
        ((10, 10), (5, 10)),
        ((10, 10), (8, 8)),
        ((20, 25), (20, 25)),
        ((3, 10), (30, 50)),
        ((30, 50), (3, 10)),
    ] {
        assert!(two_sample_normal_doubtful(a, b), "{a:?} {b:?}");
    }
    for (a, b) in [((30, 50), (20, 50)), ((15, 40), (25, 35)), ((12, 30), (30, 50))] {
        assert!(!two_sample_normal_doubtful(a, b), "{a:?} {b:?}");
    }
}

/// The "Power Analysis" group of the One-Sample Proportion Test dialog: 60 of 100 against 0.5 and 30 of 100
/// against 0.4 (Actual Power at 100, Hypothetical Power at 50 100 200, significance 0.05).
#[test]
fn one_proportion_power_matches_the_dialog() {
    use Tail::{Lower, Two, Upper};
    // (observed p, hypothesized p, powers at n = 100, 50, 200)
    let rows = [
        (0.6, 0.5, [0.516323421027342, 0.28904912956061, 0.812291820591136]),
        (0.3, 0.4, [0.534629686422704, 0.290523649212481, 0.839102957979036]),
    ];
    for (p1, p0, want) in rows {
        for (n, w) in [100.0, 50.0, 200.0].into_iter().zip(want) {
            eq(&format!("power {p1} n {n}"), power_one_proportion(p1, p0, n, 0.05, Two), w);
        }
    }
    // One-sided, as for two samples only one term of the two-sided power is returned.
    eq("upper", power_one_proportion(0.6, 0.5, 100.0, 0.05, Upper), 9.96062005464234e-5);
    eq("lower", power_one_proportion(0.6, 0.5, 100.0, 0.05, Lower), 0.641499487271636);
}
