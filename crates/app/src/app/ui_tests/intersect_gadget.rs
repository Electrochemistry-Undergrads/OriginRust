//! Gadgets > Intersect (T139, graphing "Intersect gadget").

use super::*;
use crate::gadget::GadgetKind;

#[test]
fn the_intersect_gadget_finds_where_the_plots_cross_inside_its_range() {
    // The sample's "up" (2.5, 4.5, 6.5, 8.5) and "down" (30.5, 20.5, 10.5, 0.5) cross once between t = 3 and 4.
    let mut h = start(vec![data_file("intersect-gadget")]);
    let ctx = h.ctx.clone();
    let graph = project(&h).graphs().next().unwrap().id;
    h.state_mut().apply(Action::StartIntersect { graph }, &ctx);
    settle(&mut h);
    let mut g = h.state().graphs[&graph].gadget.expect("the gadget is open");
    assert_eq!(g.kind, GadgetKind::Intersect);
    (g.x0, g.x1) = (1.0, 4.0);
    let points = g.crossings(project(&h), graph);
    assert_eq!(points.len(), 1, "{points:?}");
    // up = 2t + 0.5, down = -10t + 40.5: they meet at t = 10/3.
    assert!((points[0].x - 10.0 / 3.0).abs() < 1e-9 && (points[0].y - (2.0 * 10.0 / 3.0 + 0.5)).abs() < 1e-9);
    (g.x0, g.x1) = (1.0, 2.0);
    assert!(g.crossings(project(&h), graph).is_empty(), "outside the range");
}
