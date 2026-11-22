"""Prints crates/calc/tests/fixtures/decompose_reference.rs: classical seasonal decompositions for
`sheaf_calc::decompose` (T370), from numpy.

The trend is numpy.convolve of the series with the centred moving-average filter (half weights at the ends for an
even period, 'valid' mode, padded with NaN); seasonal indices are numpy.nanmean per position, adjusted; residuals
follow. This is statsmodels' seasonal_decompose (two-sided filter, no extrapolation), written out in numpy.

Usage: python3 tools/decompose-ref.py > crates/calc/tests/fixtures/decompose_reference.rs
None of these numbers come from Origin.
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


def reference(y, m, multiplicative):
    n = len(y)
    if m % 2:
        f = np.full(m, 1 / m)
    else:
        f = np.full(m + 1, 1 / m)
        f[0] = f[-1] = 0.5 / m
    valid = np.convolve(y, f[::-1], mode="valid")
    half = m // 2
    trend = np.full(n, np.nan)
    trend[half : n - half] = valid
    det = y / trend if multiplicative else y - trend
    idx = np.array([np.nanmean(det[k::m]) for k in range(m)])
    idx = idx / idx.mean() if multiplicative else idx - idx.mean()
    seas = idx[np.arange(n) % m]
    resid = y / (trend * seas) if multiplicative else y - trend - seas
    return trend, seas, resid, idx


def num(v):
    v = float(v)
    return "f64::NAN" if math.isnan(v) else repr(v)


def vec(v):
    return "&[" + ", ".join(num(a) for a in v) + "]"


t = np.arange(48)
monthly = 50 + 0.8 * t + 6 * np.sin(2 * np.pi * t / 12) + 2 * (uniforms(48, 3) - 0.5)
quarterly = (100 + 2 * np.arange(22)) * np.array([1.1, 0.9, 0.95, 1.05])[np.arange(22) % 4] * (1 + 0.02 * (uniforms(22, 7) - 0.5))
weekly = 10 + np.array([0, 1, 2, 3, 2, -3, -5])[np.arange(30) % 7] + (uniforms(30, 11) - 0.5)
cases = [("monthly additive", monthly, 12, False), ("quarterly multiplicative", quarterly, 4, True), ("weekly additive", weekly, 7, False)]
print("// Made by tools/decompose-ref.py (numpy). Do not edit.")
print("/// (name, series, period, multiplicative, trend, seasonal, residual, indices)")
print("#[allow(clippy::type_complexity)]")
print("pub const CASES: &[(&str, &[f64], usize, bool, &[f64], &[f64], &[f64], &[f64])] = &[")
for name, y, m, mult in cases:
    tr, se, re, idx = reference(y, m, mult)
    print(f'    ("{name}", {vec(y)}, {m}, {"true" if mult else "false"}, {vec(tr)}, {vec(se)}, {vec(re)}, {vec(idx)}),')
print("];")
