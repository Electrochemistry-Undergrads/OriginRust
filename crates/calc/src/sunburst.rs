//! The angular layout of a sunburst plot (graphing spec 14.27): the hierarchy of [`crate::treemap_tree`] as rings.
//!
//! One ring per level, the first level innermost. A node's angle is its value's share of its parent's angle (the
//! top level shares the full turn), so the angles of the children add up to the angle of the parent. Angles here are
//! fractions of a turn (0 to 1) counted from the start of the plot in the direction of the wedges; the plot turns
//! them into degrees, start angle and direction. A node's colour value is its own value for a leaf and the mean of
//! the values of its leaves for a category (Origin: "colour follows the category's mean value").

use crate::treemap_tree::{Node, Order, Tree, ordered};

/// One wedge: a node of the hierarchy at its ring.
#[derive(Clone, Debug, PartialEq)]
pub struct Wedge {
    /// 0 for the innermost ring.
    pub level: usize,
    /// Where the wedge starts, as a fraction of the turn.
    pub from: f64,
    /// The wedge's angle, as a fraction of the turn.
    pub span: f64,
    pub name: String,
    pub value: f64,
    /// The value the colour follows: the leaf's value, or the mean of the leaves below a category.
    pub color_value: f64,
    /// True when the category cell was empty (drawn unfilled).
    pub blank: bool,
    /// True when nothing is below the wedge.
    pub leaf: bool,
}

/// The wedges of every node of `tree`, parents before their children, siblings in `order`.
pub fn layout(tree: &Tree, order: Order) -> Vec<Wedge> {
    let mut out = Vec::new();
    place(&tree.roots, 0, 0.0, 1.0, order, &mut out);
    out
}

/// The sum and the count of the leaf values below (and including) `node`.
fn leaf_sum(node: &Node) -> (f64, usize) {
    if node.children.is_empty() {
        return (node.value, 1);
    }
    node.children.iter().map(leaf_sum).fold((0.0, 0), |a, b| (a.0 + b.0, a.1 + b.1))
}

fn place(nodes: &[Node], level: usize, from: f64, span: f64, order: Order, out: &mut Vec<Wedge>) {
    let total: f64 = nodes.iter().map(|n| n.value).sum();
    if total <= 0.0 {
        return;
    }
    let mut at = from;
    for &i in &ordered(nodes, order) {
        let node = &nodes[i];
        let w = span * node.value / total;
        let (sum, count) = leaf_sum(node);
        out.push(Wedge {
            level,
            from: at,
            span: w,
            name: node.name.clone(),
            value: node.value,
            color_value: sum / count.max(1) as f64,
            blank: node.blank,
            leaf: node.children.is_empty(),
        });
        place(&node.children, level + 1, at, w, order, out);
        at += w;
    }
}

/// The smallest and largest leaf value of the wedges (`None` when there are none).
pub fn leaf_range(wedges: &[Wedge]) -> Option<(f64, f64)> {
    wedges.iter().filter(|w| w.leaf).map(|w| w.value).fold(None, |r, v| match r {
        None => Some((v, v)),
        Some((lo, hi)) => Some((lo.min(v), hi.max(v))),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::treemap_tree::{Key, Row, build};

    fn row(path: &[&str], v: f64) -> Row {
        Row { path: path.iter().map(|s| s.to_string()).collect(), value: v, key: Key::Missing }
    }

    fn sample() -> Tree {
        build(&[row(&["A", "x"], 10.0), row(&["B", "y"], 20.0), row(&["A", "z"], 30.0), row(&["B", ""], 40.0)], 2)
    }

    #[test]
    fn angles_are_shares_of_the_parent() {
        let w = layout(&sample(), Order::Categorical);
        assert_eq!(w.len(), 6);
        let a = &w[0];
        assert_eq!((a.name.as_str(), a.level, a.from), ("A", 0, 0.0));
        assert!((a.span - 0.4).abs() < 1e-12);
        assert!((w[3].from - 0.4).abs() < 1e-12 && (w[3].span - 0.6).abs() < 1e-12);
        // Children of A: 10 and 30 of 40 of its 0.4.
        assert!((w[1].span - 0.1).abs() < 1e-12 && (w[2].from - 0.1).abs() < 1e-12);
        for level in 0..2 {
            let sum: f64 = w.iter().filter(|x| x.level == level).map(|x| x.span).sum();
            assert!((sum - 1.0).abs() < 1e-12);
        }
    }

    #[test]
    fn colour_values_are_leaf_values_and_category_means() {
        let w = layout(&sample(), Order::Categorical);
        assert_eq!(w[0].color_value, 20.0, "mean of 10 and 30");
        assert_eq!(w[1].color_value, 10.0);
        assert_eq!(w[3].color_value, 30.0, "mean of 20 and 40");
        assert_eq!(leaf_range(&w), Some((10.0, 40.0)));
        assert!(w[5].blank && !w[4].blank);
    }

    #[test]
    fn by_size_puts_the_largest_first() {
        let w = layout(&sample(), Order::BySize);
        assert_eq!(w[0].name, "B");
        assert_eq!(w[0].from, 0.0);
    }
}
