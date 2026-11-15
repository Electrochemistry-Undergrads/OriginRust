//! The hierarchy of a treemap and its nested layout (graphing spec 14.26).
//!
//! Rows of `(category path, value)` become a tree: one level per category column, the first column the root
//! level. Origin documents none of this; the rules are Sheaf's own and unmeasured:
//!
//! - A row whose value is missing, not finite or not positive is skipped (counted in [`Tree::skipped`]).
//! - Rows with the same path are one leaf whose value is their sum. An empty category cell is the name `(blank)`.
//! - A node's value is the sum of its children's. A node's sort key (for Order by column) is the smallest key of
//!   its rows (numbers before text, numbers by value, text alphabetically, missing last).
//! - Order: Categorical keeps the order of first appearance, Alphabetical sorts the names (case-insensitively),
//!   By Size sorts by value, largest first, By column sorts by the key ascending. Squarified layouts sort by size
//!   themselves and ignore the order of the list ([`crate::treemap::Layout::sorts_by_size`]).
//! - Nesting: the children of a node are laid out in the node's rectangle inset by half the gap (below a header
//!   band for a top-level node), and every laid-out cell is shrunk by half the gap on each side. So a child is at
//!   least one gap inside its parent and siblings are one gap apart. Depth N shows N levels; the nodes of the
//!   last shown level are leaves with the value of everything below them.
//! - A top-level node gets a header band (its name) when more than one level is shown and its rectangle is at
//!   least twice as high as the band.

use crate::treemap::{Layout, Rect, layout};
use std::cmp::Ordering;

/// A sort key of a row: the cell of the Order by column.
#[derive(Clone, Debug, PartialEq)]
pub enum Key {
    Num(f64),
    Text(String),
    Missing,
}

impl Key {
    fn rank(&self) -> u8 {
        match self {
            Key::Num(_) => 0,
            Key::Text(_) => 1,
            Key::Missing => 2,
        }
    }

    pub fn compare(&self, other: &Key) -> Ordering {
        match (self, other) {
            (Key::Num(a), Key::Num(b)) => a.total_cmp(b),
            (Key::Text(a), Key::Text(b)) => a.to_lowercase().cmp(&b.to_lowercase()).then_with(|| a.cmp(b)),
            _ => self.rank().cmp(&other.rank()),
        }
    }
}

/// One row of data: the category of each level, the value and the sort key.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub path: Vec<String>,
    pub value: f64,
    pub key: Key,
}

/// A node of the hierarchy.
#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub name: String,
    pub value: f64,
    pub key: Key,
    /// True when the category cell was empty (the name is then `(blank)`); a sunburst draws it unfilled.
    pub blank: bool,
    pub children: Vec<Node>,
}

/// The hierarchy of the rows.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Tree {
    /// The top-level nodes in order of first appearance.
    pub roots: Vec<Node>,
    /// The number of levels (category columns).
    pub levels: usize,
    /// The rows left out for a missing or non-positive value.
    pub skipped: usize,
}

fn blank(s: &str) -> String {
    if s.is_empty() { "(blank)".to_string() } else { s.to_string() }
}

/// Builds the tree of `rows`, each with a path of `levels` names.
pub fn build(rows: &[Row], levels: usize) -> Tree {
    let mut tree = Tree { roots: Vec::new(), levels, skipped: 0 };
    if levels == 0 {
        tree.skipped = rows.len();
        return tree;
    }
    for row in rows {
        if !(row.value.is_finite() && row.value > 0.0) || row.path.len() < levels {
            tree.skipped += 1;
            continue;
        }
        let mut nodes = &mut tree.roots;
        for (raw, name) in row.path.iter().take(levels).map(|s| (s.is_empty(), blank(s))) {
            let at = match nodes.iter().position(|n| n.name == name) {
                Some(i) => i,
                None => {
                    nodes.push(Node { name, value: 0.0, key: Key::Missing, blank: raw, children: Vec::new() });
                    nodes.len() - 1
                }
            };
            let node = &mut nodes[at];
            node.value += row.value;
            if row.key.compare(&node.key) == Ordering::Less {
                node.key = row.key.clone();
            }
            nodes = &mut node.children;
        }
    }
    tree
}

/// How the siblings of a level are ordered before they are laid out.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum Order {
    #[default]
    Categorical,
    Alphabetical,
    BySize,
    ByKey,
}

/// The layout and order of one level.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct Level {
    pub layout: Layout,
    pub order: Order,
}

/// Each level below the first has a header band (and header text) this fraction of the one above it.
pub const HEADER_SHRINK: f64 = 0.85;

/// What [`nested`] needs besides the tree and the rectangle.
#[derive(Clone, Debug, PartialEq)]
pub struct Params {
    /// The levels shown (`None`: all).
    pub depth: Option<usize>,
    /// The gap between rectangles, in the units of the rectangle.
    pub gap: f64,
    /// The height of the header band of a top-level node (0: none); level `L` has `HEADER_SHRINK^L` of it.
    pub header: f64,
    /// The layout and order of each level; a level past the end takes the last entry.
    pub levels: Vec<Level>,
}

/// One rectangle of the nested layout.
#[derive(Clone, Debug, PartialEq)]
pub struct Cell {
    /// 0 for a top-level node.
    pub level: usize,
    /// The index of the top-level node (in order of first appearance) this one is, or is inside.
    pub top: usize,
    pub name: String,
    pub value: f64,
    /// The cell of the layout, before the gap shrank it.
    pub slot: Rect,
    /// The rectangle drawn: the slot shrunk by half the gap on each side.
    pub rect: Rect,
    /// The band holding the name of a node with children shown, when the rectangle is tall (and, below
    /// the top level, wide) enough for it.
    pub header: Option<Rect>,
    /// True when no children are laid out inside (the last shown level, or nothing below).
    pub leaf: bool,
}

/// Orders `nodes` for a level: the indices, in layout order.
pub fn ordered(nodes: &[Node], order: Order) -> Vec<usize> {
    let mut idx: Vec<usize> = (0..nodes.len()).collect();
    match order {
        Order::Categorical => {}
        Order::Alphabetical => idx.sort_by(|&a, &b| {
            nodes[a]
                .name
                .to_lowercase()
                .cmp(&nodes[b].name.to_lowercase())
                .then_with(|| nodes[a].name.cmp(&nodes[b].name))
        }),
        Order::BySize => idx.sort_by(|&a, &b| nodes[b].value.total_cmp(&nodes[a].value)),
        Order::ByKey => idx.sort_by(|&a, &b| nodes[a].key.compare(&nodes[b].key)),
    }
    idx
}

/// The levels shown of a tree of `levels` levels.
pub fn shown_levels(depth: Option<usize>, levels: usize) -> usize {
    depth.unwrap_or(levels).clamp(1, levels.max(1))
}

/// Lays the tree out in `area`: every shown node a cell, parents before their children.
pub fn nested(tree: &Tree, area: Rect, p: &Params) -> Vec<Cell> {
    let mut out = Vec::new();
    let shown = shown_levels(p.depth, tree.levels);
    place(&tree.roots, area, 0, None, shown, p, &mut out);
    out
}

fn place(nodes: &[Node], area: Rect, level: usize, top: Option<usize>, shown: usize, p: &Params, out: &mut Vec<Cell>) {
    if nodes.is_empty() {
        return;
    }
    let setting = p.levels.get(level).or(p.levels.last()).copied().unwrap_or_default();
    let idx = ordered(nodes, setting.order);
    let weights: Vec<f64> = idx.iter().map(|&i| nodes[i].value).collect();
    let slots = layout(setting.layout, &weights, area);
    for (pos, &i) in idx.iter().enumerate() {
        let node = &nodes[i];
        let slot = slots[pos];
        let rect = slot.inset(p.gap / 2.0);
        let top_index = top.unwrap_or(i);
        let leaf = node.children.is_empty() || level + 1 >= shown;
        let mut header = None;
        let mut inner = rect.inset(p.gap / 2.0);
        let band = p.header * HEADER_SHRINK.powi(level as i32);
        if !leaf && band > 0.0 && rect.h >= 2.0 * band && (level == 0 || rect.w >= 2.0 * band) {
            header = Some(Rect::new(rect.x, rect.y, rect.w, band));
            inner = Rect::new(inner.x, rect.y + band + p.gap / 2.0, inner.w, (rect.h - band - p.gap).max(0.0));
        }
        out.push(Cell { level, top: top_index, name: node.name.clone(), value: node.value, slot, rect, header, leaf });
        if !leaf {
            place(&node.children, inner, level + 1, Some(top_index), shown, p, out);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(path: &[&str], v: f64) -> Row {
        Row { path: path.iter().map(|s| s.to_string()).collect(), value: v, key: Key::Missing }
    }

    fn sample() -> Tree {
        build(
            &[
                row(&["Asia", "China"], 1400.0),
                row(&["Europe", "France"], 68.0),
                row(&["Asia", "India"], 1380.0),
                row(&["Europe", "Germany"], 83.0),
                row(&["Asia", "China"], 100.0),
                row(&["Europe", "Spain"], 0.0),
                row(&["Europe", "Italy"], f64::NAN),
                row(&["Africa", "Egypt"], -3.0),
                row(&["Africa", ""], 5.0),
            ],
            2,
        )
    }

    #[test]
    fn duplicates_are_summed_and_bad_values_skipped() {
        let t = sample();
        assert_eq!(t.skipped, 3);
        let names: Vec<&str> = t.roots.iter().map(|n| n.name.as_str()).collect();
        assert_eq!(names, ["Asia", "Europe", "Africa"], "order of first appearance");
        assert_eq!(t.roots[0].value, 2880.0);
        assert_eq!(t.roots[0].children[0].name, "China");
        assert_eq!(t.roots[0].children[0].value, 1500.0);
        assert_eq!(t.roots[1].value, 151.0);
        assert_eq!(t.roots[2].children[0].name, "(blank)");
        assert_eq!(build(&[row(&["a"], 1.0)], 0).skipped, 1);
    }

    #[test]
    fn the_same_name_under_two_parents_is_two_nodes() {
        let t = build(&[row(&["a", "x"], 1.0), row(&["b", "x"], 2.0)], 2);
        assert_eq!(t.roots[0].children[0].value, 1.0);
        assert_eq!(t.roots[1].children[0].value, 2.0);
    }

    #[test]
    fn orders() {
        let mut t = build(
            &[
                Row { path: vec!["b".into()], value: 1.0, key: Key::Num(3.0) },
                Row { path: vec!["A".into()], value: 5.0, key: Key::Num(1.0) },
                Row { path: vec!["c".into()], value: 3.0, key: Key::Missing },
                Row { path: vec!["b".into()], value: 1.0, key: Key::Num(0.5) },
            ],
            1,
        );
        let names = |t: &Tree, o| ordered(&t.roots, o).into_iter().map(|i| t.roots[i].name.clone()).collect::<Vec<_>>();
        assert_eq!(names(&t, Order::Categorical), ["b", "A", "c"]);
        assert_eq!(names(&t, Order::Alphabetical), ["A", "b", "c"]);
        assert_eq!(names(&t, Order::BySize), ["A", "c", "b"]);
        assert_eq!(names(&t, Order::ByKey), ["b", "A", "c"], "b's smallest key is 0.5");
        t.roots[2].key = Key::Text("z".into());
        assert_eq!(names(&t, Order::ByKey), ["b", "A", "c"]);
    }

    fn params(depth: Option<usize>, gap: f64, header: f64) -> Params {
        Params { depth, gap, header, levels: vec![Level::default()] }
    }

    fn inside(inner: &Rect, outer: &Rect, margin: f64) -> bool {
        inner.x >= outer.x + margin - 1e-9
            && inner.y >= outer.y + margin - 1e-9
            && inner.x + inner.w <= outer.x + outer.w - margin + 1e-9
            && inner.y + inner.h <= outer.y + outer.h - margin + 1e-9
    }

    #[test]
    fn without_a_gap_the_leaves_tile_in_proportion() {
        let t = sample();
        let area = Rect::new(0.0, 0.0, 400.0, 300.0);
        let cells = nested(&t, area, &params(None, 0.0, 0.0));
        let leaves: Vec<&Cell> = cells.iter().filter(|c| c.leaf).collect();
        assert_eq!(leaves.len(), 5);
        let total: f64 = t.roots.iter().map(|n| n.value).sum();
        for c in &leaves {
            let want = c.value / total * area.area();
            assert!((c.rect.area() - want).abs() <= 1e-9 * want, "{}", c.name);
        }
        let tops: Vec<&Cell> = cells.iter().filter(|c| c.level == 0).collect();
        assert_eq!(tops.len(), 3);
        for c in &cells {
            assert_eq!(c.rect, c.slot);
        }
    }

    #[test]
    fn children_lie_inside_their_parent_by_the_gap() {
        let t = sample();
        let cells = nested(&t, Rect::new(0.0, 0.0, 400.0, 300.0), &params(None, 4.0, 14.0));
        for c in cells.iter().filter(|c| c.level == 1) {
            let parent = cells.iter().find(|p| p.level == 0 && p.top == c.top).unwrap();
            if parent.rect.h < 40.0 || parent.rect.w < 40.0 {
                continue; // too small for the gap: the children shrink to nothing
            }
            let band = parent.header.map_or(0.0, |h| h.h);
            assert!(
                c.rect.x >= parent.rect.x + 4.0 - 1e-9
                    && c.rect.x + c.rect.w <= parent.rect.x + parent.rect.w - 4.0 + 1e-9
            );
            assert!(c.rect.y >= parent.rect.y + band + 4.0 - 1e-9, "{c:?} {parent:?}");
            assert!(c.rect.y + c.rect.h <= parent.rect.y + parent.rect.h - 4.0 + 1e-9);
        }
        // Siblings are one gap apart at least.
        let kids: Vec<&Cell> = cells.iter().filter(|c| c.level == 1 && c.top == 0).collect();
        for a in &kids {
            for b in &kids {
                if a.name != b.name {
                    let dx = (a.rect.x - (b.rect.x + b.rect.w)).max(b.rect.x - (a.rect.x + a.rect.w));
                    let dy = (a.rect.y - (b.rect.y + b.rect.h)).max(b.rect.y - (a.rect.y + a.rect.h));
                    assert!(dx.max(dy) >= 4.0 - 1e-9);
                }
            }
        }
        assert!(cells.iter().all(|c| inside(&c.rect, &Rect::new(0.0, 0.0, 400.0, 300.0), 2.0)));
    }

    #[test]
    fn depth_one_shows_only_the_top_level_and_headers_need_depth_over_one() {
        let t = sample();
        let area = Rect::new(0.0, 0.0, 400.0, 300.0);
        let one = nested(&t, area, &params(Some(1), 2.0, 14.0));
        assert_eq!(one.len(), 3);
        assert!(one.iter().all(|c| c.leaf && c.header.is_none()));
        assert_eq!(one[0].value, 2880.0, "the value of everything below");
        let two = nested(&t, area, &params(None, 2.0, 14.0));
        assert_eq!(two.len(), 3 + 5);
        // Africa's rectangle is too small for a band.
        let with_band: Vec<&str> = two.iter().filter(|c| c.header.is_some()).map(|c| c.name.as_str()).collect();
        assert_eq!(with_band, ["Asia", "Europe"]);
        let deep = nested(&t, area, &params(Some(9), 2.0, 14.0));
        assert_eq!(deep, two, "depth past the levels shows them all");
    }

    #[test]
    fn colour_index_is_the_first_appearance_whatever_the_order() {
        let t = sample();
        let p = Params {
            depth: None,
            gap: 0.0,
            header: 0.0,
            levels: vec![Level { layout: Layout::Split, order: Order::BySize }],
        };
        let cells = nested(&t, Rect::new(0.0, 0.0, 100.0, 100.0), &p);
        assert_eq!(cells[0].name, "Asia");
        for c in &cells {
            let top = &t.roots[c.top].name;
            if c.level == 0 {
                assert_eq!(&c.name, top);
            }
        }
        let europe = cells.iter().find(|c| c.name == "Europe").unwrap();
        assert_eq!(europe.top, 1);
    }

    #[test]
    fn per_level_settings_apply_by_level() {
        let t = sample();
        let p = Params {
            depth: None,
            gap: 0.0,
            header: 0.0,
            levels: vec![Level::default(), Level { layout: Layout::Split, order: Order::Alphabetical }],
        };
        let cells = nested(&t, Rect::new(0.0, 0.0, 100.0, 100.0), &p);
        let asia: Vec<&str> = cells.iter().filter(|c| c.level == 1 && c.top == 0).map(|c| c.name.as_str()).collect();
        assert_eq!(asia, ["China", "India"]);
        let africa: Vec<&str> = cells.iter().filter(|c| c.level == 1 && c.top == 2).map(|c| c.name.as_str()).collect();
        assert_eq!(africa, ["(blank)"]);
        assert_eq!(shown_levels(None, 3), 3);
        assert_eq!(shown_levels(Some(0), 3), 1);
    }
}
