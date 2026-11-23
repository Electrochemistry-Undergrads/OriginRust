"""Prints crates/calc/tests/fixtures/forest_reference.rs: CART trees and seeded random forests for
`sheaf_calc::forest` (T377), from an independent pure-Python implementation of the rules in the module docs (exact
best splits by n x Gini or the sum of squares, midpoint thresholds, strict improvement, the 64-bit LCG for
bootstrap draws and per-node feature subsets).

Usage: python3 tools/forest-ref.py > crates/calc/tests/fixtures/forest_reference.rs
scikit-learn is not available here; none of these numbers come from Origin.
"""
import math

M = 2 ** 64


class Rng:
    def __init__(self, seed):
        self.s = seed

    def next(self):
        self.s = (self.s * 6364136223846793005 + 1442695040888963407) % M
        return (self.s >> 11) / 2 ** 53

    def below(self, n):
        return min(int(self.next() * n), n - 1)


def impurity(rows, y, classes):
    n = float(len(rows))
    if classes:
        c = [0.0] * classes
        for r in rows:
            c[y[r]] += 1.0
        return n - sum(v * v for v in c) / n
    s = s2 = 0.0
    for r in rows:
        s += y[r]
        s2 += y[r] * y[r]
    return s2 - s * s / n


def leaf(rows, y, classes):
    if classes:
        c = [0] * classes
        for r in rows:
            c[y[r]] += 1
        return float(c.index(max(c)))
    return sum(y[r] for r in rows) / len(rows)


def best_split(x, y, classes, rows, features, parent, min_leaf):
    n = len(rows)
    best = None
    for f in features:
        srt = sorted(rows, key=lambda r: (x[r][f], r))
        if classes:
            left = [0.0] * classes
            right = [0.0] * classes
            for r in srt:
                right[y[r]] += 1.0
        else:
            ts = ts2 = 0.0
            for r in srt:
                ts += y[r]
                ts2 += y[r] * y[r]
            ls = ls2 = 0.0
        for i in range(n - 1):
            r = srt[i]
            if classes:
                left[y[r]] += 1.0
                right[y[r]] -= 1.0
            else:
                ls += y[r]
                ls2 += y[r] * y[r]
            nl, nr = float(i + 1), float(n - i - 1)
            a, b = x[r][f], x[srt[i + 1]][f]
            if a >= b or i + 1 < min_leaf or n - i - 1 < min_leaf:
                continue
            if classes:
                il = nl - sum(v * v for v in left) / nl
                ir = nr - sum(v * v for v in right) / nr
                dec = parent - il - ir
            else:
                rs, rs2 = ts - ls, ts2 - ls2
                dec = parent - (ls2 - ls * ls / nl) - (rs2 - rs * rs / nr)
            if best is None or dec > best[0]:
                best = (dec, f, 0.5 * (a + b))
    return best


def grow(x, y, classes, rows, depth, min_leaf, max_depth, mtry, rng, imp):
    """Returns a nested tree: ('leaf', v) or ('split', f, t, left, right)."""
    value = leaf(rows, y, classes)
    parent = impurity(rows, y, classes)
    if parent <= 0.0 or len(rows) < 2 * min_leaf or (max_depth is not None and depth >= max_depth):
        return ("leaf", value)
    p = len(x[0])
    if rng is not None and mtry < p:
        allf = list(range(p))
        for i in range(mtry):
            j = i + rng.below(p - i)
            allf[i], allf[j] = allf[j], allf[i]
        features = allf[:mtry]
    else:
        features = list(range(p))
    bs = best_split(x, y, classes, rows, features, parent, min_leaf)
    if bs is None or bs[0] <= 0.0:
        return ("leaf", value)
    dec, f, t = bs
    imp[f] += dec
    left = [r for r in rows if x[r][f] <= t]
    right = [r for r in rows if not x[r][f] <= t]
    return ("split", f, t, grow(x, y, classes, left, depth + 1, min_leaf, max_depth, mtry, rng, imp),
            grow(x, y, classes, right, depth + 1, min_leaf, max_depth, mtry, rng, imp))


def predict(node, row):
    while node[0] == "split":
        node = node[3] if row[node[1]] <= node[2] else node[4]
    return node[1]


def preorder(node, out):
    if node[0] == "leaf":
        out.append(None)
    else:
        out.append((node[1], node[2]))
        preorder(node[3], out)
        preorder(node[4], out)
    return out


def aggregate(preds, classes):
    if not preds:
        return math.nan
    if not classes:
        return sum(preds) / len(preds)
    votes = [0] * classes
    for p in preds:
        votes[int(p)] += 1
    return float(votes.index(max(votes)))


def forest(x, y, classes, trees, mtry, min_leaf, max_depth, seed):
    n, p = len(x), len(x[0])
    rng = Rng(seed)
    imp = [0.0] * p
    oobp = [[] for _ in range(n)]
    ts = []
    for _ in range(trees):
        rows = [rng.below(n) for _ in range(n)]
        bag = set(rows)
        t = grow(x, y, classes, rows, 0, min_leaf, max_depth, mtry, rng, imp)
        for r in range(n):
            if r not in bag:
                oobp[r].append(predict(t, x[r]))
        ts.append(t)
    oob = [aggregate(pr, classes) for pr in oobp]
    sc = [(o, (y[r] if not classes else float(y[r]))) for r, o in enumerate(oob) if not math.isnan(o)]
    if classes:
        err = sum(1 for a, b in sc if a != b) / len(sc)
    else:
        err = sum((a - b) ** 2 for a, b in sc) / len(sc)
    tot = sum(imp)
    fitted = [aggregate([predict(t, row) for t in ts], classes) for row in x]
    return oob, err, [v / tot for v in imp], fitted, ts


def data(n, seed):
    rng = Rng(seed)
    x = [[round(rng.next() * 10, 3), round(rng.next() * 10, 3), float(rng.below(4)), round(rng.next(), 3)] for _ in range(n)]
    cls = [(0 if r[0] + 0.5 * r[2] < 5 else (1 if r[1] < 5 else 2)) for r in x]
    val = [r[0] * 2 - r[1] + 3 * (r[2] == 2) + 0.3 * r[3] for r in x]
    return x, cls, val


def num(v):
    return "f64::NAN" if math.isnan(v) else repr(float(v))


def pre(t):
    return "&[" + ", ".join("None" if s is None else f"Some(({s[0]}, {s[1]!r}))" for s in preorder(t, [])) + "]"


x, cls, val = data(80, 5)
rows = "&[" + ", ".join("[" + ", ".join(repr(float(v)) for v in r) + "]" for r in x) + "]"
print("// Made by tools/forest-ref.py (pure Python). Do not edit.")
print(f"pub const X: &[[f64; 4]] = {rows};")
print("pub const CLASSES: &[usize] = &[" + ", ".join(str(c) for c in cls) + "];")
print("pub const VALUES: &[f64] = &[" + ", ".join(repr(float(v)) for v in val) + "];")
for name, y, k, ml, md in [("CLASS_TREE", cls, 3, 1, None), ("VALUE_TREE", val, 0, 5, 4)]:
    imp = [0.0] * 4
    t = grow(x, y, k, list(range(len(x))), 0, ml, md, 4, None, imp)
    print(f"/// min_leaf {ml}, max_depth {md}: pre-order splits, importance (raw), predictions of the rows")
    print(f"pub const {name}: (&[Option<(usize, f64)>], [f64; 4], &[f64]) = ({pre(t)}, [{', '.join(repr(v) for v in imp)}], &[{', '.join(num(predict(t, r)) for r in x)}]);")
for name, y, k, ml in [("CLASS_FOREST", cls, 3, 1), ("VALUE_FOREST", val, 0, 5)]:
    mtry = 2 if k else 1
    oob, err, imp, fitted, ts = forest(x, y, k, 15, mtry, ml, None, 42)
    print(f"/// 15 trees, mtry {mtry}, min_leaf {ml}, seed 42: OOB predictions, OOB error, importance, fitted, first tree")
    print(f"pub const {name}: (&[f64], f64, [f64; 4], &[f64], &[Option<(usize, f64)>]) = (&[{', '.join(num(v) for v in oob)}], {err!r}, [{', '.join(repr(v) for v in imp)}], &[{', '.join(num(v) for v in fitted)}], {pre(ts[0])});")
