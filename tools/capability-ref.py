"""Prints crates/calc/tests/fixtures/capability_reference.rs: process capability indices and parts per million for
`sheaf_calc::capability` (T365), from numpy and scipy.

- d2(n) is integrated numerically (scipy.integrate.quad of 1 - Phi(x)^n - (1 - Phi(x))^n), independent of Sheaf's
  six-decimal table.
- Lognormal: maximum likelihood of the logs (divisor n); Weibull: the shape solved with brentq on the profile
  likelihood equation. Quantiles and tails from scipy.stats.

Usage: python3 tools/capability-ref.py > crates/calc/tests/fixtures/capability_reference.rs
None of these numbers come from Origin.
"""
import math

import numpy as np
from scipy import integrate, optimize, stats

M = 2 ** 64
A = stats.norm.sf(3.0)


def uniforms(n, seed):
    st = seed
    out = []
    for _ in range(n):
        st = (st * 6364136223846793005 + 1442695040888963407) % M
        out.append(((st >> 11) + 0.5) / 2 ** 53)
    return out


def d2(n):
    f = lambda x: 1 - stats.norm.cdf(x) ** n - stats.norm.sf(x) ** n
    return integrate.quad(f, -np.inf, np.inf, epsabs=1e-13, epsrel=1e-13)[0]


def within(x, n):
    if n == 1:
        r = np.abs(np.diff(x))
        return r.mean() / d2(2)
    k = len(x) // n
    g = x[: k * n].reshape(k, n)
    return (g.max(axis=1) - g.min(axis=1)).mean() / d2(n)


def nanmin(a, b):
    if math.isnan(a):
        return b
    if math.isnan(b):
        return a
    return min(a, b)


def normal_ref(x, lsl, usl, n):
    m, so, sw = x.mean(), x.std(ddof=1), within(x, n)
    lo = lsl if lsl is not None else math.nan
    hi = usl if usl is not None else math.nan
    out = []
    for s in (sw, so):
        l, u = (m - lo) / (3 * s), (hi - m) / (3 * s)
        out += [(hi - lo) / (6 * s), l, u, nanmin(l, u)]
    for s in (so, sw):
        b = stats.norm.cdf(lo, m, s) * 1e6 if lsl is not None else 0.0
        a = stats.norm.sf(hi, m, s) * 1e6 if usl is not None else 0.0
        out += [b, a]
    return [sw, so] + out


def fit_weibull(x):
    lx = np.log(x)

    def profile(k):
        xk = x ** k
        return 1 / k + lx.mean() - (xk * lx).sum() / xk.sum()

    k = optimize.brentq(profile, 1e-3, 100, xtol=1e-15, rtol=1e-15)
    return k, (x ** k).mean() ** (1 / k)


def nonnormal_ref(x, lsl, usl, model):
    if model == "lognormal":
        lx = np.log(x)
        mu, s = lx.mean(), lx.std(ddof=0)
        d = stats.lognorm(s, scale=math.exp(mu))
        params = [mu, s]
    else:
        k, lam = fit_weibull(x)
        d = stats.weibull_min(k, scale=lam)
        params = [k, lam]
    lo = lsl if lsl is not None else math.nan
    hi = usl if usl is not None else math.nan
    xl, xm, xh = d.ppf(A), d.ppf(0.5), d.ppf(1 - A)
    ppl, ppu = (xm - lo) / (xm - xl), (hi - xm) / (xh - xm)
    b = d.cdf(lo) * 1e6 if lsl is not None else 0.0
    a = d.sf(hi) * 1e6 if usl is not None else 0.0
    return params + [(hi - lo) / (xh - xl), ppl, ppu, nanmin(ppl, ppu), b, a]


norm_data = np.array([10 + 0.4 * stats.norm.ppf(u) for u in uniforms(50, 7)])
skew = np.array([2.0 * (-math.log(u)) ** (1 / 1.7) for u in uniforms(60, 11)])
logn = np.array([math.exp(1.0 + 0.3 * stats.norm.ppf(u)) for u in uniforms(45, 5)])


def vec(v):
    return "&[" + ", ".join(repr(float(a)) for a in v) + "]"


def opt(v):
    return "None" if v is None else f"Some({float(v)!r})"


print("// Made by tools/capability-ref.py (numpy, scipy). Do not edit.")
print("/// (name, data, lsl, usl, subgroup, [sd within, sd overall, Cp, CPL, CPU, Cpk, Pp, PPL, PPU, Ppk,")
print("///  ppm below and above (overall), ppm below and above (within)])")
print("pub const NORMAL: &[(&str, &[f64], Option<f64>, Option<f64>, usize, [f64; 14])] = &[")
for name, x, lsl, usl, n in [
    ("individuals", norm_data, 9.0, 11.2, 1),
    ("subgroups of 5", norm_data, 9.0, 11.2, 5),
    ("upper only, groups of 3", norm_data[:49], None, 10.8, 3),
]:
    r = normal_ref(x, lsl, usl, n)
    print(f'    ("{name}", {vec(x)}, {opt(lsl)}, {opt(usl)}, {n}, [{", ".join(('f64::NAN' if v != v else repr(float(v))) for v in r)}]),')
print("];")
print("/// (name, data, lsl, usl, weibull?, [param 1, param 2, Pp, PPL, PPU, Ppk, ppm below, ppm above])")
print("pub const NONNORMAL: &[(&str, &[f64], Option<f64>, Option<f64>, bool, [f64; 8])] = &[")
for name, x, lsl, usl, model in [
    ("weibull", skew, 0.05, 7.0, "weibull"),
    ("weibull upper only", skew, None, 5.0, "weibull"),
    ("lognormal", logn, 1.0, 6.0, "lognormal"),
]:
    r = nonnormal_ref(x, lsl, usl, model)
    w = "true" if model == "weibull" else "false"
    print(f'    ("{name}", {vec(x)}, {opt(lsl)}, {opt(usl)}, {w}, [{", ".join(('f64::NAN' if v != v else repr(float(v))) for v in r)}]),')
print("];")
