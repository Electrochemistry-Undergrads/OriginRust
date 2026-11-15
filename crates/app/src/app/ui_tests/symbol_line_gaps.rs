use super::*;

#[test]
fn symbol_line_gaps_use_dependent_controls_and_support_cancel_apply_and_history() {
    let mut h = start(vec![data_file("symbol-line-gaps")]);
    open_props(&mut h, 0, PropsTab::Plots, Some(0));
    assert!(!has_label(&h, "Gap to Symbol"), "plain lines have no symbol interface");
    click(&mut h, "Cancel");
    edit_graph(&mut h, |g| {
        for plot in &mut g.layers[0].plots {
            plot.kind = PlotKind::LineSymbol;
        }
    });
    open_props(&mut h, 0, PropsTab::Plots, Some(0));
    click(&mut h, "Drop Lines");
    assert!(is_disabled(&h, "No Line Break for Skipped Point(s)"));
    click(&mut h, "Skip Points");
    assert!(is_disabled(&h, "No Line Break for Skipped Point(s)"));
    click(&mut h, "Line");
    click(&mut h, "Gap to Symbol");
    click(&mut h, "Drop Lines");
    assert!(!is_disabled(&h, "No Line Break for Skipped Point(s)"));
    click(&mut h, "No Line Break for Skipped Point(s)");
    assert!(
        graph(&h).layers[0]
            .plots
            .iter()
            .all(|p| p.decor.symbol_line.gap_to_symbol && p.decor.symbol_line.no_break_skipped)
    );
    click(&mut h, "Cancel");
    assert!(!graph(&h).layers[0].plots[0].decor.symbol_line.gap_to_symbol);
    open_props(&mut h, 0, PropsTab::Plots, Some(0));
    click(&mut h, "Line");
    click(&mut h, "Gap to Symbol");
    click(&mut h, "Apply");
    click(&mut h, "Gap to Symbol");
    click(&mut h, "Cancel");
    assert!(graph(&h).layers[0].plots[0].decor.symbol_line.gap_to_symbol);
    menu(&mut h, "Edit", "Undo Change graph settings");
    assert!(!graph(&h).layers[0].plots[0].decor.symbol_line.gap_to_symbol);
    menu(&mut h, "Edit", "Redo Change graph settings");
    assert!(graph(&h).layers[0].plots[0].decor.symbol_line.gap_to_symbol);
}
