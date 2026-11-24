"""Prints crates/calc/tests/fixtures/capability_ci_reference.rs: confidence intervals of Pp and Ppk for
`sheaf_calc::capability::capability_with` (T425), from numpy and scipy.

Pp = (USL - LSL) / 6 s and Ppk = min(mean - LSL, USL - mean) / 3 s with the sample standard deviation s; Pp's interval
from scipy.stats.chi2 (n - 1 degrees of freedom), Ppk's by Bissell's approximation with scipy.stats.norm. The Box-Cox
case transforms values and limits with a given lambda first. The Pp interval is also checked here by simulating
normal samples (its coverage within 0.01 of the level).

Usage: python3 tools/capability-ci-ref.py > crates/calc/tests/fixtures/capability_ci_reference.rs
None of these numbers come from Origin.
"""
import math

import numpy as np
from scipy import stats

M = 2 ** 64


def uniforms(n, seed):
    st = seed
    out = []
    for _ in range(n):
        st = (st * 6364136223846793005 + 1442695040888963407) % M
        out.append(((st >> 11) + 0.5) / 2 ** 53)
    return out


def bc(v, lam):
    return math.log(v) if lam == 0 else math.expm1(lam * math.log(v)) / lam


def ci(x, lsl, usl, level, lam):
    if lam is not None:
        x = np.array([bc(v, lam) for v in x])
        lsl = None if lsl is None else bc(lsl, lam)
        usl = None if usl is None else bc(usl, lam)
    n = len(x)
    m, s = x.mean(), x.std(ddof=1)
    a = 1 - level
    pp = (usl - lsl) / (6 * s) if lsl is not None and usl is not None else math.nan
    sides = [v for v in ((m - lsl) / (3 * s) if lsl is not None else None,
                         (usl - m) / (3 * s) if usl is not None else None) if v is not None]
    ppk = min(sides)
    lo = pp * math.sqrt(stats.chi2.ppf(a / 2, n - 1) / (n - 1))
    hi = pp * math.sqrt(stats.chi2.ppf(1 - a / 2, n - 1) / (n - 1))
    half = stats.norm.ppf(1 - a / 2) * math.sqrt(1 / (9 * n) + ppk ** 2 / (2 * (n - 1)))
    return [pp, lo, hi, ppk, ppk - half, ppk + half]


# The Pp interval covers the true Pp at its level (simulation).
rng = np.random.default_rng(425)
for n in (10, 40):
    xs = rng.normal(0, 1, (100_000, n))
    s = xs.std(axis=1, ddof=1)
    pp = 6 / (6 * s)  # limits -3 and 3, true Pp 1
    c = stats.chi2.ppf([0.025, 0.975], n - 1)
    cover = ((pp * np.sqrt(c[0] / (n - 1)) <= 1) & (1 <= pp * np.sqrt(c[1] / (n - 1)))).mean()
    assert abs(cover - 0.95) < 0.01, cover

norm_data = np.array([10 + 0.4 * stats.norm.ppf(u) for u in uniforms(50, 7)])
skew = np.array([2.0 * (-math.log(u)) ** (1 / 1.7) for u in uniforms(60, 11)])


def vec(v):
    return "&[" + ", ".join(repr(float(a)) for a in v) + "]"


def opt(v):
    return "None" if v is None else f"Some({float(v)!r})"


def num(v):
    return "f64::NAN" if v != v else repr(float(v))


print("// Made by tools/capability-ci-ref.py (numpy, scipy). Do not edit.")
print("/// (name, data, lsl, usl, level, Box-Cox lambda, [Pp, Pp lower, Pp upper, Ppk, Ppk lower, Ppk upper])")
print("#[allow(clippy::type_complexity)]")
print("pub const CASES: &[(&str, &[f64], Option<f64>, Option<f64>, f64, Option<f64>, [f64; 6])] = &[")
for name, x, lsl, usl, level, lam in [
    ("normal 95 %", norm_data, 9.0, 11.2, 0.95, None),
    ("normal 90 %", norm_data, 9.0, 11.2, 0.90, None),
    ("normal 99 %, upper only", norm_data, None, 10.8, 0.99, None),
    ("short sample", norm_data[:8], 9.0, 11.2, 0.95, None),
    ("Box-Cox lambda 0.5", skew, 0.2, 6.0, 0.95, 0.5),
]:
    r = ci(x, lsl, usl, level, lam)
    print(f'    ("{name}", {vec(x)}, {opt(lsl)}, {opt(usl)}, {level!r}, {opt(lam)}, [{", ".join(num(v) for v in r)}]),')
print("];")
