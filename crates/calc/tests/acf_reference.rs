//! ACF, PACF and Ljung-Box against an independent numpy/scipy computation (the PACF from solving each lag's
//! Yule-Walker equations), made by `tools/acf-ref.py` (T363; none of these numbers come from Origin). Tolerance 1e-10
//! absolute for the correlations and errors, 1e-9 relative for Q, 1e-9 absolute for p.

#[path = "fixtures/acf_reference.rs"]
mod fixtures;

use sheaf_calc::acf::autocorrelation;

#[test]
fn autocorrelations_match_numpy_and_scipy() {
    for s in fixtures::SERIES {
        let got = autocorrelation(s.x, s.rows.len()).unwrap();
        for (g, &(lag, acf, se, pacf, pse, q, p)) in got.iter().zip(s.rows) {
            let what = format!("{} lag {lag}", s.name);
            assert_eq!(g.lag, lag, "{what}");
            for (a, b, name) in
                [(g.acf, acf, "acf"), (g.acf_se, se, "acf se"), (g.pacf, pacf, "pacf"), (g.pacf_se, pse, "pacf se")]
            {
                assert!((a - b).abs() < 1e-10, "{what} {name}: {a} vs {b}");
            }
            assert!((g.ljung_box - q).abs() <= 1e-9 * q.max(1.0), "{what} Q: {} vs {q}", g.ljung_box);
            assert!((g.p - p).abs() < 1e-9, "{what} p: {} vs {p}", g.p);
        }
    }
}
