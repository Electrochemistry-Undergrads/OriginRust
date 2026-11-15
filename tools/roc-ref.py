"""Reference numbers for crates/calc/tests/scipy_roc.rs (scipy 1.17, numpy 2.4; not Origin).

The AUC is the Mann-Whitney U of scipy.stats.mannwhitneyu over m*n; the standard error is DeLong's, computed
here by the pairwise kernel psi(x, y) = 1, 1/2, 0 (a different route from the midrank formulas of
crates/calc/src/roc.rs); the interval and p-value use scipy.stats.norm. usage: python roc-ref.py > out.txt"""
import numpy as np
from scipy import stats

CASES = {
    # (values, positive state) with ties inside and across the two states
    'ties': ([0.1, 0.35, 0.4, 0.8, 0.2, 0.7, 0.55, 0.9, 0.6, 0.3, 0.4, 0.7, 0.35, 0.6, 0.8, 0.2],
             [0, 0, 1, 1, 0, 1, 0, 1, 1, 0, 0, 1, 1, 0, 1, 0]),
    'separated': ([1.5, 2.5, 3.5, 0.5, 0.9, 1.1, 4.0], [1, 1, 1, 0, 0, 0, 1]),
    'overlap': ([2.1, 3.3, 1.7, 4.8, 2.9, 3.1, 4.2, 1.1, 2.5, 3.9, 0.7, 3.6, 2.2, 4.4, 1.9, 3.0, 2.8, 3.7],
                [0, 1, 0, 1, 0, 0, 1, 0, 1, 1, 0, 1, 0, 0, 0, 1, 1, 1]),
}


def delong(values, state, alpha=0.05):
    v = np.asarray(values, float)
    s = np.asarray(state, bool)
    pos, neg = v[s], v[~s]
    m, n = len(pos), len(neg)
    psi = (pos[:, None] > neg[None, :]) + 0.5 * (pos[:, None] == neg[None, :])
    auc = psi.mean()
    u = stats.mannwhitneyu(pos, neg, alternative='two-sided', method='asymptotic').statistic
    assert abs(auc - u / (m * n)) < 1e-12
    v10 = psi.mean(axis=1)
    v01 = psi.mean(axis=0)
    se = np.sqrt(v10.var(ddof=1) / m + v01.var(ddof=1) / n)
    z = (auc - 0.5) / se
    p = 2 * stats.norm.sf(abs(z))
    zc = stats.norm.ppf(1 - alpha / 2)
    lo, hi = max(auc - zc * se, 0.0), min(auc + zc * se, 1.0)
    # curve: one point per distinct value, high to low, plus the start
    pts = [(0.0, 0.0, np.inf)]
    for t in sorted(set(v), reverse=True):
        pts.append((float((neg >= t).mean()), float((pos >= t).mean()), float(t)))
    best = max(range(len(pts)), key=lambda k: (round(pts[k][1] - pts[k][0], 12), -k))
    return dict(m=m, n=n, auc=auc, se=se, z=z, p=p, lo=lo, hi=hi, pts=pts, best=pts[best][2])


for name, (v, s) in CASES.items():
    r = delong(v, s)
    print(f"// {name}: m={r['m']} n={r['n']}")
    print(f"auc={r['auc']!r} se={r['se']!r} z={r['z']!r} p={r['p']!r} lo={r['lo']!r} hi={r['hi']!r} best={r['best']!r}")
    print('pts=' + repr([(a, b) for a, b, _ in r['pts']]))
