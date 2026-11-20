//! File > Export > Graphs..., Graph as Image... and Multi-Page PDF... from a workbook (T182).

use super::*;
use crate::dialogs::export::ExportFormat;

#[test]
fn a_workbooks_export_items_open_the_export_window_on_every_graph() {
    let mut h = start(vec![data_file("export-no-graph")]);
    show_window(&mut h, "Book1 - sample.csv");
    menu_path(&mut h, &["File", "Export", "Multi-Page PDF..."]);
    let d = h.state().export.as_ref().expect("the Export Graphs window is open");
    assert!(d.starts_on_all_graphs());
    assert_eq!(d.format_and_multipage(), (ExportFormat::Pdf, true));
    assert_eq!(d.graph, project(&h).graphs().next().unwrap().id);
    h.state_mut().export = None;
    settle(&mut h);
    menu_path(&mut h, &["File", "Export", "Graph as Image..."]);
    let d = h.state().export.as_ref().expect("open again");
    assert!(d.starts_on_all_graphs());
    assert_eq!(d.format_and_multipage(), (ExportFormat::Png, false));
}

#[test]
fn without_a_graph_the_export_items_only_say_so() {
    let mut h = start(vec![data_file("export-no-graph-none")]);
    let graphs: Vec<WindowId> = project(&h).graphs().map(|g| g.id).collect();
    h.state_mut().doc.apply("drop graphs", |p| p.windows.retain(|w| !graphs.contains(&w.id())));
    settle(&mut h);
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::ExportProjectGraphs(crate::menus::ProjectGraphsExport::Image), &ctx);
    settle(&mut h);
    assert!(h.state().export.is_none());
}
