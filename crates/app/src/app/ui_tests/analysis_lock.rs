//! Graph analysis locks and Data > Lock Position (T186, graphing "Analysis markers").

use super::*;
use sheaf_core::analysis::{OutputTarget, Request, RowRange, Tool};
use sheaf_core::column::RecalcMode;
use sheaf_core::graph::LockPosition;

#[test]
fn lock_position_places_the_lock_of_a_linked_analysis() {
    let mut h = start(vec![data_file("analysis-lock")]);
    let g = graph(&h).id;
    let y = graph(&h).layers[0].plots[0].y;
    let places = |h: &Harness<'static, SheafApp>| {
        let p = project(h);
        let geometry = sheaf_plot::layout_geometry(p, g).unwrap();
        let to_screen = |(x, y): (f64, f64)| egui::pos2(x as f32, y as f32);
        crate::graph_view::analysis_lock_places(p, g, &geometry, &to_screen, egui::Rect::ZERO)
    };
    assert!(places(&h).is_empty(), "no lock without a linked analysis");
    let o = sheaf_calc::SmoothOptions {
        method: sheaf_calc::SmoothMethod::AdjacentAveraging { weighted: false },
        npts: 3,
        boundary: sheaf_calc::Boundary::None,
    };
    h.state_mut().doc.apply("Smooth", |p| {
        let request = Request { columns: vec![y], rows: RowRange::All, tool: Tool::Smooth(o) };
        p.analyze_linked(&request, &OutputTarget::NextToInput, RecalcMode::Manual).unwrap();
    });
    settle(&mut h);
    let at_icon = places(&h);
    assert_eq!(at_icon.len(), 1);
    assert_eq!(at_icon[0].1, egui::pos2(2.0 + 15.0, 4.0), "beside the first layer icon");
    assert!(!at_icon[0].2.pending);

    menu_path(&mut h, &["Data", "Lock Position", "Last Point"]);
    assert_eq!(graph(&h).page.style.lock_position, LockPosition::LastPoint);
    assert_eq!(h.state().doc.undo_label(), Some("Lock position"));
    let last = places(&h)[0].1;
    menu_path(&mut h, &["Data", "Lock Position", "First Point"]);
    let first = places(&h)[0].1;
    assert!(last.x > first.x + 10.0, "the last point is right of the first: {first:?} {last:?}");
    menu(&mut h, "Edit", "Undo Lock position");
    menu(&mut h, "Edit", "Undo Lock position");
    assert_eq!(graph(&h).page.style.lock_position, LockPosition::LayerIcon);
}
