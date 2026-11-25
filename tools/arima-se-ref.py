"""Prints crates/calc/tests/fixtures/arima_se_reference.rs: ARIMA coefficient standard errors and automatic order
choice for `sheaf_calc::arima` (T428), from numpy and scipy.

- Standard errors at given coefficients: sigma^2 (J'J)^-1, with J the derivatives of the conditional errors taken by
  the complex step (the error recursion run in complex numbers with a step of 1e-30 i), independent of Sheaf's
  differentiated recursion, and numpy.linalg.inv.
- Order choice: every p <= 2, q <= 2 fitted by scipy.optimize.minimize (Nelder-Mead, BFGS, Nelder-Mead, as in
  tools/arima-ref.py); the order with the smallest AIC = m ln(2 pi SS / m) + m + 2 (k + 1). The series are simulated
  so the winner leads by a clear margin, which the script asserts.

Usage: python3 tools/arima-se-ref.py > crates/calc/tests/fixtures/arima_se_reference.rs
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


def css(y, order, params):
    p, d, q, c = order
    w = np.diff(y, n=d) if d else np.array(y, dtype=float)
    params = list(params)
    mu = params[0] if c else 0.0
    k = 1 if c else 0
    phi, theta = params[k : k + p], params[k + p : k + p + q]
    e = [0.0 * params[0] if params else 0.0] * len(w)
    out = []
    for t in range(p, len(w)):
        v = w[t] - mu
        for i, ph in enumerate(phi):
            v = v - ph * (w[t - i - 1] - mu)
        for j, th in enumerate(theta):
            if t > j:
                v = v - th * e[t - j - 1]
        e[t] = v
        out.append(v)
    return out


def se(y, order, params, sigma2):
    k = len(params)
    m = len(css(y, order, params))
    J = np.zeros((m, k))
    for i in range(k):
        x = np.array(params, dtype=complex)
        x[i] += 1e-30j
        J[:, i] = np.array([v.imag for v in css(y, order, x)]) / 1e-30
    return np.sqrt(sigma2 * np.diag(np.linalg.inv(J.T @ J)))


def fit(y, order):
    p, d, q, c = order
    w = np.diff(y, n=d) if d else np.array(y, dtype=float)
    x0 = ([w.mean()] if c else []) + [0.0] * (p + q)
    f = lambda x: sum(v * v for v in css(y, order, x))
    r = optimize.minimize(f, x0, method="Nelder-Mead", options={"xatol": 1e-12, "fatol": 1e-15, "maxiter": 200000, "maxfev": 200000})
    r2 = optimize.minimize(f, r.x, method="BFGS", options={"gtol": 1e-12})
    best = r2 if r2.fun < r.fun else r
    r3 = optimize.minimize(f, best.x, method="Nelder-Mead", options={"xatol": 1e-13, "fatol": 1e-16, "maxiter": 200000, "maxfev": 200000})
    best = r3 if r3.fun < best.fun else best
    m = len(w) - p
    k = (1 if c else 0) + p + q
    return m * math.log(2 * math.pi * best.fun / m) + m + 2 * (k + 1)


def vec(v):
    return "&[" + ", ".join(repr(float(a)) for a in v) + "]"


e = normals(220, 11)
ar = np.zeros(200)
for t in range(2, 200):
    ar[t] = 0.7 * ar[t - 1] - 0.3 * ar[t - 2] + e[t]
ar += 4
ma = 2 + e[1:181] + 0.7 * e[:180]
walk = np.cumsum(0.2 + normals(150, 5))

print("// Made by tools/arima-se-ref.py (numpy, scipy). Do not edit.")
print("/// (name, data, p, d, q, constant, coefficients, sigma^2, standard errors)")
print("#[allow(clippy::type_complexity)]")
print("pub const SE: &[(&str, &[f64], usize, usize, usize, bool, &[f64], f64, &[f64])] = &[")
for name, y, order, params, sigma2 in [
    ("AR(2) with mean", ar, (2, 0, 0, True), [4.1, 0.65, -0.28], 1.1),
    ("MA(1) with mean", ma, (0, 0, 1, True), [2.05, 0.6], 0.9),
    ("ARMA(1,1)", ma, (1, 0, 1, True), [2.0, 0.2, 0.5], 1.0),
    ("ARIMA(1,1,1) with drift", walk, (1, 1, 1, True), [0.2, 0.3, -0.2], 1.2),
    ("ARIMA(0,1,2) without constant", walk, (0, 1, 2, False), [0.1, 0.05], 1.3),
]:
    s = se(y, order, params, sigma2)
    p, d, q, c = order
    print(f'    ("{name}", {vec(y)}, {p}, {d}, {q}, {"true" if c else "false"}, {vec(params)}, {sigma2!r}, {vec(s)}),')
print("];")

print("/// (name, data, d, best p, best q) over p, q <= 2 with a constant")
print("pub const AUTO: &[(&str, &[f64], usize, usize, usize)] = &[")
for name, y, d in [("AR(2) series", ar, 0), ("MA(1) series", ma, 0)]:
    aics = {(p, q): fit(y, (p, d, q, True)) for p in range(3) for q in range(3)}
    ranked = sorted(aics, key=aics.get)
    assert aics[ranked[1]] - aics[ranked[0]] > 0.5, (name, ranked[:2], [aics[r] for r in ranked[:2]])
    print(f'    ("{name}", {vec(y)}, {d}, {ranked[0][0]}, {ranked[0][1]}),')
print("];")
