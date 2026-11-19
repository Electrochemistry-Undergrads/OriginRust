//! The Object Edit toolbar's Align and Make Same Width/Height buttons (T159, graphing
//! "Aligning drawn objects and text").
//!
//! Origin's Object Edit toolbar documentation: each button acts on the selected objects and takes
//! the **last-selected** object as the reference. Left, Right, Top and Bottom line the objects' edges
//! up with that edge of the reference; Vertical lines their centres up on the reference's vertical
//! axis (same centre X) and Horizontal on its horizontal axis (same centre Y); Uniform Width and
//! Uniform Height give the objects the reference's width or height.
//!
//! Sheaf's choices where the documentation is silent (not measured): alignment uses the drawn
//! bounds of each object (what the selection handles surround); a drawn object keeps its left
//! (or top) edge when it is resized; a text label keeps its size (its size follows its font), and
//! a line with no extent in that direction is left as it is.

/// One button of the group.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Arrange {
    Left,
    Right,
    Top,
    Bottom,
    /// Origin's "Horizontal": the centres on the reference's horizontal axis (same centre Y).
    CenterH,
    /// Origin's "Vertical": the centres on the reference's vertical axis (same centre X).
    CenterV,
    SameWidth,
    SameHeight,
}

impl Arrange {
    /// The undo text of the change.
    pub fn undo_label(self) -> &'static str {
        match self {
            Arrange::SameWidth => "Make same width",
            Arrange::SameHeight => "Make same height",
            _ => "Align objects",
        }
    }
}

/// What a selected object is in its layer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    Shape(usize),
    Label(usize),
}

/// A selected object as drawn: everything in page points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Member {
    pub layer: usize,
    pub target: Target,
    /// Drawn bounds `(x, y, w, h)` on the page (y down).
    pub bounds: (f64, f64, f64, f64),
    /// The layer frame `(x, y, w, h)` the object's coordinates are fractions of.
    pub frame: (f64, f64, f64, f64),
    /// A drawn object's own extent `(width, height)` (from its two points, without line width or
    /// arrow heads); `None` for a text label.
    pub extent: Option<(f64, f64)>,
}

/// One change to make, in fractions of the object's layer frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Change {
    /// Move by `(dx, dy)`.
    Move { layer: usize, target: Target, dx: f64, dy: f64 },
    /// Give a drawn object this width or height (fractions of its frame), keeping its left/top edge.
    Resize { layer: usize, shape: usize, width: Option<f64>, height: Option<f64> },
}

/// The changes `op` makes to `others`, with `reference` the last-selected object (it stays put).
/// Objects that already match make no change.
pub fn plan(op: Arrange, reference: &Member, others: &[Member]) -> Vec<Change> {
    const EPS: f64 = 1e-9;
    let (rx, ry, rw, rh) = reference.bounds;
    let (ref_w, ref_h) = reference.extent.unwrap_or((rw, rh));
    let mut out = Vec::new();
    for m in others {
        let (x, y, w, h) = m.bounds;
        let (fw, fh) = (m.frame.2, m.frame.3);
        if fw <= 0.0 || fh <= 0.0 {
            continue;
        }
        let shift = match op {
            Arrange::Left => Some((rx - x, 0.0)),
            Arrange::Right => Some((rx + rw - (x + w), 0.0)),
            Arrange::Top => Some((0.0, ry - y)),
            Arrange::Bottom => Some((0.0, ry + rh - (y + h))),
            Arrange::CenterV => Some((rx + rw / 2.0 - (x + w / 2.0), 0.0)),
            Arrange::CenterH => Some((0.0, ry + rh / 2.0 - (y + h / 2.0))),
            Arrange::SameWidth | Arrange::SameHeight => None,
        };
        if let Some((dx, dy)) = shift {
            if dx.abs() > EPS || dy.abs() > EPS {
                out.push(Change::Move { layer: m.layer, target: m.target, dx: dx / fw, dy: dy / fh });
            }
            continue;
        }
        let (Target::Shape(shape), Some((ew, eh))) = (m.target, m.extent) else { continue };
        let (width, height) = match op {
            Arrange::SameWidth if ew > EPS && (ew - ref_w).abs() > EPS => (Some(ref_w / fw), None),
            Arrange::SameHeight if eh > EPS && (eh - ref_h).abs() > EPS => (None, Some(ref_h / fh)),
            _ => continue,
        };
        out.push(Change::Resize { layer: m.layer, shape, width, height });
    }
    out
}

/// Gives a drawn object a width of `w` (frame fractions), keeping its left edge.
pub fn set_width(s: &mut sheaf_core::shape::Shape, w: f64) {
    if s.x1 <= s.x2 {
        s.x2 = s.x1 + w;
    } else {
        s.x1 = s.x2 + w;
    }
}

/// Gives a drawn object a height of `h` (frame fractions, y down), keeping its top edge.
pub fn set_height(s: &mut sheaf_core::shape::Shape, h: f64) {
    if s.y1 <= s.y2 {
        s.y2 = s.y1 + h;
    } else {
        s.y1 = s.y2 + h;
    }
}

/// Applies the planned changes to the project: one call, so one undo step.
pub fn apply(p: &mut sheaf_core::Project, graph: sheaf_core::WindowId, changes: &[Change]) {
    let Some(g) = p.graph_mut(graph) else { return };
    for c in changes {
        match *c {
            Change::Move { layer, target, dx, dy } => {
                let Some(l) = g.layers.get_mut(layer) else { continue };
                match target {
                    Target::Shape(i) => {
                        if let Some(s) = l.shapes.get_mut(i) {
                            s.translate(dx, dy);
                        }
                    }
                    Target::Label(i) => {
                        if let Some(t) = l.labels.get_mut(i) {
                            t.x += dx;
                            t.y += dy;
                        }
                    }
                }
            }
            Change::Resize { layer, shape, width, height } => {
                let Some(s) = g.layers.get_mut(layer).and_then(|l| l.shapes.get_mut(shape)) else { continue };
                if let Some(w) = width {
                    set_width(s, w);
                }
                if let Some(h) = height {
                    set_height(s, h);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FRAME: (f64, f64, f64, f64) = (100.0, 50.0, 400.0, 200.0);

    fn shape(i: usize, bounds: (f64, f64, f64, f64)) -> Member {
        Member { layer: 0, target: Target::Shape(i), bounds, frame: FRAME, extent: Some((bounds.2, bounds.3)) }
    }

    fn label(i: usize, bounds: (f64, f64, f64, f64)) -> Member {
        Member { layer: 0, target: Target::Label(i), bounds, frame: FRAME, extent: None }
    }

    #[test]
    fn edges_and_centres_line_up_with_the_last_selected_object() {
        let reference = shape(0, (200.0, 100.0, 40.0, 20.0));
        let other = label(3, (150.0, 60.0, 20.0, 10.0));
        let moved = |op| match plan(op, &reference, &[other])[..] {
            [Change::Move { target: Target::Label(3), dx, dy, .. }] => (dx * FRAME.2, dy * FRAME.3),
            ref c => panic!("{op:?}: {c:?}"),
        };
        let close = |a: (f64, f64), b: (f64, f64)| (a.0 - b.0).abs() < 1e-9 && (a.1 - b.1).abs() < 1e-9;
        assert!(close(moved(Arrange::Left), (50.0, 0.0)));
        assert!(close(moved(Arrange::Right), (70.0, 0.0)));
        assert!(close(moved(Arrange::Top), (0.0, 40.0)));
        assert!(close(moved(Arrange::Bottom), (0.0, 50.0)));
        assert!(close(moved(Arrange::CenterV), (60.0, 0.0)));
        assert!(close(moved(Arrange::CenterH), (0.0, 45.0)));
    }

    #[test]
    fn objects_already_in_line_and_the_reference_itself_do_not_change() {
        let reference = shape(0, (200.0, 100.0, 40.0, 20.0));
        assert!(plan(Arrange::Left, &reference, &[shape(1, (200.0, 10.0, 5.0, 5.0))]).is_empty());
        assert!(plan(Arrange::SameWidth, &reference, &[shape(1, (0.0, 0.0, 40.0, 5.0))]).is_empty());
    }

    #[test]
    fn same_width_and_height_resize_drawn_objects_only() {
        let reference = shape(0, (200.0, 100.0, 40.0, 20.0));
        let others =
            [shape(1, (0.0, 0.0, 10.0, 10.0)), label(0, (0.0, 0.0, 10.0, 10.0)), shape(2, (0.0, 0.0, 0.0, 10.0))];
        assert_eq!(
            plan(Arrange::SameWidth, &reference, &others),
            [Change::Resize { layer: 0, shape: 1, width: Some(40.0 / FRAME.2), height: None }]
        );
        let height = plan(Arrange::SameHeight, &reference, &others);
        assert_eq!(height.len(), 2, "{height:?}");
        assert!(height.iter().all(|c| matches!(c, Change::Resize { width: None, height: Some(_), .. })));
    }

    #[test]
    fn a_text_label_reference_lends_its_drawn_size() {
        let reference = label(0, (0.0, 0.0, 80.0, 16.0));
        assert_eq!(
            plan(Arrange::SameWidth, &reference, &[shape(4, (0.0, 0.0, 10.0, 10.0))]),
            [Change::Resize { layer: 0, shape: 4, width: Some(80.0 / FRAME.2), height: None }]
        );
    }

    #[test]
    fn resizing_keeps_the_left_and_top_edges_whichever_point_is_first() {
        use sheaf_core::shape::{Shape, ShapeKind};
        let mut s = Shape::new(ShapeKind::Arrow, (0.6, 0.7), (0.2, 0.3));
        set_width(&mut s, 0.1);
        set_height(&mut s, 0.2);
        assert!((s.x2 - 0.2).abs() < 1e-12 && (s.x1 - 0.3).abs() < 1e-12, "{s:?}");
        assert!((s.y2 - 0.3).abs() < 1e-12 && (s.y1 - 0.5).abs() < 1e-12, "{s:?}");
    }
}
