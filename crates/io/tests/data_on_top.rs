use sheaf_core::template::Template;
use sheaf_core::{ColumnData, Designation, Project};
use sheaf_io::{open_project, save::save_project};

#[test]
fn data_on_top_survives_native_and_opj_companion_saves() {
    let folder = std::env::temp_dir().join(format!("sheaf-data-on-top-{}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    let mut project = Project::new();
    let book = project.add_workbook("Data");
    let x = project.add_column(book, 0, Designation::X, ColumnData::numeric(vec![0.0, 1.0])).unwrap();
    let y = project.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![0.0, 1.0])).unwrap();
    let graph = project.add_graph(Template::Line);
    project.add_plot_xy(graph, 0, Some(x), y, None).unwrap();
    project.graph_mut(graph).unwrap().layers[0].display.data_on_top = true;

    for extension in ["sheaf", "opj"] {
        let path = folder.join(format!("enabled.{extension}"));
        let saved = save_project(&project, &path).unwrap();
        assert!(saved.main_written && saved.refusal.is_none());
        if extension == "opj" {
            assert!(saved.companion.is_some());
            assert!(saved.warnings.iter().any(|w| w.contains("Display tab")), "{:?}", saved.warnings);
        }
        let opened = open_project(&path).unwrap();
        assert_eq!(opened.companion_used, extension == "opj");
        let layer = &opened.project.graphs().next().unwrap().layers[0];
        assert!(layer.display.data_on_top);
        assert_eq!(opened.project.column(y).unwrap().numeric(), Some(&[0.0, 1.0][..]));
        std::fs::remove_file(&path).unwrap();
        if let Some(companion) = saved.companion {
            std::fs::remove_file(companion).unwrap();
        }
    }
    std::fs::remove_dir(folder).unwrap();
}
