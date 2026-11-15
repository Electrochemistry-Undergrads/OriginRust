use sheaf_core::graph::SymbolLineInterface;
use sheaf_core::template::Template;
use sheaf_core::{ColumnData, Designation, Project};
use sheaf_io::{open_project, save::save_project};

#[test]
fn symbol_line_gaps_survive_native_and_companion_saves_with_an_opj_warning() {
    let folder = std::env::temp_dir().join(format!("sheaf-symbol-line-gaps-{}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    let mut p = Project::new();
    let b = p.add_workbook("Data");
    let source = vec![1.0, 4.0, 2.0, 5.0];
    let y = p.add_column(b, 0, Designation::Y, ColumnData::numeric(source.clone())).unwrap();
    let g = p.add_graph(Template::LineSymbol);
    p.add_plot(g, 0, y).unwrap();
    let plot = &mut p.graph_mut(g).unwrap().layers[0].plots[0];
    plot.symbol.every = 3;
    plot.decor.symbol_line = SymbolLineInterface { gap_to_symbol: true, no_break_skipped: true };
    let expected = plot.decor.symbol_line;
    for ext in ["sheaf", "opj"] {
        let path = folder.join(format!("gaps.{ext}"));
        let saved = save_project(&p, &path).unwrap();
        assert!(saved.main_written && saved.refusal.is_none());
        if ext == "opj" {
            assert!(saved.warnings.iter().any(|w| w.contains("symbol/line gaps")));
            assert!(saved.companion.is_some());
        }
        let opened = open_project(&path).unwrap();
        assert_eq!(opened.companion_used, ext == "opj");
        let plot = &opened.project.graphs().next().unwrap().layers[0].plots[0];
        assert_eq!(plot.decor.symbol_line, expected);
        assert_eq!(plot.symbol.every, 3);
        assert_eq!(opened.project.column(y).unwrap().numeric().unwrap(), source);
        std::fs::remove_file(path).unwrap();
        if let Some(path) = saved.companion {
            std::fs::remove_file(path).unwrap();
        }
    }
    std::fs::remove_dir(folder).unwrap();
}
