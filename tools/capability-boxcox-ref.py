"""Prints crates/calc/tests/fixtures/capability_boxcox_reference.rs: Box-Cox process capability and Cpm for
`sheaf_calc::capability::capability_with` (T419), from numpy and scipy.

- lambda: scipy.stats.boxcox_llf maximized by scipy.optimize.minimize_scalar (bounded to [-5, 5], xatol 1e-13);
  checked against scipy.stats.boxcox_normmax(method="mle") to 1e-5.
- The transform and the normal indices of the transformed values against the transformed limits; d2 integrated
  numerically (scipy.integrate.quad), so values from the within deviation agree to Sheaf's six-decimal d2 table.
- Cpm = (USL - LSL) / 6 sqrt(s^2 + (mean - T)^2) with the overall deviation, on the transformed scale.

Usage: python3 tools/capability-boxcox-ref.py > crates/calc/tests/fixtures/capability_boxcox_reference.rs
None of these numbers come from Origin.
"""
import math

import numpy as np
from scipy import integrate, optimize, stats

M = 2 ** 64


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
        return np.abs(np.diff(x)).mean() / d2(2)
    k = len(x) // n
    g = x[: k * n].reshape(k, n)
    return (g.max(axis=1) - g.min(axis=1)).mean() / d2(n)


def lam_mle(x):
    r = optimize.minimize_scalar(lambda l: -stats.boxcox_llf(l, x), bounds=(-5, 5), method="bounded",
                                 options={"xatol": 1e-13})
    assert abs(r.x - stats.boxcox_normmax(x, method="mle")) < 1e-5
    return r.x


def bc(v, lam):
    return math.log(v) if lam == 0 else math.expm1(lam * math.log(v)) / lam


def nanmin(a, b):
    if math.isnan(a):
        return b
    if math.isnan(b):
        return a
    return min(a, b)


def case(x, lsl, usl, n, lam, target):
    lam = lam_mle(x) if lam is None else lam
    y = np.array([bc(v, lam) for v in x])
    def t(v):
        if v is None or v < 0 or (v == 0 and lam <= 0):
            return None
        return -1.0 / lam if v == 0 else bc(v, lam)

    tl, tu = t(lsl), t(usl)
    m, so, sw = y.mean(), y.std(ddof=1), within(y, n)
    lo = tl if tl is not None else math.nan
    hi = tu if tu is not None else math.nan
    out = [lam, m, sw, so]
    for s in (sw, so):
        l, u = (m - lo) / (3 * s), (hi - m) / (3 * s)
        out += [(hi - lo) / (6 * s), l, u, nanmin(l, u)]
    b = stats.norm.cdf(lo, m, so) * 1e6 if tl is not None else 0.0
    a = stats.norm.sf(hi, m, so) * 1e6 if tu is not None else 0.0
    out += [b, a]
    if target is not None and tl is not None and tu is not None:
        tt = bc(target, lam)
        out.append((hi - lo) / (6 * math.sqrt(so ** 2 + (m - tt) ** 2)))
    else:
        out.append(math.nan)
    return out


skew = np.array([2.0 * (-math.log(u)) ** (1 / 1.7) for u in uniforms(60, 11)])
logn = np.array([math.exp(1.0 + 0.3 * stats.norm.ppf(u)) for u in uniforms(45, 5)])
left = np.array([10.0 - 2.0 * (-math.log(u)) ** 0.8 for u in uniforms(50, 3)])


def vec(v):
    return "&[" + ", ".join(repr(float(a)) for a in v) + "]"


def opt(v):
    return "None" if v is None else f"Some({float(v)!r})"


def num(v):
    return "f64::NAN" if v != v else repr(float(v))


print("// Made by tools/capability-boxcox-ref.py (numpy, scipy). Do not edit.")
print("/// (name, data, lsl, usl, subgroup, lambda given, target, [lambda, mean, sd within, sd overall, Cp, CPL, CPU,")
print("///  Cpk, Pp, PPL, PPU, Ppk, ppm below and above (overall), Cpm]), all on the transformed scale")
print("#[allow(clippy::type_complexity)]")
print("pub const CASES: &[(&str, &[f64], Option<f64>, Option<f64>, usize, Option<f64>, Option<f64>, [f64; 15])] = &[")
for name, x, lsl, usl, n, lam, target in [
    ("skewed right, individuals, target", skew, 0.05, 7.0, 1, None, 2.0),
    ("skewed right, subgroups of 5", skew, 0.2, 6.0, 5, None, None),
    ("skewed right, upper only", skew, None, 5.0, 1, None, None),
    ("lognormal, lambda 0 given, target", logn, 1.0, 6.0, 1, 0.0, 2.7),
    ("lognormal, fitted lambda", logn, 1.0, 6.0, 3, None, 3.0),
    ("skewed left (lambda above 1)", left, 4.0, 11.0, 1, None, 9.0),
    ("lambda 0.5 given, lower limit 0", skew, 0.0, 7.0, 1, 0.5, None),
]:
    r = case(x, lsl, usl, n, lam, target)
    print(f'    ("{name}", {vec(x)}, {opt(lsl)}, {opt(usl)}, {n}, {opt(lam)}, {opt(target)}, [{", ".join(num(v) for v in r)}]),')
print("];")
