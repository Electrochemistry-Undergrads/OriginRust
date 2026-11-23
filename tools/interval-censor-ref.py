"""Prints crates/calc/tests/fixtures/interval_censor_reference.rs: maximum likelihood with exact, right-, left- and
interval-censored rows for `sheaf_calc::lifedata::fit_arbitrary` (T384), from scipy.

The log-likelihood is written with scipy.stats logpdf / logsf / logcdf and differences of cdf/sf; it is maximized by
scipy.optimize.minimize (Nelder-Mead, then BFGS, then Nelder-Mead again) on (ln shape, ln scale), (mu, ln sigma) or
ln theta from several starts. Each fit is also compared loosely (1e-3) with scipy.stats' own fit(CensoredData).

Usage: python3 tools/interval-censor-ref.py > crates/calc/tests/fixtures/interval_censor_reference.rs
None of these numbers come from Origin.
"""
import math

import numpy as np
from scipy import optimize, stats

M = 2 ** 64


def uniforms(n, seed):
    st = seed
    out = []
    for _ in range(n):
        st = (st * 6364136223846793005 + 1442695040888963407) % M
        out.append(((st >> 11) + 0.5) / 2 ** 53)
    return out


def sample(n, shape, scale, seed):
    """Weibull lives inspected at times 10, 20, ...: interval-censored between inspections, some exact, some
    left-censored (failed before the first inspection), some right-censored (still working at the last)."""
    u = uniforms(2 * n, seed)
    rows = []
    for i in range(n):
        life = scale * (-math.log(u[2 * i])) ** (1 / shape)
        kind = u[2 * i + 1]
        if life > 60:
            rows.append((60.0, math.nan))
        elif kind < 0.25:
            rows.append((round(life, 4), round(life, 4)))
        elif life < 10:
            rows.append((math.nan, 10.0))
        else:
            k = math.floor(life / 10)
            rows.append((10.0 * k, 10.0 * (k + 1)))
    return rows


def dist_of(name, p):
    if name == "weibull":
        return stats.weibull_min(p[0], scale=p[1])
    if name == "lognormal":
        return stats.lognorm(p[1], scale=math.exp(p[0]))
    if name == "exponential":
        return stats.expon(scale=p[0])
    return stats.norm(p[0], p[1])


def split(rows):
    r = np.array(rows, dtype=float)
    a, b = r[:, 0], r[:, 1]
    exact = ~np.isnan(a) & ~np.isnan(b) & (a == b)
    right = np.isnan(b)
    left = np.isnan(a)
    inter = ~np.isnan(a) & ~np.isnan(b) & (a < b)
    return a, b, exact, right, left, inter


def loglik(name, p, rows):
    a, b, exact, right, left, inter = rows
    d = dist_of(name, p)
    s = d.logpdf(a[exact]).sum() + d.logsf(a[right]).sum() + d.logcdf(b[left]).sum()
    ai, bi = a[inter], b[inter]
    up = d.cdf(ai) > 0.5
    pr = np.where(up, d.sf(ai) - d.sf(bi), d.cdf(bi) - d.cdf(ai))
    return float(s + np.log(pr).sum())


def free(name, p):
    if name == "weibull":
        return [math.log(p[0]), math.log(p[1])]
    if name == "exponential":
        return [math.log(p[0])]
    return [p[0], math.log(p[1])]


def unfree(name, z):
    if name == "weibull":
        return [math.exp(z[0]), math.exp(z[1])]
    if name == "exponential":
        return [math.exp(z[0])]
    return [z[0], math.exp(z[1])]


def fit(name, rows, starts):
    rows = split(rows)
    f = lambda z: -loglik(name, unfree(name, z), rows) if all(np.isfinite(z)) else np.inf
    best = None
    for st in starts:
        z0 = free(name, st)
        r = optimize.minimize(f, z0, method="Nelder-Mead", options={"xatol": 1e-12, "fatol": 1e-14, "maxiter": 50000, "maxfev": 50000})
        r = optimize.minimize(f, r.x, method="BFGS", options={"gtol": 1e-11})
        r = optimize.minimize(f, r.x, method="Nelder-Mead", options={"xatol": 1e-13, "fatol": 1e-15, "maxiter": 50000, "maxfev": 50000})
        if best is None or r.fun < best.fun:
            best = r
    return unfree(name, best.x), -best.fun


def scipy_check(name, rows, p):
    unc = [a for a, b in rows if not math.isnan(a) and not math.isnan(b) and a == b]
    right = [a for a, b in rows if math.isnan(b)]
    left = [b for a, b in rows if math.isnan(a)]
    inter = [(a, b) for a, b in rows if not math.isnan(a) and not math.isnan(b) and a < b]
    cd = stats.CensoredData(uncensored=unc, left=left, right=right, interval=inter)
    if name == "weibull":
        c, _, sc = stats.weibull_min.fit(cd, floc=0)
        q = [c, sc]
    elif name == "lognormal":
        s, _, sc = stats.lognorm.fit(cd, floc=0)
        q = [math.log(sc), s]
    elif name == "exponential":
        _, sc = stats.expon.fit(cd, floc=0)
        q = [sc]
    else:
        q = list(stats.norm.fit(cd))
    for a, b in zip(p, q):
        assert abs(a - b) <= 1e-3 * max(abs(a), 1.0), (name, p, q)


def num(v):
    return "f64::NAN" if math.isnan(v) else repr(float(v))


samples = [("weibull 1.8, scale 30", sample(50, 1.8, 30.0, 4)), ("weibull 1.1, scale 45", sample(40, 1.1, 45.0, 9))]
starts = {
    "weibull": [[1.0, 30.0], [2.0, 40.0], [0.7, 20.0]],
    "lognormal": [[3.0, 0.8], [3.5, 0.5], [2.5, 1.2]],
    "exponential": [[30.0], [50.0]],
    "normal": [[30.0, 15.0], [25.0, 10.0], [40.0, 25.0]],
}
print("// Made by tools/interval-censor-ref.py (scipy). Do not edit.")
print("pub struct Sample {")
print("    pub name: &'static str,")
print("    /// (start, end): equal = exact, end NaN = right-censored, start NaN = left-censored, else an interval")
print("    pub rows: &'static [(f64, f64)],")
print("    /// (distribution, parameters, log-likelihood)")
print("    pub fits: &'static [(&'static str, &'static [f64], f64)],")
print("}")
print("pub const SAMPLES: &[Sample] = &[")
for name, rows in samples:
    print("    Sample {")
    print(f'        name: "{name}",')
    print("        rows: &[" + ", ".join(f"({num(a)}, {num(b)})" for a, b in rows) + "],")
    print("        fits: &[")
    for dname in ("weibull", "lognormal", "exponential", "normal"):
        p, ll = fit(dname, rows, starts[dname])
        scipy_check(dname, rows, p)
        print(f'            ("{dname}", &[{", ".join(repr(float(v)) for v in p)}], {float(ll)!r}),')
    print("        ],")
    print("    },")
print("];")
