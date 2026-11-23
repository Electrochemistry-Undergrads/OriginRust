"""Prints crates/calc/tests/fixtures/arima_reference.rs: ARIMA by conditional sum of squares for
`sheaf_calc::arima` (T376), from numpy and scipy.

The conditional errors and forecasts are written out in plain Python on numpy arrays; the psi weights come from
numpy polynomial products. scipy.optimize.minimize (Nelder-Mead with tight tolerances, then BFGS, then Nelder-Mead
again) gives the reference minimum; the test requires Sheaf's sum of squares to be no worse, and checks errors and
forecasts exactly at scipy's parameters.

Usage: python3 tools/arima-ref.py > crates/calc/tests/fixtures/arima_reference.rs
None of these numbers come from Origin.
"""
import math

import numpy as np
from scipy import optimize

M = 2 ** 64


def normals(n, seed):
    st = seed
    out = []
    for _ in range(n):
        s = 0.0
        for _ in range(12):
            st = (st * 6364136223846793005 + 1442695040888963407) % M
            s += (st >> 11) / 2 ** 53
        out.append(s - 6.0)
    return np.array(out)


def split(order, params):
    p, d, q, c = order
    mu = params[0] if c else 0.0
    k = 1 if c else 0
    return mu, list(params[k : k + p]), list(params[k + p : k + p + q])


def css(y, order, params):
    p, d, q, c = order
    w = np.diff(y, n=d) if d else np.array(y, dtype=float)
    mu, phi, theta = split(order, params)
    e = np.zeros(len(w))
    ss = 0.0
    for t in range(p, len(w)):
        v = w[t] - mu
        for i, ph in enumerate(phi):
            v -= ph * (w[t - i - 1] - mu)
        for j, th in enumerate(theta):
            if t > j:
                v -= th * e[t - j - 1]
        e[t] = v
        ss += v * v
    return e, ss


def forecast(y, order, params, sigma2, h):
    p, d, q, c = order
    e, _ = css(y, order, params)
    w = list(np.diff(y, n=d) if d else np.array(y, dtype=float))
    mu, phi, theta = split(order, params)
    n = len(w)
    errs = list(e)
    for k in range(h):
        t = n + k
        v = mu
        for i, ph in enumerate(phi):
            v += ph * (w[t - i - 1] - mu)
        for j, th in enumerate(theta):
            if t > j:
                v += th * errs[t - j - 1]
        w.append(v)
        errs.append(0.0)
    fc = np.array(w[n:])
    for lvl in range(d - 1, -1, -1):
        base = np.diff(y, n=lvl)[-1] if lvl else y[-1]
        fc = base + np.cumsum(fc)
    ar = np.array([1.0] + [-x for x in phi])
    for _ in range(d):
        ar = np.convolve(ar, [1.0, -1.0])
    psi = [1.0]
    for j in range(1, h):
        v = theta[j - 1] if j - 1 < len(theta) else 0.0
        for i in range(1, min(len(ar), j + 1)):
            v -= ar[i] * psi[j - i]
        psi.append(v)
    se = np.sqrt(sigma2 * np.cumsum(np.array(psi) ** 2))
    return fc, se


def fit(y, order):
    p, d, q, c = order
    w = np.diff(y, n=d) if d else np.array(y, dtype=float)
    x0 = ([w.mean()] if c else []) + [0.0] * (p + q)
    f = lambda x: css(y, order, x)[1]
    r = optimize.minimize(f, x0, method="Nelder-Mead", options={"xatol": 1e-12, "fatol": 1e-15, "maxiter": 200000, "maxfev": 200000})
    r2 = optimize.minimize(f, r.x, method="BFGS", options={"gtol": 1e-12})
    best = r2 if r2.fun < r.fun else r
    r3 = optimize.minimize(f, best.x, method="Nelder-Mead", options={"xatol": 1e-13, "fatol": 1e-16, "maxiter": 200000, "maxfev": 200000})
    best = r3 if r3.fun < best.fun else best
    return best.x, best.fun


def vec(v):
    return "&[" + ", ".join(repr(float(a)) for a in v) + "]"


e = normals(160, 3)
ar = np.zeros(150)
for t in range(2, 150):
    ar[t] = 0.6 * ar[t - 1] - 0.25 * ar[t - 2] + e[t]
ar += 10
ma = 5 + e[1:121] + 0.5 * e[:120]
walk = np.cumsum(0.3 + normals(100, 9))
cases = [
    ("AR(2) with mean", ar, (2, 0, 0, True)),
    ("ARMA(1,1)", ma, (1, 0, 1, True)),
    ("ARIMA(1,1,1) with drift", walk, (1, 1, 1, True)),
    ("ARIMA(0,2,1)", walk, (0, 2, 1, False)),
]
print("// Made by tools/arima-ref.py (numpy, scipy). Do not edit.")
print("pub struct Case {")
print("    pub name: &'static str,")
print("    pub y: &'static [f64],")
print("    /// (p, d, q, constant)")
print("    pub order: (usize, usize, usize, bool),")
print("    /// scipy's best parameters and sum of squares")
print("    pub params: &'static [f64],")
print("    pub ss: f64,")
print("    /// at those parameters: the errors, sigma2 = ss / m, 6 forecasts and their standard errors")
print("    pub errors: &'static [f64],")
print("    pub sigma2: f64,")
print("    pub forecast: &'static [f64],")
print("    pub se: &'static [f64],")
print("}")
print("pub const CASES: &[Case] = &[")
for name, y, order in cases:
    x, ss = fit(y, order)
    errs, ss2 = css(y, order, x)
    m = len(y) - order[1] - order[0]
    s2 = ss2 / m
    fc, se = forecast(y, order, x, s2, 6)
    print("    Case {")
    print(f'        name: "{name}",')
    print(f"        y: {vec(y)},")
    print(f"        order: ({order[0]}, {order[1]}, {order[2]}, {'true' if order[3] else 'false'}),")
    print(f"        params: {vec(x)},")
    print(f"        ss: {float(ss2)!r},")
    print(f"        errors: {vec(errs)},")
    print(f"        sigma2: {float(s2)!r},")
    print(f"        forecast: {vec(fc)},")
    print(f"        se: {vec(se)},")
    print("    },")
print("];")
