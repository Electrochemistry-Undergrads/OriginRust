"""Prints crates/calc/tests/fixtures/changepoint_exact.rs: change points by exact optimal partitioning (an O(n^2)
dynamic programme over every split) with the costs and penalties of `sheaf_calc::changepoint` (T346), so the PELT
answer of Sheaf can be checked against the true optimum.

Usage: python3 tools/changepoint-ref.py > crates/calc/tests/fixtures/changepoint_exact.rs
Needs numpy. The series use a fixed linear congruential generator, the same as the unit tests of
`changepoint.rs`. None of these numbers come from Origin (change-point analysis is an Origin 2019b app).
"""
import math
import numpy as np

M = 2 ** 64


def noise(n, seed):
    st = seed
    out = []
    for _ in range(n):
        s = 0.0
        for _ in range(12):
            st = (st * 6364136223846793005 + 1442695040888963407) % M
            s += (st >> 11) / 2 ** 53
        out.append(s - 6.0)
    return np.array(out)


def median(v):
    v = sorted(v)
    m = len(v)
    return v[m // 2] if m % 2 else 0.5 * (v[m // 2 - 1] + v[m // 2])


def mean_sigma(x):
    d = np.diff(x)
    md = median(d)
    mad = median(np.abs(d - md)) / 0.6745
    if mad > 0:
        return mad / math.sqrt(2)
    return np.std(d, ddof=1) / math.sqrt(2)


def costs(x, kind):
    n = len(x)
    cs = np.concatenate([[0.0], np.cumsum(x)])
    cq = np.concatenate([[0.0], np.cumsum(x * x)])
    mean = x.mean()
    cc = np.concatenate([[0.0], np.cumsum((x - mean) ** 2)])
    floor = cc[n] / n * 1e-12

    def ss(a, b):
        k = b - a
        s = cs[b] - cs[a]
        return max(cq[b] - cq[a] - s * s / k, 0.0)

    if kind == "Mean":
        s2 = mean_sigma(x) ** 2
        return lambda a, b: ss(a, b) / s2
    if kind == "Variance":
        return lambda a, b: (b - a) * (math.log(2 * math.pi) + math.log(max((cc[b] - cc[a]) / (b - a), floor)) + 1)
    return lambda a, b: (b - a) * (math.log(2 * math.pi) + math.log(max(ss(a, b) / (b - a), floor)) + 1)


def penalty(name, kind, n, manual):
    p = 2.0 if kind == "MeanVariance" else 1.0
    return {
        "Mbic": (p + 2) * math.log(n),
        "Bic": p * math.log(n),
        "Aic": 2 * p,
        "HannanQuinn": 2 * p * math.log(math.log(n)),
        "Manual": manual,
    }[name]


def optimal(x, kind, pen, m):
    n = len(x)
    cost = costs(x, kind)
    F = [math.inf] * (n + 1)
    F[0] = -pen
    last = [0] * (n + 1)
    for t in range(m, n + 1):
        for s in range(0, t - m + 1):
            if F[s] == math.inf:
                continue
            v = F[s] + cost(s, t) + pen
            if v < F[t]:
                F[t] = v
                last[t] = s
    ch = []
    t = n
    while t > 0:
        s = last[t]
        if s > 0:
            ch.append(s)
        t = s
    return sorted(ch), F[n]


series = []
e = noise(150, 7)
series.append(("mean_steps", np.array([(0 if i < 40 else 3 if i < 90 else 1.5) + 0.8 * e[i] for i in range(150)])))
e = noise(160, 3)
series.append(("variance", np.array([e[i] * (0.5 if i < 70 else 2.0 if i < 120 else 1.0) for i in range(160)])))
e = noise(120, 11)
series.append(("both", np.array([(1 + 0.3 * e[i]) if i < 50 else (4 + 1.5 * e[i]) for i in range(120)])))
e = noise(80, 5)
series.append(("calm", e.copy()))

runs = [
    ("Mean", "Mbic", 1, 0.0), ("Mean", "Bic", 1, 0.0), ("Mean", "Aic", 3, 0.0), ("Mean", "HannanQuinn", 5, 0.0),
    ("Mean", "Manual", 2, 20.0), ("Variance", "Mbic", 2, 0.0), ("Variance", "Bic", 4, 0.0),
    ("MeanVariance", "Mbic", 2, 0.0), ("MeanVariance", "Aic", 6, 0.0), ("MeanVariance", "Manual", 3, 30.0),
]


def vec(v):
    return "&[" + ", ".join(repr(float(a)) for a in v) + "]"


print("// Made by tools/changepoint-ref.py (exact optimal partitioning). Do not edit.")
print("pub struct Run {")
print("    pub kind: &'static str,")
print("    pub penalty: &'static str,")
print("    pub manual: f64,")
print("    pub min_segment: usize,")
print("    pub changes: &'static [usize],")
print("    pub objective: f64,")
print("}")
print()
print("pub struct Series {")
print("    pub name: &'static str,")
print("    pub x: &'static [f64],")
print("    pub runs: &'static [Run],")
print("}")
print()
print("pub const SERIES: &[Series] = &[")
for name, x in series:
    print("    Series {")
    print(f'        name: "{name}",')
    print(f"        x: {vec(x)},")
    print("        runs: &[")
    for kind, pname, m, manual in runs:
        pen = penalty(pname, kind, len(x), manual)
        ch, obj = optimal(x, kind, pen, m)
        print(
            f'            Run {{ kind: "{kind}", penalty: "{pname}", manual: {manual!r}, min_segment: {m}, '
            f"changes: &[{', '.join(str(c) for c in ch)}], objective: {float(obj)!r} }},"
        )
    print("        ],")
    print("    },")
print("];")
