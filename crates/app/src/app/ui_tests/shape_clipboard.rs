//! A drawn object's right-click Cut and Copy and the page's Paste at the clicked point (T125,
//! graphing 17.8).
use super::*;
use sheaf_core::shape::{Shape, ShapeKind};

fn context(h: &mut Harness<'static, SheafApp>, point: (f64, f64), action: &str) {
    let at = frame_point(h, point.0, point.1);
    right_click_at(h, at);
    assert!(entry_enabled(h, action), "{action} is live");
    menu_entry(h, action).click();
    settle(h);
}

#[test]
fn copy_then_paste_puts_a_same_size_copy_at_the_clicked_point_and_cut_removes_the_source() {
    crate::shape_clip::clear();
    let mut h = start(vec![data_file("shape-clip")]);
    let id = graph(&h).id;
    let mut rect = Shape::new(ShapeKind::Rectangle, (0.2, 0.3), (0.4, 0.5));
    rect.fill = Some(sheaf_core::Color::rgba(220, 20, 20, 255));
    h.state_mut().doc.apply("Shapes", |p| p.graph_mut(id).unwrap().layers[0].shapes = vec![rect.clone()]);
    settle(&mut h);
    // Nothing copied yet: the page menu has no Paste.
    let empty = frame_point(&h, 0.45, 0.72);
    right_click_at(&mut h, empty);
    assert!(h.query_by_label("Paste").is_none(), "no Paste before a copy");
    close_menus(&mut h);

    context(&mut h, (0.3, 0.4), "Copy");
    assert_eq!(shapes(&h).len(), 1, "Copy changes nothing");
    context(&mut h, (0.45, 0.72), "Paste");
    let s = shapes(&h);
    assert_eq!(s.len(), 2);
    let copy = &s[1];
    let close = |a: f64, b: f64| (a - b).abs() < 0.01;
    assert!(close(copy.x1.min(copy.x2), 0.45) && close(copy.y1.min(copy.y2), 0.72), "{copy:?}");
    assert!(close((copy.x2 - copy.x1).abs(), 0.2) && close((copy.y2 - copy.y1).abs(), 0.2), "same size");
    assert_eq!((copy.kind, copy.fill), (rect.kind, rect.fill));
    assert_eq!(h.state().doc.undo_label(), Some("Paste drawn object"));

    context(&mut h, (0.3, 0.4), "Cut");
    assert_eq!(shapes(&h).len(), 1, "the source is gone");
    assert!(close(shapes(&h)[0].x1.min(shapes(&h)[0].x2), 0.45), "the copy stays");
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::Undo, &ctx);
    settle(&mut h);
    assert_eq!(shapes(&h).len(), 2, "one undo step brings the source back");
}

#[test]
fn a_text_label_is_cut_and_pasted_with_its_anchor_at_the_clicked_point() {
    // T147: a label's right-click Cut and Copy, pasted by the page's Paste.
    crate::shape_clip::clear();
    let labels = menus::graph_label_menu(0, 0);
    for want in [menus::Cmd::CutLabel { layer: 0, label: 0 }, menus::Cmd::CopyLabel { layer: 0, label: 0 }] {
        assert!(labels.iter().any(|e| matches!(e, menus::Entry::Item(i) if i.cmd.as_ref() == Some(&want))));
    }
    let mut h = start(vec![data_file("label-clip")]);
    let ctx = h.ctx.clone();
    let graph = project(&h).graphs().next().unwrap().id;
    h.state_mut().doc.apply("Labels", |p| {
        let l = &mut p.graph_mut(graph).unwrap().layers[0];
        let font = l.new_label_font();
        l.labels.push(sheaf_core::graph::TextLabel::new("note", 0.3, 0.3, font));
    });
    let texts = |h: &Harness<'static, SheafApp>| -> Vec<(String, f64, f64)> {
        project(h).graph(graph).unwrap().layers[0].labels.iter().map(|l| (l.text.clone(), l.x, l.y)).collect()
    };
    let n = texts(&h).len();
    h.state_mut().apply(Action::CopyLabel { graph, layer: 0, label: n - 1, cut: true }, &ctx);
    settle(&mut h);
    assert_eq!(texts(&h).len(), n - 1, "Cut deletes the label");
    // Paste at the page point of the frame's centre.
    let g = project(&h).graph(graph).unwrap();
    let f = g.absolute_frame(0).unwrap();
    let page = ((f.left + 0.5 * f.width) * g.page.width_pt, (f.top + 0.5 * f.height) * g.page.height_pt);
    h.state_mut().apply(Action::PasteShape { graph, layer: 0, page }, &ctx);
    settle(&mut h);
    let last = texts(&h).last().cloned().unwrap();
    assert_eq!(last.0, "note");
    assert!((last.1 - 0.5).abs() < 1e-9 && (last.2 - 0.5).abs() < 1e-9, "{last:?}");
    assert_eq!(h.state().doc.undo_label(), Some("Paste text"));
}

#[test]
fn an_axis_title_is_copied_as_a_text_label_and_cut_deletes_the_title() {
    // T148: an axis title's right-click Cut and Copy.
    crate::shape_clip::clear();
    let mut h = start(vec![data_file("title-clip")]);
    let ctx = h.ctx.clone();
    let graph = project(&h).graphs().next().unwrap().id;
    let shown = sheaf_plot::axis_title(project(&h), graph, 0, true).unwrap();
    assert!(!shown.is_empty());
    h.state_mut().apply(Action::CopyTitle { graph, layer: 0, y: true, cut: true }, &ctx);
    settle(&mut h);
    assert!(!project(&h).graph(graph).unwrap().layers[0].y_axis.show_title, "Cut deletes the title");
    assert_eq!(h.state().doc.undo_label(), Some("Delete axis title"));
    let g = project(&h).graph(graph).unwrap();
    let f = g.absolute_frame(0).unwrap();
    let page = (f.left * g.page.width_pt, f.top * g.page.height_pt);
    let before = g.layers[0].labels.len();
    h.state_mut().apply(Action::PasteShape { graph, layer: 0, page }, &ctx);
    settle(&mut h);
    let l = &project(&h).graph(graph).unwrap().layers[0];
    assert_eq!(l.labels.len(), before + 1);
    let pasted = l.labels.last().unwrap();
    assert_eq!((pasted.text.as_str(), pasted.rotation_deg), (shown.as_str(), 90.0));
    assert_eq!(pasted.font, l.y_axis.title_font);
}

#[test]
fn the_graph_edit_menu_and_ctrl_c_v_copy_and_paste_the_selected_object() {
    // T149: Edit > Cut, Copy, Paste and Clear on a graph, and Ctrl+C / Ctrl+V.
    use sheaf_plot::scene::ObjectKind;
    crate::shape_clip::clear();
    let mut h = start(vec![data_file("graph-edit-clip")]);
    let id = graph(&h).id;
    let rect = Shape::new(ShapeKind::Rectangle, (0.2, 0.3), (0.4, 0.5));
    h.state_mut().doc.apply("Shapes", |p| p.graph_mut(id).unwrap().layers[0].shapes = vec![rect]);
    show_window(&mut h, "Graph1 - sample");
    // Nothing selected, nothing copied: the four items are greyed.
    h.state_mut().graphs.get_mut(&id).unwrap().selected = None;
    settle(&mut h);
    for op in [menus::GraphClip::Copy, menus::GraphClip::Paste, menus::GraphClip::Clear] {
        let r = menus::resolve(&menus::Cmd::GraphClipboard(op), &h.state().menu_context());
        assert!(r.is_err(), "{op:?} needs a selection or a copy");
    }
    h.state_mut().graphs.get_mut(&id).unwrap().selected = Some((0, ObjectKind::Shape(0)));
    settle(&mut h);
    menu_path(&mut h, &["Edit", "Copy"]);
    // Paste goes to the last clicked point; set one in the frame's lower half.
    let g = graph(&h);
    let f = g.absolute_frame(0).unwrap();
    let at = ((f.left + 0.6 * f.width) * g.page.width_pt, (f.top + 0.7 * f.height) * g.page.height_pt);
    h.state_mut().graphs.get_mut(&id).unwrap().last_click = Some(at);
    menu_path(&mut h, &["Edit", "Paste"]);
    let s = shapes(&h);
    assert_eq!(s.len(), 2);
    assert!((s[1].x1 - 0.6).abs() < 1e-9 && (s[1].y1 - 0.7).abs() < 1e-9, "{:?}", s[1]);
    // Ctrl+V pastes once more at the same point.
    h.event(egui::Event::Paste(String::new()));
    settle(&mut h);
    assert_eq!(shapes(&h).len(), 3);
    // Clear deletes the selected object.
    h.state_mut().graphs.get_mut(&id).unwrap().selected = Some((0, ObjectKind::Shape(0)));
    settle(&mut h);
    menu_path(&mut h, &["Edit", "Clear"]);
    assert_eq!(shapes(&h).len(), 2);
    assert_eq!(h.state().doc.undo_label(), Some("Delete drawn object"));
}

#[test]
fn format_object_properties_opens_the_selected_objects_window() {
    // T150: Format > Object Properties... on a graph.
    use sheaf_plot::scene::ObjectKind;
    let mut h = start(vec![data_file("object-props")]);
    let id = graph(&h).id;
    let rect = Shape::new(ShapeKind::Rectangle, (0.2, 0.3), (0.4, 0.5));
    h.state_mut().doc.apply("Shapes", |p| p.graph_mut(id).unwrap().layers[0].shapes = vec![rect]);
    show_window(&mut h, "Graph1 - sample");
    h.state_mut().graphs.get_mut(&id).unwrap().selected = None;
    settle(&mut h);
    let r = menus::resolve(&menus::Cmd::ObjectProperties, &h.state().menu_context());
    assert!(r.is_err(), "greyed without a selection");
    h.state_mut().graphs.get_mut(&id).unwrap().selected = Some((0, ObjectKind::Shape(0)));
    settle(&mut h);
    menu_path(&mut h, &["Format", "Object Properties..."]);
    assert!(h.state().shape_dialog.is_some(), "the drawn object's window");
    h.state_mut().shape_dialog = None;
    h.state_mut().graphs.get_mut(&id).unwrap().selected = Some((0, ObjectKind::YAxis));
    settle(&mut h);
    menu_path(&mut h, &["Format", "Object Properties..."]);
    assert_eq!(h.state().graph_props.as_ref().map(|d| d.tab()), Some(PropsTab::YAxis));
}

#[test]
fn the_plot_menus_probability_and_qq_plot_open_their_windows() {
    // T151: Plot > Statistics > Probability Plot and Q-Q Plot run the Statistics windows.
    use crate::dialogs::analysis::AnalysisKind;
    let plot = menus::bar(menus::WindowKind::Workbook).iter().find(|m| m.name() == "Plot").unwrap();
    let mut found = Vec::new();
    menus::walk(&plot.entries, &mut |i| {
        if let Some(menus::Cmd::Analyze(k @ (AnalysisKind::ProbPlot | AnalysisKind::QQPlot))) = &i.cmd {
            found.push((menus::plain(&i.label), *k));
        }
    });
    assert_eq!(
        found,
        [("Probability Plot".to_string(), AnalysisKind::ProbPlot), ("Q-Q Plot".to_string(), AnalysisKind::QQPlot)]
    );
    let mut h = start(vec![data_file("plot-menu-prob")]);
    show_window(&mut h, "Book1 - sample.csv");
    click_text(&mut h, "C(Y1)");
    let action = menus::resolve(&menus::Cmd::Analyze(AnalysisKind::QQPlot), &h.state().menu_context()).unwrap();
    let ctx = h.ctx.clone();
    h.state_mut().apply(action, &ctx);
    settle(&mut h);
    assert_eq!(h.state().analysis.as_ref().map(|a| a.kind()), Some(AnalysisKind::QQPlot));
}

#[test]
fn the_3d_scatter_and_trajectory_items_make_xyz_graphs() {
    // T155: Plot > 3D Symbol/Bar/Vector > 3D Scatter and 3D Trajectory.
    use sheaf_core::template::Template;
    let plot = menus::bar(menus::WindowKind::Workbook).iter().find(|m| m.name() == "Plot").unwrap();
    let mut found = Vec::new();
    menus::walk(&plot.entries, &mut |i| {
        if let Some(menus::Cmd::Plot(
            t @ (Template::Scatter3d | Template::Trajectory3d | Template::Bars3d | Template::Scatter3dError),
        )) = &i.cmd
        {
            found.push(*t);
        }
    });
    // T156 added 3D Bars and 3D Scatter + Error Bar.
    assert_eq!(found, [Template::Bars3d, Template::Scatter3d, Template::Trajectory3d, Template::Scatter3dError]);
    // The sample's "up" column with "down" after it as Z.
    let mut h = start(vec![data_file("plot-3d")]);
    show_window(&mut h, "Book1 - sample.csv");
    click_text(&mut h, "B(Y1)");
    let before = project(&h).graphs().count();
    let action = menus::resolve(&menus::Cmd::Plot(Template::Scatter3d), &h.state().menu_context()).unwrap();
    let ctx = h.ctx.clone();
    h.state_mut().apply(action, &ctx);
    settle(&mut h);
    assert_eq!(project(&h).graphs().count(), before + 1);
    let g = project(&h).graphs().last().unwrap();
    assert!(g.layers[0].coords.space3d().is_some());
    assert!(g.layers[0].plots[0].size_from.is_some(), "the Z column");
}
