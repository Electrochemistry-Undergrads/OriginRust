"""Prints crates/calc/tests/fixtures/turnbull_reference.rs: Turnbull's nonparametric estimate for arbitrarily
censored rows, for `sheaf_calc::turnbull` (T409), from numpy and scipy.

Independent of Sheaf's code: the innermost intervals come from the definition (a left end and a right end with no
other end strictly between them, tested pair by pair), the probabilities from a dense self-consistency iteration with
the full row-by-interval matrix run until the Kuhn-Tucker gap is below 1e-14, and the maximum log-likelihood is
confirmed by scipy.optimize.minimize (SLSQP on the simplex), which must not find a higher value.

Rows are (start, end): equal = exact, missing end = right-censored at the start, missing start = left-censored at the
end, otherwise failed in (start, end].

Usage: python3 tools/turnbull-ref.py > crates/calc/tests/fixtures/turnbull_reference.rs
None of these numbers come from Origin.
"""
import math

import numpy as np
from scipy import optimize

M = 2 ** 64
INF = math.inf
NAN = math.nan


def uniforms(n, seed):
    st = seed
    out = []
    for _ in range(n):
        st = (st * 6364136223846793005 + 1442695040888963407) % M
        out.append(((st >> 11) + 0.5) / 2 ** 53)
    return out


def keys(a, b):
    """Left and right key of a row: (time, 0 = at the time, 1 = just after)."""
    if not math.isnan(a) and not math.isnan(b) and a == b:
        return (a, 0), (b, 0)
    if math.isnan(b):
        return (a, 1), (INF, 0)
    if math.isnan(a):
        return (-INF, 0), (b, 0)
    return (a, 1), (b, 0)


def estimate(rows):
    ks = [keys(a, b) for a, b in rows]
    lefts = sorted({k[0] for k in ks})
    rights = sorted({k[1] for k in ks})
    inner = []
    for l in lefts:
        for r in rights:
            if r < l:
                continue
            if any(l < k <= r for k in lefts) or any(l <= k < r for k in rights):
                continue
            inner.append((l, r))
    inner.sort()
    A = np.array([[1.0 if L <= l and r <= R else 0.0 for (l, r) in inner] for (L, R) in ks])
    n, m = A.shape
    s = np.full(m, 1.0 / m)
    for _ in range(2_000_000):
        g = A.T @ (1.0 / (A @ s)) / n
        if g.max() - 1.0 < 1e-14:
            break
        s = s * g
        s /= s.sum()
    else:
        raise SystemExit("no convergence")
    ll = float(np.log(A @ s).sum())
    # SLSQP from the uniform start may not beat it.
    f = lambda x: -np.log(np.maximum(A @ x, 1e-300)).sum()
    jac = lambda x: -A.T @ (1.0 / np.maximum(A @ x, 1e-300))
    r = optimize.minimize(f, np.full(m, 1.0 / m), jac=jac, method="SLSQP", bounds=[(0, 1)] * m,
                          constraints=[{"type": "eq", "fun": lambda x: x.sum() - 1.0}],
                          options={"ftol": 1e-15, "maxiter": 1000})
    assert -r.fun <= ll + 1e-8, (-r.fun, ll)
    assert abs(-r.fun - ll) < 1e-5, (-r.fun, ll)
    surv = 1.0 - np.cumsum(s)
    return inner, s, np.maximum(surv, 0.0), ll


def num(v):
    v = float(v)
    if math.isnan(v):
        return "f64::NAN"
    if v == INF:
        return "f64::INFINITY"
    if v == -INF:
        return "f64::NEG_INFINITY"
    return repr(v)


def vec(v):
    return "&[" + ", ".join(num(a) for a in v) + "]"


def inspection_case():
    u = uniforms(400, 21)
    rows = []
    k = 0
    for i in range(80):
        life = 10.0 * (-math.log(u[k])) ** (1 / 1.5)
        k += 1
        if i < 50:  # inspected every 2 hours up to 20
            if life > 20:
                rows.append((20.0, NAN))
            elif life <= 2:
                rows.append((NAN, 2.0))
            else:
                hi = 2.0 * math.ceil(life / 2.0)
                rows.append((hi - 2.0, hi))
        elif i < 70:  # watched, times to half an hour, some withdrawn
            c = round(8 + 20 * u[k], 0)
            k += 1
            t = round(life * 2) / 2
            rows.append((c, NAN) if c < t else (t, t))
        else:  # own irregular windows
            a = round(life * (0.3 + 0.6 * u[k]), 1)
            b = round(life * (1.05 + 0.8 * u[k + 1]), 1)
            k += 2
            rows.append((a, b) if a < b else (a, a))
    return rows


small = [(1.0, 1.0), (NAN, 2.0), (1.0, 3.0), (2.0, 4.0), (2.0, NAN), (3.0, 3.0), (3.0, 6.0), (4.0, 5.0),
         (5.0, NAN), (NAN, 4.0), (6.0, 6.0), (2.5, 7.0)]
cases = [("small, every kind", small), ("inspections every 2 h, watched and irregular", inspection_case())]
print("// Made by tools/turnbull-ref.py (numpy, scipy). Do not edit.")
print("pub struct Case {")
for f in ["name: &'static str", "start: &'static [f64]", "end: &'static [f64]", "lower: &'static [f64]",
          "upper: &'static [f64]", "lower_closed: &'static [bool]", "probability: &'static [f64]",
          "survival: &'static [f64]", "log_likelihood: f64"]:
    print(f"    pub {f},")
print("}")
print("pub const CASES: &[Case] = &[")
for name, rows in cases:
    inner, s, surv, ll = estimate(rows)
    print("    Case {")
    print(f'        name: "{name}",')
    print(f"        start: {vec([a for a, _ in rows])},")
    print(f"        end: {vec([b for _, b in rows])},")
    print(f"        lower: {vec([l[0] for l, _ in inner])},")
    print(f"        upper: {vec([r[0] for _, r in inner])},")
    print("        lower_closed: &[" + ", ".join("true" if l[1] == 0 and math.isfinite(l[0]) else "false" for l, _ in inner) + "],")
    print(f"        probability: {vec(s)},")
    print(f"        survival: {vec(surv)},")
    print(f"        log_likelihood: {num(ll)},")
    print("    },")
print("];")
