//! The graph object clipboard (T125 and T147, graphing 17.8): a drawn line, arrow, rectangle or
//! ellipse's or a text label's right-click Cut and Copy keep a snapshot here, and the graph page's
//! right-click Paste puts a copy at the clicked point.
//!
//! A text label is pasted with its anchor (the top-left corner of a table) at the clicked point; its
//! font keeps its size. The rest below is about drawn objects.
//!
//! Sheaf's rules (Origin 2016's Paste is documented as using the clicked location; the rest is not
//! measured):
//!
//! - The snapshot is the object with its size in page points, so a copy keeps its physical size in
//!   a layer of another size. It is taken when copied: later edits or deleting the source do not
//!   change it, and it stays across projects until the program closes (it is not on the system
//!   clipboard).
//! - Paste puts the top-left corner of the object's bounding box at the clicked point, in the
//!   clicked layer (layer 1 on the bare page), with the endpoints in the same order (an arrow keeps
//!   its direction). The copy goes in front of the layer's other drawn objects.

use sheaf_core::graph::{Graph, TextLabel};
use sheaf_core::shape::Shape;
use std::cell::RefCell;

/// A copied object.
#[derive(Clone, Debug, PartialEq)]
enum Clip {
    /// A drawn object: its style, and its two points in page points from the top-left corner of
    /// its bounding box.
    Shape {
        shape: Shape,
        ends: [(f64, f64); 2],
    },
    Label(TextLabel),
}

/// What Paste adds to a layer.
#[derive(Clone, Debug, PartialEq)]
pub enum Pasted {
    Shape(Shape),
    Label(TextLabel),
}

thread_local! {
    static CLIP: RefCell<Option<Clip>> = const { RefCell::new(None) };
}

/// The page-point rectangle (left, top, width, height) of layer `layer`'s frame.
fn frame_pt(graph: &Graph, layer: usize) -> Option<(f64, f64, f64, f64)> {
    let f = graph.absolute_frame(layer)?;
    let (w, h) = (graph.page.width_pt, graph.page.height_pt);
    let r = (f.left * w, f.top * h, f.width * w, f.height * h);
    (r.2 > 0.0 && r.3 > 0.0).then_some(r)
}

/// True when a drawn object or a text label has been copied.
pub fn has() -> bool {
    CLIP.with(|c| c.borrow().is_some())
}

/// Copies drawn object `shape` of `layer`; false when there is no such object.
pub fn copy(graph: &Graph, layer: usize, shape: usize) -> bool {
    let Some(s) = graph.layers.get(layer).and_then(|l| l.shapes.get(shape)) else { return false };
    let Some((fx, fy, fw, fh)) = frame_pt(graph, layer) else { return false };
    let p1 = (fx + s.x1 * fw, fy + s.y1 * fh);
    let p2 = (fx + s.x2 * fw, fy + s.y2 * fh);
    let (left, top) = (p1.0.min(p2.0), p1.1.min(p2.1));
    let clip = Clip::Shape { shape: s.clone(), ends: [(p1.0 - left, p1.1 - top), (p2.0 - left, p2.1 - top)] };
    CLIP.with(|c| *c.borrow_mut() = Some(clip));
    true
}

/// Copies text label `label` of `layer`; false when there is no such label.
pub fn copy_label(graph: &Graph, layer: usize, label: usize) -> bool {
    let Some(l) = graph.layers.get(layer).and_then(|l| l.labels.get(label)) else { return false };
    CLIP.with(|c| *c.borrow_mut() = Some(Clip::Label(l.clone())));
    true
}

/// The copied object placed for `layer` of `graph`: a drawn object with its bounding box's
/// top-left corner at the page point `page`, a label with its anchor there; `None` when nothing is
/// copied or the layer is missing.
pub fn placed(graph: &Graph, layer: usize, page: (f64, f64)) -> Option<Pasted> {
    let clip = CLIP.with(|c| c.borrow().clone())?;
    let (fx, fy, fw, fh) = frame_pt(graph, layer)?;
    let at = |(dx, dy): (f64, f64)| ((page.0 + dx - fx) / fw, (page.1 + dy - fy) / fh);
    Some(match clip {
        Clip::Shape { shape, ends } => {
            let ((x1, y1), (x2, y2)) = (at(ends[0]), at(ends[1]));
            Pasted::Shape(Shape { x1, y1, x2, y2, ..shape })
        }
        Clip::Label(label) => {
            let (x, y) = at((0.0, 0.0));
            Pasted::Label(TextLabel { x, y, ..label })
        }
    })
}

/// Forgets the copied object (tests).
#[cfg(test)]
pub fn clear() {
    CLIP.with(|c| *c.borrow_mut() = None);
}

#[cfg(test)]
mod tests {
    use super::*;
    use sheaf_core::graph::Frame;
    use sheaf_core::ids::WindowId;
    use sheaf_core::shape::ShapeKind;
    use sheaf_core::template::Template;

    #[test]
    fn a_copy_keeps_its_size_in_points_and_its_direction_in_another_frame() {
        clear();
        let mut g = Template::Line.new_graph(WindowId(1), "Graph1");
        g.page.width_pt = 400.0;
        g.page.height_pt = 200.0;
        g.layers[0].frame = Frame { left: 0.0, top: 0.0, width: 1.0, height: 1.0 };
        // An arrow from (300, 150) back to (100, 50) points.
        g.layers[0].shapes.push(Shape::new(ShapeKind::Arrow, (0.75, 0.75), (0.25, 0.25)));
        assert!(!has());
        assert!(copy(&g, 0, 0));
        assert!(!copy(&g, 0, 5), "no such object");
        // Into a frame half as wide, at page point (10, 20): still 200 x 100 points.
        let mut dest = g.clone();
        dest.layers[0].frame = Frame { left: 0.0, top: 0.0, width: 0.5, height: 1.0 };
        let Some(Pasted::Shape(s)) = placed(&dest, 0, (10.0, 20.0)) else { panic!("a shape") };
        let pt = |x: f64, y: f64| (x * 200.0, y * 200.0);
        assert_eq!(pt(s.x1, s.y1), (210.0, 120.0), "the start stays bottom right");
        assert_eq!(pt(s.x2, s.y2), (10.0, 20.0), "the head end at the corner");
        assert_eq!((s.kind, s.head_pt), (ShapeKind::Arrow, Shape::DEFAULT_HEAD_PT));
        // Deleting the source does not change the copy.
        g.layers[0].shapes.clear();
        assert_eq!(placed(&dest, 0, (10.0, 20.0)), Some(Pasted::Shape(s)));
        assert_eq!(placed(&dest, 3, (0.0, 0.0)), None, "no such layer");
    }

    #[test]
    fn a_label_is_pasted_with_its_anchor_at_the_point() {
        clear();
        let mut g = Template::Line.new_graph(WindowId(1), "Graph1");
        g.page.width_pt = 400.0;
        g.page.height_pt = 200.0;
        g.layers[0].frame = Frame { left: 0.0, top: 0.0, width: 1.0, height: 1.0 };
        let font = g.layers[0].legend.font.clone();
        g.layers[0].labels.push(TextLabel::new("hello", 0.1, 0.1, font));
        assert!(!copy_label(&g, 0, 1));
        assert!(copy_label(&g, 0, 0));
        let Some(Pasted::Label(l)) = placed(&g, 0, (100.0, 50.0)) else { panic!("a label") };
        assert_eq!((l.text.as_str(), l.x, l.y), ("hello", 0.25, 0.25));
    }
}
