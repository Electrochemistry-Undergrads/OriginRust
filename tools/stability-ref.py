"""Prints crates/calc/tests/fixtures/stability_reference.rs: ICH Q1E stability studies for `sheaf_calc::stability`
(T368), from numpy and scipy.

The three ANCOVA models are fitted with numpy.linalg.lstsq, poolability F tests use scipy.stats.f, the bounds
scipy.stats.t with (X'X)^-1 from numpy.linalg.inv, and each shelf life is the root (scipy.optimize.brentq) of the
margin to the nearer limit after its first sign change on a grid of 20000 steps up to 10 times the last time.

Usage: python3 tools/stability-ref.py > crates/calc/tests/fixtures/stability_reference.rs
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


def study(batches, times, starts, slopes, sd, seed):
    u = uniforms(batches * len(times), seed)
    data = []
    k = 0
    for b in range(batches):
        for t in times:
            data.append((b, float(t), float(round(starts[b] + slopes[b] * t + sd * stats.norm.ppf(u[k]), 4))))
            k += 1
    return data


def design(model, g, b, t):
    if model == "separate":
        r = [0.0] * (2 * g)
        r[b] = 1.0
        r[g + b] = t
    elif model == "common":
        r = [0.0] * (g + 1)
        r[b] = 1.0
        r[g] = t
    else:
        r = [1.0, t]
    return r


def fit(model, g, data):
    X = np.array([design(model, g, b, t) for b, t, _ in data])
    y = np.array([v for _, _, v in data])
    theta, *_ = np.linalg.lstsq(X, y, rcond=None)
    sse = float(((y - X @ theta) ** 2).sum())
    return theta, np.linalg.inv(X.T @ X), sse, X.shape[1]


def reference(data, lower, upper, conf=0.95, alpha=0.25):
    g = max(d[0] for d in data) + 1
    n = len(data)
    if g == 1:
        model, ps, pi = "pooled", math.nan, math.nan
    else:
        _, _, s1, _ = fit("separate", g, data)
        _, _, s2, _ = fit("common", g, data)
        ps = stats.f.sf(((s2 - s1) / (g - 1)) / (s1 / (n - 2 * g)), g - 1, n - 2 * g)
        pi = math.nan
        if ps < alpha:
            model = "separate"
        else:
            _, _, s3, _ = fit("pooled", g, data)
            pi = stats.f.sf(((s3 - s2) / (g - 1)) / (s2 / (n - g - 1)), g - 1, n - g - 1)
            model = "common" if pi < alpha else "pooled"
    theta, inv, sse, p = fit(model, g, data)
    df = n - p
    mse = sse / df
    q = 0.5 + conf / 2 if (lower is not None and upper is not None) else conf
    tq = stats.t.ppf(q, df)
    last = max(d[1] for d in data)
    end = 10 * max(last, 1.0)
    out = []
    for b in range(g):
        def margin(t):
            r = np.array(design(model, g, b, t))
            m = r @ theta
            h = tq * math.sqrt(mse * (r @ inv @ r))
            lo = (m - h - lower) if lower is not None else math.inf
            hi = (upper - (m + h)) if upper is not None else math.inf
            return min(lo, hi)

        grid = np.linspace(0, end, 20001)
        life = math.inf
        for a, c in zip(grid[:-1], grid[1:]):
            if margin(c) <= 0:
                life = optimize.brentq(margin, a, c, xtol=1e-13, rtol=1e-15)
                break
        r0, r1 = np.array(design(model, g, b, 0.0)), np.array(design(model, g, b, 1.0))
        out.append([r0 @ theta, r1 @ theta - r0 @ theta, life])
    return model, ps, pi, mse, df, tq, out


def num(v):
    v = float(v)
    if math.isnan(v):
        return "f64::NAN"
    if math.isinf(v):
        return "f64::INFINITY"
    return repr(v)


def opt(v):
    return "None" if v is None else f"Some({float(v)!r})"


cases = [
    ("three batches, common line", study(3, [0, 3, 6, 9, 12, 18], [100.0, 100.0, 100.0], [-0.30, -0.30, -0.30], 0.25, 4), 95.0, None),
    ("three batches, different slopes", study(3, [0, 3, 6, 9, 12, 18, 24], [100.0, 100.1, 99.9], [-0.15, -0.30, -0.22], 0.15, 9), 95.0, 105.0),
    ("two batches, different starts", study(2, [0, 3, 6, 9, 12], [101.5, 99.0], [-0.2, -0.2], 0.1, 13), 95.0, None),
    ("one batch, rising impurity", study(1, [0, 1, 2, 3, 6, 9, 12], [0.1], [0.04], 0.02, 21), None, 0.8),
]
print("// Made by tools/stability-ref.py (numpy, scipy). Do not edit.")
print("pub struct Case {")
print("    pub name: &'static str,")
print("    pub data: &'static [(usize, f64, f64)],")
print("    pub lower: Option<f64>,")
print("    pub upper: Option<f64>,")
print("    /// \"separate\", \"common\" or \"pooled\"")
print("    pub model: &'static str,")
print("    pub p_slopes: f64,")
print("    pub p_intercepts: f64,")
print("    pub mse: f64,")
print("    pub df: usize,")
print("    pub t: f64,")
print("    /// (intercept, slope, shelf life) per batch")
print("    pub batches: &'static [(f64, f64, f64)],")
print("}")
print("pub const CASES: &[Case] = &[")
for name, data, lower, upper in cases:
    model, ps, pi, mse, df, tq, out = reference(data, lower, upper)
    print("    Case {")
    print(f'        name: "{name}",')
    print("        data: &[" + ", ".join(f"({b}, {t!r}, {v!r})" for b, t, v in data) + "],")
    print(f"        lower: {opt(lower)},")
    print(f"        upper: {opt(upper)},")
    print(f'        model: "{model}",')
    print(f"        p_slopes: {num(ps)},")
    print(f"        p_intercepts: {num(pi)},")
    print(f"        mse: {num(mse)},")
    print(f"        df: {df},")
    print(f"        t: {num(tq)},")
    print("        batches: &[" + ", ".join(f"({num(a)}, {num(b)}, {num(c)})" for a, b, c in out) + "],")
    print("    },")
print("];")
