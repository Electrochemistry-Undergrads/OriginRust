//! Kaplan-Meier estimate and rank tests against Origin's `kaplanmeier` X-Function (Origin 2016). The data are ours
//! (14 cases with ties and censoring, two interleaved groups); the runs are `tools/origin-ref/kaplan-suite1.lines.txt`,
//! raw output in `re-workspace/scratch/t29/km1.out` (local). Origin prints 14 digits in its table, 17 elsewhere:
//! tolerance 1e-12 relative for table cells (1e-13 absolute for the 14-digit ones is not needed since we compare
//! the 17-digit cells), 1e-12 for quartile tables.

use sheaf_calc::{RankTest, kaplan_meier, rank_test};

const T: [f64; 14] = [3.0, 5.0, 5.0, 6.0, 8.0, 8.0, 9.0, 12.0, 12.0, 15.0, 17.0, 20.0, 22.0, 25.0];
const C: [u8; 14] = [0, 0, 1, 0, 0, 0, 1, 0, 0, 1, 0, 0, 1, 0];
const G: [u8; 14] = [1, 2, 1, 2, 1, 2, 2, 1, 2, 1, 2, 1, 2, 1];

fn sample(group: Option<u8>) -> (Vec<f64>, Vec<bool>) {
    let idx: Vec<usize> = (0..14).filter(|&i| group.is_none_or(|g| G[i] == g)).collect();
    (idx.iter().map(|&i| T[i]).collect(), idx.iter().map(|&i| C[i] == 1).collect())
}

fn close(a: f64, b: f64, tol: f64, what: &str) {
    assert!((a - b).abs() <= tol * b.abs().max(1.0), "{what}: {a} vs {b}");
}

#[test]
fn single_sample_table_quartiles_and_mean() {
    let (t, c) = sample(None);
    let km = kaplan_meier(&t, &c, 95.0).unwrap();
    assert_eq!((km.n, km.events, km.censored), (14, 10, 4));
    // Origin: survival and Greenwood standard error at the event times (14-digit cells).
    let expect = [
        (3.0, 0.92857142857143, 0.068830293689959),
        (5.0, 0.85714285714286, 0.093521952958282),
        (6.0, 0.77922077922078, 0.11290817938292),
        (8.0, 0.62337662337662, 0.13369309939602),
        (12.0, 0.44526901669759, 0.14299898050865),
        (17.0, 0.33395176252319, 0.14420834510391),
        (20.0, 0.22263450834879, 0.13230158042728),
        (25.0, 0.0, 0.0),
    ];
    assert_eq!(km.steps.len(), expect.len());
    for (s, (t, v, se)) in km.steps.iter().zip(expect) {
        assert_eq!(s.time, t);
        close(s.survival, v, 1e-13, "S");
        close(s.se, se, 1e-12, "SE");
    }
    // Origin's per-case columns: events so far and cases left at risk.
    let events: Vec<usize> = km.rows.iter().map(|r| r.events).collect();
    assert_eq!(events, vec![1, 2, 2, 3, 4, 5, 5, 6, 7, 7, 8, 9, 9, 10]);
    let at_risk: Vec<usize> = km.rows.iter().map(|r| r.at_risk).collect();
    assert_eq!(at_risk, vec![13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0]);
    let shown: Vec<bool> = km.rows.iter().map(|r| r.survival.is_some()).collect();
    assert_eq!(
        shown,
        vec![true, true, false, true, false, true, false, false, true, false, true, true, false, true]
    );
    // Quartiles with limits, mean with standard error and limits.
    let q = |k: usize| (km.quartiles[k].estimate, km.quartiles[k].lower, km.quartiles[k].upper);
    assert_eq!(q(0), (Some(8.0), Some(5.0), Some(12.0)));
    assert_eq!(q(1), (Some(12.0), Some(8.0), Some(20.0)));
    assert_eq!(q(2), (Some(20.0), Some(12.0), Some(25.0)));
    close(km.mean, 14.10760667903525, 1e-12, "mean");
    close(km.mean_se, 2.283045772766588, 1e-12, "mean SE");
    close(km.mean_lower, 9.632919189356322, 1e-12, "mean lower");
    close(km.mean_upper, 18.582294168714178, 1e-12, "mean upper");
}

#[test]
fn two_groups_quartiles_means_and_rank_tests() {
    let (t1, c1) = sample(Some(1));
    let (t2, c2) = sample(Some(2));
    let a = kaplan_meier(&t1, &c1, 95.0).unwrap();
    let b = kaplan_meier(&t2, &c2, 95.0).unwrap();
    let q = |k: &sheaf_calc::KaplanMeier, i: usize| (k.quartiles[i].estimate, k.quartiles[i].lower, k.quartiles[i].upper);
    assert_eq!(q(&a, 0), (Some(8.0), Some(3.0), Some(20.0)));
    assert_eq!(q(&a, 1), (Some(20.0), Some(8.0), Some(25.0)));
    assert_eq!(q(&a, 2), (Some(25.0), Some(12.0), Some(25.0)));
    // Group 2: the upper limit is never reached for the median and the third quartile; Origin gives the largest
    // event time (17).
    assert_eq!(q(&b, 0), (Some(6.0), Some(5.0), Some(17.0)));
    assert_eq!(q(&b, 1), (Some(12.0), Some(6.0), Some(17.0)));
    assert_eq!(q(&b, 2), (Some(17.0), Some(8.0), Some(17.0)));
    close(a.mean, 15.428571428571427, 1e-12, "mean 1");
    close(a.mean_se, 3.5793992687830216, 1e-12, "se 1");
    close(a.mean_lower, 8.413077775467702, 1e-12, "lcl 1");
    close(a.mean_upper, 22.444065081675152, 1e-12, "ucl 1");
    close(b.mean, 12.428571428571429, 1e-12, "mean 2");
    close(b.mean_se, 2.4269035222248196, 1e-12, "se 2");
    close(b.mean_lower, 7.6719279310573807, 1e-12, "lcl 2");
    close(b.mean_upper, 17.185214926085479, 1e-12, "ucl 2");
    // Last censored case (group 2, time 22): the mean is cut off there.
    assert_eq!(b.last_time, 22.0);

    let groups = vec![(t1, c1), (t2, c2)];
    let expect = [
        (RankTest::LogRank, 0.2541932625715298, 0.614137863881765),
        (RankTest::Breslow, 0.18804410911201394, 0.66454950555552528),
        (RankTest::TaroneWare, 0.2396608603154573, 0.62445119350136813),
    ];
    for (test, chi, p) in expect {
        let r = rank_test(&groups, test).unwrap();
        assert_eq!(r.df, 1);
        close(r.chi_square, chi, 1e-12, test.name());
        close(r.p, p, 1e-8, test.name());
    }
}
