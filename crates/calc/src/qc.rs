//! X bar R control chart statistics (Plot > Statistics > QC (X bar R) Chart, T111).
//!
//! The values are cut into subgroups of `n` in row order; a short last subgroup is kept (Origin 2016
//! kept a last group of 2 for 20 values in groups of 3, measured 2026-11-16). Per subgroup: the mean,
//! the range and the sample standard deviation. Over all subgroups: the grand mean X bar, the mean
//! range R bar, and sigma = R bar / (d2 sqrt(n)), the standard deviation of a subgroup mean. The mean
//! chart's limits are X bar +- k sigma; the range chart's are D3 R bar and D4 R bar for k = 3, and
//! R bar (1 +- k d3 / d2) otherwise. d2, d3, D3 and D4 are the standard control chart constants.

/// The control chart constants (d2, d3, D3, D4) for subgroups of 2 to 10: d2 and d3 to six decimals (Origin's
/// limits match d2 = 1.692569 for n = 3, not the rounded 1.693), D3 and D4 the three-decimal table values
/// (Origin's range limit is 2.575 R bar for n = 3).
const CONSTANTS: [(f64, f64, f64, f64); 9] = [
    (std::f64::consts::FRAC_2_SQRT_PI, 0.852502, 0.0, 3.267),
    (1.692569, 0.888368, 0.0, 2.575),
    (2.058751, 0.879808, 0.0, 2.282),
    (2.325929, 0.864082, 0.0, 2.115),
    (2.534413, 0.848040, 0.0, 2.004),
    (2.704357, 0.833205, 0.076, 1.924),
    (2.847201, 0.819831, 0.136, 1.864),
    (2.970026, 0.807834, 0.184, 1.816),
    (3.077505, 0.797051, 0.223, 1.777),
];
/// The smallest and largest subgroup size the chart takes.
pub const MIN_GROUP: usize = 2;
pub const MAX_GROUP: usize = 10;

#[derive(Clone, Debug, PartialEq)]
pub struct XbarR {
    pub means: Vec<f64>,
    pub ranges: Vec<f64>,
    /// The sample standard deviation of each subgroup (NaN for a subgroup of one value).
    pub sds: Vec<f64>,
    pub xbar: f64,
    pub rbar: f64,
    pub sigma: f64,
    pub ucl: f64,
    pub lcl: f64,
    pub r_ucl: f64,
    pub r_lcl: f64,
}

/// The X bar R statistics of the finite `values` in subgroups of `n` with limits at `k` sigma; `None`
/// for a subgroup size outside 2..=10 or without values.
pub fn xbar_r(values: &[f64], n: usize, k: f64) -> Option<XbarR> {
    if !(MIN_GROUP..=MAX_GROUP).contains(&n) {
        return None;
    }
    let v: Vec<f64> = values.iter().copied().filter(|x| x.is_finite()).collect();
    if v.is_empty() {
        return None;
    }
    let (mut means, mut ranges, mut sds) = (Vec::new(), Vec::new(), Vec::new());
    for g in v.chunks(n) {
        let m = g.iter().sum::<f64>() / g.len() as f64;
        let (lo, hi) = g.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), x| (a.min(*x), b.max(*x)));
        let sd = if g.len() > 1 {
            (g.iter().map(|x| (x - m).powi(2)).sum::<f64>() / (g.len() - 1) as f64).sqrt()
        } else {
            f64::NAN
        };
        means.push(m);
        ranges.push(hi - lo);
        sds.push(sd);
    }
    let xbar = means.iter().sum::<f64>() / means.len() as f64;
    let rbar = ranges.iter().sum::<f64>() / ranges.len() as f64;
    let (d2, d3, big_d3, big_d4) = CONSTANTS[n - MIN_GROUP];
    let sigma = rbar / (d2 * (n as f64).sqrt());
    let (r_lcl, r_ucl) = if k == 3.0 {
        (big_d3 * rbar, big_d4 * rbar)
    } else {
        ((rbar * (1.0 - k * d3 / d2)).max(0.0), rbar * (1.0 + k * d3 / d2))
    };
    Some(XbarR { means, ranges, sds, xbar, rbar, sigma, ucl: xbar + k * sigma, lcl: xbar - k * sigma, r_ucl, r_lcl })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn origins_probe_in_groups_of_three() {
        let v = [
            10.2, 9.8, 10.5, 10.1, 9.7, 10.4, 10.0, 9.9, 10.6, 10.3, 9.6, 10.2, 10.8, 10.1, 9.9, 10.0, 10.4, 9.5, 10.2,
            10.3,
        ];
        let q = xbar_r(&v, 3, 3.0).unwrap();
        // Origin 2016: 7 subgroups, the last of 2 values.
        assert_eq!(q.means.len(), 7);
        assert!((q.means[6] - 10.25).abs() < 1e-12 && (q.ranges[6] - 0.1).abs() < 1e-9);
        assert!((q.sds[0] - 0.35119).abs() < 1e-5 && (q.sds[6] - 0.07071).abs() < 1e-5);
        let close = |a: f64, b: f64| (a - b).abs() < 5e-6;
        assert!(close(q.xbar, 10.13095), "{}", q.xbar);
        assert!(close(q.rbar, 0.671429), "{}", q.rbar);
        assert!((q.sigma - 0.2290).abs() < 5e-5, "{}", q.sigma);
        assert!(close(q.ucl, 10.81804) && close(q.lcl, 9.443862), "{} {}", q.ucl, q.lcl);
        assert!(close(q.r_ucl, 1.728929) && q.r_lcl == 0.0, "{}", q.r_ucl);
        assert!(xbar_r(&v, 1, 3.0).is_none() && xbar_r(&v, 11, 3.0).is_none());
    }
}
