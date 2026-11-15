use sheaf_core::graph::LabelFrame;
use sheaf_core::template::Template;
use sheaf_core::{Color, ColumnData, Designation, Project};
use sheaf_io::{open_project, save::save_project};

#[test]
fn label_frame_colors_margin_and_rotation_survive_native_and_companion_saves() {
    let folder = std::env::temp_dir().join(format!("sheaf-label-frames-{}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    let mut p = Project::new();
    let b = p.add_workbook("Data");
    let y = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![1.0, 2.0])).unwrap();
    let g = p.add_graph(Template::Line);
    p.add_plot(g, 0, y).unwrap();
    for frame in [LabelFrame::Box, LabelFrame::Shadow] {
        let labels = &mut p.graph_mut(g).unwrap().layers[0].plots[0].decor.labels;
        labels.frame = frame;
        labels.margin_percent = 20.0;
        labels.frame_border = Color::RED;
        labels.frame_fill = Color::TRANSPARENT;
        labels.rotate_deg = 45.0;
        let expected = labels.clone();
        for extension in ["sheaf", "opj"] {
            let path = folder.join(format!("labels-{frame:?}.{extension}"));
            let saved = save_project(&p, &path).unwrap();
            assert!(saved.main_written && saved.refusal.is_none());
            if extension == "opj" {
                assert!(saved.companion.is_some());
                assert!(saved.warnings.iter().any(|w| w.contains("data labels")));
            }
            let opened = open_project(&path).unwrap();
            assert_eq!(opened.project.graphs().next().unwrap().layers[0].plots[0].decor.labels, expected);
            assert_eq!(opened.companion_used, extension == "opj");
            std::fs::remove_file(path).unwrap();
            if let Some(companion) = saved.companion {
                std::fs::remove_file(companion).unwrap();
            }
        }
    }
    std::fs::remove_dir(folder).unwrap();
}
