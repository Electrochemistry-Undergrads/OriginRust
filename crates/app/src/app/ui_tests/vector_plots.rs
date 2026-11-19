//! Plot > Specialized > Vector XYAM and Vector XYXY (T107, graphing spec "Vector plots").

use super::*;
use sheaf_core::graph::PlotKind;
use sheaf_core::template::Template;
use sheaf_core::vector::VectorPosition;

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
