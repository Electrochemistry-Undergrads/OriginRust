//! Drawn objects of a layer (lines, arrows, rectangles, ellipses): graphing spec section 17.
//! They are drawn into the layer's overlay after the free text labels, so they lie above the data,
//! and each is recorded as a pickable [`ObjectKind::Shape`].

use crate::Rect;
use crate::fonts::TextMeasurer;
use crate::hit;
use crate::plots::line_stroke;
use crate::scene::{Item, ItemList, ObjectKind, SceneObject, line_to, move_to};
use sheaf_core::graph::{Layer, LineStyle};
use sheaf_core::shape::{Shape, ShapeKind};

/// Segments of the polygon that stands for an ellipse (every renderer takes straight segments).
const ELLIPSE_SEGMENTS: usize = 96;

/// Page points of the end points of a line or arrow, or the corners of a box, of `shape` in the
/// layer frame `f`.
pub(crate) fn page_points(shape: &Shape, f: Rect) -> ((f64, f64), (f64, f64)) {
    ((f.x + shape.x1 * f.w, f.y + shape.y1 * f.h), (f.x + shape.x2 * f.w, f.y + shape.y2 * f.h))
}

/// The head of an arrow at `tip`, pointing along `dir` (unit vector): the three corners.
/// Length is `head`, half width 0.35 of it (a head about 38 degrees wide).
fn head_corners(tip: (f64, f64), dir: (f64, f64), head: f64) -> [(f64, f64); 3] {
    let base = (tip.0 - dir.0 * head, tip.1 - dir.1 * head);
    let half = 0.35 * head;
    let n = (-dir.1, dir.0);
    [tip, (base.0 + n.0 * half, base.1 + n.1 * half), (base.0 - n.0 * half, base.1 - n.1 * half)]
}

fn polygon(points: &[(f64, f64)]) -> String {
    let mut d = String::new();
    for (i, (x, y)) in points.iter().enumerate() {
        if i == 0 {
            move_to(&mut d, *x, *y);
        } else {
            line_to(&mut d, *x, *y);
        }
    }
    d.push('Z');
    d
}

/// The items of one shape.
pub(crate) fn shape_items(shape: &Shape, f: Rect) -> Vec<Item> {
    let (a, b) = page_points(shape, f);
    let style = LineStyle {
        color: shape.color,
        width_pt: shape.width_pt,
        dash: shape.dash,
        connect: Default::default(),
        sort_x: false,
    };
    let mut stroke = line_stroke(&style);
    stroke.cap = crate::scene::Cap::Butt;
    let mut items = Vec::new();
    match shape.kind {
        ShapeKind::Line | ShapeKind::Arrow => {
            let (dx, dy) = (b.0 - a.0, b.1 - a.1);
            let len = dx.hypot(dy);
            let (mut from, mut to) = (a, b);
            let mut heads: Vec<[(f64, f64); 3]> = Vec::new();
            if shape.kind == ShapeKind::Arrow && len > 0.0 {
                let dir = (dx / len, dy / len);
                // A head never takes more than the whole line (half of it with two heads).
                let ends = if shape.head_at_start { 2.0 } else { 1.0 };
                let head = shape.head_pt.min(len / ends).max(0.0);
                heads.push(head_corners(b, dir, head));
                // The line stops at the base of the head, so a wide line does not poke out.
                to = (b.0 - dir.0 * head * 0.9, b.1 - dir.1 * head * 0.9);
                if shape.head_at_start {
                    heads.push(head_corners(a, (-dir.0, -dir.1), head));
                    from = (a.0 + dir.0 * head * 0.9, a.1 + dir.1 * head * 0.9);
                }
            }
            let mut d = String::new();
            move_to(&mut d, from.0, from.1);
            line_to(&mut d, to.0, to.1);
            items.push(Item::Path { d, fill: None, stroke: Some(stroke) });
            for h in heads {
                let mut solid = line_stroke(&LineStyle { dash: sheaf_core::graph::Dash::Solid, ..style.clone() });
                solid.width = shape.width_pt.min(1.0);
                items.push(Item::Path { d: polygon(&h), fill: Some(shape.color), stroke: Some(solid) });
            }
        }
        ShapeKind::Rectangle => {
            let (x0, x1) = (a.0.min(b.0), a.0.max(b.0));
            let (y0, y1) = (a.1.min(b.1), a.1.max(b.1));
            let d = polygon(&[(x0, y0), (x1, y0), (x1, y1), (x0, y1)]);
            items.push(Item::Path { d, fill: shape.fill, stroke: Some(stroke) });
        }
        ShapeKind::Ellipse => {
            let (cx, cy) = ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0);
            let (rx, ry) = ((b.0 - a.0).abs() / 2.0, (b.1 - a.1).abs() / 2.0);
            let pts: Vec<(f64, f64)> = (0..ELLIPSE_SEGMENTS)
                .map(|k| {
                    let t = std::f64::consts::TAU * k as f64 / ELLIPSE_SEGMENTS as f64;
                    (cx + rx * t.cos(), cy + ry * t.sin())
                })
                .collect();
            items.push(Item::Path { d: polygon(&pts), fill: shape.fill, stroke: Some(stroke) });
        }
    }
    items
}

/// Draws every shape of `layer` (model layer `index`) into `out` and records them as objects.
pub(crate) fn draw_shapes(
    out: &mut Vec<Item>,
    objects: &mut Vec<SceneObject>,
    layer: &Layer,
    index: usize,
    f: Rect,
    m: &dyn TextMeasurer,
) {
    for (k, shape) in layer.shapes.iter().enumerate() {
        let start = out.len();
        out.extend(shape_items(shape, f));
        // The recorded bounds include the stroke, so a thick line is picked on its visible width
        // and the selection box surrounds it.
        let half = shape.width_pt.max(0.0) / 2.0;
        let padded: Vec<Rect> = out[start..]
            .iter()
            .filter_map(|item| hit::item_bounds(item, m))
            .reduce(hit::union)
            .map(|b| Rect { x: b.x - half, y: b.y - half, w: b.w + 2.0 * half, h: b.h + 2.0 * half })
            .into_iter()
            .collect();
        hit::record(objects, out, start, ObjectKind::Shape(k), index, ItemList::Overlay, &padded, m);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const F: Rect = Rect { x: 100.0, y: 50.0, w: 400.0, h: 200.0 };

    #[test]
    fn an_arrow_ends_in_a_head_whose_tip_is_the_end_point() {
        let mut s = Shape::new(ShapeKind::Arrow, (0.0, 0.5), (1.0, 0.5));
        s.head_pt = 10.0;
        let items = shape_items(&s, F);
        assert_eq!(items.len(), 2);
        // Line from (100, 150) to the head base: 500 - 9 = 491 in x.
        let Item::Path { d, .. } = &items[0] else { panic!() };
        assert_eq!(d, "M100 150L491 150");
        // Head: tip (500, 150), base corners at x = 490, y = 150 +- 3.5.
        let Item::Path { d, fill, .. } = &items[1] else { panic!() };
        assert_eq!(d, "M500 150L490 153.5L490 146.5Z");
        assert_eq!(*fill, Some(s.color));
        s.head_at_start = true;
        assert_eq!(shape_items(&s, F).len(), 3);
    }

    #[test]
    fn a_rectangle_is_its_four_corners_and_an_ellipse_stays_in_its_box() {
        let s = Shape::new(ShapeKind::Rectangle, (0.75, 0.5), (0.25, 0.1));
        let Item::Path { d, fill, .. } = &shape_items(&s, F)[0] else { panic!() };
        assert_eq!(d, "M200 70L400 70L400 150L200 150Z");
        assert_eq!(*fill, Some(Shape::DEFAULT_RECT_FILL), "Origin's Rectangle tool fills LT Gray");
        let e = Shape::new(ShapeKind::Ellipse, (0.25, 0.1), (0.75, 0.5));
        let Item::Path { d, .. } = &shape_items(&e, F)[0] else { panic!() };
        let pts = hit::path_points(d);
        // Centre (300, 110), radii 100 and 40: first point at angle 0, a quarter turn is the 25th.
        assert_eq!(pts[0], (400.0, 110.0));
        assert_eq!(pts[ELLIPSE_SEGMENTS / 4], (300.0, 150.0));
        assert!(pts.iter().all(|p| ((p.0 - 300.0) / 100.0).powi(2) + ((p.1 - 110.0) / 40.0).powi(2) < 1.0 + 1e-3));
    }
}
