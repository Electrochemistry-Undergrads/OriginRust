//! Picking: which object of a graph is under a point of the page.
//!
//! The scene records, per layer, the items drawn for each axis, title, legend, label and plot
//! ([`SceneObject`]). [`hit_test`] looks at them from the top of the drawing order down: free
//! labels, the legend, titles, axes, then plots (by the distance to their lines and symbols), and
//! finally the inside of a layer frame. The desktop app uses it for double-clicks and right-click
//! menus on the graph, as in Origin.

use crate::Rect;
use crate::fonts::TextMeasurer;
use crate::scene::{Item, ItemList, LayerScene, ObjectKind, Scene, SceneObject};
use crate::text::{ASCENT, DESCENT};
use sheaf_core::graph::Font;

/// What a point of the page is on.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Hit {
    /// An object of layer `layer`.
    Object { layer: usize, kind: ObjectKind },
    /// Inside the frame of layer `layer`, on no object.
    Layer(usize),
    /// On the page, outside every frame.
    Page,
}

/// Records the items `items[start..]` (of list `list`) as object `kind` of model layer `layer`.
/// `extra` boxes also count as the object (an axis line drawn together with the other edges).
/// Nothing is recorded when nothing was drawn.
#[allow(clippy::too_many_arguments)]
pub(crate) fn record(
    objects: &mut Vec<SceneObject>,
    items: &[Item],
    start: usize,
    kind: ObjectKind,
    layer: usize,
    list: ItemList,
    extra: &[Rect],
    m: &dyn TextMeasurer,
) {
    let mut bounds: Option<Rect> = None;
    let mut boxes: Vec<Rect> = extra.to_vec();
    for item in &items[start..] {
        if let Some(b) = item_bounds(item, m) {
            if matches!(item, Item::Text(_)) {
                boxes.push(b);
            }
            bounds = Some(bounds.map_or(b, |a| union(a, b)));
        }
    }
    for &b in extra {
        bounds = Some(bounds.map_or(b, |a| union(a, b)));
    }
    if let Some(bounds) = bounds {
        objects.push(SceneObject { kind, layer, list, items: start..items.len(), bounds, boxes });
    }
}

pub(crate) fn union(a: Rect, b: Rect) -> Rect {
    let (x0, y0) = (a.x.min(b.x), a.y.min(b.y));
    let (x1, y1) = ((a.x + a.w).max(b.x + b.w), (a.y + a.h).max(b.y + b.h));
    Rect { x: x0, y: y0, w: x1 - x0, h: y1 - y0 }
}

fn of_points(points: impl IntoIterator<Item = (f64, f64)>) -> Option<Rect> {
    let mut b: Option<(f64, f64, f64, f64)> = None;
    for (x, y) in points {
        if !(x.is_finite() && y.is_finite()) {
            continue;
        }
        b = Some(match b {
            None => (x, y, x, y),
            Some((x0, y0, x1, y1)) => (x0.min(x), y0.min(y), x1.max(x), y1.max(y)),
        });
    }
    b.map(|(x0, y0, x1, y1)| Rect { x: x0, y: y0, w: x1 - x0, h: y1 - y0 })
}

/// The bounding box of one item in page points.
pub(crate) fn item_bounds(item: &Item, m: &dyn TextMeasurer) -> Option<Rect> {
    match item {
        Item::Rect { rect, .. } | Item::Image { rect, .. } => Some(*rect),
        Item::Path { d, .. } => of_points(subpaths(d).into_iter().flatten()),
        Item::Text(t) => {
            let font =
                Font { family: t.family.clone(), size_pt: t.size, bold: t.bold, italic: t.italic, color: t.color };
            let w = m.width(&t.text, &font, t.size);
            let x0 = match t.anchor {
                crate::scene::Anchor::Start => 0.0,
                crate::scene::Anchor::Middle => -w / 2.0,
                crate::scene::Anchor::End => -w,
            };
            // Corners relative to the anchor, before the counter-clockwise rotation (y down).
            let corners = [
                (x0, -ASCENT * t.size),
                (x0 + w, -ASCENT * t.size),
                (x0, DESCENT * t.size),
                (x0 + w, DESCENT * t.size),
            ];
            let (sin, cos) = t.rotate_ccw.to_radians().sin_cos();
            of_points(corners.iter().map(|&(dx, dy)| (t.x + dx * cos + dy * sin, t.y - dx * sin + dy * cos)))
        }
    }
}

/// The vertices of each subpath of SVG path data written by this crate (`M`, `L`, `C`, `A`,
/// `Z`; absolute coordinates). Curves contribute their end points only.
pub(crate) fn subpaths(d: &str) -> Vec<Vec<(f64, f64)>> {
    let mut out: Vec<Vec<(f64, f64)>> = Vec::new();
    let mut cmd = 'M';
    let mut nums: Vec<f64> = Vec::new();
    let flush = |cmd: char, nums: &mut Vec<f64>, out: &mut Vec<Vec<(f64, f64)>>| {
        // Numbers per segment, and where the end point is among them.
        let (per, end) = match cmd {
            'M' | 'L' => (2, 0),
            'C' => (6, 4),
            'A' => (7, 5),
            _ => {
                nums.clear();
                return;
            }
        };
        for (k, seg) in nums.chunks_exact(per).enumerate() {
            let p = (seg[end], seg[end + 1]);
            if cmd == 'M' && k == 0 || out.is_empty() {
                out.push(Vec::new());
            }
            if let Some(last) = out.last_mut() {
                last.push(p);
            }
        }
        nums.clear();
    };
    let mut token = String::new();
    let take = |token: &mut String, nums: &mut Vec<f64>| {
        if !token.is_empty() {
            if let Ok(v) = token.parse() {
                nums.push(v);
            }
            token.clear();
        }
    };
    for c in d.chars() {
        match c {
            'M' | 'L' | 'C' | 'A' | 'Z' => {
                take(&mut token, &mut nums);
                flush(cmd, &mut nums, &mut out);
                if c == 'Z' {
                    // Close: the next drawing starts again at the subpath's first point.
                    if let Some(first) = out.last().and_then(|s| s.first().copied()) {
                        out.last_mut().unwrap().push(first);
                    }
                }
                cmd = c;
            }
            ' ' | ',' => take(&mut token, &mut nums),
            '-' if !token.is_empty() && !token.ends_with('e') => {
                take(&mut token, &mut nums);
                token.push(c);
            }
            _ => token.push(c),
        }
    }
    take(&mut token, &mut nums);
    flush(cmd, &mut nums, &mut out);
    out
}

/// Every vertex of SVG path data written by this crate, in page points (the data points of a
/// line, the corners of bars and symbols). The desktop app draws selection handles on them.
pub fn path_points(d: &str) -> Vec<(f64, f64)> {
    subpaths(d).into_iter().flatten().collect()
}

fn contains(r: Rect, (x, y): (f64, f64), pad: f64) -> bool {
    x >= r.x - pad && x <= r.x + r.w + pad && y >= r.y - pad && y <= r.y + r.h + pad
}

fn segment_distance((px, py): (f64, f64), (ax, ay): (f64, f64), (bx, by): (f64, f64)) -> f64 {
    let (dx, dy) = (bx - ax, by - ay);
    let len2 = dx * dx + dy * dy;
    let t = if len2 > 0.0 { (((px - ax) * dx + (py - ay) * dy) / len2).clamp(0.0, 1.0) } else { 0.0 };
    let (cx, cy) = (ax + t * dx, ay + t * dy);
    ((px - cx).powi(2) + (py - cy).powi(2)).sqrt()
}

/// True when `p` is within `tol` points of a line or symbol of the items.
fn near_items(items: &[Item], p: (f64, f64), tol: f64) -> bool {
    items.iter().any(|item| match item {
        Item::Rect { rect, .. } | Item::Image { rect, .. } => contains(*rect, p, tol),
        Item::Path { d, fill, stroke } => {
            let half = stroke.as_ref().map_or(0.0, |s| s.width / 2.0);
            subpaths(d).iter().any(|sub| {
                if fill.is_some() && of_points(sub.iter().copied()).is_some_and(|b| contains(b, p, tol + half)) {
                    return true;
                }
                match sub.as_slice() {
                    [single] => segment_distance(p, *single, *single) <= tol + half,
                    pts => pts.windows(2).any(|w| segment_distance(p, w[0], w[1]) <= tol + half),
                }
            })
        }
        Item::Text(_) => false,
    })
}

/// True when `p` is on a drawn object: near its outline, or inside it when it is filled (a
/// polygon test, so the empty corners of an ellipse's box do not count).
fn near_shape(items: &[Item], p: (f64, f64), tol: f64) -> bool {
    items.iter().any(|item| {
        let Item::Path { d, fill, stroke } = item else { return false };
        let half = stroke.as_ref().map_or(0.0, |s| s.width / 2.0);
        subpaths(d).iter().any(|sub| {
            sub.windows(2).any(|w| segment_distance(p, w[0], w[1]) <= tol + half)
                || (fill.is_some() && inside_polygon(sub, p))
        })
    })
}

fn inside_polygon(poly: &[(f64, f64)], (px, py): (f64, f64)) -> bool {
    let mut inside = false;
    for w in poly.windows(2) {
        let ((ax, ay), (bx, by)) = (w[0], w[1]);
        if (ay > py) != (by > py) && px < ax + (py - ay) / (by - ay) * (bx - ax) {
            inside = !inside;
        }
    }
    inside
}

fn list(layer: &LayerScene, which: ItemList) -> &[Item] {
    match which {
        ItemList::Plots => &layer.plots,
        ItemList::Overlay => &layer.overlay,
    }
}

/// The object of `scene` under page point `p` (points, y down). `tol` is how far from a line, in
/// points, a click still counts as on it (about 3 screen pixels is a good choice).
pub fn hit_test(scene: &Scene, p: (f64, f64), tol: f64) -> Hit {
    // Objects of a layer are recorded in paint order, so within the rank of labels and drawn
    // objects the last recorded is on top and is tried first: a click picks what the eye sees.
    let all = |topmost_first: bool| {
        scene.layers.iter().rev().flat_map(move |l| {
            let objects: Box<dyn Iterator<Item = &SceneObject> + '_> =
                if topmost_first { Box::new(l.objects.iter().rev()) } else { Box::new(l.objects.iter()) };
            objects.map(move |o| (l, o))
        })
    };
    let rank = |layer: &LayerScene, k: ObjectKind| match k {
        ObjectKind::Label(_) | ObjectKind::Shape(_) | ObjectKind::Slope(_) => 0,
        // A straight line runs across the whole frame: axes and plots under it are tried first.
        ObjectKind::StraightLine(_) => 5,
        ObjectKind::Legend | ObjectKind::Scale(_) => 1,
        ObjectKind::XTitle | ObjectKind::YTitle => {
            if layer.data_on_top {
                3
            } else {
                2
            }
        }
        ObjectKind::XAxis | ObjectKind::YAxis => {
            if layer.data_on_top {
                4
            } else {
                3
            }
        }
        ObjectKind::Plot(_) => {
            if layer.data_on_top {
                2
            } else {
                4
            }
        }
    };
    for wanted in 0..=5 {
        for (layer, o) in all(wanted == 0) {
            let under_data =
                o.kind == ObjectKind::Legend && o.list == ItemList::Overlay && o.items.start < layer.underlay_len;
            let rank = if under_data { 5 } else { rank(layer, o.kind) };
            if rank != wanted || !contains(o.bounds, p, tol) {
                continue;
            }
            let hit = match o.kind {
                // Lines can be far apart inside their box (ticks on both edges, a curve across the
                // frame): only a click near a line, a symbol or a tick label counts.
                ObjectKind::Shape(_) => near_shape(&list(layer, o.list)[o.items.clone()], p, tol),
                ObjectKind::Slope(_) | ObjectKind::StraightLine(_) => {
                    o.boxes.iter().any(|&b| contains(b, p, tol))
                        || near_shape(&list(layer, o.list)[o.items.clone()], p, tol)
                }
                ObjectKind::Plot(_) | ObjectKind::XAxis | ObjectKind::YAxis => {
                    o.boxes.iter().any(|&b| contains(b, p, tol))
                        || near_items(&list(layer, o.list)[o.items.clone()], p, tol)
                }
                _ => true,
            };
            if hit {
                return Hit::Object { layer: o.layer, kind: o.kind };
            }
        }
    }
    // The topmost frame that holds the point.
    for (i, layer) in scene.layers.iter().enumerate().rev() {
        if layer.clip.w > 0.0 && layer.clip.h > 0.0 && contains(layer.clip, p, 0.0) {
            return Hit::Layer(i);
        }
    }
    Hit::Page
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_data_is_split_into_subpaths_with_end_points() {
        let s = subpaths("M0 0L10 0L10 5ZM1 1A2 2 0 1 0 5 1A2 2 0 1 0 1 1Z");
        assert_eq!(s.len(), 2);
        assert_eq!(s[0], vec![(0.0, 0.0), (10.0, 0.0), (10.0, 5.0), (0.0, 0.0)]);
        assert_eq!(s[1], vec![(1.0, 1.0), (5.0, 1.0), (1.0, 1.0), (1.0, 1.0)]);
        let c = subpaths("M0 0C1 1,2 2,3 -3.5");
        assert_eq!(c, vec![vec![(0.0, 0.0), (3.0, -3.5)]]);
    }

    #[test]
    fn distance_to_a_segment() {
        assert_eq!(segment_distance((5.0, 3.0), (0.0, 0.0), (10.0, 0.0)), 3.0);
        assert_eq!(segment_distance((13.0, 4.0), (0.0, 0.0), (10.0, 0.0)), 5.0);
    }
}
