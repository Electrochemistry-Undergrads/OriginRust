"""Prints crates/calc/tests/fixtures/doe_reference.rs: two-level factorial effects for `sheaf_calc::doe` (T380),
from numpy and scipy.

Each design is built here from its generators (standard order, A fastest), the response from a fixed generator; the
factors are coded to -1..+1, the model (intercept, mains, two-factor interactions not aliased with an earlier term,
at most n - 1 terms) is fitted with numpy.linalg.lstsq, standard errors from (X'X)^-1, p-values from scipy.stats.t,
and Lenth's PSE and margin of error as in Lenth (1989).

Usage: python3 tools/doe-ref.py > crates/calc/tests/fixtures/doe_reference.rs
None of these numbers come from Origin.
"""
import itertools
import math

import numpy as np
from scipy import stats

M = 2 ** 64


def uniforms(n, seed):
    st = seed
    out = []
    for _ in range(n):
        st = (st * 6364136223846793005 + 1442695040888963407) % M
        out.append(((st >> 11) + 0.5) / 2 ** 53)
    return np.array(out)


def build(k, gens, reps):
    base = k - len(gens)
    rows = []
    for _ in range(reps):
        for i in range(2 ** base):
            r = [1.0 if (i >> j) & 1 else -1.0 for j in range(base)]
            for g in gens:
                r.append(float(np.prod([r[j] for j in g])))
            rows.append(r)
    return np.array(rows)


def effects(x, y):
    n, k = x.shape
    lo, hi = x.min(axis=0), x.max(axis=0)
    c = (x - (lo + hi) / 2) / ((hi - lo) / 2)
    letters = "ABCDEFGHIJ"
    cand = [(letters[j], c[:, j]) for j in range(k)]
    cand += [(letters[a] + letters[b], c[:, a] * c[:, b]) for a, b in itertools.combinations(range(k), 2)]
    terms, aliased = [], []
    for name, col in cand:
        hit = None
        for other, prev in terms:
            nn = np.linalg.norm(col) * np.linalg.norm(prev)
            if nn > 0 and abs(abs(col @ prev) - nn) <= 1e-9 * nn:
                hit = other
                break
        if hit is not None:
            aliased.append((name, hit))
            continue
        if np.all(np.abs(col) < 1e-12) or len(terms) + 1 >= n:
            continue
        terms.append((name, col))
    X = np.column_stack([np.ones(n)] + [t[1] for t in terms])
    beta, *_ = np.linalg.lstsq(X, y, rcond=None)
    df = n - X.shape[1]
    sse = float(((y - X @ beta) ** 2).sum())
    inv = np.linalg.inv(X.T @ X)
    out = []
    for i, (name, _) in enumerate(terms):
        cf = beta[i + 1]
        if df > 0:
            se = math.sqrt(sse / df * inv[i + 1, i + 1])
            t = cf / se
            p = 2 * stats.t.sf(abs(t), df)
        else:
            se = t = p = math.nan
        out.append((name, cf, 2 * cf, 2 * se, t, p))
    eff = np.abs(np.array([o[2] for o in out]))
    s0 = 1.5 * np.median(eff)
    pse = 1.5 * np.median(eff[eff < 2.5 * s0])
    margin = stats.t.ppf(0.975, len(out) / 3) * pse
    return beta[0], out, aliased, df, pse, margin


def num(v):
    v = float(v)
    return "f64::NAN" if math.isnan(v) else repr(v)


cases = []
d = build(3, [], 2)
x = np.column_stack([30 + 10 * d[:, 0], 2 + d[:, 1], 150 + 50 * d[:, 2]])
y = 10 + 2 * d[:, 0] - 1.5 * d[:, 1] + 0.5 * d[:, 0] * d[:, 2] + 0.4 * (uniforms(16, 3) - 0.5)
cases.append(("replicated 2^3", x, y))
d = build(4, [], 1)
y = 50 + 4 * d[:, 0] + 3 * d[:, 2] + 2.5 * d[:, 0] * d[:, 2] + (uniforms(16, 5) - 0.5)
cases.append(("unreplicated 2^4", d, y))
d = build(5, [[0, 1, 2, 3]], 1)
y = 20 + 1.5 * d[:, 1] - 2 * d[:, 4] + 0.6 * (uniforms(16, 9) - 0.5)
cases.append(("2^(5-1)", d, y))
d = build(4, [[0, 1, 2]], 1)
y = 5 + d[:, 0] + 0.5 * d[:, 3] + 0.2 * (uniforms(8, 11) - 0.5)
cases.append(("2^(4-1)", d, y))

print("// Made by tools/doe-ref.py (numpy, scipy). Do not edit.")
print("pub struct Case {")
print("    pub name: &'static str,")
print("    pub x: &'static [&'static [f64]],")
print("    pub y: &'static [f64],")
print("    pub intercept: f64,")
print("    /// (term, coefficient, effect, se of the effect, t, p)")
print("    pub effects: &'static [(&'static str, f64, f64, f64, f64, f64)],")
print("    pub aliased: &'static [(&'static str, &'static str)],")
print("    pub df: usize,")
print("    pub pse: f64,")
print("    pub margin: f64,")
print("}")
print("pub const CASES: &[Case] = &[")
for name, x, y in cases:
    b0, out, al, df, pse, margin = effects(np.asarray(x, dtype=float), np.asarray(y, dtype=float))
    print("    Case {")
    print(f'        name: "{name}",')
    print("        x: &[" + ", ".join("&[" + ", ".join(repr(float(v)) for v in r) + "]" for r in np.asarray(x)) + "],")
    print("        y: &[" + ", ".join(repr(float(v)) for v in y) + "],")
    print(f"        intercept: {num(b0)},")
    print("        effects: &[" + ", ".join(f'("{o[0]}", {num(o[1])}, {num(o[2])}, {num(o[3])}, {num(o[4])}, {num(o[5])})' for o in out) + "],")
    print("        aliased: &[" + ", ".join(f'("{a}", "{b}")' for a, b in al) + "],")
    print(f"        df: {df},")
    print(f"        pse: {num(pse)},")
    print(f"        margin: {num(margin)},")
    print("    },")
print("];")
