use sheaf_core::template::Template;
use sheaf_core::{ColumnData, Designation, Project};
use sheaf_io::{open_project, save::save_project};

#[test]
fn skip_points_on_a_line_with_errors_warns_and_survives_native_and_companion_saves() {
    let folder = std::env::temp_dir().join(format!("sheaf-skip-points-{}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    let mut p = Project::new();
    let book = p.add_workbook("Data");
    let y = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![1.0, 4.0, 2.0, 5.0])).unwrap();
    let e = p.add_column(book, 0, Designation::YError, ColumnData::numeric(vec![0.2; 4])).unwrap();
    let graph = p.add_graph(Template::Line);
    p.add_plot_xy(graph, 0, None, y, Some(e)).unwrap();
    p.graph_mut(graph).unwrap().layers[0].plots[0].symbol.every = 3;
    for extension in ["sheaf", "opj"] {
        let path = folder.join(format!("skip.{extension}"));
        let saved = save_project(&p, &path).unwrap();
        assert!(saved.main_written && saved.refusal.is_none());
        if extension == "opj" {
            assert!(saved.warnings.iter().any(|w| w.contains("Skip Points")));
            assert!(saved.companion.is_some(), "a line without symbols also needs its skipped errors preserved");
        }
        let opened = open_project(&path).unwrap();
        assert_eq!(opened.companion_used, extension == "opj");
        assert_eq!(opened.project.graphs().next().unwrap().layers[0].plots[0].symbol.every, 3);
        assert_eq!(opened.project.column(y).unwrap().numeric(), Some(&[1.0, 4.0, 2.0, 5.0][..]));
        std::fs::remove_file(&path).unwrap();
        if let Some(companion) = saved.companion {
            std::fs::remove_file(companion).unwrap();
        }
    }
    std::fs::remove_dir(folder).unwrap();
}

#[test]
fn sort_x_warns_in_opj_and_survives_native_and_companion_saves() {
    let folder = std::env::temp_dir().join(format!("sheaf-sort-x-{}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    let mut p = Project::new();
    let book = p.add_workbook("Data");
    let x = p.add_column(book, 0, Designation::X, ColumnData::numeric(vec![3.0, 1.0, 2.0])).unwrap();
    let y = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![1.0, 4.0, 2.0])).unwrap();
    let graph = p.add_graph(Template::Line);
    p.add_plot_xy(graph, 0, Some(x), y, None).unwrap();
    p.graph_mut(graph).unwrap().layers[0].plots[0].line.sort_x = true;
    for extension in ["sheaf", "opj"] {
        let path = folder.join(format!("sort.{extension}"));
        let saved = save_project(&p, &path).unwrap();
        if extension == "opj" {
            assert!(saved.warnings.iter().any(|w| w.contains("Sort X")), "{:?}", saved.warnings);
        }
        let opened = open_project(&path).unwrap();
        assert!(opened.project.graphs().next().unwrap().layers[0].plots[0].line.sort_x, "{extension}");
        std::fs::remove_file(&path).unwrap();
        if let Some(companion) = saved.companion {
            std::fs::remove_file(companion).unwrap();
        }
    }
    std::fs::remove_dir(folder).unwrap();
}
