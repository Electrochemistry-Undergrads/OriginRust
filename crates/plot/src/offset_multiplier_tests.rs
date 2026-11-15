use super::*;
use crate::scene::{Item, ObjectKind};
use sheaf_core::graph::{LabelForm, Stack};
use sheaf_core::template::Template;
use sheaf_core::{Color, ColumnData, Designation};

fn project() -> (Project, WindowId) {
    let mut p = Project::new();
    let b = p.add_workbook("Data");
    let x = p.add_column(b, 0, Designation::X, ColumnData::numeric(vec![1.0, 2.0, 3.0])).unwrap();
    let y = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![2.0, 4.0, 6.0])).unwrap();
    let ey = p.add_column(b, 0, Designation::YError, ColumnData::numeric(vec![1.0; 3])).unwrap();
    let ex = p.add_column(b, 0, Designation::XError, ColumnData::numeric(vec![0.5; 3])).unwrap();
    let g = p.add_graph(Template::LineSymbol);
    p.add_plot_xy(g, 0, Some(x), y, Some(ey)).unwrap();
    let layer = &mut p.graph_mut(g).unwrap().layers[0];
    layer.stack = Stack::Individual;
    layer.legend.visible = false;
    let o = layer.plot_offset_mut(y);
    (o.x, o.y, o.x_multiplier, o.y_multiplier) = (5.0, 7.0, 2.0, 3.0);
    let plot = &mut layer.plots[0];
    plot.x_error = Some(ex);
    plot.symbol.color = Color::RED;
    plot.symbol.filled = true;
    plot.symbol.size_pt = 8.0;
    plot.decor.labels.enable = true;
    plot.decor.labels.form = LabelForm::XY;
    (p, g)
}

#[test]
fn origin_affine_placement_keeps_error_lengths_and_source_label_values() {
    // Origin 2016, scratch/t13-offset-multipliers/probe4/5: X'=2X+5, Y'=3Y+7;
    // errors remain +/-0.5 and +/-1, labels remain the original worksheet values.
    let (p, g) = project();
    let layer = &p.graph(g).unwrap().layers[0];
    let d = data::collect(&p, layer);
    let pd = &d.plots[0];
    assert_eq!((0..3).map(|i| (pd.x_at(i), pd.y_at(i))).collect::<Vec<_>>(), [(7.0, 13.0), (9.0, 19.0), (11.0, 25.0)]);
    assert_eq!(data::axis_range(&d.plots, data::Dim::X, ScaleKind::Linear), Some((6.5, 11.5)));
    assert_eq!(data::axis_range(&d.plots, data::Dim::Y, ScaleKind::Linear), Some((12.0, 26.0)));
    let scene = graph_scene(&p, g).unwrap();
    let labels: Vec<_> = scene.layers[0]
        .plots
        .iter()
        .filter_map(|i| match i {
            Item::Text(t) => Some(t.text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(labels, ["(1, 2)", "(2, 4)", "(3, 6)"]);
    let geometry = layout_geometry(&p, g).unwrap();
    let paths: Vec<_> = scene.layers[0]
        .plots
        .iter()
        .filter_map(|i| match i {
            Item::Path { d, .. } => Some(hit::path_points(d)),
            _ => None,
        })
        .flatten()
        .collect();
    for end in [(7.0, 12.0), (7.0, 14.0), (6.5, 13.0), (7.5, 13.0)] {
        let at = geometry[0].data_to_page(end.0, end.1);
        assert!(
            paths.iter().any(|p| (p.0 - at.0).abs() < 0.01 && (p.1 - at.1).abs() < 0.01),
            "unscaled error endpoint {end:?}"
        );
    }
}

#[test]
fn scaled_symbols_are_rasterized_and_picked_at_the_displayed_coordinates() {
    let (p, g) = project();
    let at = layout_geometry(&p, g).unwrap()[0].data_to_page(9.0, 19.0);
    let scene = graph_scene(&p, g).unwrap();
    assert_eq!(hit::hit_test(&scene, at, 0.2), hit::Hit::Object { layer: 0, kind: ObjectKind::Plot(0) });
    let r = render_raster(&p, g, 2.0).unwrap();
    let i = (((at.1 * 2.0) as u32 * r.width + (at.0 * 2.0) as u32) * 4) as usize;
    assert_eq!(&r.rgba[i..i + 4], &[255, 0, 0, 255]);
    assert!(render_svg(&p, g).unwrap().contains("(2, 4)"));
}

#[test]
fn scaling_applies_before_log_axes_and_respects_ranges_and_missing_values() {
    let (mut p, g) = project();
    let y = p.graph(g).unwrap().layers[0].plots[0].y;
    p.column_mut(y).unwrap().data = ColumnData::numeric(vec![f64::NAN, 4.0, 6.0]);
    p.graph_mut(g).unwrap().layers[0].plots[0].rows = Some((1, 3));
    let d = data::collect(&p, &p.graph(g).unwrap().layers[0]);
    assert_eq!(data::axis_range(&d.plots, data::Dim::Y, ScaleKind::Log10), Some((18.0, 26.0)));
    let y_source = p.numeric(y).unwrap();
    assert!(y_source[0].is_nan());
    assert_eq!(&y_source[1..], &[4.0, 6.0]);
}
