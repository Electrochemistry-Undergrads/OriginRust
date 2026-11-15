"""Prints crates/calc/tests/fixtures/scipy_kde.rs: reference numbers from scipy for the Gaussian
kernel density estimate of `sheaf-calc` (`kde.rs`, the violin plot).

Usage: python3 tools/kde-ref.py > crates/calc/tests/fixtures/scipy_kde.rs
Needs numpy and scipy (the numbers in the repository were made with scipy 1.18.1). All inputs are
generated here from fixed seeds; none of the numbers come from Origin.

What is taken from scipy: the density. `scipy.stats.gaussian_kde(x, bw_method=h)` with a scalar `h` uses
the kernel standard deviation `h * std(x, ddof=1)`, so for a bandwidth `w` the factor is `w / std(x, ddof=1)`;
the script asserts that `kde.factor * sqrt(kde.covariance)` is `w`. The bandwidth rules are Origin's
documented formulas (Scott `0.9 A n^-0.2`, Silverman `1.059 A n^-0.2`, `A = min(SD, IQR / 1.349)`), written
out here in numpy, with the quartiles of Origin's box chart (the Hazen rule of `quantile_origin`).
The scipy "scott" and "silverman" factors are also covered, as `Bandwidth::Sd(k)`.
"""
import math

import numpy as np
import scipy
from scipy import stats


def f(v):
    return repr(float(v))


def arr(v):
    return "&[" + ", ".join(f(x) for x in v) + "]"


def origin_quantile(sorted_x, p):
    """sheaf_calc::stats::quantile_origin."""
    n = len(sorted_x)

    def lower(q):
        pos = n * q + 0.5
        if (pos * 2.0) % 1.0 != 0.0:
            pos = math.floor(pos + 0.5)
        return pos

    pos = n + 1.0 - lower(1.0 - p) if p > 0.5 else lower(p)
    pos = min(max(pos, 1.0), float(n))
    lo, hi = math.floor(pos), math.ceil(pos)
    return (sorted_x[lo - 1] + sorted_x[hi - 1]) / 2.0


def datasets():
    r = np.random.default_rng(20261115)
    out = []
    out.append(("normal", np.round(r.normal(10.0, 2.5, 30), 4)))
    out.append(("lognormal", np.round(r.lognormal(1.2, 0.6, 57), 4)))
    out.append(("bimodal", np.round(np.concatenate([r.normal(-2, 0.7, 40), r.normal(3, 1.1, 40)]), 4)))
    out.append(("ties", np.array([5.0, 5, 5, 5, 6, 6, 7, 9, 12, 12, 12, 20])))
    out.append(("two", np.array([1.0, 4.0])))
    return out


def bandwidth(rule, x):
    xs = np.sort(x)
    n = len(xs)
    sd = float(np.std(xs, ddof=1))
    if rule[0] == "sd":
        return rule[1] * sd
    iqr = origin_quantile(xs, 0.75) - origin_quantile(xs, 0.25)
    spread = min(sd, iqr / 1.349) if iqr > 0 else sd
    return (0.9 if rule[0] == "scott" else 1.059) * spread * n ** -0.2


def main():
    print("//! Numbers from scipy %s for the kernel density estimate (made by tools/kde-ref.py). Do not edit by hand."
          % scipy.__version__)
    print("//! These numbers come from scipy, not from Origin.")
    print("#![allow(clippy::excessive_precision, dead_code)]\n")
    print("/// `rule`: 0 Scott, 1 Silverman, 2 k x SD with `k`. `grid` is 128 points from min - extend/100 w to max + extend/100 w.")
    print("pub struct Case {")
    print("    pub name: &'static str,")
    print("    pub data: &'static [f64],")
    print("    pub rule: usize,")
    print("    pub k: f64,")
    print("    pub extend_pct: f64,")
    print("    pub bandwidth: f64,")
    print("    pub grid: &'static [f64],")
    print("    pub density: &'static [f64],")
    print("}\n")
    print("pub const CASES: &[Case] = &[")
    for name, x in datasets():
        n = len(x)
        rules = [("scott",), ("silverman",), ("sd", 0.5), ("sd", n ** -0.2)]
        for ri, rule in enumerate(rules):
            w = bandwidth(rule, x)
            sd = float(np.std(x, ddof=1))
            kde = stats.gaussian_kde(x, bw_method=w / sd)
            assert abs(math.sqrt(float(kde.covariance[0, 0])) - w) < 1e-12 * w
            ext = 100.0 if ri % 2 == 0 else 300.0
            lo, hi = x.min() - ext / 100 * w, x.max() + ext / 100 * w
            grid = np.array([lo + (hi - lo) * i / 127 for i in range(128)])
            dens = kde.evaluate(grid)
            code, k = (0, 0.0) if rule[0] == "scott" else (1, 0.0) if rule[0] == "silverman" else (2, rule[1])
            print("    Case {")
            print(f'        name: "{name}",')
            print(f"        data: {arr(x)},")
            print(f"        rule: {code},")
            print(f"        k: {f(k)},")
            print(f"        extend_pct: {f(ext)},")
            print(f"        bandwidth: {f(w)},")
            print(f"        grid: {arr(grid)},")
            print(f"        density: {arr(dens)},")
            print("    },")
    # scipy's own rules, as the k x SD rule: scott factor n^(-1/5), silverman (n (d + 2) / 4)^(-1/(d + 4)).
    print("];")
    print("\n/// scipy's own factors: (data, scott factor, silverman factor, density of each at the data's mean).")
    print("pub struct Factors {")
    print("    pub data: &'static [f64],")
    print("    pub scott: f64,")
    print("    pub silverman: f64,")
    print("    pub at: f64,")
    print("    pub scott_density: f64,")
    print("    pub silverman_density: f64,")
    print("}")
    print("pub const FACTORS: &[Factors] = &[")
    for name, x in datasets()[:3]:
        a = stats.gaussian_kde(x, bw_method="scott")
        b = stats.gaussian_kde(x, bw_method="silverman")
        m = float(x.mean())
        print("    Factors {")
        print(f"        data: {arr(x)},")
        print(f"        scott: {f(a.factor)},")
        print(f"        silverman: {f(b.factor)},")
        print(f"        at: {f(m)},")
        print(f"        scott_density: {f(a.evaluate([m])[0])},")
        print(f"        silverman_density: {f(b.evaluate([m])[0])},")
        print("    },")
    print("];")


main()
