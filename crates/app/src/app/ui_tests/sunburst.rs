use super::*;
use sheaf_core::graph::{PlotKind, SunburstOrder};

/// Continent, country and population: two text columns and a value.
fn sunburst_file(tag: &str) -> PathBuf {
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

fn make_sunburst(h: &mut Harness<'static, SheafApp>) -> sheaf_core::WindowId {
    show_window(h, "Book1 - sample.csv");
    let before = project(h).graphs().count();
    menu_path(h, &["Plot"]);
    not_toolbar(h, "Categorical").into_iter().next().expect("the Categorical submenu").click();
    settle(h);
    menu_path(h, &["Sunburst Plot"]);
    project(h).graphs().nth(before).expect("a new graph").id
}

#[test]
fn the_categorical_submenu_makes_a_sunburst_of_the_text_columns_and_the_value_column() {
    let mut h = start(vec![sunburst_file("sunburst-menu")]);
    let id = make_sunburst(&mut h);
    let layer = &project(&h).graph(id).unwrap().layers[0];
    assert_eq!(layer.default_kind, PlotKind::Sunburst);
    assert_eq!(layer.plots.len(), 1);
    assert_eq!(layer.plots[0].y, column_id(&h, "C"));
    assert_eq!(layer.plots[0].sunburst.categories, vec![column_id(&h, "A"), column_id(&h, "B")]);
}

#[test]
fn plot_details_has_a_pie_geometry_tab_with_labels_and_undo() {
    let mut h = start(vec![sunburst_file("sunburst-details")]);
    let id = make_sunburst(&mut h);
    let style = |h: &Harness<'static, SheafApp>| project(h).graph(id).unwrap().layers[0].plots[0].sunburst.clone();
    let before = style(&h);
    assert_eq!(
        (before.start_angle_deg, before.hole, before.ring_gap_pt, before.show_names, before.show_values, before.order),
        (90.0, 0.3, 0.0, true, false, SunburstOrder::Worksheet)
    );
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::GraphProperties { graph: id, layer: 0, tab: PropsTab::Plots, plot: Some(0) }, &ctx);
    settle(&mut h);
    h.query_all_by_label("Pie Geometry").last().expect("the Pie Geometry tab").click();
    settle(&mut h);
    for label in ["Start Angle", "Hole", "Ring Gap", "Names", "Values"] {
        assert!(has_label(&h, label), "{label}");
    }
    click(&mut h, "Values");
    click(&mut h, "OK");
    let now = style(&h);
    assert!(now.show_values && now.show_names);
    menu(&mut h, "Edit", "Undo Change graph settings");
    assert_eq!(style(&h), before);
}
