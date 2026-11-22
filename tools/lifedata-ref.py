"""Prints crates/calc/tests/fixtures/lifedata_reference.rs: right-censored maximum likelihood fits for
`sheaf_calc::lifedata` (T366), from scipy.

- Exponential: closed form. Weibull: brentq on the profile equation for the shape. Normal and lognormal:
  scipy.optimize.root on the analytic score in (mu, sigma), tolerance 1e-14.
- Log-likelihoods from scipy.stats logpdf/logsf; each fit is also checked loosely (1e-3) against scipy.stats'
  own `fit(CensoredData)`.
- Modified Kaplan-Meier plotting positions computed directly.

Usage: python3 tools/lifedata-ref.py > crates/calc/tests/fixtures/lifedata_reference.rs
None of these numbers come from Origin.
"""
import math

import numpy as np
from scipy import optimize, stats

M = 2 ** 64


def uniforms(n, seed):
    st = seed
    out = []
    for _ in range(n):
        st = (st * 6364136223846793005 + 1442695040888963407) % M
        out.append(((st >> 11) + 0.5) / 2 ** 53)
    return out


def life_sample(n, shape, scale, censor_at, seed):
    """Weibull lifetimes, censored at random times up to `censor_at` (uniform) and at `censor_at` itself."""
    u = uniforms(2 * n, seed)
    t, c = [], []
    for i in range(n):
        life = scale * (-math.log(u[2 * i])) ** (1 / shape)
        stop = censor_at * (0.5 + u[2 * i + 1])
        if life <= stop:
            t.append(round(life, 6))
            c.append(False)
        else:
            t.append(round(stop, 6))
            c.append(True)
    return np.array(t), np.array(c)


def weibull_fit(t, c):
    r = (~c).sum()
    lt = np.log(t)
    mfl = lt[~c].mean()

    def g(k):
        tk = t ** k
        return (tk * lt).sum() / tk.sum() - 1 / k - mfl

    k = optimize.brentq(g, 1e-3, 100, xtol=1e-15, rtol=1e-15)
    lam = ((t ** k).sum() / r) ** (1 / k)
    return [k, lam], stats.weibull_min(k, scale=lam)


def normal_fit(y, c):
    f = y[~c]

    def score(p):
        mu, s = p
        z = (y - mu) / s
        h = stats.norm.pdf(z) / stats.norm.sf(z)
        a = np.where(c, h / s, z / s).sum()
        b = np.where(c, z * h / s, (z * z - 1) / s).sum()
        return [a, b]

    sol = optimize.root(score, [f.mean(), f.std()], method="lm", tol=1e-15)
    assert max(abs(v) for v in score(sol.x)) * sol.x[1] < 1e-9, (sol.message, score(sol.x))
    return list(sol.x)


def fit(t, c, dist):
    if dist == "exponential":
        th = t.sum() / (~c).sum()
        params, d = [th], stats.expon(scale=th)
    elif dist == "weibull":
        params, d = weibull_fit(t, c)
    elif dist == "lognormal":
        params = normal_fit(np.log(t), c)
        d = stats.lognorm(params[1], scale=math.exp(params[0]))
    else:
        params = normal_fit(t, c)
        d = stats.norm(params[0], params[1])
    ll = np.where(c, d.logsf(t), d.logpdf(t)).sum()
    cd = stats.CensoredData(uncensored=t[~c], right=t[c])
    if dist == "weibull":
        k, _, lam = stats.weibull_min.fit(cd, floc=0)
        assert abs(k / params[0] - 1) < 1e-3 and abs(lam / params[1] - 1) < 1e-3, (k, lam, params)
    elif dist == "lognormal":
        s, _, sc = stats.lognorm.fit(cd, floc=0)
        assert abs(s / params[1] - 1) < 1e-3 and abs(math.log(sc) - params[0]) < 1e-3, (s, sc, params)
    elif dist == "normal":
        m, s = stats.norm.fit(cd)
        assert abs(m - params[0]) < 1e-3 * abs(params[1]) and abs(s / params[1] - 1) < 1e-3, (m, s, params)
    return params, ll, d.median(), d.mean()


def positions(t, c):
    order = sorted(range(len(t)), key=lambda i: (t[i], c[i]))
    at_risk, s, out = len(t), 1.0, []
    for i in order:
        if not c[i]:
            before = s
            s *= 1 - 1 / at_risk
            out.append((t[i], 1 - 0.5 * (before + s)))
        at_risk -= 1
    return out


def vec(v):
    return "&[" + ", ".join(repr(float(a)) for a in v) + "]"


def bools(v):
    return "&[" + ", ".join("true" if b else "false" for b in v) + "]"


samples = [
    ("weibull 2, light censoring", *life_sample(40, 2.0, 100.0, 200.0, 3)),
    ("weibull 0.9, heavy censoring", *life_sample(60, 0.9, 50.0, 40.0, 17)),
]
print("// Made by tools/lifedata-ref.py (scipy). Do not edit.")
print("/// (name, times, censored, [(distribution, params, log-likelihood, median, mean)], [(t, F)])")
print("pub struct Sample {")
print("    pub name: &'static str,")
print("    pub t: &'static [f64],")
print("    pub censored: &'static [bool],")
print("    pub fits: &'static [(&'static str, &'static [f64], f64, f64, f64)],")
print("    pub positions: &'static [(f64, f64)],")
print("}")
print("pub const SAMPLES: &[Sample] = &[")
for name, t, c in samples:
    print("    Sample {")
    print(f'        name: "{name}",')
    print(f"        t: {vec(t)},")
    print(f"        censored: {bools(c)},")
    print("        fits: &[")
    for dist in ("weibull", "lognormal", "exponential", "normal"):
        params, ll, med, mean = fit(t, c, dist)
        print(f'            ("{dist}", {vec(params)}, {float(ll)!r}, {float(med)!r}, {float(mean)!r}),')
    print("        ],")
    print("        positions: &[" + ", ".join(f"({float(a)!r}, {float(b)!r})" for a, b in positions(t, c)) + "],")
    print("    },")
print("];")
