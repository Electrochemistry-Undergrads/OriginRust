"""Prints crates/calc/tests/fixtures/sarima_reference.rs: seasonal ARIMA by conditional sum of squares for
`sheaf_calc::arima` (T446), from numpy and scipy.

Written independently of Sheaf: the seasonal and ordinary polynomials are multiplied with numpy.polymul, the series is
differenced with numpy (np.diff at lag 1, y[s:] - y[:-s] at the period), the conditional errors (0 before the full AR
polynomial's last lag) are a plain loop, forecasts recurse with future errors 0 and are integrated back by cumulative
sums per lag, and the bands use psi weights from the full AR polynomial times the differencing operators. Standard
errors at given coefficients: sigma^2 (J'J)^-1 with J by the complex step. The fitted sum of squares comes from
scipy.optimize.minimize (Nelder-Mead, BFGS, Nelder-Mead); the test requires Sheaf's to be no worse.

Usage: python3 tools/sarima-ref.py > crates/calc/tests/fixtures/sarima_reference.rs
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


def unpack(order, params):
    p, d, q, c, P, D, Q, s = order
    k = 1 if c else 0
    mu = params[0] if c else 0.0
    phi = params[k : k + p]
    theta = params[k + p : k + p + q]
    Phi = params[k + p + q : k + p + q + P]
    Theta = params[k + p + q + P : k + p + q + P + Q]
    return mu, phi, theta, Phi, Theta


def full(order, params):
    p, d, q, c, P, D, Q, s = order
    mu, phi, theta, Phi, Theta = unpack(order, params)
    # numpy.polymul wants the highest power first: reverse the coefficient lists in B.
    def poly(small, big, sign):
        a = [1.0] + [sign * v for v in small]
        b = [0.0] * (len(big) * s + 1)
        b[0] = 1.0
        for j, v in enumerate(big):
            b[(j + 1) * s] = sign * v
        return list(np.polymul(np.array(a[::-1], dtype=complex if isinstance(params[0], complex) else float),
                               np.array(b[::-1], dtype=complex if isinstance(params[0], complex) else float))[::-1])
    ar = [-v for v in poly(phi, Phi, -1.0)[1:]]
    ma = list(poly(theta, Theta, 1.0)[1:])
    return mu, ar, ma


def differenced(y, order):
    p, d, q, c, P, D, Q, s = order
    w = np.array(y, dtype=float)
    stack = [(w, 0)]
    for _ in range(d):
        w = np.diff(w)
        stack.append((w, 1))
    for _ in range(D):
        w = w[s:] - w[:-s]
        stack.append((w, s))
    return stack


def css(y, order, params):
    params = list(params)
    w = differenced(y, order)[-1][0]
    mu, ar, ma = full(order, params)
    start = order[0] + order[4] * order[7]
    e = [0.0] * len(w)
    out = []
    for t in range(start, len(w)):
        v = w[t] - mu
        for i, a in enumerate(ar):
            v = v - a * (w[t - i - 1] - mu)
        for j, m in enumerate(ma):
            if t > j:
                v = v - m * e[t - j - 1]
        e[t] = v
        out.append(v)
    return e, out


def forecast(y, order, params, sigma2, h):
    p, d, q, c, P, D, Q, s = order
    e, _ = css(y, order, params)
    stack = differenced(y, order)
    w = list(stack[-1][0])
    mu, ar, ma = full(order, params)
    n = len(w)
    errs = list(e)
    for k in range(h):
        t = n + k
        v = mu
        for i, a in enumerate(ar):
            v += a * (w[t - i - 1] - mu)
        for j, m in enumerate(ma):
            if t > j:
                v += m * errs[t - j - 1]
        w.append(v)
        errs.append(0.0)
    fc = w[n:]
    for lvl in range(len(stack) - 2, -1, -1):
        lag = stack[lvl + 1][1]
        known = list(stack[lvl][0])
        base = len(known)
        for dv in fc:
            known.append(dv + known[len(known) - lag])
        fc = known[base:]
    poly = np.array([1.0] + [-a for a in ar])
    for lag in [1] * d + [s] * D:
        op = np.zeros(lag + 1)
        op[0], op[lag] = 1.0, -1.0
        poly = np.convolve(poly, op)
    psi = [1.0]
    for j in range(1, h):
        v = ma[j - 1] if j - 1 < len(ma) else 0.0
        for i in range(1, min(len(poly), j + 1)):
            v -= poly[i] * psi[j - i]
        psi.append(v)
    return fc, list(np.sqrt(sigma2 * np.cumsum(np.array(psi) ** 2)))


def se(y, order, params, sigma2):
    k = len(params)
    m = len(css(y, order, params)[1])
    J = np.zeros((m, k))
    for i in range(k):
        x = [complex(v) for v in params]
        x[i] += 1e-30j
        J[:, i] = np.array([v.imag for v in css(y, order, x)[1]]) / 1e-30
    return list(np.sqrt(sigma2 * np.diag(np.linalg.inv(J.T @ J))))


def fit_ss(y, order):
    p, d, q, c, P, D, Q, s = order
    w = differenced(y, order)[-1][0]
    x0 = ([w.mean()] if c else []) + [0.0] * (p + q + P + Q)
    f = lambda x: sum(v * v for v in css(y, order, x)[1])
    r = optimize.minimize(f, x0, method="Nelder-Mead", options={"xatol": 1e-12, "fatol": 1e-15, "maxiter": 100000, "maxfev": 100000})
    r2 = optimize.minimize(f, r.x, method="BFGS", options={"gtol": 1e-12})
    best = r2 if r2.fun < r.fun else r
    r3 = optimize.minimize(f, best.x, method="Nelder-Mead", options={"xatol": 1e-13, "fatol": 1e-16, "maxiter": 100000, "maxfev": 100000})
    return min(best.fun, r3.fun)


e = normals(200, 17)
# Monthly: (1,0,0)(1,1,0)12 around a seasonal pattern.
season = np.array([3, 1, -1, -2, -3, -1, 0, 2, 4, 3, 1, -2], dtype=float)
x = np.zeros(108)
for t in range(108):
    x[t] = (0.5 * x[t - 1] if t >= 1 else 0) + (0.3 * x[t - 12] if t >= 12 else 0) + e[t]
monthly = 50 + np.tile(season, 9) + x
# Airline-like: (0,1,1)(0,1,1)12 from cumulated noise.
air = 100 + np.cumsum(0.5 + e[100:196] + 0.4 * np.concatenate([[0], e[100:195]]))
air = air + np.tile(season, 8) * 2
# Quarterly: (1,0,1)(1,0,0)4 with a mean.
quart = 20 + np.convolve(e[:80], [1, 0.3])[:80] + np.tile([2, -1, 1, -2], 20)

cases = [
    ("monthly (1,0,0)(1,1,0)12", monthly, (1, 0, 0, True, 1, 1, 0, 12), [0.1, 0.45, 0.25]),
    ("airline (0,1,1)(0,1,1)12", air, (0, 1, 1, False, 0, 1, 1, 12), [-0.3, -0.5]),
    ("quarterly (1,0,1)(1,0,0)4", quart, (1, 0, 1, True, 1, 0, 0, 4), [20.0, 0.2, 0.3, 0.5]),
]


def vec(v):
    return "&[" + ", ".join(repr(float(a)) for a in v) + "]"


print("// Made by tools/sarima-ref.py (numpy, scipy). Do not edit.")
print("pub struct Case {")
for f in ["name: &'static str", "y: &'static [f64]", "order: (usize, usize, usize, bool, usize, usize, usize, usize)",
          "params: &'static [f64]", "sigma2: f64", "errors: &'static [f64]", "ss: f64", "forecast: &'static [f64]",
          "forecast_se: &'static [f64]", "se: &'static [f64]", "best_ss: f64"]:
    print(f"    pub {f},")
print("}")
print("pub const CASES: &[Case] = &[")
for name, y, order, params in cases:
    _, errs = css(y, order, params)
    ss = float(sum(v * v for v in errs))
    sigma2 = ss / len(errs)
    fc, fse = forecast(y, order, params, sigma2, 12)
    s_e = se(y, order, params, sigma2)
    best = float(fit_ss(y, order))
    o = ", ".join(("true" if v else "false") if isinstance(v, bool) else str(v) for v in order)
    print(f'    Case {{ name: "{name}", y: {vec(y)}, order: ({o}), params: {vec(params)}, sigma2: {sigma2!r}, '
          f"errors: {vec(errs)}, ss: {ss!r}, forecast: {vec(fc)}, forecast_se: {vec(fse)}, se: {vec(s_e)}, best_ss: {best!r} }},")
print("];")
