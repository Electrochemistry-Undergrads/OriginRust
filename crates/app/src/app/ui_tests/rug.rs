use super::*;

#[test]
fn the_symbol_menu_makes_a_scatter_rug_graph_with_rugs_on_the_bottom_and_left_axes() {
    let mut h = start(vec![data_file("rug-menu")]);
    show_window(&mut h, "Book1 - sample");
    let before = project(&h).graphs().count();
    click_text(&mut h, "C(Y1)");
    menu_path(&mut h, &["Plot", "Symbol", "Scatter + Rug"]);
    let graphs: Vec<_> = project(&h).graphs().collect();
    assert_eq!(graphs.len(), before + 1);
    let l = &graphs[before].layers[0];
    assert!(l.x_axis.rug.show && l.y_axis.rug.show && l.x_axis.rug.overlap);
    assert!(!l.x_axis.rug_opposite.show && !l.y_axis.rug_opposite.show);
}

#[test]
fn the_rug_tab_edits_the_rug_of_each_edge_and_the_change_is_one_undo_step() {
    let mut h = start(vec![data_file("rug-tab")]);
    open_props(&mut h, 0, PropsTab::XAxis, None);
    assert!(h.query_by_label("Rug").is_some(), "the Rug tab");
    click(&mut h, "Rug");
    // The four edges are listed; the rug of the bottom edge is the first one of the X axis.
    for edge in ["Bottom", "Top", "Left", "Right"] {
        assert!(h.query_by_label(edge).is_some(), "{edge}");
    }
    assert!(!graph(&h).layers[0].x_axis.rug.show);
    h.get_by_label("Show").click();
    settle(&mut h);
    assert!(graph(&h).layers[0].x_axis.rug.show);
    for label in
        ["Data", "Color", "Thickness", "Transparency (%)", "Size (%)", "Overlap", "Gap Between Rugs (%)", "Margin (%)"]
    {
        assert!(has_label(&h, label), "{label}");
    }
    h.get_by_label("Show Parallel Border").click();
    h.get_by_label("Show Perpendicular Border").click();
    settle(&mut h);
    let r = &graph(&h).layers[0].x_axis.rug;
    assert!(r.parallel_border && r.perpendicular_border);
    // The top edge has its own rug; the left edge belongs to the Y axis.
    click(&mut h, "Top");
    assert!(!graph(&h).layers[0].x_axis.rug_opposite.show);
    h.get_by_label("Show").click();
    settle(&mut h);
    assert!(graph(&h).layers[0].x_axis.rug_opposite.show);
    click(&mut h, "Left");
    assert!(!graph(&h).layers[0].y_axis.rug.show);
    h.get_by_label("Show").click();
    settle(&mut h);
    assert!(graph(&h).layers[0].y_axis.rug.show);
    click(&mut h, "OK");
    assert_eq!(h.state().doc.undo_label(), Some("Change graph settings"));
    menu(&mut h, "Edit", "Undo Change graph settings");
    let l = &graph(&h).layers[0];
    assert!(l.x_axis.rug.is_default() && l.x_axis.rug_opposite.is_default() && l.y_axis.rug.is_default());
}
