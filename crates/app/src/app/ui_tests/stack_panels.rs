use super::*;

fn counts(h: &mut Harness<'static, SheafApp>, text: &str) {
    h.get_by_label("Number of Plots in Each Layer").focus();
    settle(h);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.event(egui::Event::Text(text.into()));
    settle(h);
}

fn number(h: &mut Harness<'static, SheafApp>, name: &str, value: &str) {
    let at = h.get_by_label(name).rect().center();
    double_click_at(h, at);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.event(egui::Event::Text(value.into()));
    h.key_press(egui::Key::Enter);
    settle(h);
}

fn all_columns(h: &mut Harness<'static, SheafApp>) {
    show_window(h, "Book1 - sample.csv");
    ctrl(h, egui::Key::A);
}

#[test]
fn stack_menu_builds_linked_portrait_panels_only_on_ok_and_undo_redo_are_one_step() {
    let mut h = start(vec![data_file("stack-menu")]);
    let (book, _) = window_ids(&h);
    all_columns(&mut h);
    let before = project(&h).workbook(book).unwrap().clone();
    let ys: Vec<_> = before.sheet().y_columns().into_iter().map(|i| before.sheet().columns[i].id).collect();
    let revision = h.state().doc.revision();
    menu_path(&mut h, &["Plot", "Multi-Panel", "Stack..."]);
    assert!(h.state().stack_panels.is_some());
    assert_eq!(h.state().doc.revision(), revision);
    assert_eq!(project(&h).graphs().count(), 1);
    click(&mut h, "OK");
    assert!(h.state().stack_panels.is_none());
    let g = project(&h).graphs().last().unwrap();
    assert_eq!(h.state().active, Some(g.id));
    assert_eq!(g.layers.len(), 2);
    assert!(g.page.width_pt < g.page.height_pt);
    assert_eq!(g.layers.iter().map(|l| l.plots[0].y).collect::<Vec<_>>(), ys);
    assert!(g.page_frame(0).top > g.page_frame(1).top);
    assert_eq!(g.layers[1].link.unwrap().x, ScaleLink::Straight);
    assert_eq!(g.layers[1].link.unwrap().y, ScaleLink::None);
    assert!(g.layers[0].x_axis.show_tick_labels);
    assert!(!g.layers[1].x_axis.show_tick_labels);
    assert!(
        g.layers.iter().all(|l| l.legend.visible && l.legend.frame && l.legend.position == LegendPosition::TopLeft)
    );
    assert_eq!(project(&h).workbook(book).unwrap(), &before);
    assert_eq!(h.state().doc.undo_label(), Some("Stack panels"));
    menu(&mut h, "Edit", "Undo Stack panels");
    assert_eq!(project(&h).graphs().count(), 1);
    menu(&mut h, "Edit", "Redo Stack panels");
    assert_eq!(project(&h).graphs().count(), 2);
}

#[test]
fn stack_assignment_scatter_order_and_legend_use_the_existing_panel_model() {
    let mut h = start(vec![data_file("stack-groups")]);
    let (book, _) = window_ids(&h);
    h.state_mut().doc.apply("More curves", |p| {
        for n in 0..3 {
            p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![n as f64 + 1.0; 4])).unwrap();
        }
    });
    settle(&mut h);
    all_columns(&mut h);
    let ys: Vec<_> = sheet(&h).y_columns().into_iter().map(|i| sheet(&h).columns[i].id).collect();
    menu_path(&mut h, &["Plot", "Multi-Panel", "Stack..."]);
    counts(&mut h, "1 2");
    click(&mut h, "Plot Type");
    click(&mut h, "Scatter");
    click(&mut h, "Layer Order");
    click(&mut h, "Top to Bottom");
    click(&mut h, "Legend");
    click(&mut h, "No Legend");
    click(&mut h, "OK");
    let g = project(&h).graphs().last().unwrap();
    assert_eq!(g.layers.iter().map(|l| l.plots.len()).collect::<Vec<_>>(), [1, 2, 2]);
    assert_eq!(g.layers.iter().flat_map(|l| l.plots.iter().map(|p| p.y)).collect::<Vec<_>>(), ys);
    assert!(g.layers.iter().flat_map(|l| &l.plots).all(|p| p.kind == PlotKind::Scatter));
    assert!(g.layers.iter().all(|l| !l.legend.visible));
    assert!(g.page_frame(0).top < g.page_frame(2).top);
}

#[test]
fn stack_keeps_selected_rows_and_applies_gap_margin_and_independent_axes() {
    let mut h = start(vec![data_file("stack-rows")]);
    show_window(&mut h, "Book1 - sample.csv");
    select_block(&mut h, "4.5", "10.5");
    menu_path(&mut h, &["Plot", "Multi-Panel", "Stack..."]);
    click(&mut h, "Link Layers");
    number(&mut h, "Vertical Gap", "5");
    number(&mut h, "Left Margin", "20");
    click(&mut h, "Orientation");
    click(&mut h, "Landscape");
    click(&mut h, "OK");
    let g = project(&h).graphs().last().unwrap();
    assert_eq!(g.layers.len(), 2);
    assert!(g.page.width_pt > g.page.height_pt);
    assert!(
        g.layers.iter().all(|l| l.link.is_none() && l.x_axis.show_tick_labels),
        "{:?}",
        g.layers.iter().map(|l| (l.link, l.x_axis.show_tick_labels)).collect::<Vec<_>>()
    );
    assert!(g.layers.iter().flat_map(|l| &l.plots).all(|p| p.rows == Some((1, 3))));
    let f = g.page_frame(0);
    assert!((f.left - 0.2).abs() < 1e-9);
    assert!((f.width - 0.7).abs() < 1e-9);
    assert!((f.height - 0.375).abs() < 1e-9);
}

#[test]
fn stack_position_only_links_keep_independent_scales_and_page_frames() {
    let mut h = start(vec![data_file("stack-position-links")]);
    all_columns(&mut h);
    menu_path(&mut h, &["Plot", "Multi-Panel", "Stack..."]);
    click(&mut h, "X Axis Link");
    click(&mut h, "None");
    click(&mut h, "OK");
    let g = project(&h).graphs().last().unwrap();
    let link = g.layers[1].link.unwrap();
    assert_eq!(link.parent, 0);
    assert_eq!((link.x, link.y), (ScaleLink::None, ScaleLink::None));
    assert!(g.layers.iter().all(|l| l.x_axis.show_tick_labels));
    assert!((g.page_frame(0).top - 0.5).abs() < 1e-9);
    assert!((g.page_frame(1).top - 0.1).abs() < 1e-9);
    assert!((g.page_frame(1).height - 0.4).abs() < 1e-9);
}

#[test]
fn stack_invalid_assignment_or_geometry_and_cancel_escape_do_not_change_the_project() {
    let mut h = start(vec![data_file("stack-cancel")]);
    all_columns(&mut h);
    let before = project(&h).clone();
    let revision = h.state().doc.revision();
    menu_path(&mut h, &["Plot", "Multi-Panel", "Stack..."]);
    counts(&mut h, "0");
    assert!(h.get_by_label("OK").accesskit_node().is_disabled());
    counts(&mut h, "1");
    number(&mut h, "Left Margin", "90");
    number(&mut h, "Right Margin", "90");
    assert!(h.get_by_label("OK").accesskit_node().is_disabled());
    click(&mut h, "Cancel");
    assert!(h.state().stack_panels.is_none());
    assert_eq!(project(&h), &before);
    assert_eq!(h.state().doc.revision(), revision);
    menu_path(&mut h, &["Plot", "Multi-Panel", "Stack..."]);
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    assert!(h.state().stack_panels.is_none());
    assert_eq!(project(&h), &before);
    assert_eq!(h.state().doc.revision(), revision);
}
