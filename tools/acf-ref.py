"""Prints crates/calc/tests/fixtures/acf_reference.rs: ACF, Bartlett standard errors, PACF (from solving the
Yule-Walker equations of each lag, independent of the Durbin-Levinson recursion Sheaf uses) and Ljung-Box Q and p
(scipy.stats.chi2) for `sheaf_calc::acf` (T363).

Usage: python3 tools/acf-ref.py > crates/calc/tests/fixtures/acf_reference.rs
Needs numpy and scipy. The series are closed-form or from a fixed linear congruential generator. None of these numbers
come from Origin (the Time Series Analysis app is Origin 2024b, not Origin 2016).
"""
import numpy as np
from scipy import stats

M = 2 ** 64


def noise(n, seed):
    st = seed
    out = []
    for _ in range(n):
        s = 0.0
        for _ in range(12):
            st = (st * 6364136223846793005 + 1442695040888963407) % M
            s += (st >> 11) / 2 ** 53
        out.append(s - 6.0)
    return np.array(out)


def reference(x, lags):
    n = len(x)
    d = x - x.mean()
    c0 = (d * d).sum()
    r = np.array([(d[: n - k] * d[k:]).sum() / c0 for k in range(lags + 1)])
    rows = []
    for k in range(1, lags + 1):
        se = np.sqrt((1 + 2 * (r[1:k] ** 2).sum()) / n)
        R = np.array([[r[abs(i - j)] for j in range(k)] for i in range(k)])
        phi = np.linalg.solve(R, r[1 : k + 1])
        q = n * (n + 2) * sum(r[j] ** 2 / (n - j) for j in range(1, k + 1))
        rows.append((k, r[k], se, phi[-1], 1 / np.sqrt(n), q, stats.chi2.sf(q, k)))
    return rows


series = []
e = noise(200, 21)
ar = np.zeros(200)
for t in range(200):
    ar[t] = (0.7 * ar[t - 1] if t else 0.0) + e[t]
series.append(("ar1", ar, 15))
t = np.arange(120)
series.append(("seasonal", np.sin(2 * np.pi * t / 12) + 0.3 * noise(120, 5), 24))
series.append(("white", noise(80, 9), 10))


def vec(v):
    return "&[" + ", ".join(repr(float(a)) for a in v) + "]"


print("// Made by tools/acf-ref.py (numpy, scipy). Do not edit.")
print("pub struct Series {")
print("    pub name: &'static str,")
print("    pub x: &'static [f64],")
print("    /// (lag, acf, acf se, pacf, pacf se, Ljung-Box Q, p)")
print("    pub rows: &'static [(usize, f64, f64, f64, f64, f64, f64)],")
print("}")
print()
print("pub const SERIES: &[Series] = &[")
for name, x, lags in series:
    print("    Series {")
    print(f'        name: "{name}",')
    print(f"        x: {vec(x)},")
    print("        rows: &[")
    for row in reference(x, lags):
        print("            (" + ", ".join([str(row[0])] + [repr(float(v)) for v in row[1:]]) + "),")
    print("        ],")
    print("    },")
print("];")
