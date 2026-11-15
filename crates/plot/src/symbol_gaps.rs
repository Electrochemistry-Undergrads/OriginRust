//! Removing symbol-sized holes from connecting lines without painting over grids or fills.

use crate::scene::{line_to, move_to};
use std::collections::{HashMap, HashSet};

type Point = (f64, f64);

pub(crate) struct Gaps {
    centres: Vec<Point>,
    cells: HashMap<(i64, i64), Vec<usize>>,
    radius: f64,
    cell_size: f64,
}

impl Gaps {
    pub fn new(points: impl IntoIterator<Item = Point>, radius: f64) -> Option<Self> {
        let cell_size = (2.0 * radius).max(0.001);
        if !(radius.is_finite() && radius > 0.0 && cell_size.is_finite()) {
            return None;
        }
        let mut gaps = Self { centres: Vec::new(), cells: HashMap::new(), radius, cell_size };
        let mut seen = HashSet::new();
        for p in points {
            if !(p.0.is_finite() && p.1.is_finite()) {
                continue;
            }
            // The scene writes path coordinates to 0.001 pt. Coincident centres at that
            // precision need just one hole, including a radar's repeated closing point.
            let key = ((p.0 * 1000.0).round().to_bits(), (p.1 * 1000.0).round().to_bits());
            if !seen.insert(key) {
                continue;
            }
            let index = gaps.centres.len();
            gaps.centres.push(p);
            gaps.cells.entry(gaps.cell(p)).or_default().push(index);
        }
        (!gaps.centres.is_empty()).then_some(gaps)
    }

    fn cell(&self, p: Point) -> (i64, i64) {
        ((p.0 / self.cell_size).floor() as i64, (p.1 / self.cell_size).floor() as i64)
    }

    /// Visit nearby holes, stopping when no visible part of the segment remains. Long
    /// segments use the finite centre list instead of walking an enormous empty cell range.
    fn candidates(&self, a: Point, b: Point, mut visit: impl FnMut(Point) -> bool) {
        let lo = self.cell((a.0.min(b.0) - self.radius, a.1.min(b.1) - self.radius));
        let hi = self.cell((a.0.max(b.0) + self.radius, a.1.max(b.1) + self.radius));
        let cell_count = (hi.0 as f64 - lo.0 as f64 + 1.0) * (hi.1 as f64 - lo.1 as f64 + 1.0);
        if cell_count > self.centres.len() as f64 {
            for &centre in &self.centres {
                if !visit(centre) {
                    return;
                }
            }
        } else {
            for x in lo.0..=hi.0 {
                for y in lo.1..=hi.1 {
                    if let Some(indices) = self.cells.get(&(x, y)) {
                        for &i in indices {
                            if !visit(self.centres[i]) {
                                return;
                            }
                        }
                    }
                }
            }
        }
    }

    /// Appends the visible parts of one continuous polyline. Adjacent uncut segments stay
    /// in one subpath, preserving dash phase and joins through points without a gap.
    pub fn write_polyline(&self, points: &[Point], out: &mut String) {
        let mut intervals = Vec::with_capacity(4);
        let mut next = Vec::with_capacity(4);
        let mut previous: Option<Point> = None;
        for pair in points.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            let (dx, dy) = (b.0 - a.0, b.1 - a.1);
            let length = dx.hypot(dy);
            if !(length.is_finite() && length > 0.0) {
                continue;
            }
            let unit = (dx / length, dy / length);
            intervals.clear();
            intervals.push((0.0_f64, 1.0_f64));
            self.candidates(a, b, |centre| {
                let relative = (centre.0 - a.0, centre.1 - a.1);
                if !(relative.0.is_finite() && relative.1.is_finite()) {
                    return true;
                }
                let perpendicular = (relative.0 * unit.1 - relative.1 * unit.0).abs();
                let along = relative.0 * unit.0 + relative.1 * unit.1;
                if !perpendicular.is_finite() || !along.is_finite() || perpendicular >= self.radius {
                    return true;
                }
                let ratio = perpendicular / self.radius;
                let half = self.radius * ((1.0 - ratio) * (1.0 + ratio)).sqrt();
                let (cut_lo, cut_hi) = (((along - half) / length).max(0.0), ((along + half) / length).min(1.0));
                if cut_lo >= cut_hi {
                    return true;
                }
                next.clear();
                for &(lo, hi) in &intervals {
                    if cut_hi <= lo || cut_lo >= hi {
                        next.push((lo, hi));
                    } else {
                        if cut_lo > lo {
                            next.push((lo, cut_lo));
                        }
                        if cut_hi < hi {
                            next.push((cut_hi, hi));
                        }
                    }
                }
                std::mem::swap(&mut intervals, &mut next);
                !intervals.is_empty()
            });
            if intervals.is_empty() {
                previous = None;
            }
            for &(lo, hi) in &intervals {
                let start = (a.0 + dx * lo, a.1 + dy * lo);
                let end = (a.0 + dx * hi, a.1 + dy * hi);
                if !previous.is_some_and(|p| (p.0 - start.0).abs() < 1e-9 && (p.1 - start.1).abs() < 1e-9) {
                    move_to(out, start.0, start.1);
                }
                line_to(out, end.0, end.1);
                previous = (hi == 1.0).then_some(end);
            }
        }
    }
}
