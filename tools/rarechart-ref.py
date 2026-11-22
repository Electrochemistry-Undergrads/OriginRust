"""Prints crates/calc/tests/fixtures/rarechart_reference.rs: G and T control chart limits for
`sheaf_calc::rarechart` (T364), from scipy.

- G chart: p = 1/(gbar + 1); sigma limits gbar +- 3 sqrt(gbar(gbar + 1)); probability limits as the continuous
  geometric quantile, whose ceiling is checked against scipy.stats.geom.ppf (support shifted to 0).
- T chart: exponential quantiles from scipy.stats.expon; Weibull maximum likelihood solved with scipy.optimize.brentq
  on the profile equation for the shape, quantiles from scipy.stats.weibull_min.

Usage: python3 tools/rarechart-ref.py > crates/calc/tests/fixtures/rarechart_reference.rs
None of these numbers come from Origin (the G/T chart app is Origin 2027).
"""
import math

import numpy as np
from scipy import optimize, stats

M = 2 ** 64
ALPHA = stats.norm.sf(3.0)


def uniforms(n, seed):
    st = seed
    out = []
    for _ in range(n):
        st = (st * 6364136223846793005 + 1442695040888963407) % M
        out.append(((st >> 11) + 0.5) / 2 ** 53)
    return out


def geometric_counts(n, p, seed):
    return [float(math.floor(math.log(u) / math.log(1 - p))) for u in uniforms(n, seed)]


def weibull_times(n, shape, scale, seed):
    return [scale * (-math.log(u)) ** (1 / shape) for u in uniforms(n, seed)]


def g_ref(x):
    x = np.array(x)
    g = x.mean()
    p = 1 / (g + 1)
    s = math.sqrt(g * (g + 1))
    q = lambda u: max(0.0, math.log1p(-u) / math.log1p(-p) - 1)
    for u in (ALPHA, 0.5, 1 - ALPHA):
        assert math.ceil(q(u) - 1e-9) == max(0, stats.geom.ppf(u, p) - 1), (u, q(u), stats.geom.ppf(u, p))
    return [p, g, g + 3 * s, max(0.0, g - 3 * s), q(0.5), q(1 - ALPHA), q(ALPHA)]


def t_ref(x):
    x = np.array(x)
    m = x.mean()
    e = [m, stats.expon.ppf(0.5, scale=m), stats.expon.ppf(1 - ALPHA, scale=m), stats.expon.ppf(ALPHA, scale=m)]
    lx = np.log(x)

    def profile(k):
        xk = x ** k
        return 1 / k + lx.mean() - (xk * lx).sum() / xk.sum()

    k = optimize.brentq(profile, 1e-3, 100, xtol=1e-15, rtol=1e-15)
    lam = (x ** k).mean() ** (1 / k)
    w = [k, lam] + [stats.weibull_min.ppf(u, k, scale=lam) for u in (0.5, 1 - ALPHA, ALPHA)]
    return e, w


def vec(v):
    return "&[" + ", ".join(repr(float(a)) for a in v) + "]"


print("// Made by tools/rarechart-ref.py (scipy). Do not edit.")
print("/// (name, counts, [p, sigma CL, sigma UCL, sigma LCL, probability CL, UCL, LCL])")
print("pub const G: &[(&str, &[f64], [f64; 7])] = &[")
for name, x in [("geometric p 0.02", geometric_counts(40, 0.02, 3)), ("geometric p 0.2", geometric_counts(25, 0.2, 8))]:
    print(f'    ("{name}", {vec(x)}, [{", ".join(repr(float(v)) for v in g_ref(x))}]),')
print("];")
print("/// (name, times, [mean, CL, UCL, LCL] exponential, [shape, scale, CL, UCL, LCL] Weibull)")
print("pub const T: &[(&str, &[f64], [f64; 4], [f64; 5])] = &[")
for name, x in [("weibull 1.5", weibull_times(30, 1.5, 10.0, 4)), ("weibull 0.8", weibull_times(50, 0.8, 3.0, 12))]:
    e, w = t_ref(x)
    print(f'    ("{name}", {vec(x)}, [{", ".join(repr(float(v)) for v in e)}], [{", ".join(repr(float(v)) for v in w)}]),')
print("];")
