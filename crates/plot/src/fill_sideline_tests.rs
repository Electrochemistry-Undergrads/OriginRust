use super::*;
use crate::scene::Item;
use sheaf_core::graph::{Connect, FillUnderMode};
use sheaf_core::template::Template;
use sheaf_core::{Color, ColumnData, ColumnId, Designation};

fn project(xs: Vec<f64>, ys: Vec<f64>) -> (Project, WindowId, ColumnId) {
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
    l.x_axis.max = Some(12.0);
    l.y_axis.min = Some(-1.0);
    l.y_axis.max = Some(2.0);
    let plot = &mut l.plots[0];
    plot.line.color = Color::RED;
    plot.line.width_pt = 6.0;
    plot.decor.fill_under = Some(Color::BLUE);
    (p, g, y)
}

fn mode(p: &mut Project, g: WindowId, m: FillUnderMode) {
    p.graph_mut(g).unwrap().layers[0].plots[0].decor.fill_under_mode = m;
}

fn pixel(r: &Raster, p: (f64, f64)) -> &[u8] {
    let i = (((p.1 * 2.0) as u32 * r.width + (p.0 * 2.0) as u32) * 4) as usize;
    &r.rgba[i..i + 4]
}

#[test]
fn fill_sidelines_match_origins_unfilled_line_colored_boundaries_and_bottom_base() {
    // Origin 2016 normal, verified_sides, verified_base: blue fill becomes red boundaries;
    // both modes end at the frame's -1 bottom, not the zero line inside the frame.
    let (mut p, g, y) = project(vec![2.0, 6.0, 10.0], vec![1.0, 1.5, 1.0]);
    let geometry = layout_geometry(&p, g).unwrap();
    let geom = &geometry[0];
    let interior = geom.data_to_page(6.0, 0.0);
    assert_eq!(pixel(&render_raster(&p, g, 2.0).unwrap(), interior), [0, 0, 255, 255]);
    mode(&mut p, g, FillUnderMode::SidelinesOnly);
    let r = render_raster(&p, g, 2.0).unwrap();
    assert_eq!(pixel(&r, interior), [220, 220, 220, 255]);
    for x in [2.0, 10.0] {
        assert_eq!(pixel(&r, geom.data_to_page(x, 0.0)), [255, 0, 0, 255]);
    }
    let base = geom.data_to_page(6.0, -1.0);
    let above_base = (base.0, base.1 - 1.0);
    assert_eq!(pixel(&r, above_base), [220, 220, 220, 255]);
    mode(&mut p, g, FillUnderMode::SidelinesAndBase);
    assert_eq!(pixel(&render_raster(&p, g, 2.0).unwrap(), above_base), [255, 0, 0, 255]);
    assert_eq!(layout_geometry(&p, g).unwrap(), geometry);
    assert_eq!(p.column(y).unwrap().numeric().unwrap(), [1.0, 1.5, 1.0]);
    p.graph_mut(g).unwrap().layers[0].plots[0].decor.fill_under = None;
    let disabled = render_svg(&p, g).unwrap();
    mode(&mut p, g, FillUnderMode::Normal);
    assert_eq!(render_svg(&p, g).unwrap(), disabled, "disabled mode preferences have no drawing effect");
}

#[test]
fn fill_sidelines_use_only_outer_endpoints_across_verified_missing_data_breaks() {
    // verified_missing has ISNA=1 at rows 3 and 5. Origin adds no boundary at x=2/9 or
    // the isolated x=6 point, only at outer x=1/11. Missing source values stay missing.
    let (mut p, g, y) =
        project(vec![1.0, 2.0, 4.0, 6.0, 8.0, 9.0, 11.0], vec![1.0, 1.5, f64::NAN, 1.25, f64::NAN, 1.5, 1.0]);
    mode(&mut p, g, FillUnderMode::SidelinesOnly);
    let geom = &layout_geometry(&p, g).unwrap()[0];
    let r = render_raster(&p, g, 2.0).unwrap();
    for x in [1.0, 11.0] {
        assert_eq!(pixel(&r, geom.data_to_page(x, 0.0)), [255, 0, 0, 255]);
    }
    for x in [2.0, 6.0, 9.0] {
        assert_eq!(pixel(&r, geom.data_to_page(x, 0.0)), [220, 220, 220, 255]);
    }
    assert_eq!(pixel(&r, geom.data_to_page(6.0, 1.25)), [220, 220, 220, 255]);
    let s = graph_scene(&p, g).unwrap();
    let lines: Vec<_> = s.layers[0]
        .plots
        .iter()
        .filter_map(|i| match i {
            Item::Path { d, fill: None, stroke: Some(st) } if st.color == Color::RED => Some(d),
            _ => None,
        })
        .collect();
    assert_eq!(lines.len(), 1, "one paint operation retains opacity at boundary joins");
    assert_eq!(lines[0].matches('M').count(), 2, "the missing gap stays open");
    assert!(p.column(y).unwrap().numeric().unwrap()[2].is_nan());
    assert!(p.column(y).unwrap().numeric().unwrap()[4].is_nan());
}

#[test]
fn fill_sidelines_preserve_transparency_at_connected_boundary_joins() {
    let (mut p, g, _) = project(vec![2.0, 6.0, 10.0], vec![1.0, 1.5, 1.0]);
    mode(&mut p, g, FillUnderMode::SidelinesAndBase);
    p.graph_mut(g).unwrap().layers[0].plots[0].line.color = Color::rgba(255, 0, 0, 128);
    let geom = &layout_geometry(&p, g).unwrap()[0];
    let r = render_raster(&p, g, 2.0).unwrap();
    let side = pixel(&r, geom.data_to_page(2.0, 0.0));
    assert!((108..=112).contains(&side[1]));
    assert_eq!(
        pixel(&r, geom.data_to_page(2.0, 1.0)),
        side,
        "joining the sideline does not paint the translucent line twice"
    );
    assert_eq!(pixel(&r, geom.data_to_page(6.0, 1.5)), side);
}

#[test]
fn fill_sidelines_follow_mask_visibility_without_changing_the_source() {
    let (mut p, g, y) = project(vec![2.0, 6.0, 10.0], vec![1.0, 1.5, 1.0]);
    mode(&mut p, g, FillUnderMode::SidelinesOnly);
    p.column_mut(y).unwrap().mask.insert(0);
    p.hide_masked = true;
    let geom = &layout_geometry(&p, g).unwrap()[0];
    let hidden = render_raster(&p, g, 2.0).unwrap();
    assert_eq!(pixel(&hidden, geom.data_to_page(2.0, 0.0)), [220, 220, 220, 255]);
    assert_eq!(pixel(&hidden, geom.data_to_page(6.0, 0.0)), [255, 0, 0, 255]);
    p.hide_masked = false;
    let shown = render_raster(&p, g, 2.0).unwrap();
    assert_eq!(pixel(&shown, geom.data_to_page(2.0, 0.0)), [255, 0, 0, 255]);
    assert_eq!(pixel(&shown, geom.data_to_page(6.0, 0.0)), [220, 220, 220, 255]);
    assert_eq!(p.column(y).unwrap().numeric().unwrap(), [1.0, 1.5, 1.0]);
}

#[test]
fn fill_sidelines_keep_each_connection_and_export_and_change_the_legend_to_a_line_sample() {
    let (mut p, g, _) = project(vec![2.0, 6.0, 10.0], vec![1.0, 1.5, 1.0]);
    mode(&mut p, g, FillUnderMode::SidelinesOnly);
    for c in [
        Connect::Straight,
        Connect::StepHorizontal,
        Connect::StepVertical,
        Connect::StepHorizontalCenter,
        Connect::StepVerticalCenter,
        Connect::Spline,
        Connect::BSpline,
        Connect::Bezier,
    ] {
        p.graph_mut(g).unwrap().layers[0].plots[0].line.connect = c;
        let svg = render_svg(&p, g).unwrap();
        assert!(!svg.contains("NaN") && !svg.contains("inf"), "{c:?}");
    }
    p.graph_mut(g).unwrap().layers[0].legend.visible = true;
    let blue_box = |p: &Project| {
        graph_scene(p, g).unwrap().layers[0]
            .overlay
            .iter()
            .any(|i| matches!(i,Item::Rect { fill:Some(c),.. } if *c==Color::BLUE))
    };
    assert!(!blue_box(&p));
    mode(&mut p, g, FillUnderMode::Normal);
    assert!(blue_box(&p));
    mode(&mut p, g, FillUnderMode::SidelinesAndBase);
    assert!(!blue_box(&p));
    assert!(render_png(&p, g, 72.0).unwrap().starts_with(b"\x89PNG"));
    assert!(render_pdf(&p, g).unwrap().starts_with(b"%PDF"));
    assert!(render_eps(&p, g).unwrap().starts_with(b"%!PS"));
    assert!(!render_emf(&p, g).unwrap().is_empty());
}
