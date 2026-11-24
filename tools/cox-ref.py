"""Prints crates/calc/tests/fixtures/cox_reference.rs: Cox proportional hazards fits (Breslow ties) for
`sheaf_calc::cox` (T406), from numpy and scipy.

The partial log-likelihood, its gradient and information are written out with explicit risk sets (O(n^2), no
sorting tricks), maximized by scipy.optimize.minimize (Newton-CG with the analytic gradient and Hessian, tight
tolerances); the tests (LR, Wald, score) use scipy.stats.chi2 and the baseline survival Breslow's estimator at the
covariate means. lifelines and statsmodels are not available here.

Usage: python3 tools/cox-ref.py > crates/calc/tests/fixtures/cox_reference.rs
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
    return np.array(out)


def pl(b, X, t, d):
    eta = X @ b
    w = np.exp(eta)
    ll, g, H = 0.0, np.zeros(len(b)), np.zeros((len(b), len(b)))
    for i in np.where(d)[0]:
        R = t >= t[i]
        s0 = w[R].sum()
        s1 = (w[R, None] * X[R]).sum(axis=0)
        s2 = (w[R, None, None] * X[R][:, :, None] * X[R][:, None, :]).sum(axis=0)
        ll += eta[i] - math.log(s0)
        m = s1 / s0
        g += X[i] - m
        H += s2 / s0 - np.outer(m, m)
    return ll, g, H


def fit(X, t, d):
    p = X.shape[1]
    f = lambda b: -pl(b, X, t, d)[0]
    jac = lambda b: -pl(b, X, t, d)[1]
    hess = lambda b: pl(b, X, t, d)[2]
    r = optimize.minimize(f, np.zeros(p), jac=jac, hess=hess, method="Newton-CG", options={"xtol": 1e-14, "maxiter": 500})
    b = r.x
    ll, g, H = pl(b, X, t, d)
    ll0, g0, H0 = pl(np.zeros(p), X, t, d)
    cov = np.linalg.inv(H)
    se = np.sqrt(np.diag(cov))
    z = b / se
    pv = 2 * stats.norm.sf(np.abs(z))
    lr = 2 * (ll - ll0)
    wald = b @ H @ b
    score = g0 @ np.linalg.inv(H0) @ g0
    chi = lambda s: stats.chi2.sf(s, p)
    eta_mean = X.mean(axis=0) @ b
    w = np.exp(X @ b)
    times, surv, h0 = [], [], 0.0
    for tk in np.unique(t[d]):
        dk = ((t == tk) & d).sum()
        h0 += dk / w[t >= tk].sum()
        times.append(tk)
        surv.append(math.exp(-h0 * math.exp(eta_mean)))
    return b, se, z, pv, ll, ll0, (lr, chi(lr)), (wald, chi(wald)), (score, chi(score)), times, surv


def vec(v):
    return "&[" + ", ".join(repr(float(a)) for a in v) + "]"


n = 70
u = uniforms(4 * n, 9)
age = np.round(40 + 30 * u[:n], 1)
treated = np.array([float(i % 2) for i in range(n)])
dose = np.round(u[n : 2 * n] * 3, 2)
life = -np.log(u[2 * n : 3 * n]) / (0.02 * np.exp(0.03 * (age - 55) - 0.7 * treated + 0.2 * dose))
cens = 20 + 60 * u[3 * n :]
t = np.round(np.minimum(life, cens), 1)  # rounding makes ties
d = life <= cens
cases = [("age, treated, dose", np.column_stack([age, treated, dose])), ("treated only", treated[:, None])]
print("// Made by tools/cox-ref.py (numpy, scipy). Do not edit.")
print("pub const T: &[f64] = " + vec(t) + ";")
print("pub const FAILED: &[bool] = &[" + ", ".join("true" if v else "false" for v in d) + "];")
print("pub const AGE: &[f64] = " + vec(age) + ";")
print("pub const TREATED: &[f64] = " + vec(treated) + ";")
print("pub const DOSE: &[f64] = " + vec(dose) + ";")
print("pub struct Case {")
print("    pub name: &'static str,")
print("    pub b: &'static [f64],")
print("    pub se: &'static [f64],")
print("    pub z: &'static [f64],")
print("    pub p: &'static [f64],")
print("    pub ll: f64,")
print("    pub ll0: f64,")
print("    pub tests: [(f64, f64); 3],")
print("    pub times: &'static [f64],")
print("    pub survival: &'static [f64],")
print("}")
print("pub const CASES: &[Case] = &[")
for name, X in cases:
    b, se, z, pv, ll, ll0, lr, wald, score, times, surv = fit(X, t, d)
    print("    Case {")
    print(f'        name: "{name}",')
    print(f"        b: {vec(b)},")
    print(f"        se: {vec(se)},")
    print(f"        z: {vec(z)},")
    print(f"        p: {vec(pv)},")
    print(f"        ll: {float(ll)!r},")
    print(f"        ll0: {float(ll0)!r},")
    print(f"        tests: [({float(lr[0])!r}, {float(lr[1])!r}), ({float(wald[0])!r}, {float(wald[1])!r}), ({float(score[0])!r}, {float(score[1])!r})],")
    print(f"        times: {vec(times)},")
    print(f"        survival: {vec(surv)},")
    print("    },")
print("];")
