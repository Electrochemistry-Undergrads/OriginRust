//! ADF and KPSS statistics against numpy, made by `tools/stationarity-ref.py` (T372; none of these numbers come from
//! Origin). Tolerance 1e-9 relative; lag choices and p-values exact to 1e-12.

#[path = "fixtures/stationarity_reference.rs"]
mod fixtures;

use sheaf_calc::stationarity::{AdfTerms, KpssNull, adf, kpss};

#[test]
fn adf_and_kpss_match_numpy() {
    for (name, y, adfs, kpsses) in fixtures::CASES {
        for (terms, lags, stat, used, nobs) in *adfs {
            let t = match *terms {
                "c" => AdfTerms::Constant,
                "ct" => AdfTerms::Trend,
                _ => AdfTerms::None,
            };
            let a = adf(y, t, *lags).unwrap();
            let what = format!("{name} ADF {terms} {lags:?}");
            assert_eq!((a.lags, a.nobs), (*used, *nobs), "{what}");
            assert!((a.statistic - stat).abs() <= 1e-9 * stat.abs().max(1.0), "{what}: {} vs {stat}", a.statistic);
        }
        for (null, lags, stat, used, p) in *kpsses {
            let h = if *null == "c" { KpssNull::Level } else { KpssNull::Trend };
            let k = kpss(y, h, *lags).unwrap();
            let what = format!("{name} KPSS {null} {lags:?}");
            assert_eq!(k.lags, *used, "{what}");
            assert!((k.statistic - stat).abs() <= 1e-9 * stat.abs().max(1.0), "{what}: {} vs {stat}", k.statistic);
            assert!((k.p - p).abs() < 1e-12, "{what}: p {} vs {p}", k.p);
        }
    }
}
