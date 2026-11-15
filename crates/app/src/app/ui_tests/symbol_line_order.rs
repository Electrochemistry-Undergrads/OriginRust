use super::*;

#[test]
fn symbol_line_order_obeys_gap_dependencies_and_group_apply_cancel_and_history() {
    let mut h = start(vec![data_file("symbol-line-order")]);
    edit_graph(&mut h, |g| {
        for p in &mut g.layers[0].plots {
            p.kind = PlotKind::LineSymbol;
        }
    });
    let open = |h: &mut Harness<'static, SheafApp>| {
        open_props(h, 0, PropsTab::Plots, Some(0));
        click(h, "Line");
    };
    open(&mut h);
    assert!(!is_disabled(&h, "Draw Line in Front"));
    click(&mut h, "Draw Line in Front");
    assert!(graph(&h).layers[0].plots.iter().all(|p| p.decor.symbol_line.draw_in_front));
    click(&mut h, "Gap to Symbol");
    assert!(is_disabled(&h, "Draw Line in Front") && is_disabled(&h, "Draw Line Behind"));
    assert!(graph(&h).layers[0].plots[0].decor.symbol_line.draw_in_front);
    click(&mut h, "Gap to Symbol");
    assert!(!is_disabled(&h, "Draw Line Behind"));
    assert!(graph(&h).layers[0].plots[0].decor.symbol_line.draw_in_front);
    click(&mut h, "Cancel");
    assert!(!graph(&h).layers[0].plots[0].decor.symbol_line.draw_in_front);
    open(&mut h);
    click(&mut h, "Draw Line in Front");
    click(&mut h, "Apply");
    click(&mut h, "Draw Line Behind");
    click(&mut h, "Cancel");
    assert!(graph(&h).layers[0].plots[0].decor.symbol_line.draw_in_front);
    menu(&mut h, "Edit", "Undo Change graph settings");
    assert!(!graph(&h).layers[0].plots[0].decor.symbol_line.draw_in_front);
    menu(&mut h, "Edit", "Redo Change graph settings");
    assert!(graph(&h).layers[0].plots[0].decor.symbol_line.draw_in_front);
}
