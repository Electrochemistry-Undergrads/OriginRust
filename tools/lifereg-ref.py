"""Prints crates/calc/tests/fixtures/lifereg_reference.rs: accelerated failure time regression with right censoring
for `sheaf_calc::lifereg` (T391), from numpy and scipy.

The log-likelihood (Weibull: smallest extreme value errors; lognormal: normal errors; exponential: extreme value with
sigma = 1; the -ln t Jacobian included) is written with numpy and scipy.stats, maximized by scipy.optimize.minimize
(BFGS with the analytic gradient, then Newton-CG polish), and the standard errors come from the inverse of a central-
difference Hessian of the analytic gradient. Percentiles at given covariates (T437): exp(x'b + sigma w_p) with the
delta-method interval on the log scale from that covariance.

Usage: python3 tools/lifereg-ref.py > crates/calc/tests/fixtures/lifereg_reference.rs
lifelines and statsmodels are not available here; none of these numbers come from Origin.
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
    return np.array(out)


def terms(dist, z):
    if dist == "lognormal":
        s = stats.norm.sf(z)
        return stats.norm.logpdf(z), np.log(s), -z, -stats.norm.pdf(z) / s
    ez = np.exp(z)
    return z - ez, -ez, 1 - ez, -ez


def loglik(theta, X, lt, failed, dist):
    k = X.shape[1]
    ls = theta[k] if dist != "exponential" else 0.0
    s = math.exp(ls)
    z = (lt - X @ theta[:k]) / s
    lg, lsv, dlg, dls = terms(dist, z)
    ll = np.where(failed, -ls - lt + lg, lsv).sum()
    d = np.where(failed, dlg, dls)
    g = list(-(d / s) @ X)
    if dist != "exponential":
        g.append(float((-d * z - failed).sum()))
    return float(ll), np.array(g)


def fit(X, lt, failed, dist):
    k = X.shape[1]
    b0, *_ = np.linalg.lstsq(X[failed], lt[failed], rcond=None)
    r = lt[failed] - X[failed] @ b0
    x0 = list(b0) + ([math.log(max(r.std(), 0.1))] if dist != "exponential" else [])
    f = lambda t: -loglik(t, X, lt, failed, dist)[0]
    jac = lambda t: -loglik(t, X, lt, failed, dist)[1]
    res = optimize.minimize(f, x0, jac=jac, method="BFGS", options={"gtol": 1e-12, "maxiter": 10000})
    res = optimize.minimize(f, res.x, jac=jac, method="Newton-CG", options={"xtol": 1e-14, "maxiter": 10000})
    th = res.x
    m = len(th)
    H = np.zeros((m, m))
    for j in range(m):
        e = 1e-5 * max(abs(th[j]), 1.0)
        up, dn = th.copy(), th.copy()
        up[j] += e
        dn[j] -= e
        H[:, j] = (loglik(up, X, lt, failed, dist)[1] - loglik(dn, X, lt, failed, dist)[1]) / (2 * e)
    H = 0.5 * (H + H.T)
    cov = np.linalg.inv(-H)
    se = np.sqrt(np.diag(cov))[:k]
    ll = loglik(th, X, lt, failed, dist)[0]
    if dist != "exponential":
        scale, sse = math.exp(th[k]), math.exp(th[k]) * math.sqrt(cov[k, k])
    else:
        scale, sse = 1.0, math.nan
    return th[:k], se, scale, sse, ll, th, cov


def percentile(dist, th, cov, k, x, p, level):
    """The p percentile life at covariates x (with the leading 1) and its interval on the log scale (T437)."""
    w = stats.norm.ppf(p) if dist == "lognormal" else math.log(-math.log1p(-p))
    s = math.exp(th[k]) if dist != "exponential" else 1.0
    g = list(x) + ([s * w] if dist != "exponential" else [])
    lp = float(np.dot(th[:k], x)) + s * w
    se = math.sqrt(float(np.array(g) @ cov @ np.array(g)))
    z = stats.norm.ppf(0.5 + level / 2)
    return [math.exp(lp), se, math.exp(lp - z * se), math.exp(lp + z * se)]


def num(v):
    return "f64::NAN" if math.isnan(v) else repr(float(v))


n = 60
u = uniforms(3 * n, 5)
x1 = np.round(u[:n] * 4, 3)
x2 = np.array([float(i % 2) for i in range(n)])
w = np.log(-np.log(u[n : 2 * n]))  # smallest extreme value
life = np.exp(2.0 + 0.3 * x1 - 0.5 * x2 + 0.6 * w)
cens = 8 + 6 * u[2 * n :]
t = np.round(np.minimum(life, cens), 5)
failed = life <= cens
X = np.column_stack([np.ones(n), x1, x2])
print("// Made by tools/lifereg-ref.py (numpy, scipy). Do not edit.")
print("pub const X: &[[f64; 2]] = &[" + ", ".join(f"[{float(a)!r}, {float(b)!r}]" for a, b in zip(x1, x2)) + "];")
print("pub const T: &[f64] = &[" + ", ".join(repr(float(v)) for v in t) + "];")
print("pub const FAILED: &[bool] = &[" + ", ".join("true" if f else "false" for f in failed) + "];")
print("/// (distribution, coefficients, standard errors, scale, its standard error, log-likelihood)")
print("pub const FITS: &[(&str, [f64; 3], [f64; 3], f64, f64, f64)] = &[")
pct = []
for dist in ("weibull", "lognormal", "exponential"):
    b, se, sc, scse, ll, th, cov = fit(X, np.log(t), failed, dist)
    print(f'    ("{dist}", [{", ".join(num(v) for v in b)}], [{", ".join(num(v) for v in se)}], {num(sc)}, {num(scse)}, {num(ll)}),')
    for at, level in (([1.0, 2.0, 0.0], 0.95), ([1.0, float(x1.mean()), float(x2.mean())], 0.9)):
        for p in (0.01, 0.1, 0.5, 0.9):
            pct.append((dist, at[1:], p, level, percentile(dist, th, cov, 3, at, p, level)))
print("];")
print("/// (distribution, covariates, fraction failed, level, [percentile, se of its log, lower, upper]) (T437)")
print("pub const PERCENTILES: &[(&str, [f64; 2], f64, f64, [f64; 4])] = &[")
for dist, at, p, level, r in pct:
    print(f'    ("{dist}", [{", ".join(num(v) for v in at)}], {p!r}, {level!r}, [{", ".join(num(v) for v in r)}]),')
print("];")
