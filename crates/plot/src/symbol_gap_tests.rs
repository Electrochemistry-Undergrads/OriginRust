use super::*;
use crate::scene::{Item, Scene};
use sheaf_core::graph::{Connect, Plot, PlotKind, SymbolShape};
use sheaf_core::template::Template;
use sheaf_core::{Color, ColumnData, ColumnId, Designation};

const BACKGROUND: Color = Color::rgb(220, 220, 220);

fn project(values: Vec<f64>) -> (Project, WindowId, ColumnId) {
    let mut p = Project::new();
    let b = p.add_workbook("Data");
    p.add_column(b, 0, Designation::X, ColumnData::numeric((1..=values.len()).map(|i| i as f64).collect())).unwrap();
    let y = p.add_column(b, 0, Designation::Y, ColumnData::numeric(values)).unwrap();
    let g = p.add_graph(Template::LineSymbol);
    p.add_plot(g, 0, y).unwrap();
    let layer = &mut p.graph_mut(g).unwrap().layers[0];
    layer.background = Some(BACKGROUND);
    layer.legend.visible = false;
    layer.x_axis.min = Some(0.0);
    layer.x_axis.max = Some(12.0);
    layer.y_axis.min = Some(0.0);
    layer.y_axis.max = Some(2.0);
    let plot = &mut layer.plots[0];
    plot.line.color = Color::RED;
    plot.line.width_pt = 3.0;
    plot.symbol.color = Color::BLUE;
    plot.symbol.shape = SymbolShape::Square;
    plot.symbol.filled = true;
    plot.symbol.size_pt = 20.0;
    plot.symbol.every = 3;
    (p, g, y)
}

fn plot(p: &mut Project, g: WindowId) -> &mut Plot {
    &mut p.graph_mut(g).unwrap().layers[0].plots[0]
}

fn sample(r: &Raster, p: (f64, f64)) -> &[u8] {
    let i = (((p.1 * 2.0) as u32 * r.width + (p.0 * 2.0) as u32) * 4) as usize;
    &r.rgba[i..i + 4]
}

fn lines(s: &Scene) -> String {
    s.layers[0]
        .plots
        .iter()
        .filter_map(|i| match i {
            Item::Path { d, fill: None, stroke: Some(st) } if st.color == Color::RED && st.width == 3.0 => {
                Some(d.as_str())
            }
            _ => None,
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
fn symbol_line_gaps_match_origins_skipped_marker_breaks_without_covering_the_background() {
    // Origin 2016 gaps_skip3 and gaps_no_break: holes at every row versus shown markers only.
    let (mut p, g, y) = project(vec![1.0; 11]);
    let geometry = layout_geometry(&p, g).unwrap();
    let page = |x| geometry[0].data_to_page(x, 1.0);
    let source = p.column(y).unwrap().numeric().unwrap().to_vec();
    let before = render_svg(&p, g).unwrap();
    plot(&mut p, g).decor.symbol_line.no_break_skipped = true;
    assert_eq!(render_svg(&p, g).unwrap(), before, "No Line Break alone has no effect");
    plot(&mut p, g).decor.symbol_line.no_break_skipped = false;
    plot(&mut p, g).decor.symbol_line.gap_to_symbol = true;
    let r = render_raster(&p, g, 2.0).unwrap();
    assert_eq!(sample(&r, page(2.0)), [220, 220, 220, 255], "skipped row still cuts the line");
    assert_eq!(sample(&r, page(2.5)), [255, 0, 0, 255]);
    assert_eq!(sample(&r, page(4.0)), [0, 0, 255, 255]);
    let (x, py) = page(4.0);
    assert_eq!(sample(&r, (x + 13.0, py)), [220, 220, 220, 255], "clearance beyond the blue marker");
    assert_eq!(hit::hit_test(&graph_scene(&p, g).unwrap(), page(2.0), 0.1), hit::Hit::Layer(0));
    p.column_mut(y).unwrap().mask.insert(1);
    assert_eq!(
        sample(&render_raster(&p, g, 2.0).unwrap(), page(2.0)),
        [220, 220, 220, 255],
        "mask overlay retains gaps"
    );
    plot(&mut p, g).decor.symbol_line.no_break_skipped = true;
    assert_eq!(sample(&render_raster(&p, g, 2.0).unwrap(), page(2.0)), [255, 0, 0, 255]);
    assert_eq!(layout_geometry(&p, g).unwrap(), geometry);
    assert_eq!(p.column(y).unwrap().numeric().unwrap(), source);
    plot(&mut p, g).kind = PlotKind::Line;
    let enabled = render_svg(&p, g).unwrap();
    plot(&mut p, g).decor.symbol_line.gap_to_symbol = false;
    assert_eq!(render_svg(&p, g).unwrap(), enabled, "only Line + Symbol uses symbol gaps");
}

#[test]
fn symbol_line_gaps_retain_raw_skipped_peaks_range_phase_and_missing_data_breaks() {
    let (mut p, g, y) = project(vec![1.0, 1.5, 0.5, 1.0, 1.5, 0.5, 1.0, 1.5, 0.5, 1.0, 1.5]);
    plot(&mut p, g).decor.symbol_line.gap_to_symbol = true;
    plot(&mut p, g).decor.symbol_line.no_break_skipped = true;
    plot(&mut p, g).rows = Some((2, 10));
    let s = graph_scene(&p, g).unwrap();
    let vertices = hit::path_points(&lines(&s));
    let geom = &layout_geometry(&p, g).unwrap()[0];
    let close = |a: (f64, f64), b: (f64, f64)| (a.0 - b.0).hypot(a.1 - b.1) < 0.002;
    assert!(vertices.iter().any(|&v| close(v, geom.data_to_page(5.0, 1.5))), "raw skipped peak remains");
    assert!(!vertices.iter().any(|&v| close(v, geom.data_to_page(6.0, 0.5))), "range starts its own marker phase");
    p.column_mut(y).unwrap().numeric_mut().unwrap()[4] = f64::NAN;
    let r = render_raster(&p, g, 2.0).unwrap();
    assert_eq!(sample(&r, geom.data_to_page(5.0, 1.25)), [220, 220, 220, 255], "a missing row does not get bridged");
}

#[test]
fn symbol_line_gaps_handle_overlapping_dense_symbols_and_each_connection_and_export() {
    let (mut p, g, _) = project(vec![1.0; 20_001]);
    let layer = &mut p.graph_mut(g).unwrap().layers[0];
    layer.x_axis.max = Some(20_002.0);
    layer.plots[0].decor.symbol_line.gap_to_symbol = true;
    assert!(lines(&graph_scene(&p, g).unwrap()).is_empty(), "overlapping holes leave no connecting line");
    plot(&mut p, g).symbol.every = 4000;
    plot(&mut p, g).decor.symbol_line.no_break_skipped = true;
    assert!(!lines(&graph_scene(&p, g).unwrap()).is_empty());

    let (mut p, g, _) = project(vec![1.0, 1.5, 0.5, 1.0]);
    plot(&mut p, g).decor.symbol_line.gap_to_symbol = true;
    plot(&mut p, g).symbol.every = 1;
    for connect in [
        Connect::Straight,
        Connect::StepHorizontal,
        Connect::StepVertical,
        Connect::Spline,
        Connect::BSpline,
        Connect::Bezier,
    ] {
        plot(&mut p, g).line.connect = connect;
        let d = lines(&graph_scene(&p, g).unwrap());
        assert!(!d.is_empty() && !d.contains("NaN") && !d.contains("inf"), "{connect:?}");
        if matches!(connect, Connect::Straight | Connect::StepHorizontal | Connect::StepVertical | Connect::Spline) {
            assert!(d.matches('M').count() > 1, "{connect:?} has actual path breaks");
        }
        let geom = &layout_geometry(&p, g).unwrap()[0];
        for v in hit::path_points(&d) {
            for (x, y) in [(1.0, 1.0), (2.0, 1.5), (3.0, 0.5), (4.0, 1.0)] {
                let centre = geom.data_to_page(x, y);
                assert!((v.0 - centre.0).hypot(v.1 - centre.1) >= 16.0, "{connect:?} respects marker clearance");
            }
        }
    }
    assert!(render_svg(&p, g).unwrap().contains("#FF0000"));
    assert!(render_png(&p, g, 72.0).unwrap().starts_with(b"\x89PNG"));
    assert!(render_pdf(&p, g).unwrap().starts_with(b"%PDF"));
    assert!(render_eps(&p, g).unwrap().starts_with(b"%!PS"));
    assert!(!render_emf(&p, g).unwrap().is_empty());
}

#[test]
fn symbol_line_gaps_use_the_same_diagonal_clearance_for_square_and_circular_markers() {
    // Origin clean_square/clean_circle: three points (2,.25), (6,1), (10,1.75).
    // Changing shape leaves all 2,031 red line pixels identical in the 900 px exports.
    let (mut p, g, _) = project(vec![0.25, 1.0, 1.75]);
    let x = plot(&mut p, g).x.unwrap();
    p.column_mut(x).unwrap().numeric_mut().unwrap().copy_from_slice(&[2.0, 6.0, 10.0]);
    plot(&mut p, g).symbol.every = 1;
    plot(&mut p, g).decor.symbol_line.gap_to_symbol = true;
    let square = lines(&graph_scene(&p, g).unwrap());
    assert_eq!(square.matches('M').count(), 2, "separate gaps at the three measured positions");
    plot(&mut p, g).symbol.shape = SymbolShape::Circle;
    assert_eq!(lines(&graph_scene(&p, g).unwrap()), square);
}

#[test]
fn symbol_line_gaps_clip_intersections_and_coincident_centres_without_invalid_coordinates() {
    let gaps = symbol_gaps::Gaps::new(
        std::iter::repeat_n((0.0, 0.0), 20_001).chain([(f64::NAN, 0.0), (f64::INFINITY, 0.0)]),
        10.0,
    )
    .unwrap();
    let mut d = String::new();
    gaps.write_polyline(&[(-30.0, 0.0), (-20.0, 0.0), (0.0, 0.0), (20.0, 0.0)], &mut d);
    assert_eq!(d.matches('M').count(), 2);
    assert_eq!(hit::path_points(&d), [(-30.0, 0.0), (-20.0, 0.0), (-10.0, 0.0), (10.0, 0.0), (20.0, 0.0)]);
    d.clear();
    gaps.write_polyline(&[(-20.0, 10.0), (0.0, 10.0), (20.0, 10.0)], &mut d);
    assert_eq!(d.matches('M').count(), 1, "a tangent retains its continuous join");
    for radius in [0.0, -1.0, f64::NAN, f64::INFINITY, f64::MAX] {
        assert!(symbol_gaps::Gaps::new([(0.0, 0.0)], radius).is_none());
    }
}
