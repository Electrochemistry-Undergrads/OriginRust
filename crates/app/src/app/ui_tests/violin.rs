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
