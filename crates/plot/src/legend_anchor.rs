//! Places the visible legend bounds, excluding padding that is not painted.
use crate::Rect;
use crate::fonts::TextMeasurer;
use crate::hit;
use crate::scene::Item;
use sheaf_core::graph::LegendPosition;

pub(crate) fn place(
    out: &mut Vec<Item>,
    m: &dyn TextMeasurer,
    frame: Rect,
    (x, y): (f64, f64),
    mut draw: impl FnMut(&mut Vec<Item>, LegendPosition),
) {
    if frame.w <= 0.0 || frame.h <= 0.0 {
        return;
    }
    let mut measured = Vec::new();
    // The ordinary centre placement is linear in x/y. Measure once, then paint at the
    // translated centre; this uses the actual glyph/sample bounds without parsing paths
    // again or changing any existing position's padding or text metrics.
    draw(&mut measured, LegendPosition::Manual { x: 0.0, y: 0.0 });
    let Some(bounds) = measured.iter().filter_map(|item| hit::item_bounds(item, m)).reduce(hit::union) else {
        return;
    };
    draw(
        out,
        LegendPosition::Manual {
            x: (frame.x + x * frame.w - bounds.x) / frame.w,
            y: (frame.y + y * frame.h - bounds.y) / frame.h,
        },
    );
}
