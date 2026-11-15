use super::*;
use sheaf_core::graph::{ParallelStyle, PlotKind};

/// Species and four numeric columns, three rows per species.
fn parallel_file(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sheaf-app-ui-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("sample.csv");
    let mut text = String::from("species,p1,p2,p3,p4\n");
    for (g, name) in ["setosa", "versicolor", "virginica"].iter().enumerate() {
        for i in 0..3 {
            text.push_str(&format!("{name},{},{},{},{}\n", g + i, 10 * g + i, 2 * i + g, 5 - g));
        }
    }
    std::fs::write(&path, text).unwrap();
    path
}

fn make_parallel(h: &mut Harness<'static, SheafApp>, item: &str) -> sheaf_core::WindowId {
    show_window(h, "Book1 - sample");
    let before = project(h).graphs().count();
    select_b_and_c(h);
    menu_path(h, &["Plot"]);
    not_toolbar(h, "Categorical").into_iter().next().expect("the Categorical submenu").click();
    settle(h);
    menu_path(h, &[item]);
    project(h).graphs().nth(before).expect("a new graph").id
}

#[test]
fn the_categorical_submenu_makes_a_parallel_plot_and_refuses_a_parallel_index_without_a_group() {
    let mut h = start(vec![data_file("parallel-menu")]);
    let id = make_parallel(&mut h, "Parallel Plot");
    let layer = &project(&h).graph(id).unwrap().layers[0];
    assert_eq!(layer.default_kind, PlotKind::Parallel);
    assert_eq!(layer.plots.len(), 2);
    // The colour column is the last selected one.
    assert_eq!(layer.plots[0].parallel.color, Some(layer.plots[1].y));
    // A parallel index of two columns (and no text column) is refused with a message, and makes no graph.
    let mut h = start(vec![data_file("parallel-menu-index")]);
    show_window(&mut h, "Book1 - sample");
    let before = project(&h).graphs().count();
    select_b_and_c(&mut h);
    menu_path(&mut h, &["Plot"]);
    not_toolbar(&h, "Categorical").into_iter().next().unwrap().click();
    settle(&mut h);
    menu_path(&mut h, &["Parallel Index"]);
    assert_eq!(project(&h).graphs().count(), before);
    assert!(h.state().message.as_ref().is_some_and(|m| m.1.contains("group column")), "{:?}", h.state().message);
}

#[test]
fn a_parallel_index_takes_its_group_from_the_text_column() {
    let mut h = start(vec![parallel_file("parallel-index")]);
    show_window(&mut h, "Book1 - sample");
    let cols: Vec<_> = ["B", "C", "D", "E"].iter().map(|n| column_id(&h, n)).collect();
    let book = project(&h).workbooks().next().unwrap().id;
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::PlotColumns { book, columns: cols, template: Template::ParallelIndex }, &ctx);
    settle(&mut h);
    let g = project(&h).graphs().last().unwrap();
    let layer = &g.layers[0];
    assert_eq!(layer.default_kind, PlotKind::ParallelIndex);
    assert_eq!(layer.plots.len(), 4);
    assert_eq!(layer.plots[0].parallel.color, Some(column_id(&h, "A")), "grouped by the species column");
}

#[test]
fn plot_details_has_a_parallel_tab_shared_by_the_plots_with_undo() {
    let mut h = start(vec![data_file("parallel-details")]);
    let id = make_parallel(&mut h, "Parallel Plot");
    let styles = |h: &Harness<'static, SheafApp>| -> Vec<ParallelStyle> {
        project(h).graph(id).unwrap().layers[0].plots.iter().map(|p| p.parallel).collect()
    };
    let before = styles(&h);
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::GraphProperties { graph: id, layer: 0, tab: PropsTab::Plots, plot: Some(0) }, &ctx);
    settle(&mut h);
    click(&mut h, "Parallel");
    assert!(has_label(&h, "Rescale Each Axis Independently"));
    assert!(has_label(&h, "Width") && has_label(&h, "Transparency (%)") && has_label(&h, "Color Column"));
    click(&mut h, "Rescale Each Axis Independently");
    click(&mut h, "OK");
    let now = styles(&h);
    assert!(now.iter().all(|s| !s.rescale_each_axis), "every parallel plot of the layer shares it: {now:?}");
    assert!(before.iter().all(|s| s.rescale_each_axis));
    menu(&mut h, "Edit", "Undo Change graph settings");
    assert_eq!(styles(&h), before);
}
