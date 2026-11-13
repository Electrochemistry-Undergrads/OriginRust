#!/usr/bin/env python3
"""Prints crates/calc/tests/fixtures/scipy_stats_compare.rs: reference numbers from scipy for the
means comparisons Holm and Dunnett, the cross tabulation and the 2D binning.

    python3 tools/scipy_stats_compare.py > crates/calc/tests/fixtures/scipy_stats_compare.rs

These numbers are scipy's, NOT Origin's (Origin is not available on the machine that wrote them). Written
against scipy 1.18.1 and numpy 2.5. The inputs are made by a seeded generator and rounded to 3 digits, so the
output is the same on every run, except for scipy's own Monte Carlo noise in `dunnett`, which is fixed by
the seed given to it.

What each block checks:
  DUNNETT  scipy.stats.dunnett: statistic (exact), p-values and confidence intervals (scipy integrates the
           multivariate t by quasi Monte Carlo, absolute error about 1e-4, much worse relative to p below
           1e-5), plus an independent deterministic evaluation of the same integral with nested
           scipy.integrate.quad (error below 1e-11), which pins our Gauss-Legendre integral to 1e-9.
  HOLM     scipy has no Holm adjustment. The raw p-values are scipy.stats.t.sf of the pairwise t statistics
           with the ANOVA mean square, the step-down adjustment is written out here from its definition.
  CROSSTAB scipy.stats.chi2_contingency (Pearson, with correction=False; Yates with correction=True;
           lambda_="log-likelihood" for the likelihood ratio) and scipy.stats.fisher_exact.
  BIN2D    scipy.stats.binned_statistic_2d(statistic="count"); the edges are scipy's for bins given as a
           number, and written out here (start + width * i) for bins given as a width.
"""
import itertools
import math

import numpy as np
from scipy import integrate, optimize, stats

assert stats.__name__  # scipy is required


def f(x):
    x = float(x)
    if math.isnan(x):
        return "f64::NAN"
    if math.isinf(x):
        return "f64::INFINITY" if x > 0 else "f64::NEG_INFINITY"
    return repr(x)


def arr(xs):
    return "&[" + ", ".join(f(x) for x in xs) + "]"


def arr2(rows):
    return "&[" + ", ".join(arr(r) for r in rows) + "]"


def iarr2(rows):
    return "&[" + ", ".join("&[" + ", ".join(str(int(v)) for v in r) + "]" for r in rows) + "]"


rng = np.random.default_rng(20261113)


def sample(mean, sd, n):
    return list(np.round(rng.normal(mean, sd, n), 3))


# ---------------------------------------------------------------------------------------------------------
# Dunnett
# ---------------------------------------------------------------------------------------------------------
_GL_X, _GL_W = np.polynomial.legendre.leggauss(64)


def _gl(a, b, panels):
    """Nodes and weights of a composite 64-point Gauss-Legendre rule on [a, b]."""
    edges = np.linspace(a, b, panels + 1)
    xs, ws = [], []
    for lo, hi in zip(edges[:-1], edges[1:]):
        xs.append(0.5 * (hi - lo) * _GL_X + 0.5 * (hi + lo))
        ws.append(0.5 * (hi - lo) * _GL_W)
    return np.concatenate(xs), np.concatenate(ws)


def dunnett_ref_cdf(c, loadings, df):
    """P(max |T_j| <= c): the double integral of the one-factor form with a composite 64-point rule on
    fine panels, vectorised (different nodes and a much finer rule than the Rust code: error below 1e-12)."""
    l = np.asarray(loadings)[:, None, None]
    r = np.sqrt(1 - l * l)
    xs, wx = _gl(-12, 12, 48)
    sd = 12 / math.sqrt(2 * df)
    lo, hi = max(0.0, 1 - sd), 1 + sd + (6 if df < 4 else 0)
    ss, ws = _gl(lo, hi, 24)
    cs = c * ss[None, :, None]
    br = stats.norm.cdf((cs - l * xs[None, None, :]) / r) - stats.norm.cdf((-cs - l * xs[None, None, :]) / r)
    g = np.prod(br, axis=0) * stats.norm.pdf(xs)[None, :]  # (s, x)
    inner = g @ wx
    dens = stats.chi.pdf(ss * math.sqrt(df), df) * math.sqrt(df)
    return float(np.sum(ws * dens * inner))


def dunnett_case(name, control, groups, seed):
    r = stats.dunnett(*groups, control=control, rng=seed)
    ci = r.confidence_interval(0.95)
    nc = len(control)
    ns = [len(g) for g in groups]
    df = sum(ns) + nc - len(groups) - 1
    loadings = [math.sqrt(n / (n + nc)) for n in ns]
    p_ref = [1 - dunnett_ref_cdf(abs(t), loadings, df) for t in r.statistic]
    crit = optimize.brentq(lambda c: dunnett_ref_cdf(c, loadings, df) - 0.95, 1.0, 8.0, xtol=1e-13)
    return (
        f'    DunnettCase {{ name: "{name}", control: {arr(control)}, groups: &[{", ".join(arr(g) for g in groups)}],\n'
        f"        statistic: {arr(r.statistic)},\n"
        f"        p_scipy: {arr(r.pvalue)},\n"
        f"        ci_low_scipy: {arr(ci.low)}, ci_high_scipy: {arr(ci.high)},\n"
        f"        p_ref: {arr(p_ref)}, critical_ref: {f(crit)} }},"
    )


dunnett_cases = [
    dunnett_case("equal_n_3_treatments", sample(25, 2, 6), [sample(28, 2, 6), sample(25, 2, 6), sample(31, 2, 6)], 1),
    dunnett_case("unequal_n", sample(25, 2, 5), [sample(28, 2, 5), sample(25, 2, 6), sample(33, 2, 4)], 2),
    dunnett_case("one_treatment", sample(10, 1, 8), [sample(11, 1, 7)], 3),
    dunnett_case("five_treatments_small_df", sample(5, 1, 3), [sample(5 + k, 1, 3) for k in range(5)], 4),
    dunnett_case("small_control_large_treatments", sample(50, 5, 3), [sample(52, 5, 12), sample(58, 5, 15)], 5),
    dunnett_case("large_df", sample(0, 1, 40), [sample(0.3, 1, 40), sample(0.8, 1, 40), sample(-0.2, 1, 40)], 6),
]

# ---------------------------------------------------------------------------------------------------------
# Holm
# ---------------------------------------------------------------------------------------------------------
def holm(p):
    m = len(p)
    order = sorted(range(m), key=lambda i: p[i])
    out = [0.0] * m
    run = 0.0
    for rank, i in enumerate(order):
        run = max(run, min(1.0, (m - rank) * p[i]))
        out[i] = run
    return out


def holm_case(name, groups):
    k = len(groups)
    ns = [len(g) for g in groups]
    means = [float(np.mean(g)) for g in groups]
    sse = sum(float(np.sum((np.asarray(g) - m) ** 2)) for g, m in zip(groups, means))
    df = sum(ns) - k
    mse = sse / df
    # the order of Origin's pairs: (1,2), (1,3), (2,3), (1,4), ...
    pairs = [(i, j) for j in range(1, k) for i in range(j)]
    diffs, raw = [], []
    for i, j in pairs:
        se = math.sqrt(mse * (1 / ns[i] + 1 / ns[j]))
        t = (means[j] - means[i]) / se
        diffs.append(means[j] - means[i])
        raw.append(float(2 * stats.t.sf(abs(t), df)))
    return (
        f'    HolmCase {{ name: "{name}", groups: &[{", ".join(arr(g) for g in groups)}],\n'
        f"        diff: {arr(diffs)}, raw_p: {arr(raw)}, holm_p: {arr(holm(raw))} }},"
    )


holm_cases = [
    holm_case("four_groups", [sample(10, 2, 6), sample(12, 2, 7), sample(10.5, 2, 5), sample(15, 2, 6)]),
    holm_case("three_groups_one_clear", [sample(0, 1, 8), sample(0.2, 1, 8), sample(3, 1, 8)]),
    holm_case("two_groups", [sample(4, 1, 5), sample(5, 1, 6)]),
    holm_case("five_groups_ties_in_order", [sample(1, 1, 4), sample(1, 1, 4), sample(2, 1, 4), sample(2, 1, 4), sample(3, 1, 4)]),
]
holm_lists = [[0.01, 0.04, 0.03, 0.005], [0.5, 0.2, 0.9], [0.001], [0.2, 0.2, 0.01, 0.04, 0.5]]
holm_lists_out = [(p, holm(p)) for p in holm_lists]

# ---------------------------------------------------------------------------------------------------------
# Cross tabulation
# ---------------------------------------------------------------------------------------------------------
def crosstab_case(name, table):
    t = np.array(table)
    chi2, p, dof, exp = stats.chi2_contingency(t, correction=False)
    g, pg, _, _ = stats.chi2_contingency(t, correction=False, lambda_="log-likelihood")
    out = (
        f'    CrosstabCase {{ name: "{name}", counts: {iarr2(table)},\n'
        f"        expected: {arr2(exp)},\n"
        f"        pearson: ({f(chi2)}, {dof}.0, {f(p)}), likelihood_ratio: ({f(g)}, {dof}.0, {f(pg)}),\n"
    )
    if t.shape == (2, 2):
        cy, py, _, _ = stats.chi2_contingency(t, correction=True)
        fe = stats.fisher_exact(t)
        fl = stats.fisher_exact(t, alternative="less")
        fg = stats.fisher_exact(t, alternative="greater")
        out += (
            f"        yates: Some(({f(cy)}, {f(py)})),\n"
            f"        fisher: Some(({f(fe.statistic)}, {f(fe.pvalue)}, {f(fl.pvalue)}, {f(fg.pvalue)})) }},"
        )
    else:
        out += "        yates: None, fisher: None },"
    return out


crosstab_cases = [
    crosstab_case("tea_2x2", [[3, 1], [1, 3]]),
    crosstab_case("two_by_two_larger", [[12, 5], [7, 20]]),
    crosstab_case("two_by_two_skewed", [[1, 9], [11, 3]]),
    crosstab_case("two_by_two_zero_cell", [[0, 6], [5, 4]]),
    crosstab_case("two_by_two_ties", [[5, 5], [5, 5]]),
    crosstab_case("two_by_three", [[10, 20, 30], [25, 15, 12]]),
    crosstab_case("three_by_three_with_zero", [[8, 0, 3], [2, 9, 4], [5, 6, 0]]),
    crosstab_case("four_by_two", [[20, 15], [30, 28], [11, 19], [4, 12]]),
    crosstab_case("large_counts", [[1200, 800, 450], [900, 1100, 600]]),
]

# ---------------------------------------------------------------------------------------------------------
# 2D binning
# ---------------------------------------------------------------------------------------------------------
def bin_case(name, x, y, xspec, yspec):
    """spec: ('count', n) or ('width', min, max, w)"""
    bins = []
    for spec, v in ((xspec, x), (yspec, y)):
        if spec[0] == "count":
            bins.append(spec[1])
        else:
            _, lo, hi, w = spec
            n = int(round((hi - lo) / w))
            bins.append(np.array([lo + w * i for i in range(n + 1)]))
    r = stats.binned_statistic_2d(x, y, None, "count", bins=bins)

    def spec_code(spec):
        if spec[0] == "count":
            return f"Axis::Count({spec[1]})"
        return f"Axis::Width({f(spec[1])}, {f(spec[2])}, {f(spec[3])})"

    return (
        f'    BinCase {{ name: "{name}", x: {arr(x)}, y: {arr(y)},\n'
        f"        xaxis: {spec_code(xspec)}, yaxis: {spec_code(yspec)},\n"
        f"        x_edges: {arr(r.x_edge)}, y_edges: {arr(r.y_edge)},\n"
        f"        counts: {iarr2(r.statistic.astype(int))} }},"
    )


bx = list(np.round(rng.normal(5, 2, 60), 2))
by = list(np.round(0.5 * np.array(bx) + rng.normal(0, 1.5, 60), 2))
ix = [float(v) for v in rng.integers(0, 10, 50)]
iy = [float(v) for v in rng.integers(0, 6, 50)]
bin_cases = [
    bin_case("count_4x3", bx, by, ("count", 4), ("count", 3)),
    bin_case("count_10x10", bx, by, ("count", 10), ("count", 10)),
    bin_case("count_1x5", bx, by, ("count", 1), ("count", 5)),
    bin_case("width_range_with_outside", bx, by, ("width", 2.0, 8.0, 1.0), ("width", 0.0, 6.0, 1.5)),
    # integer data sit on the bin edges: lower edges are inclusive, the range ends beyond the data
    bin_case("width_integers_on_edges", ix, iy, ("width", 0.0, 10.0, 2.0), ("width", 0.0, 6.0, 1.0)),
    bin_case("count_integers_last_edge_closed", ix, iy, ("count", 3), ("count", 5)),
]

print("// Made by tools/scipy_stats_compare.py (scipy 1.18.1). Reference numbers are scipy's, NOT Origin's.")
print("#![allow(dead_code)]")
print()
print("pub struct DunnettCase {")
print("    pub name: &'static str,")
print("    pub control: &'static [f64],")
print("    pub groups: &'static [&'static [f64]],")
print("    pub statistic: &'static [f64],")
print("    pub p_scipy: &'static [f64],")
print("    pub ci_low_scipy: &'static [f64],")
print("    pub ci_high_scipy: &'static [f64],")
print("    pub p_ref: &'static [f64],")
print("    pub critical_ref: f64,")
print("}")
print()
print("pub struct HolmCase {")
print("    pub name: &'static str,")
print("    pub groups: &'static [&'static [f64]],")
print("    pub diff: &'static [f64],")
print("    pub raw_p: &'static [f64],")
print("    pub holm_p: &'static [f64],")
print("}")
print()
print("pub struct CrosstabCase {")
print("    pub name: &'static str,")
print("    pub counts: &'static [&'static [usize]],")
print("    pub expected: &'static [&'static [f64]],")
print("    pub pearson: (f64, f64, f64),")
print("    pub likelihood_ratio: (f64, f64, f64),")
print("    pub yates: Option<(f64, f64)>,")
print("    pub fisher: Option<(f64, f64, f64, f64)>,")
print("}")
print()
print("pub enum Axis {")
print("    Count(usize),")
print("    /// min, max, width")
print("    Width(f64, f64, f64),")
print("}")
print()
print("pub struct BinCase {")
print("    pub name: &'static str,")
print("    pub x: &'static [f64],")
print("    pub y: &'static [f64],")
print("    pub xaxis: Axis,")
print("    pub yaxis: Axis,")
print("    pub x_edges: &'static [f64],")
print("    pub y_edges: &'static [f64],")
print("    pub counts: &'static [&'static [usize]],")
print("}")
print()
print("pub const DUNNETT: &[DunnettCase] = &[")
print("\n".join(dunnett_cases))
print("];")
print()
print("pub const HOLM: &[HolmCase] = &[")
print("\n".join(holm_cases))
print("];")
print()
print("/// Raw p-values and their Holm adjustment.")
print("pub const HOLM_LISTS: &[(&[f64], &[f64])] = &[")
for p, a in holm_lists_out:
    print(f"    ({arr(p)}, {arr(a)}),")
print("];")
print()
print("pub const CROSSTAB: &[CrosstabCase] = &[")
print("\n".join(crosstab_cases))
print("];")
print()
print("pub const BIN2D: &[BinCase] = &[")
print("\n".join(bin_cases))
print("];")
