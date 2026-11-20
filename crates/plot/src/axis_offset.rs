//! Displaced Y axes (graphing spec 21.3 and 21.5): a layer's Y axis may stand outward from the edge of
//! its frame by a percentage of the frame's width (Origin's axis position "% From Left" / "% From
//! Right"), as the extra Y axes of the 3Ys and 4Ys graphs do. The line, ticks, tick labels and title
//! move together; the frame, the grid and the data stay.

use crate::Rect;
use sheaf_core::graph::{Axis, AxisSide};

/// The offset in percent that puts `axis` at its "At Position =" value (T243): where `other` (the
/// other axis of the layer) has that value, measured inward from the axis' own edge, the frame's
/// height for an X axis and its width for a Y axis. `None` without a value, for a hidden axis, or
/// for a value the other axis cannot place (a non-positive value on a log axis, inside a break).
pub(crate) fn at_position_pct(axis: &Axis, other: &crate::ticks::ResolvedAxis) -> Option<f64> {
    let v = axis.at_position?;
    if axis.side == AxisSide::Hidden || !v.is_finite() {
        return None;
    }
    let u = other.to_unit(v);
    if !u.is_finite() {
        return None;
    }
    let u = u.clamp(0.0, 1.0);
    // 0 at the bottom (left) of the frame: the Normal edge for an X (Y) axis, the far one for Opposite.
    let inward = if axis.side == AxisSide::Opposite { 1.0 - u } else { u };
    Some(-inward * 100.0)
}

/// The distance in points the Y axis line stands from its edge, outward positive.
pub(crate) fn offset_pt(axis: &Axis, frame: Rect) -> f64 {
    if axis.side == AxisSide::Hidden || !axis.offset_pct.is_finite() { 0.0 } else { axis.offset_pct / 100.0 * frame.w }
}

/// The distance in points the X axis line stands from its edge, outward positive: a percentage of the
/// frame's height ("% From Bottom" / "% From Top").
pub(crate) fn x_offset_pt(axis: &Axis, frame: Rect) -> f64 {
    if axis.side == AxisSide::Hidden || !axis.offset_pct.is_finite() { 0.0 } else { axis.offset_pct / 100.0 * frame.h }
}

/// The rectangle the X axis is laid out against: `frame` with the edge the axis is on moved outward by
/// the offset (inward when negative). The other edge, and the horizontal extent, stay.
pub(crate) fn x_axis_frame(axis: &Axis, frame: Rect) -> Rect {
    let d = x_offset_pt(axis, frame);
    if d == 0.0 {
        return frame;
    }
    if axis.side == AxisSide::Opposite {
        Rect { y: frame.y - d, h: frame.h + d, ..frame }
    } else {
        Rect { h: frame.h + d, ..frame }
    }
}

/// The page Y of the X axis line.
pub(crate) fn x_line_y(axis: &Axis, frame: Rect) -> f64 {
    let f = x_axis_frame(axis, frame);
    if axis.side == AxisSide::Opposite { f.y } else { f.y + f.h }
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
