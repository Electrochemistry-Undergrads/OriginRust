use super::*;
use sheaf_core::graph::{PlotKind, TreemapLayout, TreemapOrder};

/// Continent, country and population: three levels of columns, two of them text.
fn treemap_file(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sheaf-app-ui-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("sample.csv");
    std::fs::write(
        &path,
        "continent,country,population\nAsia,China,1400\nAsia,India,1380\nEurope,France,68\nEurope,Germany,83\nAfrica,Egypt,100\n",
    )
    .unwrap();
    path
}

fn make_treemap(h: &mut Harness<'static, SheafApp>) -> sheaf_core::WindowId {
    show_window(h, "Book1 - sample.csv");
    let before = project(h).graphs().count();
    menu_path(h, &["Plot"]);
    not_toolbar(h, "Categorical").into_iter().next().expect("the Categorical submenu").click();
    settle(h);
    menu_path(h, &["Treemap"]);
    project(h).graphs().nth(before).expect("a new graph").id
}

#[test]
fn the_categorical_submenu_makes_a_treemap_of_the_text_columns_and_the_value_column() {
    let mut h = start(vec![treemap_file("treemap-menu")]);
    let id = make_treemap(&mut h);
    let layer = &project(&h).graph(id).unwrap().layers[0];
    assert_eq!(layer.default_kind, PlotKind::Treemap);
    assert_eq!(layer.plots.len(), 1);
    assert_eq!(layer.plots[0].y, column_id(&h, "C"));
    assert_eq!(layer.plots[0].treemap.categories, vec![column_id(&h, "A"), column_id(&h, "B")]);
}

#[test]
fn plot_details_has_a_layout_tab_with_undo() {
    let mut h = start(vec![treemap_file("treemap-details")]);
    let id = make_treemap(&mut h);
    let style = |h: &Harness<'static, SheafApp>| project(h).graph(id).unwrap().layers[0].plots[0].treemap.clone();
    let before = style(&h);
    assert_eq!(
        (before.layout, before.order, before.depth, before.gap_pt),
        (TreemapLayout::Squarified, TreemapOrder::Categorical, None, 1.5)
    );
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::GraphProperties { graph: id, layer: 0, tab: PropsTab::Plots, plot: Some(0) }, &ctx);
    settle(&mut h);
    // "Layout" is also the name of the layer node in the tree: the tab is the last one.
    h.query_all_by_label("Layout").last().expect("the Layout tab").click();
    settle(&mut h);
    for label in ["Order", "Depth", "Gap", "Different Layout and Order for Each Level Below the First"] {
        assert!(has_label(&h, label), "{label}");
    }
    // The lists show their choice as their value; the Order is greyed out for a Squarified layout.
    assert!(h.query_by_value("Infinite").is_some());
    assert!(h.get_by_value("Categorical Order").accesskit_node().is_disabled());
    h.get_by_value("Squarified").click();
    settle(&mut h);
    click(&mut h, "Pivot by Size");
    assert!(!h.get_by_value("Categorical Order").accesskit_node().is_disabled());
    click(&mut h, "Different Layout and Order for Each Level Below the First");
    assert!(has_label(&h, "Level 2"));
    click(&mut h, "OK");
    let now = style(&h);
    assert_eq!(now.layout, TreemapLayout::PivotSize);
    assert!(now.per_level_on && now.per_level.len() == 1);
    menu(&mut h, "Edit", "Undo Change graph settings");
    assert_eq!(style(&h), before);
}
