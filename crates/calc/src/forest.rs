//! Classification and regression trees (CART) and random forests (Origin 2026b's Random Forest app; not in Origin
//! 2016, so nothing here is measured in Origin; `docs/spec/statistics.md` section 44).
//!
//! **Trees**: a node is split on the feature and threshold that most reduce the impurity (`n - sum c_k^2 / n`, `n`
//! times the Gini index, for classes; `sum y^2 - (sum y)^2 / n`, the sum of squares, for values). Thresholds are the
//! midpoints between consecutive distinct sorted values with at least `min_leaf` rows on each side; rows with a value
//! at or below the threshold go left. Features are tried in their order (or in the drawn order of a forest's subset)
//! and thresholds from the smallest; a later candidate replaces the best only when strictly better. A node becomes a
//! leaf when it is pure, has fewer than `2 min_leaf` rows, reaches the depth limit or has no valid split. A leaf
//! predicts the most frequent class (the lowest index on ties) or the mean.
//!
//! **Forests**: each tree grows on a bootstrap sample (`n` draws with replacement) and tries `mtry` features drawn at
//! each node (a partial Fisher-Yates shuffle). Draws come from the 64-bit generator `s = s * 6364136223846793005 +
//! 1442695040888963407`, `u = (s >> 11) / 2^53`, seeded with the given seed. Out-of-bag predictions use the trees whose
//! sample left the row out (majority vote, lowest class on ties, or mean); the out-of-bag error is the share
//! misclassified or the mean squared error over rows with at least one such tree. Importance is each feature's total
//! impurity decrease over all trees, scaled to sum to 1.
//!
//! Checked against an independent Python implementation of the same rules in `crates/calc/tests/forest_reference.rs`
//! (made by `tools/forest-ref.py`): single trees and whole seeded forests agree exactly.

/// What is predicted.
#[derive(Debug, Clone, PartialEq)]
pub enum Target {
    /// Class indices `0..classes`.
    Class {
        y: Vec<usize>,
        classes: usize,
    },
    Value(Vec<f64>),
}

impl Target {
    fn len(&self) -> usize {
        match self {
            Target::Class { y, .. } => y.len(),
            Target::Value(y) => y.len(),
        }
    }

    fn is_class(&self) -> bool {
        matches!(self, Target::Class { .. })
    }
}

/// How the forest grows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ForestOptions {
    pub trees: usize,
    /// Features tried at each node; `None`: `floor(sqrt p)` for classes, `floor(p / 3)` for values (at least 1).
    pub mtry: Option<usize>,
    pub min_leaf: usize,
    pub max_depth: Option<usize>,
    pub seed: u64,
    /// Grow each tree on a bootstrap sample (off: every tree sees all rows, so there is no out-of-bag error).
    pub bootstrap: bool,
}

impl Default for ForestOptions {
    fn default() -> Self {
        ForestOptions { trees: 100, mtry: None, min_leaf: 1, max_depth: None, seed: 1, bootstrap: true }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Node {
    Leaf(f64),
    Split { feature: usize, threshold: f64, left: usize, right: usize },
}

/// One tree.
#[derive(Debug, Clone, PartialEq)]
pub struct Tree {
    nodes: Vec<Node>,
}

impl Tree {
    /// The prediction for a row: a class index (as a number) or a value.
    pub fn predict(&self, row: &[f64]) -> f64 {
        let mut i = 0;
        loop {
            match &self.nodes[i] {
                Node::Leaf(v) => return *v,
                Node::Split { feature, threshold, left, right } => {
                    i = if row[*feature] <= *threshold { *left } else { *right };
                }
            }
        }
    }

    /// The splits in pre-order: `(feature, threshold)`, and `None` for a leaf.
    pub fn preorder(&self) -> Vec<Option<(usize, f64)>> {
        let mut out = Vec::new();
        let mut stack = vec![0];
        while let Some(i) = stack.pop() {
            match &self.nodes[i] {
                Node::Leaf(_) => out.push(None),
                Node::Split { feature, threshold, left, right } => {
                    out.push(Some((*feature, *threshold)));
                    stack.push(*right);
                    stack.push(*left);
                }
            }
        }
        out
    }

    pub fn leaves(&self) -> usize {
        self.nodes.iter().filter(|n| matches!(n, Node::Leaf(_))).count()
    }
}

/// A grown forest.
#[derive(Debug, Clone, PartialEq)]
pub struct Forest {
    pub trees: Vec<Tree>,
    /// Out-of-bag prediction of each row (NaN when every tree saw it).
    pub oob: Vec<f64>,
    /// Share misclassified or mean squared error, out of bag (NaN without bootstrap).
    pub oob_error: f64,
    pub importance: Vec<f64>,
    /// Prediction of each row by all trees.
    pub fitted: Vec<f64>,
    classes: usize,
}

impl Forest {
    /// The forest's prediction for a row.
    pub fn predict(&self, row: &[f64]) -> f64 {
        let preds: Vec<f64> = self.trees.iter().map(|t| t.predict(row)).collect();
        aggregate(&preds, self.classes)
    }
}

/// Why the forest could not be grown.
#[derive(Debug, Clone, PartialEq)]
pub enum ForestError {
    Empty,
    Ragged,
    Missing(usize),
    BadOption(&'static str),
}

impl std::fmt::Display for ForestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ForestError::Empty => f.write_str("no rows or no predictors"),
            ForestError::Ragged => f.write_str("the rows and the target differ in length"),
            ForestError::Missing(r) => write!(f, "row {} has a missing value", r + 1),
            ForestError::BadOption(what) => f.write_str(what),
        }
    }
}

impl std::error::Error for ForestError {}

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> f64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }

    /// A whole number in `0..n`.
    fn below(&mut self, n: usize) -> usize {
        ((self.next() * n as f64) as usize).min(n - 1)
    }
}

/// The majority class (lowest on ties) or the mean of `preds`.
fn aggregate(preds: &[f64], classes: usize) -> f64 {
    if preds.is_empty() {
        return f64::NAN;
    }
    if classes == 0 {
        return preds.iter().sum::<f64>() / preds.len() as f64;
    }
    let mut votes = vec![0usize; classes];
    for p in preds {
        votes[*p as usize] += 1;
    }
    let best = votes.iter().copied().max().unwrap_or(0);
    votes.iter().position(|v| *v == best).unwrap_or(0) as f64
}

struct Grower<'a> {
    x: &'a [Vec<f64>],
    y: &'a Target,
    min_leaf: usize,
    max_depth: Option<usize>,
    mtry: usize,
    importance: Vec<f64>,
}

impl Grower<'_> {
    fn impurity(&self, rows: &[usize]) -> f64 {
        let n = rows.len() as f64;
        match self.y {
            Target::Class { y, classes } => {
                let mut c = vec![0.0; *classes];
                for &r in rows {
                    c[y[r]] += 1.0;
                }
                n - c.iter().map(|v| v * v).sum::<f64>() / n
            }
            Target::Value(y) => {
                let (mut s, mut s2) = (0.0, 0.0);
                for &r in rows {
                    s += y[r];
                    s2 += y[r] * y[r];
                }
                s2 - s * s / n
            }
        }
    }

    fn leaf(&self, rows: &[usize]) -> f64 {
        match self.y {
            Target::Class { y, classes } => {
                let mut c = vec![0usize; *classes];
                for &r in rows {
                    c[y[r]] += 1;
                }
                let best = c.iter().copied().max().unwrap_or(0);
                c.iter().position(|v| *v == best).unwrap_or(0) as f64
            }
            Target::Value(y) => rows.iter().map(|&r| y[r]).sum::<f64>() / rows.len() as f64,
        }
    }

    /// The best split of `rows` over `features`: `(decrease, feature, threshold)`.
    fn best_split(&self, rows: &[usize], features: &[usize], parent: f64) -> Option<(f64, usize, f64)> {
        let n = rows.len();
        let mut best: Option<(f64, usize, f64)> = None;
        for &f in features {
            let mut sorted = rows.to_vec();
            sorted.sort_by(|&a, &b| self.x[a][f].total_cmp(&self.x[b][f]).then(a.cmp(&b)));
            match self.y {
                Target::Class { y, classes } => {
                    let mut left = vec![0.0; *classes];
                    let mut right = vec![0.0; *classes];
                    for &r in &sorted {
                        right[y[r]] += 1.0;
                    }
                    for i in 0..n - 1 {
                        let r = sorted[i];
                        left[y[r]] += 1.0;
                        right[y[r]] -= 1.0;
                        let (nl, nr) = ((i + 1) as f64, (n - i - 1) as f64);
                        let (a, b) = (self.x[r][f], self.x[sorted[i + 1]][f]);
                        if a >= b || i + 1 < self.min_leaf || n - i - 1 < self.min_leaf {
                            continue;
                        }
                        let il = nl - left.iter().map(|v| v * v).sum::<f64>() / nl;
                        let ir = nr - right.iter().map(|v| v * v).sum::<f64>() / nr;
                        let dec = parent - il - ir;
                        if best.is_none_or(|bst| dec > bst.0) {
                            best = Some((dec, f, 0.5 * (a + b)));
                        }
                    }
                }
                Target::Value(y) => {
                    let (ts, ts2) = sorted.iter().fold((0.0, 0.0), |(s, s2), &r| (s + y[r], s2 + y[r] * y[r]));
                    let (mut ls, mut ls2) = (0.0, 0.0);
                    for i in 0..n - 1 {
                        let r = sorted[i];
                        ls += y[r];
                        ls2 += y[r] * y[r];
                        let (nl, nr) = ((i + 1) as f64, (n - i - 1) as f64);
                        let (a, b) = (self.x[r][f], self.x[sorted[i + 1]][f]);
                        if a >= b || i + 1 < self.min_leaf || n - i - 1 < self.min_leaf {
                            continue;
                        }
                        let (rs, rs2) = (ts - ls, ts2 - ls2);
                        let dec = parent - (ls2 - ls * ls / nl) - (rs2 - rs * rs / nr);
                        if best.is_none_or(|bst| dec > bst.0) {
                            best = Some((dec, f, 0.5 * (a + b)));
                        }
                    }
                }
            }
        }
        best
    }

    fn grow(&mut self, rows: Vec<usize>, depth: usize, nodes: &mut Vec<Node>, rng: &mut Option<Rng>) -> usize {
        let at = nodes.len();
        nodes.push(Node::Leaf(self.leaf(&rows)));
        let parent = self.impurity(&rows);
        if parent <= 0.0 || rows.len() < 2 * self.min_leaf || self.max_depth.is_some_and(|m| depth >= m) {
            return at;
        }
        let p = self.x[0].len();
        let features: Vec<usize> = match rng {
            Some(g) if self.mtry < p => {
                let mut all: Vec<usize> = (0..p).collect();
                for i in 0..self.mtry {
                    let j = i + g.below(p - i);
                    all.swap(i, j);
                }
                all.truncate(self.mtry);
                all
            }
            _ => (0..p).collect(),
        };
        let Some((dec, feature, threshold)) = self.best_split(&rows, &features, parent) else { return at };
        if dec <= 0.0 {
            return at;
        }
        self.importance[feature] += dec;
        let (l, r): (Vec<usize>, Vec<usize>) = rows.iter().partition(|&&i| self.x[i][feature] <= threshold);
        let left = self.grow(l, depth + 1, nodes, rng);
        let right = self.grow(r, depth + 1, nodes, rng);
        nodes[at] = Node::Split { feature, threshold, left, right };
        at
    }
}

fn validate(x: &[Vec<f64>], y: &Target) -> Result<usize, ForestError> {
    if x.is_empty() || x[0].is_empty() {
        return Err(ForestError::Empty);
    }
    if x.len() != y.len() || x.iter().any(|r| r.len() != x[0].len()) {
        return Err(ForestError::Ragged);
    }
    if let Some(r) = x.iter().position(|row| row.iter().any(|v| !v.is_finite())) {
        return Err(ForestError::Missing(r));
    }
    if let Target::Value(v) = y
        && let Some(r) = v.iter().position(|v| !v.is_finite())
    {
        return Err(ForestError::Missing(r));
    }
    if let Target::Class { y, classes } = y
        && y.iter().any(|c| c >= classes)
    {
        return Err(ForestError::BadOption("a class index is out of range"));
    }
    Ok(x[0].len())
}

/// One tree on all rows and all features.
pub fn tree(
    x: &[Vec<f64>],
    y: &Target,
    min_leaf: usize,
    max_depth: Option<usize>,
) -> Result<(Tree, Vec<f64>), ForestError> {
    let p = validate(x, y)?;
    if min_leaf == 0 {
        return Err(ForestError::BadOption("the smallest leaf must hold at least 1 row"));
    }
    let mut g = Grower { x, y, min_leaf, max_depth, mtry: p, importance: vec![0.0; p] };
    let mut nodes = Vec::new();
    g.grow((0..x.len()).collect(), 0, &mut nodes, &mut None);
    Ok((Tree { nodes }, g.importance))
}

/// Grows a random forest.
pub fn forest(x: &[Vec<f64>], y: &Target, opts: ForestOptions) -> Result<Forest, ForestError> {
    let p = validate(x, y)?;
    if opts.trees == 0 {
        return Err(ForestError::BadOption("grow at least one tree"));
    }
    if opts.min_leaf == 0 {
        return Err(ForestError::BadOption("the smallest leaf must hold at least 1 row"));
    }
    let mtry = opts.mtry.unwrap_or(if y.is_class() { (p as f64).sqrt().floor() as usize } else { p / 3 }).clamp(1, p);
    let n = x.len();
    let classes = match y {
        Target::Class { classes, .. } => *classes,
        Target::Value(_) => 0,
    };
    let mut g = Grower { x, y, min_leaf: opts.min_leaf, max_depth: opts.max_depth, mtry, importance: vec![0.0; p] };
    let mut rng = Some(Rng(opts.seed));
    let mut trees = Vec::with_capacity(opts.trees);
    let mut oob_preds: Vec<Vec<f64>> = vec![Vec::new(); n];
    for _ in 0..opts.trees {
        let rows: Vec<usize> = if opts.bootstrap {
            let r = rng.as_mut().expect("the generator");
            (0..n).map(|_| r.below(n)).collect()
        } else {
            (0..n).collect()
        };
        let mut in_bag = vec![false; n];
        for &r in &rows {
            in_bag[r] = true;
        }
        let mut nodes = Vec::new();
        g.grow(rows, 0, &mut nodes, &mut rng);
        let t = Tree { nodes };
        for (r, seen) in in_bag.iter().enumerate() {
            if !seen {
                oob_preds[r].push(t.predict(&x[r]));
            }
        }
        trees.push(t);
    }
    let oob: Vec<f64> = oob_preds.iter().map(|p| aggregate(p, classes)).collect();
    let scored: Vec<(f64, f64)> = oob
        .iter()
        .enumerate()
        .filter(|(_, v)| v.is_finite())
        .map(|(r, v)| match y {
            Target::Class { y, .. } => (*v, y[r] as f64),
            Target::Value(y) => (*v, y[r]),
        })
        .collect();
    let oob_error = if scored.is_empty() {
        f64::NAN
    } else if classes > 0 {
        scored.iter().filter(|(p, t)| p != t).count() as f64 / scored.len() as f64
    } else {
        scored.iter().map(|(p, t)| (p - t).powi(2)).sum::<f64>() / scored.len() as f64
    };
    let total: f64 = g.importance.iter().sum();
    let importance = g.importance.iter().map(|v| if total > 0.0 { v / total } else { 0.0 }).collect();
    let mut f = Forest { trees, oob, oob_error, importance, fitted: Vec::new(), classes };
    f.fitted = x.iter().map(|row| f.predict(row)).collect();
    Ok(f)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tree_finds_the_one_useful_split() {
        // Class 1 exactly when the first feature is above 2.5; the second feature is noise.
        let x: Vec<Vec<f64>> = (0..6).map(|i| vec![f64::from(i), f64::from((i * 7) % 5)]).collect();
        let y = Target::Class { y: vec![0, 0, 0, 1, 1, 1], classes: 2 };
        let (t, imp) = tree(&x, &y, 1, None).unwrap();
        assert_eq!(t.preorder(), [Some((0, 2.5)), None, None]);
        assert_eq!((imp[0], imp[1]), (3.0, 0.0), "6 - 36/6: the root's n times Gini");
        assert_eq!(t.predict(&[2.0, 9.0]), 0.0);
        let v = Target::Value(vec![1.0, 1.0, 1.0, 5.0, 5.0, 5.0]);
        let (t, _) = tree(&x, &v, 1, None).unwrap();
        assert_eq!(t.predict(&[4.0, 0.0]), 5.0);
        assert_eq!(t.leaves(), 2);
    }

    #[test]
    fn a_forest_is_reproducible_and_learns() {
        let x: Vec<Vec<f64>> =
            (0..60).map(|i| vec![f64::from(i % 10), f64::from((i * 13) % 7), f64::from(i / 10)]).collect();
        let y = Target::Class { y: (0..60).map(|i| usize::from(i % 10 >= 5)).collect(), classes: 2 };
        let opts = ForestOptions { trees: 25, ..Default::default() };
        let a = forest(&x, &y, opts).unwrap();
        let b = forest(&x, &y, opts).unwrap();
        assert_eq!(a, b, "same seed, same forest");
        assert!(a.oob_error < 0.1, "{}", a.oob_error);
        assert!(a.importance[0] > 0.5, "{:?}", a.importance);
        assert!((a.importance.iter().sum::<f64>() - 1.0).abs() < 1e-12);
        let c = forest(&x, &y, ForestOptions { seed: 2, ..opts }).unwrap();
        assert_ne!(a.trees, c.trees);
        let all = forest(&x, &y, ForestOptions { bootstrap: false, trees: 3, ..opts }).unwrap();
        assert!(all.oob_error.is_nan());
    }

    #[test]
    fn bad_input_is_refused() {
        let y = Target::Value(vec![1.0, 2.0]);
        assert_eq!(forest(&[], &y, ForestOptions::default()), Err(ForestError::Empty));
        assert_eq!(forest(&[vec![1.0]], &y, ForestOptions::default()), Err(ForestError::Ragged));
        assert_eq!(forest(&[vec![1.0], vec![f64::NAN]], &y, ForestOptions::default()), Err(ForestError::Missing(1)));
        assert!(forest(&[vec![1.0], vec![2.0]], &y, ForestOptions { trees: 0, ..Default::default() }).is_err());
    }
}
