//! Lines across axis breaks (graphing spec 3.6). Origin 2016 draws the segment that joins a point
//! before a break to one after it, but leaves it out where it passes the band of the break's gap
//! (the strip of the frame between the two cut ends of the axis), unless the page's "Connect line
//! across Axis Break" is ticked (it is not by default).

use crate::LayerGeometry;
use crate::scene::{line_to, move_to};

/// The bands of the breaks of both axes in page points: `(x0, x1)` strips across the frame for X
/// breaks, `(y0, y1)` strips for Y breaks.
fn bands(geom: &LayerGeometry) -> (Vec<(f64, f64)>, Vec<(f64, f64)>) {
    let f = geom.frame;
    let xs = geom
        .x
        .gaps
        .iter()
        .map(|g| {
            let (lo, hi) = g.span(geom.x.reversed);
            (f.x + lo * f.w, f.x + hi * f.w)
        })
        .collect();
    let ys = geom
        .y
        .gaps
        .iter()
        .map(|g| {
            let (lo, hi) = g.span(geom.y.reversed);
            (f.y + (1.0 - hi) * f.h, f.y + (1.0 - lo) * f.h)
        })
        .collect();
    (xs, ys)
}

/// The parameter interval `t` in [0, 1] of the segment `a + t (b - a)` inside `lo..hi` along one
/// coordinate, if any.
fn inside(a: f64, b: f64, (lo, hi): (f64, f64)) -> Option<(f64, f64)> {
    let d = b - a;
    if d.abs() < 1e-12 {
        return (a > lo && a < hi).then_some((0.0, 1.0));
    }
    let (t0, t1) = ((lo - a) / d, (hi - a) / d);
    let (t0, t1) = (t0.min(t1).max(0.0), t0.max(t1).min(1.0));
    (t1 > t0).then_some((t0, t1))
}

/// `d` (path data of straight pieces, as the line drawing writes it) with every part that lies in the
/// band of a break taken out; `d` itself when the layer has no breaks, the page connects lines across
/// them, or the path is closed (an outline of a filled run keeps its shape).
pub(crate) fn cut_at_breaks(d: String, geom: &LayerGeometry) -> String {
    if geom.connect_across_breaks || (geom.x.gaps.is_empty() && geom.y.gaps.is_empty()) || d.contains('Z') {
        return d;
    }
    let (xs, ys) = bands(geom);
    let mut out = String::with_capacity(d.len());
    for sub in crate::hit::subpaths(&d) {
        let mut open = false;
        if sub.len() == 1 {
            move_to(&mut out, sub[0].0, sub[0].1);
            line_to(&mut out, sub[0].0, sub[0].1);
            continue;
        }
        for w in sub.windows(2) {
            let (a, b) = (w[0], w[1]);
            let mut cuts: Vec<(f64, f64)> = xs
                .iter()
                .filter_map(|&band| inside(a.0, b.0, band))
                .chain(ys.iter().filter_map(|&band| inside(a.1, b.1, band)))
                .collect();
            cuts.sort_by(|p, q| p.0.total_cmp(&q.0));
            let at = |t: f64| (a.0 + t * (b.0 - a.0), a.1 + t * (b.1 - a.1));
            let mut t = 0.0;
            for (c0, c1) in cuts {
                if c0 > t {
                    let (p, q) = (at(t), at(c0));
                    if !open {
                        move_to(&mut out, p.0, p.1);
                    }
                    line_to(&mut out, q.0, q.1);
                }
                open = false;
                t = t.max(c1);
            }
            if t < 1.0 {
                if !open {
                    let p = at(t);
                    move_to(&mut out, p.0, p.1);
                }
                line_to(&mut out, b.0, b.1);
                open = true;
            }
        }
    }
    out
}
