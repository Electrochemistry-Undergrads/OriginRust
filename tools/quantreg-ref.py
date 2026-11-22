"""Prints crates/calc/tests/fixtures/scipy_quantreg.rs: exact linear quantile regression coefficients from
scipy.optimize.linprog (HiGHS) for `sheaf_calc::quantreg` (T342).

Usage: python3 tools/quantreg-ref.py > crates/calc/tests/fixtures/scipy_quantreg.rs
Needs numpy and scipy (made with scipy 1.17.0). The inputs are closed-form (sines of the row number), so no
random generator is involved. None of these numbers come from Origin: quantile regression is an Origin 2019b
app, not part of Origin 2016.

The linear programme solved is the primal: minimize tau 1'u + (1 - tau) 1'v over b = b+ - b-, u, v >= 0 with
X b + u - v = y. The data have no ties, so the optimum is unique.
"""
import numpy as np
from scipy.optimize import linprog


def solve(X, y, tau):
    n, p = X.shape
    c = np.concatenate([np.zeros(2 * p), tau * np.ones(n), (1 - tau) * np.ones(n)])
    A = np.hstack([X, -X, np.eye(n), -np.eye(n)])
    res = linprog(c, A_eq=A, b_eq=y, bounds=[(0, None)] * (2 * p + 2 * n), method="highs")
    assert res.status == 0, res.message
    b = res.x[:p] - res.x[p:2 * p]
    return b, res.fun


def vec(v):
    return "&[" + ", ".join(repr(float(x)) for x in v) + "]"


cases = []
i = np.arange(1, 61, dtype=float)
x1 = i / 4.0
y1 = 1.5 + 0.8 * x1 + (0.4 + 0.15 * x1) * np.sin(7.3 * i) + 0.3 * np.sin(2.9 * i * i)
cases.append(("one_x", [x1], y1, True))
j = np.arange(1, 81, dtype=float)
xa = np.sin(0.37 * j) * 5.0
xb = np.cos(1.13 * j) * 3.0 + j / 20.0
y2 = 2.0 - 0.6 * xa + 1.1 * xb + (1.0 + 0.2 * np.abs(xa)) * np.sin(5.1 * j)
cases.append(("two_x", [xa, xb], y2, True))
k = np.arange(1, 31, dtype=float)
x3 = k / 3.0
y3 = 0.7 * x3 + 0.5 * np.sin(11.0 * k)
cases.append(("no_intercept", [x3], y3, False))
m = np.arange(1, 501, dtype=float)
x4 = m / 50.0
y4 = np.exp(0.1 * x4) + np.sin(3.7 * m) * (0.2 + 0.05 * x4)
cases.append(("five_hundred", [x4], y4, True))

taus = [0.1, 0.25, 0.5, 0.75, 0.9]
print("// Made by tools/quantreg-ref.py (scipy.optimize.linprog, HiGHS). Do not edit.")
print("#![allow(clippy::approx_constant)] // the x grids pass 3.14 and 6.28")
print("pub struct Case {")
print("    pub name: &'static str,")
print("    pub xs: &'static [&'static [f64]],")
print("    pub y: &'static [f64],")
print("    pub intercept: bool,")
print("    /// (tau, coefficients (intercept first), loss)")
print("    pub fits: &'static [(f64, &'static [f64], f64)],")
print("}")
print()
print("pub const CASES: &[Case] = &[")
for name, xs, y, intercept in cases:
    X = np.column_stack(([np.ones(len(y))] if intercept else []) + xs)
    print("    Case {")
    print(f'        name: "{name}",')
    print("        xs: &[" + ", ".join(vec(x) for x in xs) + "],")
    print(f"        y: {vec(y)},")
    print(f"        intercept: {'true' if intercept else 'false'},")
    print("        fits: &[")
    for tau in taus:
        b, loss = solve(X, y, tau)
        print(f"            ({tau!r}, {vec(b)}, {float(loss)!r}),")
    print("        ],")
    print("    },")
print("];")
