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

use sheaf_plot::scene::ObjectKind;

/// One button of the Align group.
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

impl Target {
    /// The target of a picked drawn object or text label.
    pub fn of(kind: ObjectKind) -> Option<Target> {
        match kind {
            ObjectKind::Shape(i) => Some(Target::Shape(i)),
            ObjectKind::Label(i) => Some(Target::Label(i)),
            _ => None,
        }
    }
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

/// The Object Edit group of a drawn object or text label (T162).
pub fn group_of(graph: &sheaf_core::graph::Graph, layer: usize, kind: ObjectKind) -> Option<u32> {
    let l = graph.layers.get(layer)?;
    match kind {
        ObjectKind::Shape(i) => l.shapes.get(i)?.group,
        ObjectKind::Label(i) => l.labels.get(i)?.group,
        _ => None,
    }
}

/// The object a click picked, with the other members of its group before it (T162): a click on
/// a grouped object selects the whole group, the clicked one the reference.
pub fn with_group(graph: &sheaf_core::graph::Graph, layer: usize, kind: ObjectKind) -> Vec<(usize, ObjectKind)> {
    let Some(group) = group_of(graph, layer, kind) else { return vec![(layer, kind)] };
    let l = &graph.layers[layer];
    let shapes = l.shapes.iter().enumerate().filter(|(_, s)| s.group == Some(group)).map(|(i, _)| ObjectKind::Shape(i));
    let labels = l.labels.iter().enumerate().filter(|(_, t)| t.group == Some(group)).map(|(i, _)| ObjectKind::Label(i));
    let mut all: Vec<_> = shapes.chain(labels).filter(|k| *k != kind).map(|k| (layer, k)).collect();
    all.push((layer, kind));
    all
}

/// Whether Group can join these selected objects: two or more, all in one layer, and not already
/// exactly one group.
pub fn can_group(graph: &sheaf_core::graph::Graph, selection: &[(usize, ObjectKind)]) -> bool {
    let Some(&(layer, _)) = selection.first() else { return false };
    if selection.len() < 2 || selection.iter().any(|(l, _)| *l != layer) {
        return false;
    }
    let groups: Vec<_> = selection.iter().map(|(l, k)| group_of(graph, *l, *k)).collect();
    let one = groups[0].is_some() && groups.iter().all(|g| *g == groups[0]);
    !(one && with_group(graph, selection[0].0, selection[0].1).len() == selection.len())
}

/// Whether Ungroup has something to do: a selected object is in a group.
pub fn can_ungroup(graph: &sheaf_core::graph::Graph, selection: &[(usize, ObjectKind)]) -> bool {
    selection.iter().any(|(l, k)| group_of(graph, *l, *k).is_some())
}

/// Group: the selected objects (one layer) get a new group number of their layer. One call, one
/// undo step. Objects that were in other groups leave them; the rest of those groups stay together.
pub fn group(p: &mut sheaf_core::Project, graph: sheaf_core::WindowId, selection: &[(usize, ObjectKind)]) {
    let Some(&(layer, _)) = selection.first() else { return };
    let Some(l) = p.graph_mut(graph).and_then(|g| g.layers.get_mut(layer)) else { return };
    let next = l
        .shapes
        .iter()
        .filter_map(|s| s.group)
        .chain(l.labels.iter().filter_map(|t| t.group))
        .max()
        .map_or(1, |g| g + 1);
    for (_, kind) in selection {
        match *kind {
            ObjectKind::Shape(i) => {
                if let Some(s) = l.shapes.get_mut(i) {
                    s.group = Some(next);
                }
            }
            ObjectKind::Label(i) => {
                if let Some(t) = l.labels.get_mut(i) {
                    t.group = Some(next);
                }
            }
            _ => {}
        }
    }
}

/// Ungroup: every group a selected object is in is dissolved. One call, one undo step.
pub fn ungroup(p: &mut sheaf_core::Project, graph: sheaf_core::WindowId, selection: &[(usize, ObjectKind)]) {
    let Some(g) = p.graph_mut(graph) else { return };
    let mut gone: Vec<(usize, u32)> = Vec::new();
    for (layer, kind) in selection {
        if let Some(id) = group_of(g, *layer, *kind) {
            gone.push((*layer, id));
        }
    }
    for (layer, id) in gone {
        let Some(l) = g.layers.get_mut(layer) else { continue };
        for s in l.shapes.iter_mut().filter(|s| s.group == Some(id)) {
            s.group = None;
        }
        for t in l.labels.iter_mut().filter(|t| t.group == Some(id)) {
            t.group = None;
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

    #[test]
    fn group_ungroup_and_a_click_on_a_member_select_the_whole_group() {
        use sheaf_core::graph::TextLabel;
        use sheaf_core::shape::{Shape, ShapeKind};
        let mut p = sheaf_core::Project::default();
        let id = p.add_graph(sheaf_core::template::Template::Line);
        let l = &mut p.graph_mut(id).unwrap().layers[0];
        let font = l.legend.font.clone();
        l.shapes = (0..3).map(|i| Shape::new(ShapeKind::Line, (0.1 * i as f64, 0.1), (0.2, 0.2))).collect();
        l.labels = vec![TextLabel::new("a", 0.5, 0.5, font)];
        let sel = [(0, ObjectKind::Shape(0)), (0, ObjectKind::Label(0))];
        let graph = |p: &sheaf_core::Project| p.graph(id).unwrap().clone();
        assert!(can_group(&graph(&p), &sel) && !can_ungroup(&graph(&p), &sel));
        assert!(!can_group(&graph(&p), &sel[..1]), "one object");
        assert!(!can_group(&graph(&p), &[(0, ObjectKind::Shape(0)), (1, ObjectKind::Shape(0))]), "two layers");
        group(&mut p, id, &sel);
        let g = graph(&p);
        assert_eq!((g.layers[0].shapes[0].group, g.layers[0].labels[0].group), (Some(1), Some(1)));
        assert_eq!(with_group(&g, 0, ObjectKind::Label(0)), vec![(0, ObjectKind::Shape(0)), (0, ObjectKind::Label(0))]);
        assert_eq!(with_group(&g, 0, ObjectKind::Shape(2)), vec![(0, ObjectKind::Shape(2))]);
        assert!(!can_group(&g, &sel), "already exactly this group");
        // Grouping a member with another object makes group 2 of the two.
        group(&mut p, id, &[(0, ObjectKind::Shape(0)), (0, ObjectKind::Shape(1))]);
        let g = graph(&p);
        assert_eq!(g.layers[0].shapes.iter().map(|s| s.group).collect::<Vec<_>>(), [Some(2), Some(2), None]);
        assert_eq!(g.layers[0].labels[0].group, Some(1));
        ungroup(&mut p, id, &[(0, ObjectKind::Shape(1))]);
        let g = graph(&p);
        assert_eq!(g.layers[0].shapes.iter().map(|s| s.group).collect::<Vec<_>>(), [None, None, None]);
        assert_eq!(g.layers[0].labels[0].group, Some(1), "other groups stay");
    }
}
