"""Prints crates/calc/tests/fixtures/scipy_stats_dist.rs: reference numbers from scipy for the
distribution fit and the probability plot of `sheaf-calc`.

Usage: python3 tools/stats-dist-ref.py > crates/calc/tests/fixtures/scipy_stats_dist.rs
Needs numpy and scipy (the numbers in the repository were made with scipy 1.18.1). All inputs are
generated here from fixed seeds; none of the numbers come from Origin.

What is taken from scipy, and how:
  fit:       scipy.stats.{norm,lognorm,expon,gamma,weibull_min}.fit(x, floc=0) for the parameters,
             .logpdf for the log-likelihood, scipy.stats.kstest for D and its p-value, and
             the Anderson-Darling sum evaluated with scipy's logcdf / logsf of the fitted distribution
             (scipy.stats.anderson does not take a gamma or lognormal distribution and estimates the normal
             standard deviation with n - 1).
             The shapes of gamma and Weibull are also solved with scipy.optimize.brentq on the likelihood
             equation (xtol 1e-15), because scipy's own fit stops about 1e-8 from the maximum.
  probplot:  scipy.stats.probplot(x, sparams=..., dist=...) : quantiles, ordered data, slope, intercept, r.
"""
import numpy as np
import scipy
from scipy import optimize, special, stats

FAMILIES = ["norm", "lognorm", "expon", "gamma", "weibull_min"]


def f(v):
    v = float(v)
    if v != v:
        return "f64::NAN"
    if v in (float("inf"), float("-inf")):
        return "f64::INFINITY" if v > 0 else "f64::NEG_INFINITY"
    return repr(v)


def arr(a):
    return "&[" + ", ".join(f(x) for x in a) + "]"


def sample(kind, n, seed):
    r = np.random.RandomState(seed)
    if kind == "norm":
        x = r.normal(10.0, 2.5, n)
    elif kind == "lognorm":
        x = r.lognormal(1.2, 0.6, n)
    elif kind == "expon":
        x = r.exponential(3.0, n)
    elif kind == "gamma":
        x = r.gamma(2.5, 1.8, n)
    else:
        x = r.weibull(1.7, n) * 4.0
    return np.round(x, 4)


def fit_scipy(fam, x):
    """(params in the order of sheaf-calc, frozen scipy distribution)."""
    if fam == "norm":
        m, s = stats.norm.fit(x)
        return [m, s], stats.norm(m, s)
    if fam == "lognorm":
        s, loc, scale = stats.lognorm.fit(x, floc=0)
        return [float(np.log(scale)), s], stats.lognorm(s, 0, scale)
    if fam == "expon":
        loc, scale = stats.expon.fit(x, floc=0)
        return [scale], stats.expon(0, scale)
    if fam == "gamma":
        a, loc, scale = stats.gamma.fit(x, floc=0)
        return [a, scale], stats.gamma(a, 0, scale)
    c, loc, scale = stats.weibull_min.fit(x, floc=0)
    return [c, scale], stats.weibull_min(c, 0, scale)


def refined(fam, x, params):
    """The same estimates with the shape solved to machine precision (None for the closed forms)."""
    n = len(x)
    if fam == "gamma":
        s = np.log(x.mean()) - np.log(x).mean()
        k = optimize.brentq(lambda k: np.log(k) - special.digamma(k) - s, 1e-3, 1e4, xtol=1e-15, rtol=1e-15)
        return [k, x.mean() / k]
    if fam == "weibull_min":
        xm = x.max()
        l = np.log(x / xm)

        def g(c):
            w = np.exp(c * l)
            return (w * l).sum() / w.sum() - 1 / c - l.mean()

        c = optimize.brentq(g, 1e-3, 1e3, xtol=1e-15, rtol=1e-15)
        scale = xm * (np.exp(c * l).mean()) ** (1 / c)
        return [c, scale]
    return params


def ad_stat(dist, x):
    xs = np.sort(x)
    n = len(xs)
    i = np.arange(1, n + 1)
    return -n - np.sum((2 * i - 1) * (dist.logcdf(xs) + dist.logsf(xs[::-1]))) / n


def emit_fit():
    out = []
    cases = []
    seed = 100
    for kind, n in [("norm", 12), ("norm", 60), ("lognorm", 30), ("lognorm", 100), ("expon", 25),
                    ("expon", 80), ("gamma", 40), ("gamma", 12), ("weibull_min", 50), ("weibull_min", 15)]:
        seed += 1
        x = sample(kind, n, seed)
        # every family that can take the data is fitted to it
        for fam in FAMILIES:
            if fam != "norm" and (x <= 0).any():
                continue
            if fam == kind or fam == "norm" or (kind, fam) in [("lognorm", "gamma"), ("gamma", "weibull_min"), ("weibull_min", "gamma"), ("expon", "weibull_min")]:
                cases.append((f"{kind}{n}", fam, x))
    for name, fam, x in cases:
        params, dist = fit_scipy(fam, x)
        ll = float(dist.logpdf(x).sum())
        k = len(params)
        ks = stats.kstest(x, dist.cdf)
        ad = ad_stat(dist, x)
        ref = refined(fam, x, params)
        if fam in ("gamma", "weibull_min"):
            d2 = dict(gamma=lambda p: stats.gamma(p[0], 0, p[1]), weibull_min=lambda p: stats.weibull_min(p[0], 0, p[1]))[fam](ref)
            ll_ref = float(d2.logpdf(x).sum())
            ks2 = stats.kstest(x, d2.cdf)
            ad_ref = ad_stat(d2, x)
        else:
            ll_ref, ks2, ad_ref = ll, ks, ad
        n_ = len(x)
        code = FAMILIES.index(fam)
        out.append(
            f"    FitCase {{ name: \"{name}\", family: {code}, data: {arr(x)}, params_scipy: {arr(params)}, "
            f"params: {arr(ref)}, log_likelihood: {f(ll_ref)}, aic: {f(2 * k - 2 * ll_ref)}, "
            f"bic: {f(k * np.log(n_) - 2 * ll_ref)}, ks_d: {f(ks2.statistic)}, ks_p: {f(ks2.pvalue)}, ad: {f(ad_ref)} }},"
        )
    return "pub const FITS: &[FitCase] = &[\n" + "\n".join(out) + "\n];\n"


def emit_ks():
    """P(D_n >= d) from scipy.stats.kstwo.sf, for samples above 140 (below it the matrix method is the same
    algorithm as scipy's and the fit fixtures cover it)."""
    rows = []
    for n in [141, 200, 300, 500, 1000, 1001, 1200, 1500, 1999, 2000, 10000]:
        for lam in [0.4, 0.67, 0.8, 1.0, 1.3, 1.7, 2.1, 2.5, 2.9, 3.1, 3.5]:
            d = round(lam / np.sqrt(n), 6)
            rows.append(f"    KsCase {{ n: {n}, d: {f(d)}, p: {f(stats.kstwo.sf(d, n))} }},")
    return "pub const KS_P: &[KsCase] = &[\n" + "\n".join(rows) + "\n];\n"


def emit_probplot():
    out = []
    seed = 500
    for kind, n in [("norm", 15), ("norm", 50), ("lognorm", 20), ("expon", 30), ("gamma", 25), ("weibull_min", 40)]:
        seed += 1
        x = sample(kind, n, seed)
        params, dist = fit_scipy(kind, x)
        if kind == "norm" or kind == "expon":
            sp, shape = (), None
            d = {"norm": stats.norm, "expon": stats.expon}[kind]
        elif kind == "lognorm":
            shape = params[1]
            sp, d = (shape,), stats.lognorm
        elif kind == "gamma":
            shape = params[0]
            sp, d = (shape,), stats.gamma
        else:
            shape = params[0]
            sp, d = (shape,), stats.weibull_min
        (osm, osr), (slope, intercept, r) = stats.probplot(x, sparams=sp, dist=d, fit=True)
        code = FAMILIES.index(kind)
        out.append(
            f"    ProbCase {{ name: \"{kind}{n}\", family: {code}, data: {arr(x)}, shape: {f(shape) if shape is not None else 'f64::NAN'}, "
            f"theoretical: {arr(osm)}, ordered: {arr(osr)}, slope: {f(slope)}, intercept: {f(intercept)}, r: {f(r)} }},"
        )
    return "pub const PROBPLOTS: &[ProbCase] = &[\n" + "\n".join(out) + "\n];\n"


HEADER = '''//! Numbers from scipy {ver} for the distribution fit and the probability plot (made by
//! tools/stats-dist-ref.py). Do not edit by hand. These numbers come from scipy, not from Origin.
#![allow(clippy::excessive_precision, dead_code)]

/// `family` is the index into norm, lognorm, expon, gamma, weibull_min. `params_scipy` is what scipy.stats.<dist>.fit
/// returned (loc fixed at 0); `params` is the same estimate with the shape solved to machine precision.
pub struct FitCase {{
    pub name: &'static str,
    pub family: usize,
    pub data: &'static [f64],
    pub params_scipy: &'static [f64],
    pub params: &'static [f64],
    pub log_likelihood: f64,
    pub aic: f64,
    pub bic: f64,
    pub ks_d: f64,
    pub ks_p: f64,
    pub ad: f64,
}}
pub struct ProbCase {{
    pub name: &'static str,
    pub family: usize,
    pub data: &'static [f64],
    /// The shape passed to scipy.stats.probplot (NaN for the families without one).
    pub shape: f64,
    pub theoretical: &'static [f64],
    pub ordered: &'static [f64],
    pub slope: f64,
    pub intercept: f64,
    pub r: f64,
}}
/// `p` is P(D_n >= d) of the two-sided Kolmogorov statistic, `scipy.stats.kstwo.sf(d, n)`.
pub struct KsCase {{
    pub n: usize,
    pub d: f64,
    pub p: f64,
}}

'''


if __name__ == "__main__":
    print(HEADER.format(ver=scipy.__version__), end="")
    print(emit_fit())
    print(emit_ks())
    print(emit_probplot())
