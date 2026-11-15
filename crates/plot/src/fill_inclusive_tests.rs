use super::*;
use sheaf_core::graph::{Connect, FillUnderMode};
use sheaf_core::template::Template;
use sheaf_core::{Color, ColumnData, ColumnId, Designation};

fn project(xs: Vec<f64>, ys: Vec<f64>, xmax: f64) -> (Project, WindowId, ColumnId) {
    let mut p = Project::new();
    let b = p.add_workbook("Data");
    p.add_column(b, 0, Designation::X, ColumnData::numeric(xs)).unwrap();
    let y = p.add_column(b, 0, Designation::Y, ColumnData::numeric(ys)).unwrap();
    let g = p.add_graph(Template::Line);
    p.add_plot(g, 0, y).unwrap();
    let l = &mut p.graph_mut(g).unwrap().layers[0];
    l.background = Some(Color::rgb(220, 220, 220));
    l.legend.visible = false;
    l.x_axis.min = Some(0.0);
    l.x_axis.max = Some(xmax);
    l.y_axis.min = Some(-1.0);
    l.y_axis.max = Some(4.0);
    let plot = &mut l.plots[0];
    plot.line.color = Color::RED;
    plot.line.width_pt = 6.0;
    plot.decor.fill_under = Some(Color::BLUE);
    plot.decor.fill_under_mode = FillUnderMode::InclusiveBrokenByMissingValues;
    (p, g, y)
}

fn pixel(r: &Raster, p: (f64, f64)) -> &[u8] {
    let i = (((p.1 * 2.0) as u32 * r.width + (p.0 * 2.0) as u32) * 4) as usize;
    &r.rgba[i..i + 4]
}

#[test]
fn inclusive_fill_uses_the_sloping_endpoint_chord_instead_of_the_frame_bottom() {
    let (mut p, g, y) = project(vec![1.0, 5.0, 10.0], vec![1.0, 3.0, 2.0], 12.0);
    let geometry = layout_geometry(&p, g).unwrap();
    let geom = &geometry[0];
    let filled = geom.data_to_page(5.0, 2.0);
    let below = geom.data_to_page(5.0, 0.0);
    let r = render_raster(&p, g, 2.0).unwrap();
    assert_eq!(pixel(&r, filled), [0, 0, 255, 255]);
    assert_eq!(pixel(&r, below), [220, 220, 220, 255]);
    assert_eq!(
        pixel(&r, geom.data_to_page(5.5, 1.5)),
        [255, 0, 0, 255],
        "the endpoint chord uses the connecting stroke"
    );
    assert_eq!(pixel(&r, geom.data_to_page(1.0, 0.0)), [220, 220, 220, 255], "no outline sideline");
    p.graph_mut(g).unwrap().layers[0].plots[0].decor.fill.transparency_pct = 50;
    let r = render_raster(&p, g, 2.0).unwrap();
    let c = pixel(&r, filled);
    assert!((109..=111).contains(&c[0]) && (109..=111).contains(&c[1]) && (236..=239).contains(&c[2]));
    assert_eq!(pixel(&r, below), [220, 220, 220, 255]);
    assert_eq!(layout_geometry(&p, g).unwrap(), geometry);
    assert_eq!(p.column(y).unwrap().numeric().unwrap(), [1.0, 3.0, 2.0]);
    p.graph_mut(g).unwrap().layers[0].legend.visible = true;
    assert!(
        graph_scene(&p, g).unwrap().layers[0]
            .overlay
            .iter()
            .any(|item| { matches!(item, crate::scene::Item::Rect { fill: Some(c), .. } if *c == Color::BLUE) }),
        "a filled mode keeps the filled legend sample"
    );
    let plot = &mut p.graph_mut(g).unwrap().layers[0].plots[0];
    plot.decor.fill_under = Some(Color::rgba(0, 0, 255, 0));
    plot.line.color = Color::rgba(255, 0, 0, 128);
    let r = render_raster(&p, g, 2.0).unwrap();
    let chord = pixel(&r, geom.data_to_page(5.5, 1.5));
    assert!((109..=111).contains(&chord[1]));
    assert_eq!(pixel(&r, geom.data_to_page(1.0, 1.0)), chord, "closing the run does not double paint its join");
    assert_eq!(pixel(&r, geom.data_to_page(5.0, 3.0)), chord);
}

#[test]
fn inclusive_fill_closes_each_missing_run_and_keeps_an_unfilled_isolated_point() {
    let (p, g, y) = project(
        (1..=13).map(f64::from).collect(),
        vec![1.0, 3.0, 1.0, f64::NAN, 2.0, 3.0, 1.0, f64::NAN, 2.0, f64::NAN, 1.0, 3.0, 2.0],
        14.0,
    );
    let geom = &layout_geometry(&p, g).unwrap()[0];
    let r = render_raster(&p, g, 2.0).unwrap();
    for (x, y) in [(2.0, 2.0), (6.0, 2.0), (12.0, 2.0)] {
        assert_eq!(pixel(&r, geom.data_to_page(x, y)), [0, 0, 255, 255]);
    }
    for (x, y) in [(2.0, 0.0), (6.0, 1.0), (9.0, 1.0), (4.0, 2.0), (8.0, 2.0), (10.0, 2.0)] {
        assert_eq!(pixel(&r, geom.data_to_page(x, y)), [220, 220, 220, 255]);
    }
    assert_eq!(pixel(&r, geom.data_to_page(9.0, 2.0)), [255, 0, 0, 255], "the isolated run keeps its round stroke cap");
    let s = graph_scene(&p, g).unwrap();
    let paths: Vec<_> = s.layers[0]
        .plots
        .iter()
        .filter_map(|item| match item {
            crate::scene::Item::Path { d, fill: Some(c), stroke: None } if *c == Color::BLUE => Some(d),
            _ => None,
        })
        .collect();
    assert_eq!(paths.len(), 1);
    assert_eq!(paths[0].matches('M').count(), 3);
    assert_eq!(paths[0].matches('Z').count(), 3);
    for row in [3, 7, 9] {
        assert!(p.column(y).unwrap().numeric().unwrap()[row].is_nan());
    }
}

#[test]
fn inclusive_fill_keeps_closed_polygons_connections_and_vector_exports() {
    let (mut p, g, _) = project(vec![2.0, 2.0, 6.0, 6.0, 2.0], vec![1.0, 3.0, 3.0, 1.0, 1.0], 12.0);
    let geom = &layout_geometry(&p, g).unwrap()[0];
    let r = render_raster(&p, g, 2.0).unwrap();
    assert_eq!(pixel(&r, geom.data_to_page(4.0, 2.0)), [0, 0, 255, 255]);
    assert_eq!(pixel(&r, geom.data_to_page(4.0, 0.0)), [220, 220, 220, 255]);
    for c in [
        Connect::Straight,
        Connect::StepHorizontal,
        Connect::StepVertical,
        Connect::Spline,
        Connect::BSpline,
        Connect::Bezier,
    ] {
        p.graph_mut(g).unwrap().layers[0].plots[0].line.connect = c;
        let svg = render_svg(&p, g).unwrap();
        assert!(!svg.contains("NaN") && !svg.contains("inf"), "{c:?}");
    }
    assert!(render_pdf(&p, g).unwrap().starts_with(b"%PDF"));
    assert!(render_eps(&p, g).unwrap().starts_with(b"%!PS"));
    assert!(!render_emf(&p, g).unwrap().is_empty());
    p.graph_mut(g).unwrap().layers[0].plots[0].decor.fill_under = None;
    let disabled = render_svg(&p, g).unwrap();
    p.graph_mut(g).unwrap().layers[0].plots[0].decor.fill_under_mode = FillUnderMode::Normal;
    assert_eq!(render_svg(&p, g).unwrap(), disabled);
}
