"""Prints crates/calc/tests/fixtures/qpcr_reference.rs: qPCR relative quantification and standard curves for
`sheaf_calc::qpcr` (T397), from numpy and scipy.

Per group: dCt mean and SD (numpy, ddof=1), ddCt against the control, the expression ratio
exp(dt ln E_t - dr ln E_r) (Livak 2^-ddCt for E = 2), Welch's t test of dCt (scipy.stats.ttest_ind,
equal_var=False) and the interval ddCt -+ t(Welch df) SE mapped with the mean log factor. Standard curves by
scipy.stats.linregress on log10 quantity.

Usage: python3 tools/qpcr-ref.py > crates/calc/tests/fixtures/qpcr_reference.rs
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
    return np.array(out)


def quant(groups, t, r, control, et, er, conf=0.95):
    names = list(dict.fromkeys(groups))
    g = np.array(groups)
    out = []
    dc = (t - r)[g == control]
    ctc, crc = t[g == control].mean(), r[g == control].mean()
    for name in names:
        m = g == name
        d = (t - r)[m]
        dm, ds = d.mean(), d.std(ddof=1)
        ddct = dm - dc.mean()
        dt, dr = ctc - t[m].mean(), crc - r[m].mean()
        ratio = math.exp(dt * math.log(et) - dr * math.log(er))
        if name == control:
            out.append((name, len(d), dm, ds, ddct, math.nan, ratio, math.nan, math.nan, math.nan))
            continue
        va, vb = ds ** 2 / len(d), dc.std(ddof=1) ** 2 / len(dc)
        se = math.sqrt(va + vb)
        df = (va + vb) ** 2 / (va ** 2 / (len(d) - 1) + vb ** 2 / (len(dc) - 1))
        tq = stats.t.ppf(0.5 + conf / 2, df)
        ml = 0.5 * (math.log(et) + math.log(er))
        low = math.exp(dt * math.log(et) - dr * math.log(er) - tq * se * ml)
        high = math.exp(dt * math.log(et) - dr * math.log(er) + tq * se * ml)
        p = stats.ttest_ind(d, dc, equal_var=False).pvalue
        out.append((name, len(d), dm, ds, ddct, se, ratio, low, high, p))
    return out


def num(v):
    v = float(v)
    return "f64::NAN" if math.isnan(v) else repr(v)


u = uniforms(60, 7)
groups, t, r = [], [], []
for gi, (name, shift) in enumerate([("control", 0.0), ("drug A", -1.6), ("drug B", 0.7)]):
    for k in range(5):
        groups.append(name)
        t.append(round(26.0 + shift + 0.3 * (u[gi * 10 + k] - 0.5), 3))
        r.append(round(18.0 + 0.2 * (u[gi * 10 + k + 5] - 0.5), 3))
t, r = np.array(t), np.array(r)
cases = [("full efficiency", 2.0, 2.0), ("Pfaffl 1.92 / 1.98", 1.92, 1.98)]
q = np.array([1e1, 1e2, 1e3, 1e4, 1e5, 1e1, 1e2, 1e3, 1e4, 1e5])
ct = 35.5 - 3.42 * np.log10(q) + 0.15 * (uniforms(10, 3) - 0.5)
sc = stats.linregress(np.log10(q), ct)
factor = 10 ** (-1 / sc.slope)

print("// Made by tools/qpcr-ref.py (numpy, scipy). Do not edit.")
print("pub const GROUPS: &[&str] = &[" + ", ".join(f'"{g}"' for g in groups) + "];")
print("pub const TARGET: &[f64] = &[" + ", ".join(repr(float(v)) for v in t) + "];")
print("pub const REFERENCE: &[f64] = &[" + ", ".join(repr(float(v)) for v in r) + "];")
print("/// (name, E target, E reference, [(group, n, mean dCt, sd dCt, ddCt, se, ratio, low, high, p)])")
print("pub const CASES: &[(&str, f64, f64, &[(&str, usize, f64, f64, f64, f64, f64, f64, f64, f64)])] = &[")
for name, et, er in cases:
    rows = quant(groups, t, r, "control", et, er)
    print(f'    ("{name}", {et!r}, {er!r}, &[' + ", ".join(f'("{o[0]}", {o[1]}, ' + ", ".join(num(v) for v in o[2:]) + ")" for o in rows) + "]),")
print("];")
print("pub const QUANTITY: &[f64] = &[" + ", ".join(repr(float(v)) for v in q) + "];")
print("pub const CT: &[f64] = &[" + ", ".join(repr(float(v)) for v in ct) + "];")
print(f"/// slope, intercept, R^2, factor, efficiency %")
print(f"pub const CURVE: [f64; 5] = [{float(sc.slope)!r}, {float(sc.intercept)!r}, {float(sc.rvalue ** 2)!r}, {float(factor)!r}, {float((factor - 1) * 100)!r}];")
