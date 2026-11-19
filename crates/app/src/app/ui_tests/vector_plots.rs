//! Plot > Specialized > Vector XYAM and Vector XYXY (T107, graphing spec "Vector plots").

use super::*;
use sheaf_core::graph::PlotKind;
use sheaf_core::template::Template;
use sheaf_core::vector::{VectorAngleConvention, VectorPosition};

#[test]
fn vector_xyam_plots_the_selected_y_angle_and_magnitude_columns() {
    let mut h = start(vec![data_file("vector-plots")]);
    show_window(&mut h, "Book1 - sample.csv");
    let book = h.state().books.keys().next().copied().unwrap();
    let ctx = h.ctx.clone();
    // A fourth column for the magnitudes.
    h.state_mut().doc.apply("Add", |p| {
        p.add_column(book, 0, sheaf_core::Designation::Y, sheaf_core::ColumnData::numeric(vec![1.0, 2.0, 3.0, 4.0]))
            .unwrap();
    });
    let cols: Vec<_> = project(&h).workbook(book).unwrap().sheet().columns.iter().map(|c| c.id).collect();
    h.state_mut().apply(
        Action::PlotColumns { book, columns: cols.clone(), template: sheaf_core::template::Template::VectorXyam },
        &ctx,
    );
    settle(&mut h);
    let g = project(&h).graphs().last().unwrap();
    let plot = &g.layers[0].plots[0];
    assert_eq!(plot.kind, PlotKind::VectorXyam);
    let v = plot.decor.vector.unwrap();
    assert_eq!((plot.y, v.a, v.b), (cols[1], cols[2], cols[3]));
}

#[test]
fn vector_details_edits_position_and_arrow_style_with_live_preview_and_undo() {
    let mut h = start(vec![data_file("vector-details")]);
    let book = project(&h).workbooks().next().unwrap().id;
    let mut id = sheaf_core::WindowId(0);
    h.state_mut().doc.apply("Make vector plot", |p| {
        let cols: Vec<_> = p.workbook(book).unwrap().sheet().columns.iter().skip(1).map(|c| c.id).collect();
        let mag = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![50.0, 100.0, 100.0])).unwrap();
        id = p.plot_columns(Template::VectorXyam, &[cols[0], cols[1], mag]).unwrap();
    });
    settle(&mut h);
    let ctx = h.ctx.clone();
    let open = |h: &mut Harness<'static, SheafApp>| {
        h.state_mut().apply(Action::GraphProperties { graph: id, layer: 0, tab: PropsTab::Plots, plot: Some(0) }, &ctx);
        settle(h);
        click(h, "Vector");
    };
    let plot = |h: &Harness<'static, SheafApp>| project(h).graph(id).unwrap().layers[0].plots[0].clone();
    let before = plot(&h);
    open(&mut h);
    assert_eq!(plot(&h), before);
    click(&mut h, "Midpoint");
    click(&mut h, "Open");
    assert_eq!(plot(&h).decor.vector.unwrap().position, VectorPosition::Midpoint);
    assert!(plot(&h).decor.vector.unwrap().open_head);
    click(&mut h, "Cancel");
    assert_eq!(plot(&h), before);

    open(&mut h);
    click(&mut h, "Head");
    click(&mut h, "Degrees");
    type_into_number(&mut h, 0, "1.5");
    type_into_number(&mut h, 1, "8");
    type_into_number(&mut h, 2, "45");
    type_into_number(&mut h, 3, "0.5");
    click(&mut h, "OK");
    let edited = plot(&h);
    let spec = edited.decor.vector.unwrap();
    assert_eq!((spec.position, spec.degrees), (VectorPosition::Head, true));
    assert_eq!((edited.line.width_pt, spec.head_pt, spec.head_angle_deg, spec.multiplier), (1.5, 8.0, 45.0, 0.5));
    menu(&mut h, "Edit", "Undo Change graph settings");
    assert_eq!(plot(&h), before);
}

#[test]
fn xyam_vector_convention_shows_presets_and_edits_the_custom_offset() {
    use VectorAngleConvention::{Custom, Meteorological, Polar};

    let mut h = start(vec![data_file("vector-conventions")]);
    let book = project(&h).workbooks().next().unwrap().id;
    let mut id = WindowId(0);
    h.state_mut().doc.apply("Make vector plot", |p| {
        let columns = &p.workbook(book).unwrap().sheet().columns;
        let (y, angle) = (columns[1].id, columns[2].id);
        let magnitude = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![10.0; 4])).unwrap();
        id = p.plot_columns(Template::VectorXyam, &[y, angle, magnitude]).unwrap();
    });
    settle(&mut h);
    let ctx = h.ctx.clone();
    let before = project(&h).graph(id).unwrap().clone();
    h.state_mut().apply(Action::GraphProperties { graph: id, layer: 0, tab: PropsTab::Plots, plot: Some(0) }, &ctx);
    settle(&mut h);
    click(&mut h, "Vector");
    let convention = |h: &Harness<'static, SheafApp>| {
        project(h).graph(id).unwrap().layers[0].plots[0].decor.vector.unwrap().convention
    };
    assert_eq!(convention(&h), Polar);
    assert!(is_disabled(&h, "CW") && is_disabled(&h, "CCW"));
    click(&mut h, "Meteorological");
    assert_eq!(convention(&h), Meteorological);
    assert!(is_disabled(&h, "CW") && is_disabled(&h, "CCW"));
    click(&mut h, "Custom");
    assert_eq!(convention(&h), Custom { offset_deg: 90.0, clockwise: true });
    type_into_number(&mut h, 4, "45");
    click(&mut h, "CCW");
    click(&mut h, "OK");
    let mut expected = before.clone();
    expected.layers[0].plots[0].decor.vector.as_mut().unwrap().convention =
        Custom { offset_deg: 45.0, clockwise: false };
    assert_eq!(project(&h).graph(id), Some(&expected));
    menu(&mut h, "Edit", "Undo Change graph settings");
    assert_eq!(project(&h).graph(id), Some(&before));
}

#[test]
fn xyam_real_world_magnitude_previews_and_undoes_the_mode() {
    let mut h = start(vec![data_file("vector-world-magnitude")]);
    let book = project(&h).workbooks().next().unwrap().id;
    let mut id = WindowId(0);
    h.state_mut().doc.apply("Make vector plot", |p| {
        let columns = &p.workbook(book).unwrap().sheet().columns;
        let (y, angle) = (columns[1].id, columns[2].id);
        let magnitude = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![10.0; 4])).unwrap();
        id = p.plot_columns(Template::VectorXyam, &[y, angle, magnitude]).unwrap();
    });
    settle(&mut h);
    let before = project(&h).graph(id).unwrap().clone();
    let before_svg = sheaf_plot::render_svg(project(&h), id).unwrap();
    let ctx = h.ctx.clone();
    h.state_mut().apply(Action::GraphProperties { graph: id, layer: 0, tab: PropsTab::Plots, plot: Some(0) }, &ctx);
    settle(&mut h);
    click(&mut h, "Vector");
    assert_eq!(project(&h).graph(id), Some(&before));

    click(&mut h, "Magnitude in real world space");
    let mut expected = before.clone();
    expected.layers[0].plots[0].decor.vector.as_mut().unwrap().real_world_space = true;
    assert_eq!(project(&h).graph(id), Some(&expected));
    assert_ne!(sheaf_plot::render_svg(project(&h), id).unwrap(), before_svg);
    click(&mut h, "OK");
    assert_eq!(project(&h).graph(id), Some(&expected));
    menu(&mut h, "Edit", "Undo Change graph settings");
    assert_eq!(project(&h).graph(id), Some(&before));
}

#[test]
fn xyam_vector_data_rebinds_the_angle_and_magnitude_columns() {
    check_vector_data_selectors(Template::VectorXyam, "vector-xyam-data", "Vector Data");
}

#[test]
fn xyxy_vector_endpoints_rebind_the_x_and_y_columns() {
    check_vector_data_selectors(Template::VectorXyxy, "vector-xyxy-data", "End Point");
}

fn check_vector_data_selectors(template: Template, fixture: &str, heading: &str) {
    let path = data_file(fixture);
    let saved = path.with_file_name("vectors.sheaf");
    let mut h = start(vec![path]);
    let book = project(&h).workbooks().next().unwrap().id;
    let mut graph_id = WindowId(0);
    let mut new_columns = None;
    h.state_mut().doc.apply("Make vector plot", |p| {
        let sheet = &p.workbook(book).unwrap().sheets[0];
        let (y, a) = (sheet.columns[1].id, sheet.columns[2].id);
        let b = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![1.0, 2.0, 3.0, 4.0])).unwrap();
        let new_a = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![5.0, 6.0, 7.0, 8.0])).unwrap();
        let new_b = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![9.0, 10.0, 11.0, 12.0])).unwrap();
        new_columns = Some((new_a, new_b));
        p.add_column(book, 0, Designation::Label, ColumnData::text(vec!["text".into()])).unwrap();
        p.workbook_mut(book).unwrap().sheets.push(sheaf_core::Worksheet::new("Other sheet"));
        let other = p.add_column(book, 1, Designation::Y, ColumnData::numeric(vec![0.0])).unwrap();
        p.column_mut(other).unwrap().short_name = "Other".into();
        if template == Template::VectorXyam {
            graph_id = p.plot_columns(template, &[y, a, b]).unwrap();
        } else {
            p.column_mut(a).unwrap().designation = Designation::X;
            graph_id = p.plot_columns(template, &[y, b]).unwrap();
        }
    });
    settle(&mut h);
    let ctx = h.ctx.clone();
    let open = |h: &mut Harness<'static, SheafApp>| {
        h.state_mut()
            .apply(Action::GraphProperties { graph: graph_id, layer: 0, tab: PropsTab::Plots, plot: Some(0) }, &ctx);
        settle(h);
        click(h, "Vector");
        assert!(has_label(h, heading));
    };
    let rebind = |h: &mut Harness<'static, SheafApp>| {
        h.get_by_value("Col(C)").click();
        settle(h);
        assert!(h.query_by_label("Col(G)").is_none(), "text column");
        assert!(h.query_by_label("Col(Other)").is_none(), "different sheet");
        click(h, "Col(E)");
        h.get_by_value("Col(D)").click();
        settle(h);
        click(h, "Col(F)");
    };

    let before = project(&h).graph(graph_id).unwrap().clone();
    let before_svg = sheaf_plot::render_svg(project(&h), graph_id).unwrap();
    let mut expected = before.clone();
    let spec = expected.layers[0].plots[0].decor.vector.as_mut().unwrap();
    (spec.a, spec.b) = new_columns.unwrap();

    open(&mut h);
    rebind(&mut h);
    assert_eq!(project(&h).graph(graph_id), Some(&expected));
    assert_ne!(sheaf_plot::render_svg(project(&h), graph_id).unwrap(), before_svg);
    click(&mut h, "Cancel");
    assert_eq!(project(&h).graph(graph_id), Some(&before));

    open(&mut h);
    rebind(&mut h);
    click(&mut h, "Apply");
    click(&mut h, "Cancel");
    assert_eq!(project(&h).graph(graph_id), Some(&expected));
    sheaf_io::save::save_project(project(&h), &saved).unwrap();
    let reopened = sheaf_io::open_project(&saved).unwrap().project;
    assert_eq!(reopened.graph(graph_id), Some(&expected));
    menu(&mut h, "Edit", "Undo Change graph settings");
    assert_eq!(project(&h).graph(graph_id), Some(&before));
}
