"""Prints crates/calc/tests/fixtures/expsmooth_reference.rs: exponential smoothing for `sheaf_calc::expsmooth`
(T374), from numpy and scipy.

The recursions (statsmodels' Holt-Winters equations with its "legacy-heuristic" initial states) are written out in
plain Python on numpy arrays; at fixed constants the fitted values, SSE, final states and forecasts are exact
references. The fitted constants come from scipy.optimize.minimize (L-BFGS-B on [0, 1], many starts, then Powell
polish); the test only requires Sheaf's SSE to be no worse than scipy's.

Usage: python3 tools/expsmooth-ref.py > crates/calc/tests/fixtures/expsmooth_reference.rs
None of these numbers come from Origin.
"""
import itertools
import math

import numpy as np
from scipy import optimize

M = 2 ** 64


def uniforms(n, seed):
    st = seed
    out = []
    for _ in range(n):
        st = (st * 6364136223846793005 + 1442695040888963407) % M
        out.append(((st >> 11) + 0.5) / 2 ** 53)
    return np.array(out)


def run(y, model, m, c, h):
    n = len(y)
    a = c[0]
    beta = c[1] if len(c) > 1 else 0.0
    gamma = c[2] if len(c) > 2 else 0.0
    mult = model == "mul"
    if model == "simple":
        l, b, s = y[0], 0.0, []
    elif model == "holt":
        l, b, s = y[0], y[1] - y[0], []
    else:
        l = y[:m].mean()
        b = np.mean((y[m : 2 * m] - y[:m]) / m)
        s = list(y[:m] / l if mult else y[:m] - l)
    fitted, sse = [], 0.0
    for t, v in enumerate(y):
        season = s[t % m] if model in ("add", "mul") else (1.0 if mult else 0.0)
        f = (l + b) * season if mult else l + b + season
        fitted.append(f)
        sse += (v - f) ** 2
        if model == "simple":
            nl = a * v + (1 - a) * l
        elif model == "holt":
            nl = a * v + (1 - a) * (l + b)
        elif model == "add":
            nl = a * (v - season) + (1 - a) * (l + b)
        else:
            nl = a * v / season + (1 - a) * (l + b)
        if model in ("add", "mul"):
            s[t % m] = gamma * v / (l + b) + (1 - gamma) * season if mult else gamma * (v - l - b) + (1 - gamma) * season
        if model != "simple":
            b = beta * (nl - l) + (1 - beta) * b
        l = nl
    fc = []
    for k in range(1, h + 1):
        if model == "simple":
            fc.append(l)
        elif model == "holt":
            fc.append(l + k * b)
        elif model == "add":
            fc.append(l + k * b + s[(n + k - 1) % m])
        else:
            fc.append((l + k * b) * s[(n + k - 1) % m])
    return np.array(fitted), sse, l, b, fc


def fit(y, model, m):
    k = {"simple": 1, "holt": 2}.get(model, 3)
    f = lambda c: run(y, model, m, list(c), 0)[1]
    best = None
    for start in itertools.product([0.05, 0.3, 0.6, 0.95], repeat=k):
        r = optimize.minimize(f, start, method="L-BFGS-B", bounds=[(0, 1)] * k, options={"ftol": 1e-15, "gtol": 1e-12})
        if best is None or r.fun < best.fun:
            best = r
    r = optimize.minimize(f, best.x, method="Powell", bounds=[(0, 1)] * k, options={"xtol": 1e-12, "ftol": 1e-15})
    return (r.x, r.fun) if r.fun < best.fun else (best.x, best.fun)


def vec(v):
    return "&[" + ", ".join(repr(float(a)) for a in v) + "]"


t = np.arange(36)
level = 20 + 0.5 * np.cumsum(uniforms(30, 3) - 0.5)
trend = 5 + 0.3 * np.arange(30) + 0.8 * (uniforms(30, 5) - 0.5)
seasonal = 50 + 0.4 * t + 5 * np.array([3, 1, -2, -4, -1, 3])[t % 6] * 0.5 + (uniforms(36, 9) - 0.5)
mseason = (30 + 0.5 * t) * np.array([1.2, 0.9, 0.8, 1.1])[t % 4] * (1 + 0.03 * (uniforms(36, 13) - 0.5))
cases = [
    ("level", level, "simple", 0, [0.4]),
    ("trend", trend, "holt", 0, [0.5, 0.2]),
    ("additive season", seasonal, "add", 6, [0.3, 0.1, 0.2]),
    ("multiplicative season", mseason, "mul", 4, [0.4, 0.05, 0.3]),
]
print("// Made by tools/expsmooth-ref.py (numpy, scipy). Do not edit.")
print("pub struct Case {")
print("    pub name: &'static str,")
print("    pub y: &'static [f64],")
print("    pub model: &'static str,")
print("    pub period: usize,")
print("    pub constants: &'static [f64],")
print("    pub fitted: &'static [f64],")
print("    pub sse: f64,")
print("    pub level: f64,")
print("    pub trend: f64,")
print("    pub forecast: &'static [f64],")
print("    /// scipy's best SSE over the fitted constants, and those constants")
print("    pub best_sse: f64,")
print("    pub best: &'static [f64],")
print("}")
print("pub const CASES: &[Case] = &[")
for name, y, model, m, c in cases:
    fitted, sse, l, b, fc = run(y, model, m, c, 5)
    bx, bf = fit(y, model, m)
    print("    Case {")
    print(f'        name: "{name}",')
    print(f"        y: {vec(y)},")
    print(f'        model: "{model}",')
    print(f"        period: {m},")
    print(f"        constants: {vec(c)},")
    print(f"        fitted: {vec(fitted)},")
    print(f"        sse: {float(sse)!r},")
    print(f"        level: {float(l)!r},")
    print(f"        trend: {float(b)!r},")
    print(f"        forecast: {vec(fc)},")
    print(f"        best_sse: {float(bf)!r},")
    print(f"        best: {vec(bx)},")
    print("    },")
print("];")
