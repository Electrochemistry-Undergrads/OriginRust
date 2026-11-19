//! Gadgets > Differentiate and Interpolate (T145, graphing "Differentiate and Interpolate gadgets").

use super::*;
use crate::gadget::GadgetKind;

#[test]
fn differentiate_and_interpolate_work_on_the_rows_inside_the_range() {
    // The sample's "up" column is the line y = 2t + 0.5 at t = 1..4.
    let mut h = start(vec![data_file("diff-interp-gadgets")]);
    let ctx = h.ctx.clone();
    let graph = project(&h).graphs().next().unwrap().id;
    h.state_mut().apply(Action::StartDifferentiate { graph }, &ctx);
    settle(&mut h);
    let mut g = h.state().graphs[&graph].gadget.expect("the gadget is open");
    assert_eq!(g.kind, GadgetKind::Differentiate);
    (g.x0, g.x1) = (0.5, 4.5);
    let (x, y) = crate::gadget::plot_xy(project(&h), graph, 0, 0).unwrap();
    let d = g.derivative(&x, &y);
    assert_eq!(d.len(), 4);
    assert!(d.iter().all(|p| (p.1 - 2.0).abs() < 1e-9), "{d:?}");
    h.state_mut().apply(Action::StartInterpolate { graph }, &ctx);
    settle(&mut h);
    let mut i = h.state().graphs[&graph].gadget.unwrap();
    assert_eq!(i.kind, GadgetKind::Interpolate);
    (i.x0, i.x1) = (0.5, 4.5);
    let curve = i.interpolated(&x, &y);
    assert_eq!(curve.len(), 201);
    assert!(curve.iter().all(|(u, v)| (v - (2.0 * u + 0.5)).abs() < 1e-9), "a line stays a line");
    (i.x0, i.x1) = (0.5, 2.5);
    assert!(i.interpolated(&x, &y).is_empty(), "two points are too few");
}
