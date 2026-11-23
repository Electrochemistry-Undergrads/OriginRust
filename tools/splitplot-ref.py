"""Prints crates/calc/tests/fixtures/splitplot_reference.rs: two-stratum split-plot effects for
`sheaf_calc::doe::split_plot_effects` (T382), from numpy and scipy.

The design (standard order: replicates, then whole-plot combinations with A fastest, then subplot combinations) and the
response (a fixed model plus a whole-plot random effect and noise from a fixed generator) are built here. The full
model (intercept, mains, two-factor interactions) is fitted with numpy.linalg.lstsq; the whole-plot error is b times
the residual sum of squares of the whole-plot means on the whole-plot terms; the subplot error is the rest; standard
errors sqrt(4 MS / N), p-values from scipy.stats.t; Lenth's margin per stratum.

Usage: python3 tools/splitplot-ref.py > crates/calc/tests/fixtures/splitplot_reference.rs
None of these numbers come from Origin.
"""
import itertools
import math

import numpy as np
from scipy import stats

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


def design(wp, sp, reps):
    plots, rows = [], []
    lv = lambda i, j: 1.0 if (i >> j) & 1 else -1.0
    w = 0
    for _ in range(reps):
        for i in range(2 ** wp):
            w += 1
            for k in range(2 ** sp):
                rows.append([lv(i, j) for j in range(wp)] + [lv(k, j) for j in range(sp)])
                plots.append(w)
    return np.array(rows), np.array(plots)


def lenth(eff):
    a = np.abs(np.array(eff))
    if len(a) == 0:
        return math.nan
    s0 = 1.5 * np.median(a)
    pse = 1.5 * np.median(a[a < 2.5 * s0])
    return stats.t.ppf(0.975, len(a) / 3) * pse


def analyze(x, y, plots, wp):
    n, k = x.shape
    letters = "ABCDEFGH"
    terms = [(letters[j], x[:, j]) for j in range(k)]
    terms += [(letters[a] + letters[b], x[:, a] * x[:, b]) for a, b in itertools.combinations(range(k), 2)]
    X = np.column_stack([np.ones(n)] + [t[1] for t in terms])
    beta, *_ = np.linalg.lstsq(X, y, rcond=None)
    df_full = n - X.shape[1]
    sse_full = float(((y - X @ beta) ** 2).sum())
    whole = [all(letters.index(c) < wp for c in t[0]) for t in terms]
    ids = sorted(set(plots))
    b = int((plots == ids[0]).sum())
    means = np.array([y[plots == i].mean() for i in ids])
    first = [np.where(plots == i)[0][0] for i in ids]
    W = np.column_stack([np.ones(len(ids))] + [t[1][first] for t, w in zip(terms, whole) if w])
    bw, *_ = np.linalg.lstsq(W, means, rcond=None)
    wp_ss = b * float(((means - W @ bw) ** 2).sum())
    wp_df = len(ids) - W.shape[1]
    sp_df = df_full - wp_df
    sp_ss = max(sse_full - wp_ss, 0.0)
    out = []
    for i, ((name, _), w) in enumerate(zip(terms, whole)):
        eff = 2 * beta[i + 1]
        ss, df = (wp_ss, wp_df) if w else (sp_ss, sp_df)
        se = math.sqrt(4 * ss / df / n)
        t = eff / se
        p = 2 * stats.t.sf(abs(t), df)
        out.append((name, w, eff, se, t, p))
    wpm = lenth([o[2] for o in out if o[1]])
    spm = lenth([o[2] for o in out if not o[1]])
    return out, (wp_ss, wp_df, wpm), (sp_ss, sp_df, spm)


def num(v):
    v = float(v)
    return "f64::NAN" if math.isnan(v) else repr(v)


cases = []
x, plots = design(2, 2, 2)
z = normals(8, 3) * 0.6
e = normals(len(x), 7) * 0.3
y = 20 + 1.5 * x[:, 0] - 0.8 * x[:, 1] + 1.2 * x[:, 2] + 0.5 * x[:, 0] * x[:, 2] + z[plots - 1] + e
cases.append(("2 whole-plot, 2 subplot factors, 2 replicates", x, y, plots, 2))
x, plots = design(1, 3, 3)
z = normals(6, 11) * 0.4
e = normals(len(x), 13) * 0.2
y = 7 + 0.9 * x[:, 0] + 0.6 * x[:, 1] - 0.4 * x[:, 3] + z[plots - 1] + e
cases.append(("1 whole-plot, 3 subplot factors, 3 replicates", x, y, plots, 1))

print("// Made by tools/splitplot-ref.py (numpy, scipy). Do not edit.")
print("pub struct Case {")
print("    pub name: &'static str,")
print("    pub x: &'static [&'static [f64]],")
print("    pub y: &'static [f64],")
print("    pub plot: &'static [usize],")
print("    pub wp: usize,")
print("    /// (term, whole-plot?, effect, se, t, p)")
print("    pub effects: &'static [(&'static str, bool, f64, f64, f64, f64)],")
print("    /// (ss, df, Lenth margin) of the whole-plot and the subplot stratum")
print("    pub whole_plot: (f64, usize, f64),")
print("    pub subplot: (f64, usize, f64),")
print("}")
print("pub const CASES: &[Case] = &[")
for name, x, y, plots, wp in cases:
    out, wps, sps = analyze(x, y, plots, wp)
    print("    Case {")
    print(f'        name: "{name}",')
    print("        x: &[" + ", ".join("&[" + ", ".join(repr(float(v)) for v in r) + "]" for r in x) + "],")
    print("        y: &[" + ", ".join(repr(float(v)) for v in y) + "],")
    print("        plot: &[" + ", ".join(str(int(p)) for p in plots) + "],")
    print(f"        wp: {wp},")
    print("        effects: &[" + ", ".join(f'("{o[0]}", {"true" if o[1] else "false"}, {num(o[2])}, {num(o[3])}, {num(o[4])}, {num(o[5])})' for o in out) + "],")
    print(f"        whole_plot: ({num(wps[0])}, {wps[1]}, {num(wps[2])}),")
    print(f"        subplot: ({num(sps[0])}, {sps[1]}, {num(sps[2])}),")
    print("    },")
print("];")
