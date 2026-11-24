"""Prints crates/calc/tests/fixtures/power_rate_reference.rs: power and sample size of the one- and two-proportion z
tests and the one- and two-variance chi-square and F tests, for `sheaf_calc::rate_power` and friends (T415), from
scipy.

The powers are written out from scipy.stats (norm, chi2, f); the variance powers are also checked here by simulating
the tests (200 000 draws, to 0.005), so the formula is the test's real power. Sample sizes come from a plain scan
upward from 2. None of these numbers come from Origin.

Usage: python3 tools/power-rate-ref.py > crates/calc/tests/fixtures/power_rate_reference.rs
"""
import math

import numpy as np
from scipy import stats

TESTS = ["OneProportion", "TwoProportions", "OneVariance", "TwoVariances"]


def power(test, n, a, b, alpha, two):
    tail = alpha / 2 if two else alpha
    if test in ("OneProportion", "TwoProportions"):
        z = stats.norm.ppf(1 - tail)
        if test == "OneProportion":
            se0, se1 = math.sqrt(b * (1 - b)), math.sqrt(a * (1 - a))
        else:
            se0 = se1 = math.sqrt(a * (1 - a) + b * (1 - b))
        d = abs(a - b) * math.sqrt(n)
        up = stats.norm.cdf((d - z * se0) / se1)
        return up + stats.norm.cdf((-d - z * se0) / se1) if two else up
    r = a / b
    dist = stats.chi2(n - 1) if test == "OneVariance" else stats.f(n - 1, n - 1)
    below = dist.cdf(dist.ppf(tail) / r)
    above = dist.sf(dist.ppf(1 - tail) / r)
    return below + above if two else (above if r >= 1 else below)


def simulate(test, n, a, b, alpha, two, rng, draws=200_000):
    tail = alpha / 2 if two else alpha
    if test == "OneVariance":
        x = rng.normal(0, math.sqrt(a), (draws, n))
        stat = (n - 1) * x.var(axis=1, ddof=1) / b
        lo, hi = stats.chi2.ppf(tail, n - 1), stats.chi2.ppf(1 - tail, n - 1)
    else:
        x = rng.normal(0, math.sqrt(a), (draws, n))
        y = rng.normal(0, math.sqrt(b), (draws, n))
        stat = x.var(axis=1, ddof=1) / y.var(axis=1, ddof=1)
        lo, hi = stats.f.ppf(tail, n - 1, n - 1), stats.f.ppf(1 - tail, n - 1, n - 1)
    rej = (stat < lo) | (stat > hi) if two else ((stat > hi) if a >= b else (stat < lo))
    return rej.mean()


def size(test, target, a, b, alpha, two):
    n = 2
    while power(test, n, a, b, alpha, two) < target:
        n += 1
    return n


rng = np.random.default_rng(415)
cases = []
for test, pairs in [
    ("OneProportion", [(0.6, 0.5), (0.3, 0.4), (0.05, 0.1), (0.5, 0.5)]),
    ("TwoProportions", [(0.6, 0.5), (0.2, 0.35), (0.9, 0.8), (0.4, 0.4)]),
    ("OneVariance", [(2.0, 1.0), (0.5, 1.0), (1.3, 1.0), (4.0, 4.0)]),
    ("TwoVariances", [(2.0, 1.0), (1.0, 3.0), (1.5, 1.0), (2.0, 2.0)]),
]:
    for a, b in pairs:
        for alpha, two in [(0.05, True), (0.05, False), (0.01, True), (0.1, False)]:
            sizes = [2, 5, 10, 30, 100, 1000]
            pw = [power(test, n, a, b, alpha, two) for n in sizes]
            if test in ("OneVariance", "TwoVariances") and alpha == 0.05 and a != b:
                for n in (5, 30):
                    sim = simulate(test, n, a, b, alpha, two, rng)
                    assert abs(sim - power(test, n, a, b, alpha, two)) < 0.005, (test, a, b, n, sim)
            wanted = [0.5, 0.8, 0.9, 0.99]
            if a == b:
                ns = None
            else:
                low = power(test, 2, a, b, alpha, two)
                ns = [float(size(test, w, a, b, alpha, two)) if w > low else math.nan for w in wanted]
            cases.append((test, a, b, alpha, two, sizes, pw, wanted, ns))


def num(v):
    v = float(v)
    return "f64::NAN" if math.isnan(v) else repr(v)


def vec(v):
    return "&[" + ", ".join(num(x) for x in v) + "]"


print("// Made by tools/power-rate-ref.py (scipy). Do not edit.")
print("use sheaf_calc::RateTest;")
print("pub struct Case {")
for f in ["test: RateTest", "a: f64", "b: f64", "alpha: f64", "two_sided: bool", "sizes: &'static [f64]",
          "powers: &'static [f64]", "wanted: &'static [f64]", "needed: Option<&'static [f64]>"]:
    print(f"    pub {f},")
print("}")
print("pub const CASES: &[Case] = &[")
for test, a, b, alpha, two, sizes, pw, wanted, ns in cases:
    needed = "None" if ns is None else f"Some({vec(ns)})"
    print(f"    Case {{ test: RateTest::{test}, a: {num(a)}, b: {num(b)}, alpha: {num(alpha)}, two_sided: {'true' if two else 'false'}, "
          f"sizes: {vec(sizes)}, powers: {vec(pw)}, wanted: {vec(wanted)}, needed: {needed} }},")
print("];")
