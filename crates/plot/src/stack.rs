//! The waterfall and the places of stacked curves (graphing spec section 13). The offsets of a
//! stack are worked out in `sheaf_core::stack`.

use crate::Rect;
use sheaf_core::graph::{Graph, Stack};
pub(crate) use sheaf_core::stack::places;

/// The waterfall shift of layer `index` as fractions of the front frame `(sx, sy)`, when the layer
/// is drawn as a waterfall: its mode is [`Stack::Waterfall`] and it is neither linked to another
/// layer nor has linked layers. Otherwise `None`.
pub(crate) fn waterfall(g: &Graph, index: usize) -> Option<(f64, f64)> {
    let Stack::Waterfall { dx_percent, dy_percent } = g.layers.get(index)?.stack else { return None };
    (g.group(index).len() == 1).then(|| (finite(dx_percent) / 100.0, finite(dy_percent) / 100.0))
}

/// A stored number that is not finite counts as 0.
fn finite(v: f64) -> f64 {
    if v.is_finite() { v } else { 0.0 }
}

/// The front frame of a waterfall with `n` members inside the layer frame `f` (spec 13.6): the
/// frame shrinks so that the last member, moved by `(n - 1)` shifts, still ends at the edge of `f`.
pub(crate) fn front_frame(f: Rect, n: usize, sx: f64, sy: f64) -> Rect {
    if n <= 1 {
        return f;
    }
    let k = (n - 1) as f64;
    let w = f.w / (1.0 + k * sx.abs());
    let h = f.h / (1.0 + k * sy.abs());
    let x = if sx >= 0.0 { f.x } else { f.x + f.w - w };
    let y = if sy >= 0.0 { f.y + f.h - h } else { f.y };
    Rect { x, y, w, h }
}
