//! Chord diagram layout (T402, Origin 2021's Chord Diagram; Sheaf's own rules, not measured): rows of
//! (source, target, weight) become arcs round a circle, one per name, and ribbons between them, one per
//! direction of flow.
//!
//! - Nodes are the distinct names of both columns, in order of first appearance. Rows with an empty name or a
//!   weight that is missing or not above 0 are left out. Rows of the same source and target are one flow whose
//!   value is their sum; A to B and B to A are two flows. A row from a name to itself is a flow that starts and
//!   ends on its own arc.
//! - A node's total is the sum of the flows leaving it and the flows reaching it (a flow to itself counts
//!   twice). The arcs share the circle in proportion to their totals, clockwise from the top in node order,
//!   with a gap of [`GAP_DEG`] degrees between neighbours (less when there are so many nodes that the gaps
//!   would take more than half the circle).
//! - Within its arc a node gives its outgoing flows first, in target order, then its incoming flows, in
//!   source order; each flow takes a part of the arc in proportion to its value.
//! - Angles are in radians, 0 at the top, growing clockwise.

use std::collections::HashMap;
use std::f64::consts::{PI, TAU};

/// The gap between neighbouring arcs, in degrees.
pub const GAP_DEG: f64 = 2.0;

/// One flow of a chord diagram: its two nodes, its value and the parts of their arcs it takes.
#[derive(Debug, Clone, PartialEq)]
pub struct Ribbon {
    pub source: usize,
    pub target: usize,
    pub value: f64,
    /// Start and end angle on the source's arc.
    pub from: (f64, f64),
    /// Start and end angle on the target's arc.
    pub to: (f64, f64),
}

/// A laid-out chord diagram.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Chord {
    pub names: Vec<String>,
    /// Each node's total of outgoing and incoming flows.
    pub totals: Vec<f64>,
    /// Each node's arc: start and end angle.
    pub arcs: Vec<(f64, f64)>,
    pub ribbons: Vec<Ribbon>,
}

/// Lays out the chord diagram of `rows` (source, target, weight).
pub fn layout(rows: &[(String, String, f64)]) -> Chord {
    let mut index: HashMap<String, usize> = HashMap::new();
    let mut names: Vec<String> = Vec::new();
    let mut id = |name: &str, names: &mut Vec<String>| -> usize {
        *index.entry(name.to_string()).or_insert_with(|| {
            names.push(name.to_string());
            names.len() - 1
        })
    };
    let mut flows: Vec<(usize, usize, f64)> = Vec::new();
    for (s, t, w) in rows {
        let (s, t) = (s.trim(), t.trim());
        if s.is_empty() || t.is_empty() || !(w.is_finite() && *w > 0.0) {
            continue;
        }
        let (a, b) = (id(s, &mut names), id(t, &mut names));
        match flows.iter_mut().find(|f| (f.0, f.1) == (a, b)) {
            Some(f) => f.2 += w,
            None => flows.push((a, b, *w)),
        }
    }
    let n = names.len();
    if n == 0 {
        return Chord::default();
    }
    let mut totals = vec![0.0; n];
    for &(a, b, w) in &flows {
        totals[a] += w;
        totals[b] += w;
    }
    let sum: f64 = totals.iter().sum();
    let gap = GAP_DEG.to_radians().min(PI / n as f64);
    let scale = (TAU - gap * n as f64) / sum;
    let mut arcs = Vec::with_capacity(n);
    let mut cursor = gap / 2.0;
    for t in &totals {
        arcs.push((cursor, cursor + t * scale));
        cursor += t * scale + gap;
    }
    // Each node's parts: outgoing flows by target, then incoming flows by source.
    let mut from = vec![(0.0, 0.0); flows.len()];
    let mut to = vec![(0.0, 0.0); flows.len()];
    for (node, arc) in arcs.iter().enumerate() {
        let mut at = arc.0;
        let mut out: Vec<usize> = (0..flows.len()).filter(|&k| flows[k].0 == node).collect();
        out.sort_by_key(|&k| flows[k].1);
        for k in out {
            let span = flows[k].2 * scale;
            from[k] = (at, at + span);
            at += span;
        }
        let mut inc: Vec<usize> = (0..flows.len()).filter(|&k| flows[k].1 == node).collect();
        inc.sort_by_key(|&k| flows[k].0);
        for k in inc {
            let span = flows[k].2 * scale;
            to[k] = (at, at + span);
            at += span;
        }
    }
    let ribbons = flows
        .iter()
        .enumerate()
        .map(|(k, &(source, target, value))| Ribbon { source, target, value, from: from[k], to: to[k] })
        .collect();
    Chord { names, totals, arcs, ribbons }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(v: &[(&str, &str, f64)]) -> Vec<(String, String, f64)> {
        v.iter().map(|(a, b, w)| (a.to_string(), b.to_string(), *w)).collect()
    }

    #[test]
    fn arcs_share_the_circle_by_total_with_gaps() {
        let c = layout(&rows(&[("A", "B", 3.0), ("B", "A", 1.0), ("A", "C", 2.0), ("A", "B", 1.0)]));
        assert_eq!(c.names, ["A", "B", "C"]);
        // A: out 4 + 2, in 1 = 7; B: out 1, in 4 = 5; C: in 2.
        assert_eq!(c.totals, [7.0, 5.0, 2.0]);
        assert_eq!(c.ribbons.len(), 3, "A to B twice is one flow; B to A is another");
        let gap = GAP_DEG.to_radians();
        assert!((c.arcs[0].0 - gap / 2.0).abs() < 1e-12, "the first arc starts after half a gap from the top");
        let spans: Vec<f64> = c.arcs.iter().map(|a| a.1 - a.0).collect();
        assert!((spans[0] / spans[1] - 7.0 / 5.0).abs() < 1e-12);
        let used: f64 = spans.iter().sum::<f64>() + 3.0 * gap;
        assert!((used - TAU).abs() < 1e-12, "{used}");
        for w in c.arcs.windows(2) {
            assert!((w[1].0 - w[0].1 - gap).abs() < 1e-12);
        }
    }

    #[test]
    fn a_node_gives_outgoing_flows_first_then_incoming() {
        let c = layout(&rows(&[("A", "B", 3.0), ("B", "A", 1.0), ("A", "C", 2.0)]));
        let ab = &c.ribbons[0];
        let ac = &c.ribbons[2];
        let ba = &c.ribbons[1];
        // On A's arc: to B, to C, then from B.
        assert!((ab.from.0 - c.arcs[0].0).abs() < 1e-12);
        assert!((ac.from.0 - ab.from.1).abs() < 1e-12);
        assert!((ba.to.0 - ac.from.1).abs() < 1e-12);
        assert!((ba.to.1 - c.arcs[0].1).abs() < 1e-12, "the parts fill the arc");
        // On B's arc: to A first, then from A.
        assert!((ba.from.0 - c.arcs[1].0).abs() < 1e-12);
        assert!((ab.to.0 - ba.from.1).abs() < 1e-12);
        let scale = (c.arcs[0].1 - c.arcs[0].0) / c.totals[0];
        assert!(((ab.from.1 - ab.from.0) - 3.0 * scale).abs() < 1e-12);
    }

    #[test]
    fn bad_rows_are_left_out_and_a_flow_to_itself_counts_twice() {
        let c = layout(&rows(&[
            ("A", "A", 2.0),
            ("", "B", 1.0),
            ("A", "B", f64::NAN),
            ("A", "B", -1.0),
            (" B ", "A", 1.0),
        ]));
        assert_eq!(c.names, ["A", "B"]);
        assert_eq!(c.totals, [5.0, 1.0]);
        assert!(layout(&[]).names.is_empty());
        // Very many nodes: the gaps take at most half the circle.
        let many: Vec<(String, String, f64)> =
            (0..400).map(|i| (format!("n{i}"), format!("n{}", i + 1), 1.0)).collect();
        let c = layout(&many);
        let spans: f64 = c.arcs.iter().map(|a| a.1 - a.0).sum();
        assert!(spans >= PI - 1e-9, "{spans}");
    }
}
