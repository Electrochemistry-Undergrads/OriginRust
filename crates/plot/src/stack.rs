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

/// The colour scale of a colour mapped waterfall (graphing spec 13.17): the bar of the shared ramp beside the
/// layer's whole frame (the back curve's end), titled Y or Z, ticks at nice values of the scale.
pub(crate) fn draw_scale(
    out: &mut Vec<crate::scene::Item>,
    layer: &sheaf_core::graph::Layer,
    pds: &[crate::data::PlotData],
    geom: &crate::LayerGeometry,
    m: &dyn crate::fonts::TextMeasurer,
) {
    let Some(wf) = pds.iter().find_map(|pd| pd.wf) else { return };
    let title = if wf.mode == sheaf_core::graph::WaterfallColor::Z { "Z" } else { "Y" };
    let mut whole = geom.clone();
    whole.frame = geom.clip;
    crate::hexbin::draw_color_bar(
        out,
        layer,
        &whole,
        m,
        title,
        &crate::sunburst::scale_ticks(wf.lo, wf.hi),
        (0.0, 0.0),
    );
}
