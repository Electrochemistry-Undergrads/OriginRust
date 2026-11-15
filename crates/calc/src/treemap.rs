//! Treemap layouts (Origin 2025b "Treemap plot", graphing spec 14.26): pure functions that cut a rectangle into
//! one rectangle per item, the area of each proportional to the item's weight.
//!
//! Origin names six layouts in the Plot Details Layout tab (Squarified, Squarified+, Split, Pivot by Size,
//! Pivot by Middle, Pivot by Split Size) and describes them in a line each; it gives no algorithm. The
//! algorithms below are the published ones and Sheaf's reading of the line Origin gives. Nothing here is measured
//! against Origin:
//!
//! - **Squarified** (Bruls, Huizing and van Wijk, 2000): the items are sorted by size, largest first; rows are
//!   laid along the shorter side of the rectangle left over, an item joining the row while the worst aspect ratio of
//!   the row does not get worse. The largest item is at the top left. The order of the input is ignored.
//! - **Squarified+**: as Squarified, but for every row both directions are tried (the row along the shorter side
//!   and along the longer side of the rest) and the one whose row has the better (smaller) worst aspect ratio is
//!   kept; a tie keeps the shorter side. (Sheaf's reading of "per row, keep the direction with the better aspect
//!   ratio".) The order of the input is ignored.
//! - **Split**: the ordered list is cut into two parts, in order, whose total weights are as equal as the cut
//!   allows (the first cut position of the smallest difference); the rectangle is cut across its longer side
//!   in the ratio of the two weights and each part is cut again in the same way. Order is kept: the first part
//!   is the left (or top) one.
//! - **Pivot treemaps** (Shneiderman and Wattenberg, "Ordered Treemap Layouts", 2001; Bederson, Shneiderman and
//!   Wattenberg, 2002): a pivot item is chosen; the items before it form `L1`, the items after it are cut into
//!   `L2` (the next ones) and `L3` (the last ones). In a rectangle at least as wide as high, `L1` fills a column
//!   on the left, the pivot is at the top left of the rest with `L2` on its right, and `L3` fills the band along
//!   the bottom; in a taller rectangle the same with the axes exchanged (`L1` a band along the top, the pivot at
//!   the top left of the rest with `L2` below it, `L3` a column on the right). The cut between `L2` and `L3` is
//!   the one that brings the pivot's aspect ratio closest to 1 (the first one on a tie). The three lists are laid
//!   out again in their rectangles in the same way. The pivot is the largest item (Pivot by Size, the first on a
//!   tie), the item at index `n / 2` of the list (Pivot by Middle), or the item that leaves the least difference
//!   between the total weight before it and after it (Pivot by Split Size, the first on a tie).
//!
//! Weights that are not finite and positive get a zero-size rectangle at the origin of the area and take no part.

use serde::{Deserialize, Serialize};

/// A rectangle: `x`, `y` is the top left corner.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl Rect {
    pub fn new(x: f64, y: f64, w: f64, h: f64) -> Rect {
        Rect { x, y, w, h }
    }

    pub fn area(&self) -> f64 {
        self.w * self.h
    }

    /// The rectangle shrunk by `d` on every side (never below zero size, kept centred).
    pub fn inset(&self, d: f64) -> Rect {
        let dx = d.min(self.w / 2.0);
        let dy = d.min(self.h / 2.0);
        Rect { x: self.x + dx, y: self.y + dy, w: self.w - 2.0 * dx, h: self.h - 2.0 * dy }
    }

    fn transpose(&self) -> Rect {
        Rect { x: self.y, y: self.x, w: self.h, h: self.w }
    }
}

/// The layout algorithm of a treemap (Plot Details Layout tab).
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Layout {
    #[default]
    Squarified,
    SquarifiedPlus,
    Split,
    PivotSize,
    PivotMiddle,
    PivotSplitSize,
}

impl Layout {
    pub const ALL: [Layout; 6] = [
        Layout::Squarified,
        Layout::SquarifiedPlus,
        Layout::Split,
        Layout::PivotSize,
        Layout::PivotMiddle,
        Layout::PivotSplitSize,
    ];

    /// The name Origin's Layout list shows.
    pub fn label(&self) -> &'static str {
        match self {
            Layout::Squarified => "Squarified",
            Layout::SquarifiedPlus => "Squarified+",
            Layout::Split => "Split",
            Layout::PivotSize => "Pivot by Size",
            Layout::PivotMiddle => "Pivot by Middle",
            Layout::PivotSplitSize => "Pivot by Split Size",
        }
    }

    /// The short name used on the command line.
    pub fn name(&self) -> &'static str {
        match self {
            Layout::Squarified => "squarified",
            Layout::SquarifiedPlus => "squarified-plus",
            Layout::Split => "split",
            Layout::PivotSize => "pivot-size",
            Layout::PivotMiddle => "pivot-middle",
            Layout::PivotSplitSize => "pivot-split-size",
        }
    }

    pub fn from_name(s: &str) -> Option<Layout> {
        let s = s.trim().to_ascii_lowercase().replace('+', "-plus").replace([' ', '_'], "-");
        let s = s.trim_end_matches('-');
        match s {
            "squarified" => Some(Layout::Squarified),
            "squarified-plus" | "squarifiedplus" => Some(Layout::SquarifiedPlus),
            "split" => Some(Layout::Split),
            "pivot-size" | "pivot-by-size" => Some(Layout::PivotSize),
            "pivot-middle" | "pivot-by-middle" => Some(Layout::PivotMiddle),
            "pivot-split-size" | "pivot-by-split-size" => Some(Layout::PivotSplitSize),
            _ => None,
        }
    }

    /// True for the layouts that sort the items by size themselves and so ignore the order of the list.
    pub fn sorts_by_size(&self) -> bool {
        matches!(self, Layout::Squarified | Layout::SquarifiedPlus)
    }
}

/// Cuts `rect` into one rectangle per weight, in the order of `weights`; see the module documentation.
/// The rectangles tile `rect` and the area of each is `weight / total * rect.area()`.
pub fn layout(kind: Layout, weights: &[f64], rect: Rect) -> Vec<Rect> {
    let mut out = vec![Rect::new(rect.x, rect.y, 0.0, 0.0); weights.len()];
    let live: Vec<usize> = (0..weights.len()).filter(|&i| weights[i].is_finite() && weights[i] > 0.0).collect();
    let total: f64 = live.iter().map(|&i| weights[i]).sum();
    if live.is_empty() || !(rect.w > 0.0 && rect.h > 0.0) || !total.is_finite() {
        return out;
    }
    let scale = rect.area() / total;
    let items: Vec<(usize, f64)> = live.iter().map(|&i| (i, weights[i] * scale)).collect();
    match kind {
        Layout::Squarified | Layout::SquarifiedPlus => {
            let mut sorted = items;
            sorted.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
            squarify(&sorted, rect, kind == Layout::SquarifiedPlus, &mut out);
        }
        Layout::Split => split(&items, rect, &mut out),
        Layout::PivotSize | Layout::PivotMiddle | Layout::PivotSplitSize => pivot(kind, &items, rect, &mut out),
    }
    out
}

// ----- squarified ----------------------------------------------------------------------------

/// The worst aspect ratio of a row of areas with total `s` and extremes `lo` and `hi`, laid along a side of length `l`.
fn worst(s: f64, lo: f64, hi: f64, l: f64) -> f64 {
    let l2 = l * l;
    (l2 * hi / (s * s)).max(s * s / (l2 * lo))
}

/// How many of the leading items of `items` make the row along a side of length `l` (the greedy rule of Bruls et
/// al.), and its worst aspect ratio.
fn build_row(items: &[(usize, f64)], l: f64) -> (usize, f64) {
    let (mut s, mut lo, mut hi) = (items[0].1, items[0].1, items[0].1);
    let mut w = worst(s, lo, hi, l);
    let mut k = 1;
    while k < items.len() {
        let a = items[k].1;
        let nw = worst(s + a, lo.min(a), hi.max(a), l);
        if nw > w {
            break;
        }
        s += a;
        lo = lo.min(a);
        hi = hi.max(a);
        w = nw;
        k += 1;
    }
    (k, w)
}

fn squarify(items: &[(usize, f64)], rect: Rect, plus: bool, out: &mut [Rect]) {
    let mut r = rect;
    let mut i = 0;
    while i < items.len() {
        let rest = &items[i..];
        let short = r.w.min(r.h);
        let long = r.w.max(r.h);
        let (mut k, w_short) = build_row(rest, short);
        let mut along_short = true;
        if plus && long > short {
            let (k_long, w_long) = build_row(rest, long);
            if w_long < w_short - 1e-12 {
                k = k_long;
                along_short = false;
            }
        }
        // A column on the left when the row runs along the height, else a band along the top.
        let vertical = (r.w >= r.h) == along_short;
        let l = if vertical { r.h } else { r.w };
        let last = i + k == items.len();
        let s: f64 = rest[..k].iter().map(|it| it.1).sum();
        let t = if last { if vertical { r.w } else { r.h } } else { (s / l).min(if vertical { r.w } else { r.h }) };
        let mut at = if vertical { r.y } else { r.x };
        let end = if vertical { r.y + r.h } else { r.x + r.w };
        for (j, &(idx, a)) in rest[..k].iter().enumerate() {
            let len = if j + 1 == k { end - at } else { (a / t).min(end - at) };
            out[idx] = if vertical { Rect::new(r.x, at, t, len) } else { Rect::new(at, r.y, len, t) };
            at += len;
        }
        if vertical {
            r = Rect::new(r.x + t, r.y, r.w - t, r.h);
        } else {
            r = Rect::new(r.x, r.y + t, r.w, r.h - t);
        }
        i += k;
    }
}

// ----- split ---------------------------------------------------------------------------------

fn split(items: &[(usize, f64)], r: Rect, out: &mut [Rect]) {
    match items.len() {
        0 => {}
        1 => out[items[0].0] = r,
        n => {
            let total: f64 = items.iter().map(|it| it.1).sum();
            let (mut best, mut best_diff, mut left) = (1, f64::INFINITY, 0.0);
            for (k, it) in items.iter().enumerate().take(n - 1) {
                left += it.1;
                let diff = (total - 2.0 * left).abs();
                if diff < best_diff - 1e-12 * total {
                    best = k + 1;
                    best_diff = diff;
                }
            }
            let left: f64 = items[..best].iter().map(|it| it.1).sum();
            let (a, b) = if r.w >= r.h {
                let w = r.w * left / total;
                (Rect::new(r.x, r.y, w, r.h), Rect::new(r.x + w, r.y, r.w - w, r.h))
            } else {
                let h = r.h * left / total;
                (Rect::new(r.x, r.y, r.w, h), Rect::new(r.x, r.y + h, r.w, r.h - h))
            };
            split(&items[..best], a, out);
            split(&items[best..], b, out);
        }
    }
}

// ----- pivot ---------------------------------------------------------------------------------

/// The index of the pivot among `items`.
fn pick_pivot(kind: Layout, items: &[(usize, f64)]) -> usize {
    match kind {
        Layout::PivotSize => {
            let mut best = 0;
            for (i, it) in items.iter().enumerate() {
                if it.1 > items[best].1 {
                    best = i;
                }
            }
            best
        }
        Layout::PivotMiddle => items.len() / 2,
        _ => {
            let total: f64 = items.iter().map(|it| it.1).sum();
            let (mut best, mut best_diff, mut before) = (0, f64::INFINITY, 0.0);
            for (i, it) in items.iter().enumerate() {
                let after = total - before - it.1;
                let diff = (before - after).abs();
                if diff < best_diff - 1e-12 * total {
                    best = i;
                    best_diff = diff;
                }
                before += it.1;
            }
            best
        }
    }
}

/// The rectangles of one pivot step, in the orientation where the rectangle is at least as wide as high.
struct Step {
    pivot: usize,
    /// `L2` is `items[pivot + 1..pivot + 1 + n2]`.
    n2: usize,
    r1: Rect,
    rp: Rect,
    r2: Rect,
    r3: Rect,
}

fn pivot_step(kind: Layout, items: &[(usize, f64)], r: Rect) -> Step {
    let p = pick_pivot(kind, items);
    let sum1: f64 = items[..p].iter().map(|it| it.1).sum();
    let sp = items[p].1;
    let after = &items[p + 1..];
    let w1 = if p == 0 { 0.0 } else { r.w * sum1 / (sum1 + sp + after.iter().map(|it| it.1).sum::<f64>()) };
    let r1 = Rect::new(r.x, r.y, w1, r.h);
    let rest = Rect::new(r.x + w1, r.y, r.w - w1, r.h);
    let total_rest = sp + after.iter().map(|it| it.1).sum::<f64>();
    let place = |n2: usize| -> (Rect, Rect, Rect) {
        let s2: f64 = after[..n2].iter().map(|it| it.1).sum();
        let top_h = if n2 == after.len() { rest.h } else { rest.h * (sp + s2) / total_rest };
        let wp = if n2 == 0 { rest.w } else { rest.w * sp / (sp + s2) };
        let rp = Rect::new(rest.x, rest.y, wp, top_h);
        let r2 = Rect::new(rest.x + wp, rest.y, rest.w - wp, top_h);
        let r3 = Rect::new(rest.x, rest.y + top_h, rest.w, rest.h - top_h);
        (rp, r2, r3)
    };
    let aspect = |q: &Rect| if q.w > 0.0 && q.h > 0.0 { (q.w / q.h).max(q.h / q.w) } else { f64::INFINITY };
    let (mut n2, mut best) = (0, f64::INFINITY);
    for j in 0..=after.len() {
        let a = aspect(&place(j).0);
        if a < best - 1e-12 {
            best = a;
            n2 = j;
        }
    }
    let (rp, r2, r3) = place(n2);
    Step { pivot: p, n2, r1, rp, r2, r3 }
}

fn pivot(kind: Layout, items: &[(usize, f64)], r: Rect, out: &mut [Rect]) {
    match items.len() {
        0 => return,
        1 => {
            out[items[0].0] = r;
            return;
        }
        _ => {}
    }
    let tall = r.h > r.w;
    let mut step = pivot_step(kind, items, if tall { r.transpose() } else { r });
    if tall {
        for q in [&mut step.r1, &mut step.rp, &mut step.r2, &mut step.r3] {
            *q = q.transpose();
        }
    }
    let p = step.pivot;
    out[items[p].0] = step.rp;
    pivot(kind, &items[..p], step.r1, out);
    pivot(kind, &items[p + 1..p + 1 + step.n2], step.r2, out);
    pivot(kind, &items[p + 1 + step.n2..], step.r3, out);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A small deterministic generator (xorshift), so the random cases are the same every run.
    pub(crate) struct Rng(pub u64);
    impl Rng {
        pub fn next(&mut self) -> f64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            (self.0 >> 11) as f64 / (1u64 << 53) as f64
        }
    }

    fn overlap(a: &Rect, b: &Rect) -> f64 {
        let w = (a.x + a.w).min(b.x + b.w) - a.x.max(b.x);
        let h = (a.y + a.h).min(b.y + b.h) - a.y.max(b.y);
        if w > 0.0 && h > 0.0 { w * h } else { 0.0 }
    }

    fn check(kind: Layout, weights: &[f64], rect: Rect) -> Vec<Rect> {
        let rects = layout(kind, weights, rect);
        assert_eq!(rects.len(), weights.len());
        let total: f64 = weights.iter().sum();
        let mut sum = 0.0;
        for (i, (q, w)) in rects.iter().zip(weights).enumerate() {
            let want = w / total * rect.area();
            assert!((q.area() - want).abs() <= 1e-9 * want, "{kind:?} item {i}: {} vs {want}", q.area());
            let eps = 1e-9 * (rect.w + rect.h);
            assert!(q.x >= rect.x - eps && q.y >= rect.y - eps, "{kind:?} {q:?} outside {rect:?}");
            assert!(q.x + q.w <= rect.x + rect.w + eps && q.y + q.h <= rect.y + rect.h + eps, "{kind:?} {q:?} outside");
            sum += q.area();
        }
        assert!((sum - rect.area()).abs() <= 1e-9 * rect.area());
        for i in 0..rects.len() {
            for j in i + 1..rects.len() {
                assert!(overlap(&rects[i], &rects[j]) <= 1e-9 * rect.area(), "{kind:?}: {i} and {j} overlap");
            }
        }
        rects
    }

    #[test]
    fn every_layout_tiles_the_rectangle_with_areas_in_proportion() {
        let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
        for kind in Layout::ALL {
            for case in 0..60 {
                let n = 1 + (rng.next() * 24.0) as usize;
                let weights: Vec<f64> = (0..n).map(|_| 0.01 + rng.next().powi(3) * 100.0).collect();
                let rect = match case % 3 {
                    0 => Rect::new(0.0, 0.0, 400.0, 300.0),
                    1 => Rect::new(10.0, -5.0, 120.0, 500.0),
                    _ => Rect::new(3.0, 7.0, 640.0, 90.0),
                };
                check(kind, &weights, rect);
            }
        }
    }

    #[test]
    fn equal_weights_and_extreme_weights_still_tile() {
        for kind in Layout::ALL {
            check(kind, &[1.0; 9], Rect::new(0.0, 0.0, 90.0, 90.0));
            check(kind, &[1e4, 1.0, 1e-1, 5.0], Rect::new(0.0, 0.0, 200.0, 100.0));
            check(kind, &[3.0], Rect::new(1.0, 2.0, 30.0, 40.0));
        }
    }

    #[test]
    fn degenerate_input_gives_zero_size_rectangles() {
        for kind in Layout::ALL {
            assert!(layout(kind, &[], Rect::new(0.0, 0.0, 10.0, 10.0)).is_empty());
            let r = layout(kind, &[1.0, 2.0], Rect::new(0.0, 0.0, 0.0, 10.0));
            assert!(r.iter().all(|q| q.area() == 0.0));
            // Weights that are not positive take no part and the others share the rectangle.
            let r = layout(kind, &[2.0, 0.0, f64::NAN, 2.0, -1.0], Rect::new(0.0, 0.0, 10.0, 10.0));
            assert_eq!((r[1].area(), r[2].area(), r[4].area()), (0.0, 0.0, 0.0));
            assert!((r[0].area() - 50.0).abs() < 1e-9 && (r[3].area() - 50.0).abs() < 1e-9);
        }
    }

    #[test]
    fn squarified_puts_the_largest_at_the_top_left_and_ignores_the_order() {
        // The example of Bruls et al.: 6, 6, 4, 3, 2, 2, 1 in a 6 by 4 rectangle.
        let w = [6.0, 6.0, 4.0, 3.0, 2.0, 2.0, 1.0];
        let r = check(Layout::Squarified, &w, Rect::new(0.0, 0.0, 6.0, 4.0));
        assert_eq!((r[0].x, r[0].y), (0.0, 0.0));
        // The first row is a column of the two 6s, 3 wide... 6 and 6 in a column of width 3 and height 4.
        assert!((r[0].w - 3.0).abs() < 1e-9 && (r[0].h - 2.0).abs() < 1e-9, "{:?}", r[0]);
        assert!((r[1].y - 2.0).abs() < 1e-9 && r[1].x == 0.0);
        let distinct = [6.5, 6.0, 4.0, 3.0, 2.5, 2.0, 1.0];
        let a = check(Layout::Squarified, &distinct, Rect::new(0.0, 0.0, 6.0, 4.0));
        let mut reversed = distinct;
        reversed.reverse();
        let mut b = check(Layout::Squarified, &reversed, Rect::new(0.0, 0.0, 6.0, 4.0));
        b.reverse();
        for (a, b) in a.iter().zip(&b) {
            assert!((a.x - b.x).abs() < 1e-9 && (a.w - b.w).abs() < 1e-9 && (a.y - b.y).abs() < 1e-9);
        }
    }

    fn worst_aspect(r: &[Rect]) -> f64 {
        r.iter().map(|q| (q.w / q.h).max(q.h / q.w)).fold(0.0, f64::max)
    }

    #[test]
    fn squarified_plus_is_never_worse_than_squarified_for_the_first_row() {
        // On a wide strip the alternate direction is chosen for some rows, so the mean aspect ratio improves or equals.
        let mut rng = Rng(42);
        let mut better = 0;
        for _ in 0..80 {
            let n = 3 + (rng.next() * 12.0) as usize;
            let w: Vec<f64> = (0..n).map(|_| 0.1 + rng.next() * 10.0).collect();
            let rect = Rect::new(0.0, 0.0, 500.0, 100.0);
            let a = check(Layout::Squarified, &w, rect);
            let b = check(Layout::SquarifiedPlus, &w, rect);
            if worst_aspect(&b) < worst_aspect(&a) - 1e-9 {
                better += 1;
            }
        }
        assert!(better > 0, "Squarified+ never changed a layout");
    }

    #[test]
    fn split_keeps_the_order_and_halves_the_weight() {
        let r = check(Layout::Split, &[1.0, 1.0, 1.0, 1.0], Rect::new(0.0, 0.0, 20.0, 10.0));
        // Cut across the longer (horizontal) side: items 0 and 1 on the left half, in order.
        assert!((r[0].x - 0.0).abs() < 1e-9 && (r[1].x - 5.0).abs() < 1e-9 && (r[2].x - 10.0).abs() < 1e-9);
        assert!(r[0].y == 0.0 && r[1].y == 0.0 && r[2].y == 0.0 && r[3].y == 0.0 && (r[3].x - 15.0).abs() < 1e-9);
        // Order: reading the rectangles by (x, y) never goes backwards across the first cut.
        let mut rng = Rng(7);
        for _ in 0..30 {
            let n = 2 + (rng.next() * 15.0) as usize;
            let w: Vec<f64> = (0..n).map(|_| 0.5 + rng.next() * 5.0).collect();
            let rect = Rect::new(0.0, 0.0, 300.0, 100.0);
            let r = check(Layout::Split, &w, rect);
            let total: f64 = w.iter().sum();
            let (mut best, mut diff, mut left) = (1, f64::INFINITY, 0.0);
            for (k, wk) in w.iter().enumerate().take(n - 1) {
                left += wk;
                if (total - 2.0 * left).abs() < diff - 1e-12 * total {
                    best = k + 1;
                    diff = (total - 2.0 * left).abs();
                }
            }
            let cut = r[best - 1].x + r[best - 1].w;
            assert!(r[..best].iter().all(|q| q.x + q.w <= cut + 1e-9));
            assert!(r[best..].iter().all(|q| q.x >= cut - 1e-9));
        }
    }

    #[test]
    fn pivot_by_size_places_the_pivot_and_the_lists_around_it() {
        // Wide rectangle, pivot = the largest (index 2). L1 = items 0 and 1 in the left column, in order.
        let w = [1.0, 1.0, 6.0, 1.0, 1.0];
        let r = check(Layout::PivotSize, &w, Rect::new(0.0, 0.0, 100.0, 60.0));
        let left = r[0].x + r[0].w;
        assert!(r[0].x == 0.0 && r[1].x == 0.0 && (r[0].w - r[1].w).abs() < 1e-9);
        assert!(r[0].y < r[1].y, "L1 keeps its order down the column");
        assert!((left - 100.0 * 2.0 / 10.0).abs() < 1e-9, "L1 is 2/10 of the width");
        // The pivot starts right of L1 at the top; nothing of L2 or L3 is left of it.
        assert!((r[2].x - left).abs() < 1e-9 && r[2].y == 0.0);
        assert!(r[3].x >= left - 1e-9 && r[4].x >= left - 1e-9);
        // The pivot is the largest, so it is the largest rectangle too.
        assert!(r.iter().all(|q| q.area() <= r[2].area() + 1e-9));
    }

    #[test]
    fn pivot_by_middle_and_split_size_pick_their_pivots() {
        let w = [1.0, 2.0, 3.0, 4.0, 5.0];
        let items: Vec<(usize, f64)> = w.iter().copied().enumerate().collect();
        assert_eq!(pick_pivot(Layout::PivotMiddle, &items), 2);
        assert_eq!(pick_pivot(Layout::PivotSize, &items), 4);
        // Before 1+2+3 = 6 and after 4 differ by 2 at index 3 (before 6, after 5 -> 1); at index 2: before 3, after 9.
        assert_eq!(pick_pivot(Layout::PivotSplitSize, &items), 3);
        let r = check(Layout::PivotMiddle, &w, Rect::new(0.0, 0.0, 150.0, 100.0));
        // L1 = items 0, 1 in the left column.
        assert!(r[0].x == 0.0 && r[1].x == 0.0);
        let left = r[0].x + r[0].w;
        assert!((left - 150.0 * 3.0 / 15.0).abs() < 1e-9);
        assert!((r[2].x - left).abs() < 1e-9 && r[2].y == 0.0);
    }

    #[test]
    fn pivot_in_a_tall_rectangle_exchanges_the_axes() {
        let w = [1.0, 1.0, 6.0, 1.0, 1.0];
        let r = check(Layout::PivotSize, &w, Rect::new(0.0, 0.0, 60.0, 100.0));
        // L1 is a band along the top, in order left to right.
        assert!(r[0].y == 0.0 && r[1].y == 0.0 && r[0].x < r[1].x);
        assert!((r[0].h - 100.0 * 2.0 / 10.0).abs() < 1e-9);
        assert!((r[2].y - r[0].h).abs() < 1e-9 && r[2].x == 0.0);
    }

    #[test]
    fn layout_names_round_trip() {
        for l in Layout::ALL {
            assert_eq!(Layout::from_name(l.name()), Some(l));
            assert_eq!(Layout::from_name(l.label()), Some(l), "{}", l.label());
        }
        assert_eq!(Layout::from_name("nope"), None);
    }
}
