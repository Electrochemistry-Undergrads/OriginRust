use sheaf_core::template::Template;
use sheaf_core::{ColumnData, Designation, Project};
use sheaf_io::{open_project, save::save_project};

#[test]
fn symbol_line_order_survives_native_and_companion_saves_with_an_explicit_opj_warning() {
    let folder = std::env::temp_dir().join(format!("sheaf-symbol-line-order-{}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    let mut p = Project::new();
    let b = p.add_workbook("Data");
    let y = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![1.0, 2.0, 3.0])).unwrap();
    let g = p.add_graph(Template::LineSymbol);
    p.add_plot(g, 0, y).unwrap();
    p.graph_mut(g).unwrap().layers[0].plots[0].decor.symbol_line.draw_in_front = true;
    for ext in ["sheaf", "opj"] {
        let path = folder.join(format!("order.{ext}"));
        let saved = save_project(&p, &path).unwrap();
        assert!(saved.main_written && saved.refusal.is_none());
        if ext == "opj" {
            assert!(saved.warnings.iter().any(|w| w.contains("draw order")));
            assert!(saved.companion.is_some());
        }
        let opened = open_project(&path).unwrap();
        assert_eq!(opened.companion_used, ext == "opj");
        let interface = opened.project.graphs().next().unwrap().layers[0].plots[0].decor.symbol_line;
        assert!(interface.draw_in_front && !interface.gap_to_symbol);
        assert_eq!(opened.project.column(y).unwrap().numeric().unwrap(), [1.0, 2.0, 3.0]);
        std::fs::remove_file(path).unwrap();
        if let Some(path) = saved.companion {
            std::fs::remove_file(path).unwrap();
        }
    }
    std::fs::remove_dir(folder).unwrap();
}
