use sheaf_core::graph::{FillUnderMode, PlotDecor};
use sheaf_core::template::Template;
use sheaf_core::{ColumnData, Designation, Project};

#[test]
fn fill_sidelines_keep_old_defaults_and_copy_the_saved_mode_in_dependent_groups() {
    let old: PlotDecor = serde_json::from_str("{}").unwrap();
    assert_eq!(old.fill_under_mode, FillUnderMode::Normal);
    assert!(!serde_json::to_string(&old).unwrap().contains("fill_under_mode"));
    for mode in FillUnderMode::ALL {
        let d = PlotDecor { fill_under_mode: mode, ..old.clone() };
        assert_eq!(serde_json::from_str::<PlotDecor>(&serde_json::to_string(&d).unwrap()).unwrap(), d);
    }
    let mut p = Project::new();
    let b = p.add_workbook("Data");
    let y = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![1.0, 2.0])).unwrap();
    let g = p.add_graph(Template::Line);
    p.add_plot(g, 0, y).unwrap();
    p.add_plot(g, 0, y).unwrap();
    let l = &mut p.graph_mut(g).unwrap().layers[0];
    l.plots[1].decor.fill_under_mode = FillUnderMode::SidelinesAndBase;
    l.follow_leader(1);
    assert!(l.plots.iter().all(|p| p.decor.fill_under_mode == FillUnderMode::SidelinesAndBase));
}
