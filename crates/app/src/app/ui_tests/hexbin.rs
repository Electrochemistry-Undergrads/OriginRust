use super::*;
use sheaf_core::graph::{HexbinBy, HexbinStyle, PlotKind};

fn make_hexbin(h: &mut Harness<'static, SheafApp>) -> sheaf_core::WindowId {
    show_window(h, "Book1 - sample.csv");
    let before = project(h).graphs().count();
    select_b_and_c(h);
    menu_path(h, &["Plot"]);
    not_toolbar(h, "Contour/Heat Map").into_iter().next().expect("the Contour/Heat Map submenu").click();
    settle(h);
    menu_path(h, &["Hexbin"]);
    let graphs: Vec<_> = project(h).graphs().collect();
    assert_eq!(graphs.len(), before + 1);
    assert_eq!(graphs[before].layers[0].default_kind, PlotKind::Hexbin);
    assert_eq!(graphs[before].layers[0].plots.len(), 1, "one hexbin plot per layer");
    graphs[before].id
}

#[test]
fn the_contour_heat_map_menu_has_a_live_hexbin_item_and_the_rest_stay_grey() {
    let mut h = start(vec![data_file("hexbin-menu")]);
    make_hexbin(&mut h);
    // The other items of the submenu are still there, greyed.
    let mut h = start(vec![data_file("hexbin-menu-grey")]);
    show_window(&mut h, "Book1 - sample.csv");
    select_b_and_c(&mut h);
    menu_path(&mut h, &["Plot"]);
    not_toolbar(&h, "Contour/Heat Map").into_iter().next().unwrap().click();
    settle(&mut h);
    assert!(has_label(&h, "Color Fill") && has_label(&h, "Heat Map") && has_label(&h, "Hexbin"));
}

#[test]
fn plot_details_has_the_hexbin_bin_controls_with_undo() {
    let mut h = start(vec![data_file("hexbin-details")]);
    let id = make_hexbin(&mut h);
    let style = |h: &Harness<'static, SheafApp>| project(h).graph(id).unwrap().layers[0].plots[0].hexbin;
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::GraphProperties { graph: id, layer: 0, tab: PropsTab::Plots, plot: Some(0) }, &ctx);
    settle(&mut h);
    click(&mut h, "Bins");
    assert!(has_label(&h, "Prefer Regular Hexagons") && has_label(&h, "Derive Y from X"));
    assert!(has_label(&h, "Min Center") && has_label(&h, "Number of Bins"));
    click(&mut h, "Prefer Regular Hexagons");
    click(&mut h, "Derive X from Y");
    click(&mut h, "OK");
    let s = style(&h);
    assert!(!s.prefer_regular && !s.derive_y_from_x);
    assert_eq!(s.x.by, HexbinBy::Bins(20));
    menu(&mut h, "Edit", "Undo Change graph settings");
    assert_eq!(style(&h), HexbinStyle::default());
}

#[test]
fn the_derived_axis_keeps_its_min_and_max_center_editable() {
    let mut h = start(vec![data_file("hexbin-derived")]);
    let id = make_hexbin(&mut h);
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::GraphProperties { graph: id, layer: 0, tab: PropsTab::Plots, plot: Some(0) }, &ctx);
    settle(&mut h);
    click(&mut h, "Bins");
    // Derive Y from X (the default): the four Auto boxes (Min and Max Center of X and of Y) are all live.
    let autos: Vec<bool> = h.query_all_by_label("Auto").map(|n| n.accesskit_node().is_disabled()).collect();
    assert_eq!(autos, vec![false; 4]);
    // The binning of X is live, that of Y (derived) is not: exactly one of the two "Number of Bins" combo
    // boxes is disabled.
    let combos: Vec<bool> = h
        .query_all_by_role(egui::accesskit::Role::ComboBox)
        .filter(|n| n.accesskit_node().value().is_some_and(|v| v == "Number of Bins"))
        .map(|n| n.accesskit_node().is_disabled())
        .collect();
    assert_eq!(combos, vec![false, true], "X binning live, Y binning disabled");
    click(&mut h, "Derive X from Y");
    let autos: Vec<bool> = h.query_all_by_label("Auto").map(|n| n.accesskit_node().is_disabled()).collect();
    assert_eq!(autos, vec![false; 4]);
    let combos: Vec<bool> = h
        .query_all_by_role(egui::accesskit::Role::ComboBox)
        .filter(|n| n.accesskit_node().value().is_some_and(|v| v == "Number of Bins"))
        .map(|n| n.accesskit_node().is_disabled())
        .collect();
    assert_eq!(combos, vec![true, false]);
}
