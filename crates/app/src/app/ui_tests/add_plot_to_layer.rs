use super::*;
use sheaf_core::graph::PlotKind;

/// Graph > Add Plot to Layer > X... opens Layer Contents with X chosen on the plot type button, as in
/// Origin 2016 (T63 group J).
#[test]
fn each_item_opens_layer_contents_with_its_plot_type() {
    let cases = [
        ("Line...", PlotKind::Line),
        ("Scatter...", PlotKind::Scatter),
        ("Line + Symbol...", PlotKind::LineSymbol),
        ("Column...", PlotKind::Column),
        ("Area...", PlotKind::Area),
    ];
    let mut h = start(vec![data_file("add-plot-kinds")]);
    for (item, kind) in cases {
        menu_path(&mut h, &["Graph", "Add Plot to Layer", item]);
        let lc = h.state().layer_contents.as_ref().unwrap_or_else(|| panic!("{item} opens Layer Contents"));
        assert_eq!(lc.kind(), kind, "{item}");
        click(&mut h, "Cancel");
        assert!(h.state().layer_contents.is_none());
        pause(&mut h);
    }
}

#[test]
fn a_column_added_from_scatter_is_a_scatter_plot_and_cancel_takes_it_back() {
    let mut h = start(vec![data_file("add-plot-scatter")]);
    let (_, graph) = window_ids(&h);
    let before = project(&h).graph(graph).unwrap().layers[0].plots.clone();
    menu_path(&mut h, &["Graph", "Add Plot to Layer", "Scatter..."]);
    // take the last plot out (right list), then add its column back from the left list
    let y = before.last().unwrap().y;
    let short = format!("{}(Y)", project(&h).column(y).unwrap().short_name);
    let right =
        h.query_all_by_label(&short).max_by(|a, b| a.rect().min.x.total_cmp(&b.rect().min.x)).unwrap().rect().center();
    click_at(&mut h, right);
    click(&mut h, "<=");
    click(&mut h, &short);
    click(&mut h, "=>");
    let plots = project(&h).graph(graph).unwrap().layers[0].plots.clone();
    assert_eq!(plots.len(), before.len());
    assert_eq!((plots.last().unwrap().y, plots.last().unwrap().kind), (y, PlotKind::Scatter));
    assert_ne!(before.last().unwrap().kind, PlotKind::Scatter, "the sample's plots are lines");
    click(&mut h, "Cancel");
    assert_eq!(project(&h).graph(graph).unwrap().layers[0].plots, before, "Cancel takes it back");
}
