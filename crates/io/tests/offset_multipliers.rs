use sheaf_core::graph::Stack;
use sheaf_core::template::Template;
use sheaf_core::{ColumnData, Designation, Project};
use sheaf_io::{open_project, save::save_project};

#[test]
fn multipliers_survive_native_and_warned_opj_companion_saves_without_rewriting_data() {
    let folder = std::env::temp_dir().join(format!("sheaf-offset-multipliers-{}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    let mut p = Project::new();
    let b = p.add_workbook("Data");
    let y = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![2.0, 4.0])).unwrap();
    let g = p.add_graph(Template::Line);
    p.add_plot(g, 0, y).unwrap();
    let layer = &mut p.graph_mut(g).unwrap().layers[0];
    layer.stack = Stack::Individual;
    let o = layer.plot_offset_mut(y);
    (o.x, o.y, o.x_multiplier, o.y_multiplier) = (5.0, 7.0, 0.5, 3.0);
    let expected = *o;
    for extension in ["sheaf", "opj"] {
        let path = folder.join(format!("scaled.{extension}"));
        let saved = save_project(&p, &path).unwrap();
        assert!(saved.main_written && saved.refusal.is_none());
        if extension == "opj" {
            assert!(saved.companion.is_some());
            assert!(saved.warnings.iter().any(|w| w.contains("multipliers were not written")));
        }
        let opened = open_project(&path).unwrap();
        let layer = &opened.project.graphs().next().unwrap().layers[0];
        assert_eq!(*layer.plot_offset(y).unwrap(), expected);
        assert_eq!(opened.project.numeric(y).unwrap(), [2.0, 4.0]);
        assert_eq!(opened.companion_used, extension == "opj");
        std::fs::remove_file(path).unwrap();
        if let Some(companion) = saved.companion {
            std::fs::remove_file(companion).unwrap();
        }
    }
    std::fs::remove_dir(folder).unwrap();
}
