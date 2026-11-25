"""Prints crates/calc/tests/fixtures/expsmooth_pi_reference.rs: damped-trend exponential smoothing and prediction
intervals for `sheaf_calc::expsmooth::smooth_with` (T427), from numpy and scipy.

- The recursions with a damping factor phi are written out in plain Python (statsmodels' damped Holt-Winters
  equations with its "legacy-heuristic" initial states); at fixed constants the fitted values, SSE and forecasts are
  exact references.
- The intervals: sigma^2 = SSE / n and the h-step variance sigma^2 (1 + sum_{j<h} c_j^2), c_j = a (1 + beta (phi + ...
  + phi^j)) + gamma [m divides j]. The script also checks that variance by simulating the error-correction recursions
  forward from the final states (200 000 paths, to 2 %), so the formula is the model's.
- Fitted damping: scipy.optimize.minimize (L-BFGS-B over the constants in [0, 1] and phi in [0.8, 0.98], many starts);
  the test only requires Sheaf's SSE to be no worse.

Usage: python3 tools/expsmooth-pi-ref.py > crates/calc/tests/fixtures/expsmooth_pi_reference.rs
None of these numbers come from Origin.
"""
import itertools
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
    return np.array(out)


def run(y, model, m, c, phi, h):
    a = c[0]
    beta = c[1] if len(c) > 1 else 0.0
    gamma = c[2] if len(c) > 2 else 0.0
    if model == "simple":
        l, b, s = y[0], 0.0, []
    elif model == "holt":
        l, b, s = y[0], y[1] - y[0], []
    else:
        l = y[:m].mean()
        b = np.mean((y[m : 2 * m] - y[:m]) / m)
        s = list(y[:m] - l)
    fitted, sse = [], 0.0
    for t, v in enumerate(y):
        season = s[t % m] if model == "add" else 0.0
        f = l + phi * b + season
        fitted.append(f)
        sse += (v - f) ** 2
        if model == "simple":
            nl = a * v + (1 - a) * l
        else:
            nl = a * (v - season) + (1 - a) * (l + phi * b)
        if model == "add":
            s[t % m] = gamma * (v - l - phi * b) + (1 - gamma) * season
        if model != "simple":
            b = beta * (nl - l) + (1 - beta) * phi * b
        l = nl
    n = len(y)
    fc = []
    for k in range(1, h + 1):
        damp = sum(phi ** i for i in range(1, k + 1))
        fc.append(l + damp * b + (s[(n + k - 1) % m] if model == "add" else 0.0))
    return fitted, sse, l, b, s, fc


def variances(model, m, c, phi, sigma2, h):
    a = c[0]
    beta = c[1] if len(c) > 1 else 0.0
    gamma = c[2] if len(c) > 2 else 0.0
    out, total = [], 0.0
    for k in range(1, h + 1):
        if k > 1:
            j = k - 1
            damp = sum(phi ** i for i in range(1, j + 1))
            cj = a * (1 + beta * damp) + (gamma if model == "add" and j % m == 0 else 0.0)
            total += cj * cj
        out.append(sigma2 * (1 + total))
    return out


def simulate(model, m, c, phi, sigma, l, b, s, n, h, rng, paths=200_000):
    a = c[0]
    beta = c[1] if len(c) > 1 else 0.0
    gamma = c[2] if len(c) > 2 else 0.0
    L = np.full(paths, l)
    B = np.full(paths, b)
    S = [np.full(paths, v) for v in s] if model == "add" else []
    ys = []
    for k in range(h):
        t = n + k
        season = S[t % m] if model == "add" else 0.0
        e = rng.normal(0, sigma, paths)
        yk = L + phi * B + season + e
        ys.append(yk)
        nl = L + phi * B + a * e
        if model == "add":
            S[t % m] = season + gamma * e
        B = phi * B + a * beta * e if model != "simple" else B
        L = nl
    return [y.var() for y in ys]


def series(n, seed, season):
    u = uniforms(n, seed)
    return np.array([20 + 0.6 * i + (season[i % len(season)] if season else 0) + 2 * stats.norm.ppf(u[i]) for i in range(n)])


trend = series(30, 3, None)
seas = series(32, 5, [3.0, -2.0, 1.0, -2.0])
rng = np.random.default_rng(427)

fixed = [
    ("simple 90 %", trend, "simple", 0, [0.4], 1.0, 6, 0.90),
    ("Holt 95 %", trend, "holt", 0, [0.5, 0.2], 1.0, 6, 0.95),
    ("damped Holt 0.9", trend, "holt", 0, [0.5, 0.2], 0.9, 8, 0.95),
    ("additive Holt-Winters 99 %", seas, "add", 4, [0.3, 0.1, 0.2], 1.0, 9, 0.99),
    ("damped additive Holt-Winters 0.85", seas, "add", 4, [0.3, 0.1, 0.2], 0.85, 9, 0.95),
]


def num(v):
    return "f64::NAN" if v != v else repr(float(v))


def vec(v):
    return "&[" + ", ".join(num(x) for x in v) + "]"


print("// Made by tools/expsmooth-pi-ref.py (numpy, scipy). Do not edit.")
print("pub struct Fixed {")
for f in ["name: &'static str", "y: &'static [f64]", "model: &'static str", "period: usize", "constants: &'static [f64]",
          "phi: f64", "level: f64", "fitted: &'static [f64]", "sse: f64", "forecast: &'static [f64]",
          "lower: &'static [f64]", "upper: &'static [f64]"]:
    print(f"    pub {f},")
print("}")
print("pub const FIXED: &[Fixed] = &[")
for name, y, model, m, c, phi, h, level in fixed:
    fitted, sse, l, b, s, fc = run(y, model, m, c, phi, h)
    n = len(y)
    var = variances(model, m, c, phi, sse / n, h)
    sim = simulate(model, m, c, phi, math.sqrt(sse / n), l, b, s, n, h, rng)
    for k, (v, w) in enumerate(zip(var, sim)):
        assert abs(w / v - 1) < 0.02, (name, k, v, w)
    z = stats.norm.ppf(0.5 + level / 2)
    lo = [f - z * math.sqrt(v) for f, v in zip(fc, var)]
    hi = [f + z * math.sqrt(v) for f, v in zip(fc, var)]
    print(f'    Fixed {{ name: "{name}", y: {vec(y)}, model: "{model}", period: {m}, constants: {vec(c)}, phi: {num(phi)}, '
          f"level: {num(level)}, fitted: {vec(fitted)}, sse: {num(sse)}, forecast: {vec(fc)}, lower: {vec(lo)}, upper: {vec(hi)} }},")
print("];")

# Fitted damping: scipy's best SSE.
print("/// (name, data, model, period, scipy's best SSE with damping fitted in [0.8, 0.98])")
print("pub const FITTED: &[(&str, &[f64], &str, usize, f64)] = &[")
for name, y, model, m in [("damped Holt fitted", trend, "holt", 0), ("damped additive Holt-Winters fitted", seas, "add", 4)]:
    k = 2 if model == "holt" else 3
    f = lambda p: run(y, model, m, p[:k], p[k], 0)[1]
    bounds = [(0, 1)] * k + [(0.8, 0.98)]
    best = math.inf
    for start in itertools.product(*([[0.1, 0.5, 0.9]] * k + [[0.8, 0.9, 0.98]])):
        r = optimize.minimize(f, start, method="L-BFGS-B", bounds=bounds, options={"ftol": 1e-15, "gtol": 1e-12})
        best = min(best, r.fun)
    print(f'    ("{name}", {vec(y)}, "{model}", {m}, {num(best)}),')
print("];")
