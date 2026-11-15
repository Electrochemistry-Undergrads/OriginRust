use sheaf_core::graph::FillUnderMode;
use sheaf_core::template::Template;
use sheaf_core::{Color, ColumnData, Designation, Project};
use sheaf_io::{open_project, save::save_project};

#[test]
fn fill_sidelines_survive_native_and_companion_saves_with_source_values_intact() {
    let dir = std::env::temp_dir().join(format!("sheaf-fill-sidelines-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut p = Project::new();
    let b = p.add_workbook("Data");
    let y = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![1.0, 1.5, 1.0])).unwrap();
    let g = p.add_graph(Template::Line);
    p.add_plot(g, 0, y).unwrap();
    for mode in FillUnderMode::ALL {
        let plot = &mut p.graph_mut(g).unwrap().layers[0].plots[0];
        plot.decor.fill_under = Some(Color::BLUE);
        plot.decor.fill_under_mode = mode;
        for ext in ["sheaf", "opj"] {
            let path = dir.join(format!("mode.{ext}"));
            let saved = save_project(&p, &path).unwrap();
            assert!(saved.main_written && saved.refusal.is_none());
            if ext == "opj" {
                assert!(saved.companion.is_some());
                assert!(saved.warnings.iter().any(|w| w.contains("fills under curves")));
            }
            let opened = open_project(&path).unwrap();
            assert_eq!(opened.companion_used, ext == "opj");
            let d = &opened.project.graphs().next().unwrap().layers[0].plots[0].decor;
            assert_eq!(d.fill_under_mode, mode);
            assert_eq!(d.fill_under, Some(Color::BLUE));
            assert_eq!(opened.project.column(y).unwrap().numeric().unwrap(), [1.0, 1.5, 1.0]);
            std::fs::remove_file(path).unwrap();
            if let Some(path) = saved.companion {
                std::fs::remove_file(path).unwrap();
            }
        }
    }
    std::fs::remove_dir(dir).unwrap();
}
