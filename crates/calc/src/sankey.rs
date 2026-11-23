//! Sankey diagram layout (T379, Origin 2020's Sankey plot; Sheaf's own rules, not measured): rows of
//! (source, target, value) become nodes in stages and links whose widths are proportional to their values.
//!
//! - Nodes are the distinct names of both columns, in order of first appearance; links with the same source
//!   and target are summed. Rows with an empty name, a value that is missing or not above 0, or the same
//!   source and target are left out.
//! - A node's stage is the length of the longest path to it from a node without incoming links. A link that
//!   would close a cycle (found by a depth-first search in node order) is left out of the layout.
//! - Stages run from left (0) to right; a node's height is proportional to its throughput, the larger of its
//!   inflow and outflow. One scale fits the fullest stage between 0 and 1 with a gap between its nodes;
//!   every stage is centred vertically. Nodes keep their order of first appearance within a stage.
//! - A link leaves its source from the node's right edge and enters its target at the left edge; the links
//!   of a node are stacked from the top in the order of the other end's position.
//!
//! Coordinates are fractions of the plotting area: x from 0 (left) to 1, y from 0 (top) to 1.

use std::collections::HashMap;

/// The width of a node as a fraction of the area's width.
pub const NODE_WIDTH: f64 = 0.03;
/// The free space between the nodes of the fullest stage, as a fraction of the height.
pub const GAP_SHARE: f64 = 0.2;

/// One node: its name, stage and box.
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub name: String,
    pub stage: usize,
    pub x0: f64,
    pub x1: f64,
    pub y0: f64,
    pub y1: f64,
    /// The larger of the inflow and the outflow.
    pub value: f64,
}

/// One link: its ends, value and band at each end (top and bottom).
#[derive(Debug, Clone, PartialEq)]
pub struct Link {
    pub source: usize,
    pub target: usize,
    pub value: f64,
    pub source_y: (f64, f64),
    pub target_y: (f64, f64),
}

/// The laid-out diagram.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Sankey {
    pub nodes: Vec<Node>,
    pub links: Vec<Link>,
    /// The number of stages.
    pub stages: usize,
    /// Links left out because they would close a cycle.
    pub dropped_cycles: usize,
}

/// Lays out the diagram of `rows` (source, target, value).
pub fn layout(rows: &[(String, String, f64)]) -> Sankey {
    layout_in_stages(rows, &|_| None)
}

/// [`layout`] with stages given for some nodes: `stage(name)` overrides the longest-path stage when it is
/// `Some` (an alluvial diagram puts every node in the stage of its column, T381).
pub fn layout_in_stages(rows: &[(String, String, f64)], fixed: &dyn Fn(&str) -> Option<usize>) -> Sankey {
    let mut index: HashMap<String, usize> = HashMap::new();
    let mut names: Vec<String> = Vec::new();
    let mut id = |name: &str, names: &mut Vec<String>| -> usize {
        *index.entry(name.to_string()).or_insert_with(|| {
            names.push(name.to_string());
            names.len() - 1
        })
    };
    let mut sums: Vec<((usize, usize), f64)> = Vec::new();
    for (s, t, v) in rows {
        let (s, t) = (s.trim(), t.trim());
        if s.is_empty() || t.is_empty() || s == t || !(v.is_finite() && *v > 0.0) {
            continue;
        }
        let key = (id(s, &mut names), id(t, &mut names));
        match sums.iter_mut().find(|(k, _)| *k == key) {
            Some((_, total)) => *total += v,
            None => sums.push((key, *v)),
        }
    }
    let n = names.len();
    if n == 0 {
        return Sankey::default();
    }
    // Drop links that close a cycle: depth-first search in node order, back edges out.
    let mut out: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (k, ((s, _), _)) in sums.iter().enumerate() {
        out[*s].push(k);
    }
    let mut state = vec![0u8; n]; // 0 new, 1 on the stack, 2 done
    let mut keep = vec![true; sums.len()];
    fn visit(u: usize, out: &[Vec<usize>], sums: &[((usize, usize), f64)], state: &mut [u8], keep: &mut [bool]) {
        state[u] = 1;
        for &k in &out[u] {
            let v = sums[k].0.1;
            match state[v] {
                0 => visit(v, out, sums, state, keep),
                1 => keep[k] = false,
                _ => {}
            }
        }
        state[u] = 2;
    }
    for u in 0..n {
        if state[u] == 0 {
            visit(u, &out, &sums, &mut state, &mut keep);
        }
    }
    let dropped_cycles = keep.iter().filter(|k| !**k).count();
    let links_raw: Vec<(usize, usize, f64)> =
        sums.iter().zip(&keep).filter(|(_, k)| **k).map(|(((s, t), v), _)| (*s, *t, *v)).collect();
    // Stages: the longest path from a node without incoming links (relaxation over the acyclic links).
    let mut stage = vec![0usize; n];
    for _ in 0..n {
        let mut changed = false;
        for &(s, t, _) in &links_raw {
            if stage[t] < stage[s] + 1 {
                stage[t] = stage[s] + 1;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    for (i, name) in names.iter().enumerate() {
        if let Some(s) = fixed(name) {
            stage[i] = s;
        }
    }
    let stages = stage.iter().copied().max().unwrap_or(0) + 1;
    let (mut inflow, mut outflow) = (vec![0.0; n], vec![0.0; n]);
    for &(s, t, v) in &links_raw {
        outflow[s] += v;
        inflow[t] += v;
    }
    let value: Vec<f64> = (0..n).map(|i| f64::max(inflow[i], outflow[i])).collect();
    let members: Vec<Vec<usize>> =
        (0..stages).map(|s| (0..n).filter(|&i| stage[i] == s && value[i] > 0.0).collect()).collect();
    // One scale for all stages: the fullest stage fills the height less its gaps.
    let gap = |count: usize| if count > 1 { GAP_SHARE / (count - 1) as f64 } else { 0.0 };
    let scale = members
        .iter()
        .filter(|m| !m.is_empty())
        .map(|m| {
            let total: f64 = m.iter().map(|&i| value[i]).sum();
            (1.0 - if m.len() > 1 { GAP_SHARE } else { 0.0 }) / total
        })
        .fold(f64::INFINITY, f64::min);
    let max_gap = members.iter().map(|m| gap(m.len())).filter(|g| *g > 0.0).fold(f64::INFINITY, f64::min);
    let max_gap = if max_gap.is_finite() { max_gap } else { 0.0 };
    let mut nodes: Vec<Node> = names
        .iter()
        .enumerate()
        .map(|(i, name)| Node {
            name: name.clone(),
            stage: stage[i],
            x0: 0.0,
            x1: 0.0,
            y0: 0.0,
            y1: 0.0,
            value: value[i],
        })
        .collect();
    for (s, m) in members.iter().enumerate() {
        let x0 = if stages > 1 { (1.0 - NODE_WIDTH) * s as f64 / (stages - 1) as f64 } else { 0.5 - NODE_WIDTH / 2.0 };
        let height: f64 = m.iter().map(|&i| value[i] * scale).sum::<f64>() + max_gap * m.len().saturating_sub(1) as f64;
        let mut y = (1.0 - height) / 2.0;
        for &i in m {
            let h = value[i] * scale;
            let node = &mut nodes[i];
            (node.x0, node.x1, node.y0, node.y1) = (x0, x0 + NODE_WIDTH, y, y + h);
            y += h + max_gap;
        }
    }
    // Stack the links on each node, ordered by the other end's position.
    let mut links: Vec<Link> = links_raw
        .iter()
        .map(|&(s, t, v)| Link { source: s, target: t, value: v, source_y: (0.0, 0.0), target_y: (0.0, 0.0) })
        .collect();
    let mid = |i: usize, nodes: &[Node]| (nodes[i].y0 + nodes[i].y1) / 2.0;
    let mut by_source: Vec<usize> = (0..links.len()).collect();
    by_source.sort_by(|&a, &b| {
        (links[a].source, mid(links[a].target, &nodes))
            .partial_cmp(&(links[b].source, mid(links[b].target, &nodes)))
            .unwrap()
    });
    let mut top = vec![f64::NAN; n];
    for &k in &by_source {
        let s = links[k].source;
        let y = if top[s].is_nan() { nodes[s].y0 } else { top[s] };
        let h = links[k].value * scale;
        links[k].source_y = (y, y + h);
        top[s] = y + h;
    }
    let mut by_target: Vec<usize> = (0..links.len()).collect();
    by_target.sort_by(|&a, &b| {
        (links[a].target, mid(links[a].source, &nodes))
            .partial_cmp(&(links[b].target, mid(links[b].source, &nodes)))
            .unwrap()
    });
    let mut top = vec![f64::NAN; n];
    for &k in &by_target {
        let t = links[k].target;
        let y = if top[t].is_nan() { nodes[t].y0 } else { top[t] };
        let h = links[k].value * scale;
        links[k].target_y = (y, y + h);
        top[t] = y + h;
    }
    nodes.retain(|_| true);
    Sankey { nodes, links, stages, dropped_cycles }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(v: &[(&str, &str, f64)]) -> Vec<(String, String, f64)> {
        v.iter().map(|(s, t, x)| (s.to_string(), t.to_string(), *x)).collect()
    }

    #[test]
    fn stages_follow_the_longest_path_and_links_are_summed() {
        let s = layout(&rows(&[("A", "B", 2.0), ("B", "C", 1.0), ("A", "C", 1.0), ("A", "B", 3.0)]));
        let stage: Vec<usize> = s.nodes.iter().map(|n| n.stage).collect();
        assert_eq!(stage, [0, 1, 2], "C is two steps from A through B");
        assert_eq!(s.links.len(), 3);
        assert_eq!(s.links[0].value, 5.0, "the two A to B rows are one link");
        assert_eq!(s.stages, 3);
    }

    #[test]
    fn bands_are_as_wide_at_both_ends_and_fill_their_nodes() {
        let s = layout(&rows(&[
            ("Coal", "Power", 4.0),
            ("Gas", "Power", 2.0),
            ("Power", "Homes", 3.0),
            ("Power", "Industry", 3.0),
        ]));
        for l in &s.links {
            let (a, b) = (l.source_y.1 - l.source_y.0, l.target_y.1 - l.target_y.0);
            assert!((a - b).abs() < 1e-12 && a > 0.0);
        }
        let power = s.nodes.iter().position(|n| n.name == "Power").unwrap();
        let n = &s.nodes[power];
        let inflow: f64 = s.links.iter().filter(|l| l.target == power).map(|l| l.target_y.1 - l.target_y.0).sum();
        assert!((inflow - (n.y1 - n.y0)).abs() < 1e-12, "the incoming bands fill the node");
        // Everything inside the area.
        for n in &s.nodes {
            assert!(n.x0 >= 0.0 && n.x1 <= 1.0 + 1e-12 && n.y0 >= -1e-12 && n.y1 <= 1.0 + 1e-12, "{n:?}");
        }
    }

    #[test]
    fn cycles_empty_names_and_bad_values_are_left_out() {
        let s =
            layout(&rows(&[("A", "B", 1.0), ("B", "A", 1.0), ("", "B", 1.0), ("A", "C", f64::NAN), ("A", "A", 2.0)]));
        assert_eq!((s.links.len(), s.dropped_cycles), (1, 1));
        assert!(layout(&[]).nodes.is_empty());
    }

    #[test]
    fn fixed_stages_override_the_longest_path() {
        // "b" has no incoming link, so by itself it would be in stage 0.
        let s = layout_in_stages(&rows(&[("a", "x", 1.0), ("b", "y", 1.0)]), &|n| (n == "b").then_some(1));
        let stage = |name: &str| s.nodes.iter().find(|n| n.name == name).unwrap().stage;
        assert_eq!((stage("a"), stage("b"), stage("y")), (0, 1, 1), "y keeps its longest path, 1");
        assert_eq!(s.stages, 2);
    }
}
