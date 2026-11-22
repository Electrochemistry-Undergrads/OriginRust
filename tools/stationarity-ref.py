"""Prints crates/calc/tests/fixtures/stationarity_reference.rs: ADF and KPSS statistics for
`sheaf_calc::stationarity` (T372), from numpy.

ADF follows statsmodels' adfuller (lagmat with trim='both', AIC on a common sample of len(dx) - maxlag points,
maxlag = ceil(12 (n/100)^(1/4)) capped at n//2 - ntrend - 1, the final fit on all points the chosen order allows,
the trend counted from 1 in the regression sample), written out with numpy.linalg.lstsq and the t value from
(X'X)^-1. KPSS follows statsmodels' kpss with nlags='legacy' and its table interpolation (numpy.interp).

Usage: python3 tools/stationarity-ref.py > crates/calc/tests/fixtures/stationarity_reference.rs
None of these numbers come from Origin, and the critical values are not checked here.
"""
import math

import numpy as np

M = 2 ** 64


def uniforms(n, seed):
    st = seed
    out = []
    for _ in range(n):
        st = (st * 6364136223846793005 + 1442695040888963407) % M
        out.append(((st >> 11) + 0.5) / 2 ** 53)
    return np.array(out)


def ols(X, y):
    beta, *_ = np.linalg.lstsq(X, y, rcond=None)
    r = y - X @ beta
    ssr = float(r @ r)
    s2 = ssr / (len(y) - X.shape[1])
    se = np.sqrt(np.diag(np.linalg.inv(X.T @ X)) * s2)
    return beta, se, ssr


def design(x, p, m, terms):
    d = np.diff(x)
    rows = []
    for j, i in enumerate(range(len(d) - m, len(d))):
        r = [x[i]] + [d[i - l] for l in range(1, p + 1)]
        if terms != "n":
            r.append(1.0)
        if terms == "ct":
            r.append(j + 1.0)
        rows.append(r)
    return np.array(rows), d[len(d) - m :]


def adf(x, terms, lags=None):
    n = len(x)
    kdet = {"n": 0, "c": 1, "ct": 2}[terms]
    if lags is None:
        maxlag = min(math.ceil(12 * (n / 100) ** 0.25), n // 2 - kdet - 1)
        m = n - 1 - maxlag
        best = (math.inf, 0)
        for p in range(maxlag + 1):
            X, y = design(x, p, m, terms)
            _, _, ssr = ols(X, y)
            ll = -m / 2 * (math.log(2 * math.pi) + math.log(ssr / m) + 1)
            aic = -2 * ll + 2 * X.shape[1]
            if aic < best[0]:
                best = (aic, p)
        lags = best[1]
    m = n - 1 - lags
    X, y = design(x, lags, m, terms)
    beta, se, _ = ols(X, y)
    return beta[0] / se[0], lags, m


def kpss(x, null, lags=None):
    n = len(x)
    lags = min(math.ceil(12 * (n / 100) ** 0.25) if lags is None else lags, n - 1)
    if null == "c":
        e = x - x.mean()
        crit = [0.347, 0.463, 0.574, 0.739]
    else:
        X = np.column_stack([np.ones(n), np.arange(1, n + 1)])
        beta, *_ = np.linalg.lstsq(X, x, rcond=None)
        e = x - X @ beta
        crit = [0.119, 0.146, 0.176, 0.216]
    eta = (np.cumsum(e) ** 2).sum() / n ** 2
    s = (e ** 2).sum()
    for i in range(1, lags + 1):
        s += 2 * (1 - i / (lags + 1)) * (e[i:] @ e[:-i])
    stat = eta / (s / n)
    p = float(np.interp(stat, crit, [0.10, 0.05, 0.025, 0.01]))
    return stat, lags, p


walk = np.cumsum(uniforms(150, 5) - 0.5)
ar = np.zeros(120)
u = uniforms(120, 8) - 0.5
for t in range(1, 120):
    ar[t] = 0.5 * ar[t - 1] + u[t]
trend = 0.05 * np.arange(90) + (uniforms(90, 12) - 0.5)
series = [("random walk", walk), ("AR(1) 0.5", ar), ("trend plus noise", trend)]

print("// Made by tools/stationarity-ref.py (numpy). Do not edit.")
print("/// (name, series, [(terms, lags (None: AIC), statistic, used lags, nobs)], [(null, lags, statistic, used, p)])")
print("#[allow(clippy::type_complexity)]")
print("pub const CASES: &[(&str, &[f64], &[(&str, Option<usize>, f64, usize, usize)], &[(&str, Option<usize>, f64, usize, f64)])] = &[")
for name, x in series:
    adfs = []
    for terms in ("c", "ct", "n"):
        for lags in (None, 2):
            s, l, m = adf(x, terms, lags)
            adfs.append(f'("{terms}", {"None" if lags is None else f"Some({lags})"}, {float(s)!r}, {l}, {m})')
    kp = []
    for null in ("c", "ct"):
        for lags in (None, 4):
            s, l, p = kpss(x, null, lags)
            kp.append(f'("{null}", {"None" if lags is None else f"Some({lags})"}, {float(s)!r}, {l}, {p!r})')
    xs = "&[" + ", ".join(repr(float(v)) for v in x) + "]"
    print(f'    ("{name}", {xs}, &[{", ".join(adfs)}], &[{", ".join(kp)}]),')
print("];")
