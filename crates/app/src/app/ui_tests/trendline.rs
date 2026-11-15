use super::*;
use sheaf_core::graph::{TrendKind, Trendline};

fn trends(h: &Harness<'static, SheafApp>, plot: usize) -> Vec<Trendline> {
    graph(h).layers[0].plots[plot].decor.trendlines.clone()
}

fn open_menu(h: &mut Harness<'static, SheafApp>, plot: usize) {
    let at = on(h, 0, ObjectKind::Plot(plot));
    click_at(h, at);
    right_click_at(h, at);
}

#[test]
fn trendlines_are_added_edited_and_removed_from_the_plot_menu_one_undo_step_each() {
    let mut h = start(vec![data_file("trend")]);
    open_menu(&mut h, 0);
    assert!(h.query_by_label("Add Trendline...").is_some());
    assert!(h.query_by_label("Edit Trendline...").is_none(), "nothing to edit yet");
    assert!(h.query_by_label("Remove Trendline").is_none());
    click(&mut h, "Add Trendline...");
    assert!(h.state().trendline.is_some());
    // Cancel changes nothing.
    let revision = h.state().doc.revision();
    click(&mut h, "Cancel");
    assert!(h.state().trendline.is_none());
    assert_eq!(h.state().doc.revision(), revision);

    open_menu(&mut h, 0);
    click(&mut h, "Add Trendline...");
    click(&mut h, "Type");
    click(&mut h, "Polynomial");
    click(&mut h, "Show R\u{b2}");
    click(&mut h, "OK");
    assert!(h.state().trendline.is_none());
    let t = trends(&h, 0);
    assert_eq!(t.len(), 1);
    assert_eq!(t[0].kind, TrendKind::Polynomial { order: 2 });
    assert!(t[0].show_equation && !t[0].show_r2);
    assert!(t[0].color.is_none(), "Auto colour");
    assert!(trends(&h, 1).is_empty(), "only the clicked plot");
    assert_eq!(h.state().doc.undo_label(), Some("Add trendline"));

    // Edit loads the trendline and replaces it.
    open_menu(&mut h, 0);
    click(&mut h, "Edit Trendline...");
    assert!(h.query_by_label("Order").is_some(), "the loaded polynomial shows its order");
    click(&mut h, "Type");
    click(&mut h, "Moving Average");
    click(&mut h, "OK");
    assert_eq!(trends(&h, 0).len(), 1);
    assert_eq!(trends(&h, 0)[0].kind, TrendKind::MovingAverage { window: 2 });
    assert_eq!(h.state().doc.undo_label(), Some("Edit trendline"));
    menu(&mut h, "Edit", "Undo Edit trendline");
    assert_eq!(trends(&h, 0)[0].kind, TrendKind::Polynomial { order: 2 });
    menu(&mut h, "Edit", "Redo Edit trendline");
    assert_eq!(trends(&h, 0)[0].kind, TrendKind::MovingAverage { window: 2 });

    // A second trendline; Remove takes off the last one.
    open_menu(&mut h, 0);
    click(&mut h, "Add Trendline...");
    click(&mut h, "OK");
    assert_eq!(trends(&h, 0).len(), 2);
    assert_eq!(trends(&h, 0)[1].kind, TrendKind::Linear);
    open_menu(&mut h, 0);
    click(&mut h, "Remove Trendline");
    assert_eq!(trends(&h, 0).len(), 1);
    assert_eq!(trends(&h, 0)[0].kind, TrendKind::MovingAverage { window: 2 });
    assert_eq!(h.state().doc.undo_label(), Some("Remove trendline"));
    menu(&mut h, "Edit", "Undo Remove trendline");
    assert_eq!(trends(&h, 0).len(), 2);
    menu(&mut h, "Edit", "Redo Remove trendline");
    assert_eq!(trends(&h, 0).len(), 1);
    menu(&mut h, "Edit", "Undo Remove trendline");
    menu(&mut h, "Edit", "Undo Add trendline");
    menu(&mut h, "Edit", "Undo Edit trendline");
    menu(&mut h, "Edit", "Undo Add trendline");
    assert!(trends(&h, 0).is_empty());
}
