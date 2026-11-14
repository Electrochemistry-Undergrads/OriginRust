"""Prints crates/calc/tests/fixtures/scipy_stats_dist.rs: reference numbers from scipy for the
distribution fit, the probability plot and the power calculations of `sheaf-calc`.

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
  nct, ncf:  scipy.stats.nct.cdf / .sf and scipy.stats.ncf.cdf / .sf.
  power:     scipy.stats.t.ppf, nct.sf / cdf, f.ppf, ncf.sf; sample sizes by counting up from n = 2.
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
    for n in [141, 200, 300, 500, 1000, 2000, 10000]:
        for lam in [0.4, 0.8, 1.0, 1.3, 1.7, 2.1, 2.5, 2.9, 3.1, 3.5]:
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


def consistent(cdf, sf):
    """scipy's nct returns nan, or a cdf and sf that do not add up to 1 within 1e-8, for some arguments (negative t
    with a large ncp); those are left out (nan: the test skips the number)."""
    if cdf != cdf or sf != sf or abs(cdf + sf - 1) > 1e-8:
        return float("nan"), float("nan")
    return cdf, sf


def emit_noncentral():
    nct = []
    for df in [1.0, 2.5, 5.0, 10.0, 30.0, 100.0, 1000.0]:
        for ncp in [-3.0, -0.5, 0.0, 0.3, 1.0, 2.5, 5.0, 10.0, 25.0, 35.0]:
            for t in [-4.0, -1.0, 0.0, 0.5, 2.0, 3.5, 7.0, 15.0, 40.0]:
                cdf, sf = consistent(stats.nct.cdf(t, df, ncp), stats.nct.sf(t, df, ncp))
                nct.append(f"    NctCase {{ t: {f(t)}, df: {f(df)}, ncp: {f(ncp)}, cdf: {f(cdf)}, sf: {f(sf)} }},")
    # |ncp| above 37.62: the Poisson mixture summed from its largest weight (the series of AS 243 would underflow)
    for df in [1.0, 2.0, 3.0, 10.0, 30.0, 200.0]:
        for ncp in [-60.0, 38.0, 40.0, 60.0, 100.0, 300.0]:
            for t in [-4.0, 0.5, 0.9 * ncp, ncp, 1.1 * ncp, 1.5 * ncp, 3.0 * ncp]:
                cdf, sf = consistent(stats.nct.cdf(t, df, ncp), stats.nct.sf(t, df, ncp))
                nct.append(f"    NctCase {{ t: {f(t)}, df: {f(df)}, ncp: {f(ncp)}, cdf: {f(cdf)}, sf: {f(sf)} }},")
    ncf = []
    for d1, d2 in [(1.0, 5.0), (2.0, 10.0), (3.0, 27.0), (5.0, 100.0), (4.0, 1000.0), (10.0, 20.0)]:
        for ncp in [0.0, 0.5, 2.0, 8.0, 20.0, 60.0, 200.0]:
            for x in [0.1, 0.7, 1.5, 3.0, 6.0, 12.0, 40.0]:
                if ncp == 0:  # scipy's ncf.sf is wrong at ncp = 0; the central F distribution is the same thing
                    cdf, sf = stats.f.cdf(x, d1, d2), stats.f.sf(x, d1, d2)
                else:
                    cdf, sf = consistent(stats.ncf.cdf(x, d1, d2, ncp), stats.ncf.sf(x, d1, d2, ncp))
                ncf.append(f"    NcfCase {{ x: {f(x)}, df1: {f(d1)}, df2: {f(d2)}, ncp: {f(ncp)}, cdf: {f(cdf)}, sf: {f(sf)} }},")
    return ("pub const NCT: &[NctCase] = &[\n" + "\n".join(nct) + "\n];\n\n"
            + "pub const NCF: &[NcfCase] = &[\n" + "\n".join(ncf) + "\n];\n")


def t_power(design, d, n1, alpha, tail, ratio=1.0):
    if design == "two":
        n2 = max(2, int(np.ceil(n1 * ratio)))
        df = n1 + n2 - 2
        ncp = d / np.sqrt(1 / n1 + 1 / n2)
    else:
        n2 = n1
        df = n1 - 1
        ncp = d * np.sqrt(n1)
    if tail == 0:
        c = stats.t.ppf(1 - alpha / 2, df)
        lower = stats.nct.cdf(-c, df, ncp)
        if lower != lower:  # scipy returns nan for some negative arguments; the mirrored tail is the same number
            lower = stats.nct.sf(c, df, -ncp)
        p = stats.nct.sf(c, df, ncp) + lower
    else:
        c = stats.t.ppf(1 - alpha, df)
        p = stats.nct.sf(c, df, ncp)
    return n2, df, ncp, c, p


def emit_power():
    rows = []
    for design, code in [("one", 0), ("paired", 1), ("two", 2)]:
        for d in [0.2, 0.5, 0.8, 1.3]:
            for alpha in [0.05, 0.01]:
                for tail in [0, 1]:
                    ratio = 2.0 if (design == "two" and d == 0.8) else 1.0
                    for n1 in [5, 12, 30, 64]:
                        n2, df, ncp, c, p = t_power(design, d, n1, alpha, tail, ratio)
                        rows.append(f"    TPowerCase {{ design: {code}, ratio: {f(ratio)}, effect: {f(d)}, n1: {n1}, alpha: {f(alpha)}, tail: {tail}, n2: {n2}, df: {f(df)}, ncp: {f(ncp)}, critical: {f(c)}, power: {f(p)} }},")
    # large effects with few observations: the non-centrality passes 37.62 at one or two degrees of freedom
    for design, code in [("one", 0), ("paired", 1), ("two", 2)]:
        for d in [30.0, 60.0]:
            for alpha in [0.05, 0.001]:
                for tail in [0, 1]:
                    for n1 in [2, 3, 5, 12]:
                        n2, df, ncp, c, p = t_power(design, d, n1, alpha, tail)
                        rows.append(f"    TPowerCase {{ design: {code}, ratio: 1.0, effect: {f(d)}, n1: {n1}, alpha: {f(alpha)}, tail: {tail}, n2: {n2}, df: {f(df)}, ncp: {f(ncp)}, critical: {f(c)}, power: {f(p)} }},")
    sizes = []
    # two samples with unequal group sizes: the size of the first group is counted up from 2
    for ratio in [0.5, 2.0, 3.0]:
        for d in [0.3, 0.6, 1.0]:
            for target in [0.8, 0.9]:
                for tail in [0, 1]:
                    n = 2
                    while t_power("two", d, n, 0.05, tail, ratio)[4] < target:
                        n += 1
                    n2, df, ncp, c, p = t_power("two", d, n, 0.05, tail, ratio)
                    sizes.append(f"    TSizeCase {{ design: 2, ratio: {f(ratio)}, effect: {f(d)}, target: {f(target)}, alpha: 0.05, tail: {tail}, n: {n}, n2: {n2}, power: {f(p)} }},")
    for design, code in [("one", 0), ("paired", 1), ("two", 2)]:
        for d in [0.3, 0.5, 0.8, 1.5]:
            for target in [0.8, 0.9, 0.95]:
                for tail in [0, 1]:
                    n = 2
                    while t_power(design, d, n, 0.05, tail)[4] < target:
                        n += 1
                    n2, df, ncp, c, p = t_power(design, d, n, 0.05, tail)
                    sizes.append(f"    TSizeCase {{ design: {code}, ratio: 1.0, effect: {f(d)}, target: {f(target)}, alpha: 0.05, tail: {tail}, n: {n}, n2: {n2}, power: {f(p)} }},")
    an = []
    for k in [2, 3, 4, 6]:
        for fe in [0.1, 0.25, 0.4, 0.6]:
            for alpha in [0.05, 0.01]:
                for n in [4, 10, 25, 60]:
                    N = k * n
                    df1, df2, ncp = k - 1, N - k, fe * fe * N
                    c = stats.f.ppf(1 - alpha, df1, df2)
                    p = stats.ncf.sf(c, df1, df2, ncp)
                    an.append(f"    AnovaPowerCase {{ groups: {k}, effect: {f(fe)}, n: {n}, alpha: {f(alpha)}, df1: {f(df1)}, df2: {f(df2)}, ncp: {f(ncp)}, critical: {f(c)}, power: {f(p)} }},")
    ans = []
    for k in [2, 3, 5]:
        for fe in [0.15, 0.25, 0.4]:
            for target in [0.8, 0.9]:
                n = 2
                while True:
                    N = k * n
                    c = stats.f.ppf(0.95, k - 1, N - k)
                    p = stats.ncf.sf(c, k - 1, N - k, fe * fe * N)
                    if p >= target:
                        break
                    n += 1
                ans.append(f"    AnovaSizeCase {{ groups: {k}, effect: {f(fe)}, target: {f(target)}, alpha: 0.05, n: {n}, power: {f(p)} }},")
    return ("pub const T_POWER: &[TPowerCase] = &[\n" + "\n".join(rows) + "\n];\n\n"
            + "pub const T_SIZE: &[TSizeCase] = &[\n" + "\n".join(sizes) + "\n];\n\n"
            + "pub const ANOVA_POWER: &[AnovaPowerCase] = &[\n" + "\n".join(an) + "\n];\n\n"
            + "pub const ANOVA_SIZE: &[AnovaSizeCase] = &[\n" + "\n".join(ans) + "\n];\n")


HEADER = '''//! Numbers from scipy {ver} for the distribution fit, the probability plot, the noncentral t and F distributions
//! and the power calculations (made by tools/stats-dist-ref.py). Do not edit by hand. These numbers come from
//! scipy, not from Origin.
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
pub struct NctCase {{
    pub t: f64,
    pub df: f64,
    pub ncp: f64,
    pub cdf: f64,
    pub sf: f64,
}}
pub struct NcfCase {{
    pub x: f64,
    pub df1: f64,
    pub df2: f64,
    pub ncp: f64,
    pub cdf: f64,
    pub sf: f64,
}}
/// design: 0 one sample, 1 paired, 2 two samples (`ratio` = n2 / n1); tail: 0 two-sided, 1 upper.
pub struct TPowerCase {{
    pub design: u8,
    pub ratio: f64,
    pub effect: f64,
    pub n1: usize,
    pub alpha: f64,
    pub tail: u8,
    pub n2: usize,
    pub df: f64,
    pub ncp: f64,
    pub critical: f64,
    pub power: f64,
}}
pub struct TSizeCase {{
    pub design: u8,
    pub ratio: f64,
    pub effect: f64,
    pub target: f64,
    pub alpha: f64,
    pub tail: u8,
    pub n: usize,
    pub n2: usize,
    pub power: f64,
}}
pub struct AnovaPowerCase {{
    pub groups: usize,
    pub effect: f64,
    pub n: usize,
    pub alpha: f64,
    pub df1: f64,
    pub df2: f64,
    pub ncp: f64,
    pub critical: f64,
    pub power: f64,
}}
pub struct AnovaSizeCase {{
    pub groups: usize,
    pub effect: f64,
    pub target: f64,
    pub alpha: f64,
    pub n: usize,
    pub power: f64,
}}

'''


if __name__ == "__main__":
    print(HEADER.format(ver=scipy.__version__), end="")
    print(emit_fit())
    print(emit_ks())
    print(emit_probplot())
    print(emit_noncentral())
    print(emit_power())
