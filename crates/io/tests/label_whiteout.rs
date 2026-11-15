use sheaf_core::template::Template;
use sheaf_core::{ColumnData, Designation, Project};
use sheaf_io::{open_project, save::save_project};

#[test]
fn label_whiteout_survives_native_and_opj_companion_saves() {
    let folder = std::env::temp_dir().join(format!("sheaf-label-whiteout-{}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    let mut p = Project::new();
    let book = p.add_workbook("Data");
    let y = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![1.0, 2.0])).unwrap();
    let graph = p.add_graph(Template::Line);
    p.add_plot(graph, 0, y).unwrap();
    let labels = &mut p.graph_mut(graph).unwrap().layers[0].plots[0].decor.labels;
    labels.enable = true;
    labels.white_out = true;
    labels.rotate_deg = 45.0;
    for extension in ["sheaf", "opj"] {
        let path = folder.join(format!("labels.{extension}"));
        let saved = save_project(&p, &path).unwrap();
        assert!(saved.main_written && saved.refusal.is_none());
        if extension == "opj" {
            assert!(saved.companion.is_some());
            assert!(saved.warnings.iter().any(|w| w.contains("data labels")));
        }
        let opened = open_project(&path).unwrap();
        let labels = &opened.project.graphs().next().unwrap().layers[0].plots[0].decor.labels;
        assert!(labels.enable && labels.white_out);
        assert_eq!(labels.rotate_deg, 45.0);
        assert_eq!(opened.companion_used, extension == "opj");
        std::fs::remove_file(path).unwrap();
        if let Some(companion) = saved.companion {
            std::fs::remove_file(companion).unwrap();
        }
    }
    std::fs::remove_dir(folder).unwrap();
}
