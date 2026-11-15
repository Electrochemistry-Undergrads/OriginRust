use sheaf_core::graph::{PlotDecor, SymbolLineInterface};
use sheaf_core::template::Template;
use sheaf_core::{ColumnData, Designation, Project};

#[test]
fn symbol_line_gaps_load_unchecked_and_follow_dependent_group_edits() {
    let old: PlotDecor = serde_json::from_str("{}").unwrap();
    assert_eq!(old.symbol_line, SymbolLineInterface::default());
    assert!(!old.symbol_line.draw_in_front, "old projects paint the line behind symbols");
    assert!(!serde_json::to_string(&old).unwrap().contains("symbol_line"));
    let enabled = SymbolLineInterface { gap_to_symbol: true, no_break_skipped: true, draw_in_front: true };
    let decor = PlotDecor { symbol_line: enabled, ..old };
    assert_eq!(serde_json::from_str::<PlotDecor>(&serde_json::to_string(&decor).unwrap()).unwrap(), decor);

    let mut p = Project::new();
    let b = p.add_workbook("Data");
    let y = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![1.0, 2.0])).unwrap();
    let g = p.add_graph(Template::LineSymbol);
    p.add_plot(g, 0, y).unwrap();
    p.add_plot(g, 0, y).unwrap();
    let layer = &mut p.graph_mut(g).unwrap().layers[0];
    layer.plots[1].decor.symbol_line = enabled;
    layer.follow_leader(1);
    assert!(layer.plots.iter().all(|p| p.decor.symbol_line == enabled));
}
