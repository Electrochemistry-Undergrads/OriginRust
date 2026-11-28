//! Network plot layout (T383, Origin 2021b's Network plot; Sheaf's own rules, not measured): rows of
//! (source, target, weight) become nodes and undirected edges placed by a force-directed layout.
//!
//! - Nodes are the distinct names of both columns, in order of first appearance. Edges join two different
//!   names; rows joining the same pair (in either direction) are one edge whose weight is their sum. Rows
//!   with an empty name or a weight that is missing or not above 0 are left out; a row naming the same node
//!   twice adds the node only.
//! - Layout (Fruchterman and Reingold 1991): the nodes start evenly on a circle in node order, so the result
//!   does not depend on chance; 300 steps with the ideal distance k = sqrt(1 / n), repulsion k^2 / d between
//!   every pair, attraction d^2 / k along each edge, the step limited by a temperature falling from 0.1 to 0.
//! - The positions are then fitted into 0..1 in both directions with a margin of 0.08, keeping the aspect.
//! - A node's degree is the sum of the weights of its edges.
//! - The directed arcs (T510) keep the direction: rows of the same source and target are one arc whose weight is
//!   their sum; the layout still uses the undirected edges.

use std::collections::HashMap;

/// The margin kept free around the nodes, as a fraction of the area.
pub const MARGIN: f64 = 0.08;
const STEPS: usize = 300;

/// A laid-out network.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Network {
    pub names: Vec<String>,
    /// Positions, x from 0 (left) to 1, y from 0 (top) to 1.
    pub positions: Vec<(f64, f64)>,
    /// The weighted degree of each node.
    pub degree: Vec<f64>,
    /// Edges: the two nodes (lower index first) and the weight.
    pub edges: Vec<(usize, usize, f64)>,
    /// Directed arcs (T510): source node, target node and the summed weight, in order of first appearance.
    pub arcs: Vec<(usize, usize, f64)>,
}

/// Lays out the network of `rows` (source, target, weight).
pub fn layout(rows: &[(String, String, f64)]) -> Network {
    let mut index: HashMap<String, usize> = HashMap::new();
    let mut names: Vec<String> = Vec::new();
    let mut id = |name: &str, names: &mut Vec<String>| -> usize {
        *index.entry(name.to_string()).or_insert_with(|| {
            names.push(name.to_string());
            names.len() - 1
        })
    };
    let mut edges: Vec<(usize, usize, f64)> = Vec::new();
    let mut arcs: Vec<(usize, usize, f64)> = Vec::new();
    for (s, t, w) in rows {
        let (s, t) = (s.trim(), t.trim());
        if s.is_empty() || t.is_empty() || !(w.is_finite() && *w > 0.0) {
            continue;
        }
        let (a, b) = (id(s, &mut names), id(t, &mut names));
        if a == b {
            continue;
        }
        let key = (a.min(b), a.max(b));
        match edges.iter_mut().find(|e| (e.0, e.1) == key) {
            Some(e) => e.2 += w,
            None => edges.push((key.0, key.1, *w)),
        }
        match arcs.iter_mut().find(|e| (e.0, e.1) == (a, b)) {
            Some(e) => e.2 += w,
            None => arcs.push((a, b, *w)),
        }
    }
    let n = names.len();
    if n == 0 {
        return Network::default();
    }
    let mut degree = vec![0.0; n];
    for &(a, b, w) in &edges {
        degree[a] += w;
        degree[b] += w;
    }
    let mut pos: Vec<(f64, f64)> = (0..n)
        .map(|i| {
            let a = 2.0 * std::f64::consts::PI * i as f64 / n as f64;
            (0.5 + 0.4 * a.cos(), 0.5 + 0.4 * a.sin())
        })
        .collect();
    if n > 1 {
        let k = (1.0 / n as f64).sqrt();
        for step in 0..STEPS {
            let temp = 0.1 * (1.0 - step as f64 / STEPS as f64);
            let mut disp = vec![(0.0, 0.0); n];
            for i in 0..n {
                for j in (i + 1)..n {
                    let (dx, dy) = (pos[i].0 - pos[j].0, pos[i].1 - pos[j].1);
                    let d = (dx * dx + dy * dy).sqrt().max(1e-6);
                    let f = k * k / d;
                    let (fx, fy) = (dx / d * f, dy / d * f);
                    disp[i].0 += fx;
                    disp[i].1 += fy;
                    disp[j].0 -= fx;
                    disp[j].1 -= fy;
                }
            }
            for &(a, b, _) in &edges {
                let (dx, dy) = (pos[a].0 - pos[b].0, pos[a].1 - pos[b].1);
                let d = (dx * dx + dy * dy).sqrt().max(1e-6);
                let f = d * d / k;
                let (fx, fy) = (dx / d * f, dy / d * f);
                disp[a].0 -= fx;
                disp[a].1 -= fy;
                disp[b].0 += fx;
                disp[b].1 += fy;
            }
            for i in 0..n {
                let len = (disp[i].0 * disp[i].0 + disp[i].1 * disp[i].1).sqrt().max(1e-12);
                let m = len.min(temp);
                pos[i].0 += disp[i].0 / len * m;
                pos[i].1 += disp[i].1 / len * m;
            }
        }
    }
    // Fit into the area with the margin, keeping the aspect.
    let (x0, x1) = pos.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), p| (a.min(p.0), b.max(p.0)));
    let (y0, y1) = pos.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), p| (a.min(p.1), b.max(p.1)));
    let span = (x1 - x0).max(y1 - y0);
    let room = 1.0 - 2.0 * MARGIN;
    let positions = if span > 1e-12 {
        let (cx, cy) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
        pos.iter().map(|p| (0.5 + (p.0 - cx) / span * room, 0.5 + (p.1 - cy) / span * room)).collect()
    } else {
        vec![(0.5, 0.5); n]
    };
    Network { names, positions, degree, edges, arcs }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(v: &[(&str, &str, f64)]) -> Vec<(String, String, f64)> {
        v.iter().map(|(s, t, x)| (s.to_string(), t.to_string(), *x)).collect()
    }

    #[test]
    fn edges_are_undirected_and_summed_and_bad_rows_left_out() {
        let n =
            layout(&rows(&[("a", "b", 1.0), ("b", "a", 2.0), ("a", "a", 1.0), ("", "c", 1.0), ("a", "c", f64::NAN)]));
        assert_eq!(n.names, ["a", "b"]);
        assert_eq!(n.edges, [(0, 1, 3.0)]);
        assert_eq!(n.degree, [3.0, 3.0]);
        assert_eq!(n.arcs, [(0, 1, 1.0), (1, 0, 2.0)], "T510: both directions kept");
    }

    #[test]
    fn the_layout_is_repeatable_inside_the_area_and_keeps_groups_together() {
        // Two triangles joined by one edge.
        let r = rows(&[
            ("a", "b", 1.0),
            ("b", "c", 1.0),
            ("c", "a", 1.0),
            ("d", "e", 1.0),
            ("e", "f", 1.0),
            ("f", "d", 1.0),
            ("c", "d", 1.0),
        ]);
        let (n1, n2) = (layout(&r), layout(&r));
        assert_eq!(n1, n2, "no randomness");
        for p in &n1.positions {
            assert!(
                (MARGIN - 1e-9..=1.0 - MARGIN + 1e-9).contains(&p.0)
                    && (MARGIN - 1e-9..=1.0 - MARGIN + 1e-9).contains(&p.1),
                "{p:?}"
            );
        }
        let d = |i: usize, j: usize| {
            let (a, b) = (n1.positions[i], n1.positions[j]);
            ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
        };
        let inside = (d(0, 1) + d(1, 2) + d(0, 2) + d(3, 4) + d(4, 5) + d(3, 5)) / 6.0;
        let across = (d(0, 4) + d(0, 5) + d(1, 4) + d(1, 5)) / 4.0;
        assert!(inside < across, "inside {inside} across {across}");
    }

    #[test]
    fn one_node_sits_in_the_middle_and_nothing_gives_an_empty_network() {
        assert!(layout(&[]).names.is_empty());
        let n = layout(&rows(&[("a", "a", 1.0)]));
        assert_eq!((n.names.len(), n.positions[0]), (1, (0.5, 0.5)));
    }
}
