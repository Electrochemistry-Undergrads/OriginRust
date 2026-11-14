//! Level crossing as Origin's `levelcrossing` X-Function (measured; analysis-tools.md 14): the X
//! values where a curve reaches a level, with the direction and the row of each.

/// Which row [`level_crossings`] gives for a crossing between two rows (Origin's `type`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CrossingIndex {
    /// The row whose X is nearest the crossing, the smaller X on a tie (Origin's "Any").
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
/// the crossing is interpolated linearly, with the direction of that step. A point with a missing
/// value is skipped and its neighbours are joined, as in Origin; for [`CrossingIndex::Nearest`] the
/// rows skipped that way still count (Origin gives the row whose X is nearest, the smaller X on a
/// tie). With `min_gap` above 1, the crossings are taken in pairs in order (first and second, third
/// and fourth, ...) and both of a pair are dropped when their rows are closer than `min_gap`
/// (Origin's `pts`; how Origin pairs them is only partly measured).
pub fn level_crossings(x: &[f64], y: &[f64], level: f64, index: CrossingIndex, min_gap: usize) -> Vec<Crossing> {
    let n = x.len().min(y.len());
    let valid: Vec<usize> = (0..n).filter(|&i| x[i].is_finite() && y[i].is_finite()).collect();
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
    for (k, &i) in valid.iter().enumerate() {
        let next = valid.get(k + 1).copied();
        if y[i] == level {
            let back = k.checked_sub(1).map(|p| sign(y[i] - y[valid[p]]));
            let fwd = next.map(|j| sign(y[j] - y[i]));
            let direction = match (back, fwd) {
                (Some(b), Some(f)) if b == f => b,
                (Some(_), Some(_)) | (None, None) => 0,
                (Some(d), None) | (None, Some(d)) => d,
            };
            out.push(Crossing { x: x[i], direction, row: i + 1 });
        }
        let Some(j) = next else { continue };
        if (y[i] - level) * (y[j] - level) < 0.0 {
            let t = (level - y[i]) / (y[j] - y[i]);
            let cx = x[i] + t * (x[j] - x[i]);
            let row = match index {
                CrossingIndex::Left => i,
                CrossingIndex::Right => j,
                CrossingIndex::Nearest => (i..=j)
                    .filter(|&r| x[r].is_finite())
                    .min_by(|&a, &b| (x[a] - cx).abs().total_cmp(&(x[b] - cx).abs()).then(x[a].total_cmp(&x[b])))
                    .unwrap_or(i),
            };
            out.push(Crossing { x: cx, direction: sign(y[j] - y[i]), row: row + 1 });
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

    fn close(got: &[Crossing], want: &[(f64, i8, usize)]) {
        assert_eq!(got.len(), want.len(), "{:?}", rows(got));
        for (g, w) in rows(got).iter().zip(want) {
            assert!((g.0 - w.0).abs() < 1e-12 && g.1 == w.1 && g.2 == w.2, "{:?} != {want:?}", rows(got));
        }
    }

    /// Origin's numbers (re-workspace/scratch/lcross/lc2.out.txt, lc3.out.txt).
    #[test]
    fn more_cases_match_origin() {
        let x: Vec<f64> = (0..11).map(f64::from).collect();
        let y = [0.0, 2.0, 4.0, 1.0, -1.0, -2.0, 1.0, 1.0, 3.0, 0.5, 2.0];
        let any = level_crossings(&x, &y, 1.5, CrossingIndex::Nearest, 1);
        close(
            &any,
            &[
                (0.75, 1, 2),
                (2.833_333_333_333_333_5, -1, 4),
                (7.25, 1, 8),
                (8.6, -1, 10),
                (9.666_666_666_666_666, 1, 11),
            ],
        );
        let rows_of = |i| level_crossings(&x, &y, 1.5, i, 1).iter().map(|c| c.row).collect::<Vec<_>>();
        assert_eq!(rows_of(CrossingIndex::Left), [1, 3, 8, 9, 10]);
        assert_eq!(rows_of(CrossingIndex::Right), [2, 4, 9, 10, 11]);
        let one = level_crossings(&x, &y, 1.0, CrossingIndex::Nearest, 1);
        close(
            &one,
            &[(0.5, 1, 1), (3.0, -1, 4), (6.0, 0, 7), (7.0, 0, 8), (8.8, -1, 10), (9.333_333_333_333_334, 1, 10)],
        );
        // The last point on the level takes the direction of its one step.
        close(
            &level_crossings(&x[..4], &[0.0, 2.0, 0.0, 1.0], 1.0, CrossingIndex::Nearest, 1),
            &[(0.5, 1, 1), (1.5, -1, 2), (3.0, 1, 4)],
        );
        // A flat run that comes back to the same side.
        close(
            &level_crossings(&x[..6], &[0.0, 1.0, 1.0, 1.0, 0.0, 2.0], 1.0, CrossingIndex::Nearest, 1),
            &[(1.0, 0, 2), (2.0, 0, 3), (3.0, 0, 4), (4.5, 1, 5)],
        );
        // A missing Y is bridged; its row still counts for the nearest X (the crossing at 2 is row 3).
        close(
            &level_crossings(&x[..5], &[0.0, 2.0, f64::NAN, 0.0, 2.0], 1.0, CrossingIndex::Nearest, 1),
            &[(0.5, 1, 1), (2.0, -1, 3), (3.5, 1, 4)],
        );
        // Falling X: the direction follows the rows and a tie goes to the smaller X.
        close(
            &level_crossings(&[10.0, 8.0, 7.0, 3.0, 0.0], &[0.0, 2.0, 0.0, 2.0, 0.0], 1.0, CrossingIndex::Nearest, 1),
            &[(9.0, 1, 2), (7.5, -1, 3), (5.0, 1, 4), (1.5, -1, 5)],
        );
    }
}
