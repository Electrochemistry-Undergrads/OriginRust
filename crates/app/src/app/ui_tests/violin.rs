use super::*;
use sheaf_core::graph::PlotKind;

#[test]
fn the_statistics_menu_has_live_violin_items_that_make_one_violin_per_y_column() {
    for (item, kind) in [
        ("Violin Plot", PlotKind::Violin),
        ("Violin with Box", PlotKind::ViolinBox),
        ("Violin with Point", PlotKind::ViolinPoint),
        ("Violin with Quartile", PlotKind::ViolinQuartile),
        ("Violin with Stick", PlotKind::ViolinStick),
        ("Half Violin", PlotKind::HalfViolin),
    ] {
        let mut h = start(vec![data_file("violin-menu")]);
        show_window(&mut h, "Book1 - sample");
        let before = project(&h).graphs().count();
        select_b_and_c(&mut h);
        menu_path(&mut h, &["Plot"]);
        // The Plot menu's Statistics submenu, not the Statistics menu of the bar (which lies further right).
        not_toolbar(&h, "Statistics")
            .into_iter()
            .min_by(|a, b| a.rect().min.x.total_cmp(&b.rect().min.x))
            .expect("the Statistics submenu")
            .click();
        settle(&mut h);
        menu_path(&mut h, &[item]);
        let graphs: Vec<_> = project(&h).graphs().collect();
        assert_eq!(graphs.len(), before + 1, "{item}");
        let layer = &graphs[before].layers[0];
        assert_eq!(layer.default_kind, kind, "{item}");
        assert_eq!(layer.plots.len(), 2, "{item}: one violin per Y column");
        assert!(layer.plots.iter().all(|p| p.kind == kind), "{item}");
    }
}

fn make_ridgeline(h: &mut Harness<'static, SheafApp>) -> sheaf_core::WindowId {
    show_window(h, "Book1 - sample");
    let before = project(h).graphs().count();
    select_b_and_c(h);
    menu_path(h, &["Plot"]);
    not_toolbar(h, "Statistics")
        .into_iter()
        .min_by(|a, b| a.rect().min.x.total_cmp(&b.rect().min.x))
        .expect("the Statistics submenu")
        .click();
    settle(h);
    menu_path(h, &["Ridgeline Chart"]);
    let graphs: Vec<_> = project(h).graphs().collect();
    assert_eq!(graphs.len(), before + 1);
    assert_eq!(graphs[before].layers[0].default_kind, PlotKind::Ridgeline);
    assert_eq!(graphs[before].layers[0].plots.len(), 2, "one ridge per Y column");
    graphs[before].id
}

#[test]
fn the_statistics_menu_makes_a_ridgeline_chart() {
    let mut h = start(vec![data_file("ridge-menu")]);
    make_ridgeline(&mut h);
}

#[test]
fn plot_details_has_the_ridgeline_display_and_distribution_controls_with_undo() {
    use sheaf_core::graph::{RidgeBandwidth, RidgeDisplay};
    let mut h = start(vec![data_file("ridge-details")]);
    let id = make_ridgeline(&mut h);
    let ridge =
        |h: &Harness<'static, SheafApp>, i: usize| project(h).graph(id).unwrap().layers[0].plots[i].ridge.clone();
    let ctx = h.ctx.clone();
    let open = |h: &mut Harness<'static, SheafApp>| {
        h.state_mut().apply(Action::GraphProperties { graph: id, layer: 0, tab: PropsTab::Plots, plot: Some(0) }, &ctx);
        settle(h);
    };
    open(&mut h);
    click(&mut h, "Distribution");
    assert!(has_label(&h, "Bandwidth"));
    h.get_by_value("Scott").click();
    settle(&mut h);
    click(&mut h, "Silverman");
    click(&mut h, "Display");
    click(&mut h, "Quantile Lines");
    h.get_by_value("Distribution Curve").click();
    settle(&mut h);
    click(&mut h, "Single Block Bar");
    click(&mut h, "OK");
    for i in 0..2 {
        let r = ridge(&h, i);
        assert_eq!(r.bandwidth, RidgeBandwidth::Silverman, "shared by the ridges of the layer");
        assert!(r.quantile_lines);
        assert_eq!(r.display, RidgeDisplay::SingleBlockBar);
    }
    menu(&mut h, "Edit", "Undo Change graph settings");
    assert_eq!(ridge(&h, 0), sheaf_core::graph::RidgelineStyle::default());
}
