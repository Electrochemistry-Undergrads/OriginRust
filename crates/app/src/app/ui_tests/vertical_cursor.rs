//! Gadgets > Vertical Cursor (T144, graphing "Vertical Cursor gadget").

use super::*;
use crate::gadget::GadgetKind;

#[test]
fn the_vertical_cursor_reads_every_plot_at_its_x() {
    // The sample: up = 2t + 0.5 and down = -10t + 40.5 at t = 1..4.
    let mut h = start(vec![data_file("vertical-cursor")]);
    let ctx = h.ctx.clone();
    let graph = project(&h).graphs().next().unwrap().id;
    h.state_mut().apply(Action::StartVerticalCursor { graph }, &ctx);
    settle(&mut h);
    let mut g = h.state().graphs[&graph].gadget.expect("the gadget is open");
    assert_eq!(g.kind, GadgetKind::VerticalCursor);
    assert!((g.x1 - g.x0).abs() < 0.1 * (g.x0.abs() + g.x1.abs()), "a thin band");
    (g.x0, g.x1) = (2.45, 2.55);
    let values = g.cursor_values(project(&h), graph);
    assert_eq!(values.len(), 2, "{values:?}");
    assert!((values[0].1 - 5.5).abs() < 1e-9 && (values[1].1 - 15.5).abs() < 1e-9, "{values:?}");
    (g.x0, g.x1) = (9.0, 9.1);
    assert!(g.cursor_values(project(&h), graph).is_empty(), "beyond the data");
    assert_eq!(crate::gadget::y_at(&[0.0, 2.0], &[0.0, 4.0], 0.5), Some(1.0));
}
