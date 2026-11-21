use super::*;
use sheaf_core::graph::{IndexedError, PlotKind};

/// Electrolyte (text), Scan (1 or 2, a number) and Current: three replicates per cell.
fn indexed_file(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sheaf-app-ui-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("sample.csv");
    let mut text = String::from("Electrolyte,Scan,Current\n");
    for (i, e) in ["KCl", "NaCl", "LiCl"].iter().enumerate() {
        for scan in [1, 2] {
            for r in 0..3 {
                text.push_str(&format!("{e},{scan},{}\n", 1.0 + i as f64 + scan as f64 * 0.5 + r as f64 * 0.1));
            }
        }
    }
    std::fs::write(&path, text).unwrap();
    path
}

fn plot(h: &mut Harness<'static, SheafApp>, names: &[&str], template: Template) {
    let cols: Vec<_> = names.iter().map(|n| column_id(h, n)).collect();
    let book = project(h).workbooks().next().unwrap().id;
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::PlotColumns { book, columns: cols, template }, &ctx);
    settle(h);
}

/// Scan is a number: it is a group column once it is not designated Y.
fn scan_is_a_group(h: &mut Harness<'static, SheafApp>) {
    let scan = column_id(h, "B");
    h.state_mut().doc.apply("Set X", |p| p.column_mut(scan).unwrap().designation = Designation::X);
}

#[test]
fn both_menu_items_are_live_and_plot_the_selected_group_and_data_columns() {
    let mut h = start(vec![indexed_file("grouped-menu")]);
    show_window(&mut h, "Book1 - sample.csv");
    scan_is_a_group(&mut h);
    menu_path(&mut h, &["Plot", "Column/Bar/Pie"]);
    assert!(entry_enabled(&h, "Grouped Columns (indexed data)"));
    close_menus(&mut h);
    menu_path(&mut h, &["Plot"]);
    not_toolbar(&h, "Statistics")
        .into_iter()
        .min_by(|a, b| a.rect().min.x.total_cmp(&b.rect().min.x))
        .expect("the Statistics submenu")
        .click();
    settle(&mut h);
    assert!(entry_enabled(&h, "Grouped Box Charts"));
    assert!(entry_enabled(&h, "Interval Plot"));
    assert!(entry_enabled(&h, "Pareto"));
    close_menus(&mut h);

    plot(&mut h, &["Electrolyte", "Current"], Template::GroupedColumns);
    let g = project(&h).graphs().last().unwrap();
    assert_eq!(g.layers[0].default_kind, PlotKind::Column);
    assert_eq!(g.layers[0].plots.len(), 1);
    let ix = g.layers[0].plots[0].indexed.clone().expect("an indexed plot");
    assert_eq!((ix.group, ix.subset, ix.error), (column_id(&h, "A"), None, IndexedError::None));

    plot(&mut h, &["Electrolyte", "Scan", "Current"], Template::GroupedBox);
    let g = project(&h).graphs().last().unwrap();
    assert_eq!(g.layers[0].default_kind, PlotKind::Box);
    let labels: Vec<_> = g.layers[0].plots.iter().map(|p| p.label.clone()).collect();
    assert_eq!(labels, vec![Some("1".to_string()), Some("2".to_string())]);
}

#[test]
fn a_wrong_selection_warns_and_makes_no_graph() {
    let mut h = start(vec![indexed_file("grouped-warn")]);
    show_window(&mut h, "Book1 - sample.csv");
    scan_is_a_group(&mut h);
    let before = project(&h).graphs().count();
    // Only a data column; only groups; nothing selected.
    for names in [vec!["Current"], vec!["Electrolyte", "Scan"], vec![]] {
        plot(&mut h, &names, Template::GroupedColumns);
        assert_eq!(project(&h).graphs().count(), before, "{names:?}");
        assert!(h.state().status.contains("Select one or two group columns and one data column"));
        assert!(h.state().message.is_none(), "a status warning, not a dialog");
    }
}

#[test]
fn plot_details_sets_the_error_bar_of_every_subgroup_plot_with_undo() {
    let mut h = start(vec![indexed_file("grouped-error")]);
    show_window(&mut h, "Book1 - sample.csv");
    scan_is_a_group(&mut h);
    plot(&mut h, &["Electrolyte", "Scan", "Current"], Template::GroupedColumns);
    let id = project(&h).graphs().last().unwrap().id;
    let errors = |h: &Harness<'static, SheafApp>| -> Vec<IndexedError> {
        project(h).graph(id).unwrap().layers[0].plots.iter().map(|p| p.indexed.clone().unwrap().error).collect()
    };
    assert_eq!(errors(&h), vec![IndexedError::None; 2]);
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::GraphProperties { graph: id, layer: 0, tab: PropsTab::Plots, plot: Some(0) }, &ctx);
    settle(&mut h);
    h.query_all_by_label("Error Bar").last().expect("the Error Bar tab").click();
    settle(&mut h);
    assert!(has_label(&h, "Error Bar"));
    h.get_by_value("None").click();
    settle(&mut h);
    h.get_by_label("Standard Error").click();
    settle(&mut h);
    click(&mut h, "OK");
    assert_eq!(errors(&h), vec![IndexedError::Se; 2]);
    menu(&mut h, "Edit", "Undo Change graph settings");
    assert_eq!(errors(&h), vec![IndexedError::None; 2]);
}

#[test]
fn interval_plot_and_pareto_plot_from_a_selection_and_warn_on_a_wrong_one() {
    use sheaf_core::graph::StatPlot;
    let mut h = start(vec![indexed_file("interval-pareto")]);
    show_window(&mut h, "Book1 - sample.csv");
    let before = project(&h).graphs().count();

    plot(&mut h, &["Current"], Template::IntervalPlot);
    let g = project(&h).graphs().last().unwrap();
    assert_eq!(g.layers[0].plots.len(), 1);
    assert!(matches!(g.layers[0].plots[0].stat, Some(StatPlot::Interval { .. })));

    // A text column is counted by a Pareto chart (two layers: columns and the cumulative line).
    plot(&mut h, &["Electrolyte"], Template::Pareto);
    let g = project(&h).graphs().last().unwrap();
    assert_eq!(g.layers.len(), 2);
    assert_eq!(g.layers[0].plots[0].stat, Some(StatPlot::Pareto));
    assert_eq!(project(&h).graphs().count(), before + 2);

    // Two columns: categories and counts, a binned Pareto chart (T319) from a new ParetoBin sheet.
    plot(&mut h, &["Electrolyte", "Current"], Template::Pareto);
    assert_eq!(project(&h).graphs().count(), before + 3);
    // Wrong selections: a Pareto of three columns, an interval plot of a text column.
    plot(&mut h, &["Electrolyte", "Scan", "Current"], Template::Pareto);
    assert!(h.state().status.contains("Select one column"));
    plot(&mut h, &["Electrolyte"], Template::IntervalPlot);
    assert_eq!(project(&h).graphs().count(), before + 3);
    assert!(h.state().status.contains("Nothing to plot"));
}
