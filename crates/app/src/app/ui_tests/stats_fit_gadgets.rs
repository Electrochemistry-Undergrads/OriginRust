//! Gadgets > Statistics and Quick Fit (T143, graphing "Statistics and Quick Fit gadgets").

use super::*;
use crate::gadget::GadgetKind;

#[test]
fn statistics_and_quick_fit_work_on_the_rows_inside_the_range() {
    // The sample's "up" column: 2.5, 4.5, 6.5, 8.5 at t = 1..4 (y = 2t + 0.5).
    let mut h = start(vec![data_file("stats-fit-gadgets")]);
    let ctx = h.ctx.clone();
    let graph = project(&h).graphs().next().unwrap().id;
    h.state_mut().apply(Action::StartStatisticsGadget { graph }, &ctx);
    settle(&mut h);
    let mut g = h.state().graphs[&graph].gadget.expect("the gadget is open");
    assert_eq!(g.kind, GadgetKind::Statistics);
    (g.x0, g.x1) = (0.5, 3.5);
    let (x, y) = crate::gadget::plot_xy(project(&h), graph, 0, 0).unwrap();
    let s = g.stats(&x, &y).unwrap();
    assert_eq!((s.n, s.mean, s.min, s.max), (3, 4.5, 2.5, 6.5));
    assert!((s.sd - 2.0).abs() < 1e-12);
    h.state_mut().apply(Action::StartQuickFit { graph }, &ctx);
    settle(&mut h);
    let mut f = h.state().graphs[&graph].gadget.unwrap();
    assert_eq!(f.kind, GadgetKind::QuickFit);
    (f.x0, f.x1) = (0.5, 4.5);
    let l = f.line_fit(&x, &y).unwrap();
    assert!((l.slope - 2.0).abs() < 1e-12 && (l.intercept - 0.5).abs() < 1e-12 && (l.r2 - 1.0).abs() < 1e-12);
    (f.x0, f.x1) = (0.5, 1.5);
    assert!(f.line_fit(&x, &y).is_none(), "one point");
}
