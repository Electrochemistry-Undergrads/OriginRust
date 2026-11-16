//! Displaced Y axes (graphing spec 21.3 and 21.5): a layer's Y axis may stand outward from the edge of
//! its frame by a percentage of the frame's width (Origin's axis position "% From Left" / "% From
//! Right"), as the extra Y axes of the 3Ys and 4Ys graphs do. The line, ticks, tick labels and title
//! move together; the frame, the grid and the data stay.

use crate::Rect;
use sheaf_core::graph::{Axis, AxisSide};

/// The distance in points the Y axis line stands from its edge, outward positive.
pub(crate) fn offset_pt(axis: &Axis, frame: Rect) -> f64 {
    if axis.side == AxisSide::Hidden || !axis.offset_pct.is_finite() { 0.0 } else { axis.offset_pct / 100.0 * frame.w }
}

/// The rectangle the Y axis is laid out against: `frame` with the edge the axis is on moved outward by
/// the offset. The other edge, and the vertical extent, stay.
pub(crate) fn y_axis_frame(axis: &Axis, frame: Rect) -> Rect {
    let d = offset_pt(axis, frame);
    if d == 0.0 {
        return frame;
    }
    if axis.side == AxisSide::Opposite {
        Rect { w: frame.w + d, ..frame }
    } else {
        Rect { x: frame.x - d, w: frame.w + d, ..frame }
    }
}

/// The page X of the Y axis line.
pub(crate) fn y_line_x(axis: &Axis, frame: Rect) -> f64 {
    let f = y_axis_frame(axis, frame);
    if axis.side == AxisSide::Opposite { f.x + f.w } else { f.x }
}
