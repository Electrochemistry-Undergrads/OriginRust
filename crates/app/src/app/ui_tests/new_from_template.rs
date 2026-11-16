//! File > New > From Template (T103, graphing.md 24).

use super::*;

#[test]
fn new_from_template_makes_an_empty_graph_in_the_templates_look() {
    let dir = std::env::temp_dir().join(format!("sheaf-ui-new-from-template-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    // A template saved from a graph with a thick Y axis.
    let mut p = sheaf_core::Project::new();
    let g = p.add_graph(sheaf_core::template::Template::Line);
    p.graph_mut(g).unwrap().layers[0].y_axis.line_width_pt = 3.0;
    let t = sheaf_core::user_template::UserTemplate::from_graph("Thick", p.graph(g).unwrap()).unwrap();
    sheaf_io::user_templates::save(&dir, &t).unwrap();

    let startup = Startup { templates_dir: Some(dir.clone()), ..Default::default() };
    let mut h = Harness::builder().with_size(egui::vec2(1280.0, 900.0)).build_eframe(|cc| SheafApp::new(cc, startup));
    settle(&mut h);
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::NewFromTemplate(Some("Thick".into())), &ctx);
    settle(&mut h);
    let graphs: Vec<_> = project(&h).graphs().collect();
    assert_eq!(graphs.len(), 1);
    assert!(graphs[0].layers.iter().all(|l| l.plots.is_empty()), "no data");
    assert_eq!(graphs[0].layers[0].y_axis.line_width_pt, 3.0, "the template's look");
    assert_eq!(h.state().doc.undo_label(), Some("New graph from template"));
    let _ = std::fs::remove_dir_all(&dir);
}
