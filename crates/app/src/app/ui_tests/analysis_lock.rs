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
    assert_eq!(at_icon[0].1, egui::pos2(2.0 + 7.5, 2.0 + 15.0 + 9.0), "below the first layer icon");
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

#[test]
fn a_click_on_the_lock_recalculates_or_goes_to_the_results() {
    // T195: the lock's menu.
    let mut h = start(vec![data_file("analysis-lock-menu")]);
    let g = graph(&h).id;
    let y = graph(&h).layers[0].plots[0].y;
    let o = sheaf_calc::SmoothOptions {
        method: sheaf_calc::SmoothMethod::AdjacentAveraging { weighted: false },
        npts: 3,
        boundary: sheaf_calc::Boundary::None,
    };
    let out = h.state_mut().doc.apply("Smooth", |p| {
        let request = Request { columns: vec![y], rows: RowRange::All, tool: Tool::Smooth(o) };
        p.analyze_linked(&request, &OutputTarget::NextToInput, RecalcMode::Manual).unwrap().new_columns[0]
    });
    // An input changes: the result waits (yellow lock).
    h.state_mut().doc.apply("Edit", |p| {
        let before = p.clone();
        p.column_mut(y).unwrap().data = sheaf_core::ColumnData::numeric(vec![1.0, 2.0, 3.0, 4.0]);
        p.update_analysis_links(&before);
    });
    settle(&mut h);
    let layer = project(&h).graph(g).unwrap().layers[0].clone();
    assert!(project(&h).layer_analysis_lock(&layer).unwrap().pending);
    click(&mut h, "Analysis lock");
    click(&mut h, "Recalculate");
    settle(&mut h);
    let layer = project(&h).graph(g).unwrap().layers[0].clone();
    assert!(!project(&h).layer_analysis_lock(&layer).unwrap().pending, "green again");
    assert_eq!(project(&h).column(out).unwrap().analysis.unwrap().state, sheaf_core::column::FormulaState::Current);
    assert_eq!(h.state().doc.undo_label(), Some("Recalculate"));
    // Go to Results shows the result's workbook.
    let (book, _, _) = project(&h).locate(out).unwrap();
    let book = book.id;
    click(&mut h, "Analysis lock");
    click(&mut h, "Go to Results");
    settle(&mut h);
    assert_eq!(h.state().active, Some(book));
}
