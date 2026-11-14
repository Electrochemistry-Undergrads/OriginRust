//! Level crossing as Origin's `levelcrossing` X-Function (measured; analysis-tools.md 14): the X
//! values where a curve reaches a level, with the direction and the row of each.

/// Which row [`level_crossings`] gives for a crossing between two rows (Origin's `type`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CrossingIndex {
    /// The nearer of the two rows, the left one on a tie (Origin's "Any").
    #[default]
    Nearest,
    Left,
    Right,
}

/// One crossing of the level.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Crossing {
    pub x: f64,
    /// 1 upward, -1 downward, 0 where the curve only touches the level or stays on it.
    pub direction: i8,
    /// 1-based row of the data point (see [`CrossingIndex`]).
    pub row: usize,
}

/// The crossings of `level` by the curve `x`, `y`, in row order.
///
/// A point exactly on the level is a crossing at its own X; its direction is the sign of the steps
/// before and after it when they agree, else 0 (a touch, or a run of points on the level), and the
/// one step there is at the first and last point. Between two points on opposite sides of the level
/// the crossing is interpolated linearly, with the direction of that step. Pairs with a missing
/// value are skipped. With `min_gap` above 1, the crossings are taken in pairs in order (first and
/// second, third and fourth, ...) and both of a pair are dropped when their rows are closer than
/// `min_gap` (Origin's `pts`; how Origin pairs them is only partly measured).
pub fn level_crossings(x: &[f64], y: &[f64], level: f64, index: CrossingIndex, min_gap: usize) -> Vec<Crossing> {
    let n = x.len().min(y.len());
    let ok = |i: usize| x[i].is_finite() && y[i].is_finite();
    let sign = |v: f64| {
        if v > 0.0 {
            1i8
        } else if v < 0.0 {
            -1
        } else {
            0
        }
    };
    let mut out = Vec::new();
    for i in 0..n {
        if ok(i) && y[i] == level {
            let back = (i > 0 && ok(i - 1)).then(|| sign(y[i] - y[i - 1]));
            let fwd = (i + 1 < n && ok(i + 1)).then(|| sign(y[i + 1] - y[i]));
            let direction = match (back, fwd) {
                (Some(b), Some(f)) if b == f => b,
                (Some(_), Some(_)) | (None, None) => 0,
                (Some(d), None) | (None, Some(d)) => d,
            };
            out.push(Crossing { x: x[i], direction, row: i + 1 });
        }
        if i + 1 < n && ok(i) && ok(i + 1) && (y[i] - level) * (y[i + 1] - level) < 0.0 {
            let t = (level - y[i]) / (y[i + 1] - y[i]);
            let cx = x[i] + t * (x[i + 1] - x[i]);
            let row = match index {
                CrossingIndex::Left => i + 1,
                CrossingIndex::Right => i + 2,
                CrossingIndex::Nearest => {
                    if (cx - x[i]).abs() <= (x[i + 1] - cx).abs() {
                        i + 1
                    } else {
                        i + 2
                    }
                }
            };
            out.push(Crossing { x: cx, direction: sign(y[i + 1] - y[i]), row });
        }
    }
    if min_gap > 1 {
        let mut kept = Vec::with_capacity(out.len());
        for pair in out.chunks(2) {
            if pair.len() == 2 && pair[1].row.abs_diff(pair[0].row) < min_gap {
                continue;
            }
            kept.extend_from_slice(pair);
        }
        out = kept;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(c: &[Crossing]) -> Vec<(f64, i8, usize)> {
        c.iter().map(|c| (c.x, c.direction, c.row)).collect()
    }

    #[test]
    fn level_crossings_match_origin() {
        let x: Vec<f64> = (0..10).map(f64::from).collect();
        let y = [-1.0, 1.0, 2.0, 0.0, -2.0, -1.0, 0.0, 0.0, 3.0, -3.0];
        let any = level_crossings(&x, &y, 0.0, CrossingIndex::Nearest, 1);
        assert_eq!(rows(&any), [(0.5, 1, 1), (3.0, -1, 4), (6.0, 0, 7), (7.0, 0, 8), (8.5, -1, 9)]);
        let right = level_crossings(&x, &y, 0.0, CrossingIndex::Right, 1);
        assert_eq!(right.iter().map(|c| c.row).collect::<Vec<_>>(), [2, 4, 7, 8, 10]);
        let half = level_crossings(&x, &y, 0.5, CrossingIndex::Nearest, 1);
        let want = [(0.75, 1, 2), (2.75, -1, 4), (7.0 + 0.5 / 3.0, 1, 8), (8.0 + 2.5 / 6.0, -1, 9)];
        for (g, w) in rows(&half).iter().zip(want) {
            assert!((g.0 - w.0).abs() < 1e-12 && g.1 == w.1 && g.2 == w.2, "{g:?} vs {w:?}");
        }
        assert!(level_crossings(&x, &y, 0.5, CrossingIndex::Nearest, 3).is_empty());
        // A touch at x = 1 has direction 0; the first point counts with its one step.
        let x2 = [0.0, 0.5, 1.0, 1.5, 2.0, 2.5, 3.0];
        let y2 = [0.0, 1.0, 0.0, 1.0, 0.0, -1.0, 1.0];
        let c = level_crossings(&x2, &y2, 0.0, CrossingIndex::Nearest, 1);
        assert_eq!(rows(&c), [(0.0, 1, 1), (1.0, 0, 3), (2.0, -1, 5), (2.75, 1, 6)]);
        assert_eq!(rows(&level_crossings(&x2, &y2, 0.5, CrossingIndex::Nearest, 2)), [(2.875, 1, 7)]);
        assert!(level_crossings(&x2, &y2, 5.0, CrossingIndex::Nearest, 1).is_empty());
    }
}
