use super::*;
use sheaf_core::graph::{DropColorMode, DropTarget};

/// Whether a widget or text shows this text, as a label or as a value (a combo box's choice).
fn shown(h: &Harness<'static, SheafApp>, text: &str) -> bool {
    has_label(h, text) || h.query_all(By::new().value(text)).next().is_some()
}

#[test]
fn the_symbol_menu_has_a_live_lollipop_item_that_needs_two_y_columns() {
    let mut h = start(vec![data_file("lollipop-menu")]);
    show_window(&mut h, "Book1 - sample");
    let before = project(&h).graphs().count();
    // One Y column: refused with a message, no graph.
    click_text(&mut h, "C(Y1)");
    menu_path(&mut h, &["Plot", "Symbol", "Lollipop Plot"]);
    assert_eq!(project(&h).graphs().count(), before);
    let (title, text) = h.state().message.clone().expect("a message explains the refusal");
    assert!(title.contains("Could not make the graph") && text.contains("two or more Y columns"), "{text}");
    // Two Y columns: a lollipop graph.
    h.state_mut().message = None;
    select_b_and_c(&mut h);
    menu_path(&mut h, &["Plot", "Symbol", "Lollipop Plot"]);
    let graphs: Vec<_> = project(&h).graphs().collect();
    assert_eq!(graphs.len(), before + 1);
    let plots = &graphs[before].layers[0].plots;
    assert_eq!(plots.len(), 2);
    assert!(plots.iter().all(|p| p.decor.drop.vertical.on
        && p.decor.drop.vertical.target == DropTarget::NextDataPlot
        && p.decor.drop.vertical.color_mode == DropColorMode::Auto));
}

#[test]
fn the_drop_lines_tab_has_drop_to_and_offers_plus_minus_only_for_the_next_data_plot() {
    let mut h = start(vec![data_file("lollipop-drop")]);
    open_props(&mut h, 0, PropsTab::Plots, Some(0));
    click(&mut h, "Drop Lines");
    assert!(has_label(&h, "Drop To"));
    assert!(!shown(&h, "Y Value: Plus-Minus"), "plus-minus is for lollipop plots only");
    click(&mut h, "Cancel");
    // A lollipop-style drop line: the target and the colour mode are shown, and applied with OK.
    edit_graph(&mut h, |g| {
        let d = &mut g.layers[0].plots[0].decor.drop.vertical;
        (d.on, d.target) = (true, DropTarget::NextDataPlot);
    });
    open_props(&mut h, 0, PropsTab::Plots, Some(0));
    click(&mut h, "Drop Lines");
    assert!(shown(&h, "Next Data Plot"), "the Drop To box shows the target");
    // The colour box of the vertical drop line (the only combo box that reads Auto).
    h.get(By::new().role(egui::accesskit::Role::ComboBox).value("Auto")).click();
    settle(&mut h);
    click(&mut h, "Y Value: Plus-Minus");
    click(&mut h, "Apply");
    assert_eq!(graph(&h).layers[0].plots[0].decor.drop.vertical.color_mode, DropColorMode::PlusMinus);
    click(&mut h, "Cancel");
}
