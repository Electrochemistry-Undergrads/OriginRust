"""Prints crates/calc/tests/fixtures/cox_reference.rs: Cox proportional hazards fits (Breslow ties) for
`sheaf_calc::cox` (T408), from numpy and scipy.

The partial log-likelihood, its gradient and information are written out with explicit risk sets (O(n^2), no
sorting tricks), maximized by scipy.optimize.minimize (Newton-CG with the analytic gradient and Hessian, tight
tolerances); the tests (LR, Wald, score) use scipy.stats.chi2 and the baseline survival Breslow's estimator at the
covariate means. lifelines and statsmodels are not available here. Efron's ties (T445): at a time with tied failures
D the l-th of them (l = 0..d-1) sees the risk set minus l/d of D's sums, written out failure time by failure time;
the baseline hazard increment is sum_l 1 / (S - l/d S_D). Strata (T451): risk sets within each stratum, the
likelihood summed over strata, one baseline curve per stratum. Residuals (T457): martingale d_i - H0(t_i) exp(x_i'b)
with the stratum's baseline cumulative hazard at x = 0, deviance sign(M) sqrt(-2 (M + d ln(d - M))), Schoenfeld x_i
minus the risk-set mean (Efron: the mean of the stepped means), each written out case by case. The proportional-
hazards test (T458): g = mean rank of the failure time (scipy.stats.rankdata), U = sum (g - mean g) r, V = cov(b), per
covariate d (V U)_j^2 / (V_jj S) and global d U'VU / S with S = sum (g - mean g)^2, chi-square from scipy.

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


def pl(b, X, t, d, efron=False, st=None):
    st = np.zeros(len(t), dtype=int) if st is None else st
    eta = X @ b
    w = np.exp(eta)
    ll, g, H = 0.0, np.zeros(len(b)), np.zeros((len(b), len(b)))
    for s, tk in sorted({(int(s), float(tk)) for s, tk in zip(st[d], t[d])}):
        R = (t >= tk) & (st == s)
        D = (t == tk) & d & (st == s)
        nd = int(D.sum())
        s0 = w[R].sum()
        s1 = (w[R, None] * X[R]).sum(axis=0)
        s2 = (w[R, None, None] * X[R][:, :, None] * X[R][:, None, :]).sum(axis=0)
        a0 = w[D].sum()
        a1 = (w[D, None] * X[D]).sum(axis=0)
        a2 = (w[D, None, None] * X[D][:, :, None] * X[D][:, None, :]).sum(axis=0)
        ll += eta[D].sum()
        g += X[D].sum(axis=0)
        for l in range(nd):
            f = l / nd if efron else 0.0
            den = s0 - f * a0
            m = (s1 - f * a1) / den
            ll -= math.log(den)
            g -= m
            H += (s2 - f * a2) / den - np.outer(m, m)
    return ll, g, H


def fit(X, t, d, efron=False, st=None):
    p = X.shape[1]
    st = np.zeros(len(t), dtype=int) if st is None else st
    f = lambda b: -pl(b, X, t, d, efron, st)[0]
    jac = lambda b: -pl(b, X, t, d, efron, st)[1]
    hess = lambda b: pl(b, X, t, d, efron, st)[2]
    r = optimize.minimize(f, np.zeros(p), jac=jac, hess=hess, method="Newton-CG", options={"xtol": 1e-14, "maxiter": 500})
    b = r.x
    ll, g, H = pl(b, X, t, d, efron, st)
    ll0, g0, H0 = pl(np.zeros(p), X, t, d, efron, st)
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
    curves = []
    for s in sorted(set(int(v) for v in st)):
        times, surv, h0 = [], [], 0.0
        for tk in np.unique(t[d & (st == s)]):
            D = (t == tk) & d & (st == s)
            dk = int(D.sum())
            S = w[(t >= tk) & (st == s)].sum()
            h0 += sum(1 / (S - (l / dk if efron else 0.0) * w[D].sum()) for l in range(dk))
            times.append(tk)
            surv.append(math.exp(-h0 * math.exp(eta_mean)))
        curves.append((times, surv))
    # Residuals, case by case.
    n = len(t)
    H = np.zeros(n)
    schoen = []
    for i in range(n):
        s = st[i]
        tot = 0.0
        for tk in np.unique(t[d & (st == s)]):
            if tk > t[i]:
                break
            D = (t == tk) & d & (st == s)
            dk = int(D.sum())
            S = w[(t >= tk) & (st == s)].sum()
            tot += sum(1 / (S - (l / dk if efron else 0.0) * w[D].sum()) for l in range(dk))
        H[i] = tot
    mart = d.astype(float) - H * w
    dev = []
    for i in range(n):
        inner = mart[i] + (math.log(1 - mart[i]) if d[i] else 0.0)
        dev.append(math.copysign(math.sqrt(max(-2 * inner, 0.0)), mart[i]))
    for i in sorted(np.where(d)[0], key=lambda i: (st[i], t[i], i)):
        s = st[i]
        R = (t >= t[i]) & (st == s)
        D = (t == t[i]) & d & (st == s)
        dk = int(D.sum())
        S0, S1 = w[R].sum(), (w[R, None] * X[R]).sum(axis=0)
        A0, A1 = w[D].sum(), (w[D, None] * X[D]).sum(axis=0)
        if efron:
            mean = sum((S1 - l / dk * A1) / (S0 - l / dk * A0) for l in range(dk)) / dk
        else:
            mean = S1 / S0
        schoen.append((int(i), list(X[i] - mean)))
    ftimes = np.array([t[i] for i, _ in schoen])
    g = stats.rankdata(ftimes)
    gc = g - g.mean()
    S = float((gc ** 2).sum())
    U = sum(gk * np.array(r) for gk, (_, r) in zip(gc, schoen))
    VU = cov @ U
    nd = len(schoen)
    per = [(float(nd * VU[j] ** 2 / (cov[j, j] * S)), float(stats.chi2.sf(nd * VU[j] ** 2 / (cov[j, j] * S), 1))) for j in range(p)]
    glob = float(nd * U @ VU / S)
    ph = (per, (glob, float(stats.chi2.sf(glob, p))))
    return b, se, z, pv, ll, ll0, (lr, chi(lr)), (wald, chi(wald)), (score, chi(score)), curves, list(mart), dev, schoen, ph


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
X3 = np.column_stack([age, treated, dose])
X2 = np.column_stack([treated, dose])
band = np.where(age < 50, 0, np.where(age < 60, 1, 2))
cases = [
    ("age, treated, dose", X3, False, None),
    ("treated only", treated[:, None], False, None),
    ("age, treated, dose (Efron)", X3, True, None),
    ("treated only (Efron)", treated[:, None], True, None),
    ("treated, dose by age band", X2, False, band),
    ("treated, dose by age band (Efron)", X2, True, band),
]
print("// Made by tools/cox-ref.py (numpy, scipy). Do not edit.")
print("pub const T: &[f64] = " + vec(t) + ";")
print("pub const FAILED: &[bool] = &[" + ", ".join("true" if v else "false" for v in d) + "];")
print("pub const AGE: &[f64] = " + vec(age) + ";")
print("pub const TREATED: &[f64] = " + vec(treated) + ";")
print("pub const DOSE: &[f64] = " + vec(dose) + ";")
print("pub const BAND: &[usize] = &[" + ", ".join(str(int(v)) for v in band) + "];")
print("pub struct Case {")
print("    pub name: &'static str,")
print("    pub efron: bool,")
print("    pub stratified: bool,")
print("    pub b: &'static [f64],")
print("    pub se: &'static [f64],")
print("    pub z: &'static [f64],")
print("    pub p: &'static [f64],")
print("    pub ll: f64,")
print("    pub ll0: f64,")
print("    pub tests: [(f64, f64); 3],")
print("    pub times: &'static [f64],")
print("    pub survival: &'static [f64],")
print("    pub curves: &'static [(&'static [f64], &'static [f64])],")
print("    pub martingale: &'static [f64],")
print("    pub deviance: &'static [f64],")
print("    pub schoenfeld: &'static [(usize, &'static [f64])],")
print("    pub ph: &'static [(f64, f64)],")
print("    pub ph_global: (f64, f64),")
print("}")
print("pub const CASES: &[Case] = &[")
for name, X, efron, st in cases:
    b, se, z, pv, ll, ll0, lr, wald, score, curves, mart, dev, schoen, ph = fit(X, t, d, efron, st)
    times, surv = curves[0]
    print("    Case {")
    print(f'        name: "{name}",')
    print(f"        efron: {'true' if efron else 'false'},")
    print(f"        stratified: {'false' if st is None else 'true'},")
    print(f"        b: {vec(b)},")
    print(f"        se: {vec(se)},")
    print(f"        z: {vec(z)},")
    print(f"        p: {vec(pv)},")
    print(f"        ll: {float(ll)!r},")
    print(f"        ll0: {float(ll0)!r},")
    print(f"        tests: [({float(lr[0])!r}, {float(lr[1])!r}), ({float(wald[0])!r}, {float(wald[1])!r}), ({float(score[0])!r}, {float(score[1])!r})],")
    print(f"        times: {vec(times)},")
    print(f"        survival: {vec(surv)},")
    print("        curves: &[" + ", ".join(f"({vec(a)}, {vec(c)})" for a, c in curves) + "],")
    print(f"        martingale: {vec(mart)},")
    print(f"        deviance: {vec(dev)},")
    print("        schoenfeld: &[" + ", ".join(f"({i}, {vec(r)})" for i, r in schoen) + "],")
    print("        ph: &[" + ", ".join(f"({a!r}, {c!r})" for a, c in ph[0]) + "],")
    print(f"        ph_global: ({ph[1][0]!r}, {ph[1][1]!r}),")
    print("    },")
print("];")
