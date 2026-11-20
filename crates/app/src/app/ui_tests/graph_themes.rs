//! An axis's Save Format as Theme... and Tools > Theme Organizer (T102, graphing.md "Graph themes").

use super::*;

#[test]
fn an_axis_theme_is_saved_and_applied_to_another_graph() {
    let dir = std::env::temp_dir().join(format!("sheaf-ui-graph-themes-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let startup =
        Startup { paths: vec![data_file("graph-themes")], graph_themes_dir: Some(dir.clone()), ..Default::default() };
    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 900.0))
        .with_step_dt(0.02)
        .build_eframe(|cc| SheafApp::new(cc, startup));
    settle(&mut h);
    let ctx = h.ctx.clone();
    let first = project(&h).graphs().next().unwrap().id;
    h.state_mut().doc.apply("Thick", |p| p.graph_mut(first).unwrap().layers[0].y_axis.line_width_pt = 3.0);
    h.state_mut().apply(Action::SaveAxisTheme { graph: first, layer: 0, y: true }, &ctx);
    settle(&mut h);
    assert_eq!(h.state().save_theme.as_ref().unwrap().0.name, "Theme1", "Origin's default name");
    click(&mut h, "OK");
    assert!(h.state().save_theme.is_none());
    assert_eq!(sheaf_io::graph_themes::list(&dir).len(), 1);
    // Another graph, then Theme Organizer > Apply Now.
    h.state_mut().apply(Action::DuplicateWindow(first), &ctx);
    settle(&mut h);
    let second = project(&h).graphs().last().unwrap().id;
    h.state_mut().doc.apply("Thin", |p| {
        let l = &mut p.graph_mut(second).unwrap().layers[0];
        l.x_axis.line_width_pt = 0.5;
        l.y_axis.line_width_pt = 0.5;
    });
    h.state_mut().apply(Action::Focus(second), &ctx);
    h.state_mut().apply(Action::ThemeOrganizer, &ctx);
    settle(&mut h);
    {
        let o = h.state_mut().theme_organizer.as_mut().expect("the organizer opened");
        assert_eq!(o.entries.len(), 1);
        o.selected = Some("Theme1".into());
    }
    settle(&mut h);
    click(&mut h, "Apply Now");
    let l = &project(&h).graph(second).unwrap().layers[0];
    assert_eq!((l.x_axis.line_width_pt, l.y_axis.line_width_pt), (3.0, 3.0), "every axis takes the theme");
    assert_eq!(h.state().doc.undo_label(), Some("Apply theme"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_legend_theme_is_saved_from_the_legend_menu_and_applied_to_every_legend() {
    let dir = std::env::temp_dir().join(format!("sheaf-ui-legend-themes-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let startup =
        Startup { paths: vec![data_file("legend-themes")], graph_themes_dir: Some(dir.clone()), ..Default::default() };
    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 900.0))
        .with_step_dt(0.02)
        .build_eframe(|cc| SheafApp::new(cc, startup));
    settle(&mut h);
    let ctx = h.ctx.clone();
    let first = project(&h).graphs().next().unwrap().id;
    assert!(project(&h).graph(first).unwrap().layers[0].legend.visible, "the start-up graph has a legend");
    h.state_mut().doc.apply("Big legend", |p| p.graph_mut(first).unwrap().layers[0].legend.font.size_pt = 31.0);
    // The legend's context menu resolves to the save.
    let c = h.state().menu_context();
    let action = crate::menus::resolve(&crate::menus::Cmd::SaveLegendTheme { layer: 0 }, &c).unwrap();
    drop(c);
    h.state_mut().apply(action, &ctx);
    settle(&mut h);
    assert!(h.state().save_theme.is_some(), "the name window is open");
    click(&mut h, "OK");
    let saved = sheaf_io::graph_themes::load(&dir, "Theme1").unwrap();
    assert!(matches!(saved.format, sheaf_core::format_clip::FormatClip::LegendStyles(_)));

    h.state_mut().apply(Action::DuplicateWindow(first), &ctx);
    settle(&mut h);
    let second = project(&h).graphs().last().unwrap().id;
    h.state_mut().doc.apply("Small", |p| p.graph_mut(second).unwrap().layers[0].legend.font.size_pt = 9.0);
    h.state_mut().apply(Action::Focus(second), &ctx);
    h.state_mut().apply(Action::ThemeOrganizer, &ctx);
    settle(&mut h);
    h.state_mut().theme_organizer.as_mut().unwrap().selected = Some("Theme1".into());
    settle(&mut h);
    click(&mut h, "Apply Now");
    assert_eq!(project(&h).graph(second).unwrap().layers[0].legend.font.size_pt, 31.0, "the legend takes the theme");
    let _ = std::fs::remove_dir_all(&dir);
}
