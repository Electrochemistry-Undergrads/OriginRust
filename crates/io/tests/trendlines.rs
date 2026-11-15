use sheaf_core::graph::{TrendKind, Trendline};
use sheaf_core::template::Template;
use sheaf_core::{ColumnData, Designation, Project};
use sheaf_io::{open_project, save::save_project};

#[test]
fn trendlines_warn_in_opj_and_survive_native_and_companion_saves() {
    let folder = std::env::temp_dir().join(format!("sheaf-trendlines-{}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    let mut p = Project::new();
    let book = p.add_workbook("Data");
    let x = p.add_column(book, 0, Designation::X, ColumnData::numeric(vec![1.0, 2.0, 3.0, 4.0])).unwrap();
    let y = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![1.0, 4.0, 2.0, 5.0])).unwrap();
    let graph = p.add_graph(Template::Scatter);
    p.add_plot_xy(graph, 0, Some(x), y, None).unwrap();
    let trends = vec![Trendline::new(TrendKind::Linear), Trendline::new(TrendKind::MovingAverage { window: 2 })];
    p.graph_mut(graph).unwrap().layers[0].plots[0].decor.trendlines = trends.clone();
    for extension in ["sheaf", "opj"] {
        let path = folder.join(format!("t.{extension}"));
        let saved = save_project(&p, &path).unwrap();
        if extension == "opj" {
            assert!(saved.warnings.iter().any(|w| w.contains("trendlines")), "{:?}", saved.warnings);
            assert!(!saved.warnings.iter().any(|w| w.contains("drop lines")), "{:?}", saved.warnings);
        }
        let opened = open_project(&path).unwrap();
        assert_eq!(opened.project.graphs().next().unwrap().layers[0].plots[0].decor.trendlines, trends, "{extension}");
        std::fs::remove_file(&path).unwrap();
        if let Some(companion) = saved.companion {
            std::fs::remove_file(companion).unwrap();
        }
    }
    std::fs::remove_dir(folder).unwrap();
}
