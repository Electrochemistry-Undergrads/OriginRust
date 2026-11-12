//! Tests of the whole pipeline: project -> scene -> SVG / PNG / PDF.

use super::*;
use sheaf_core::graph::{Connect, PlotKind, ScaleKind, SymbolShape, TickDirection};
use sheaf_core::template::Template;
use sheaf_core::{Color, ColumnData, Designation};

/// A project with one workbook: X "Potential (V)" and the given Y columns.
fn project(template: Template, x: Vec<f64>, ys: Vec<Vec<f64>>) -> (Project, WindowId) {
    let mut p = Project::new();
    let book = p.add_workbook("data");
    let xc = p.add_column(book, 0, Designation::X, ColumnData::numeric(x)).unwrap();
    {
        let c = p.column_mut(xc).unwrap();
        c.long_name = "Potential".into();
        c.units = "V".into();
    }
    let g = p.add_graph(template);
    for (i, y) in ys.into_iter().enumerate() {
        let yc = p.add_column(book, 0, Designation::Y, ColumnData::numeric(y)).unwrap();
        let c = p.column_mut(yc).unwrap();
        c.long_name = format!("Current {}", i + 1);
        c.units = "mA".into();
        c.comments = format!("Scan {}", i + 1);
        p.add_plot(g, 0, yc).unwrap();
    }
    (p, g)
}

fn svg_of(p: &Project, g: WindowId) -> String {
    render_svg_with(p, g, &FixedMetrics).unwrap()
}

fn doc(svg: &str) -> roxmltree::Document<'_> {
    roxmltree::Document::parse(svg).expect("well-formed XML")
}

fn with_class<'a, 'b>(d: &'b roxmltree::Document<'a>, class: &str) -> Vec<roxmltree::Node<'a, 'b>> {
    d.descendants().filter(|n| n.attribute("class") == Some(class)).collect()
}

fn texts(d: &roxmltree::Document) -> Vec<String> {
    d.descendants().filter(|n| n.has_tag_name("text")).map(|n| n.text().unwrap_or("").to_string()).collect()
}

fn paths_in<'a, 'b>(n: roxmltree::Node<'a, 'b>) -> Vec<roxmltree::Node<'a, 'b>> {
    n.children().filter(|c| c.has_tag_name("path")).collect()
}

fn loop_data(n: usize, amp: f64) -> (Vec<f64>, Vec<f64>) {
    let x: Vec<f64> = (0..n).map(|i| -0.2 + 0.8 * (i as f64 / (n - 1) as f64)).collect();
    let y: Vec<f64> = x.iter().map(|v| amp * (6.0 * v).sin()).collect();
    (x, y)
}

#[test]
fn line_template_svg_is_well_formed_with_one_path_per_plot() {
    let x: Vec<f64> = (0..10).map(f64::from).collect();
    let y1: Vec<f64> = x.iter().map(|v| v * 2.0).collect();
    let mut y2 = y1.clone();
    y2[5] = f64::NAN; // a gap
    let (p, g) = project(Template::Line, x, vec![y1, y2]);
    let svg = svg_of(&p, g);
    usvg::Tree::from_str(&svg, &usvg::Options::default()).expect("usvg reads the SVG");
    let d = doc(&svg);

    let root = d.root_element();
    assert_eq!(root.attribute("width"), Some("460.8pt"));
    assert_eq!(root.attribute("viewBox"), Some("0 0 460.8 345.6"));

    let plots = with_class(&d, "plots");
    assert_eq!(plots.len(), 1);
    assert!(plots[0].attribute("clip-path").is_some(), "plots sit in a clip group");
    // First plot: one piece. Second plot: a gap gives two pieces.
    assert_eq!(paths_in(plots[0]).len(), 1 + 2);
    assert!(paths_in(plots[0]).iter().all(|p| p.attribute("fill") == Some("none")));
    // The clip rectangle is the frame: 0.16 * 460.8 = 73.728.
    let clip = d.descendants().find(|n| n.has_tag_name("clipPath")).unwrap();
    let rect = clip.first_element_child().unwrap();
    assert_eq!(rect.attribute("x"), Some("73.728"));
    assert_eq!(rect.attribute("width"), Some("359.424"));

    // Axis titles are built from long name and units.
    let t = texts(&d);
    assert!(t.contains(&"Potential (V)".to_string()), "{t:?}");
    assert!(t.contains(&"Current 1 (mA)".to_string()), "{t:?}");
    // Legend entries are the plot labels.
    for plot in &p.graph(g).unwrap().layers[0].plots {
        assert!(t.contains(&p.plot_label(plot)), "{t:?}");
    }
    // The Y title is rotated.
    assert!(svg.contains("rotate(-90 "));
}

#[test]
fn cv_template_has_a_closed_box_no_grid_and_a_symmetric_current_axis() {
    let (x, y1) = loop_data(200, 33.55);
    let (_, y2) = loop_data(200, 20.0);
    let (p, g) = project(Template::CyclicVoltammetry, x, vec![y1, y2]);
    let svg = svg_of(&p, g);
    let d = doc(&svg);
    assert!(with_class(&d, "grid").is_empty(), "no grid lines");

    let overlay = with_class(&d, "overlay")[0];
    let box_paths: Vec<_> =
        paths_in(overlay).into_iter().filter(|n| n.attribute("d").is_some_and(|d| d.ends_with('Z'))).collect();
    assert_eq!(box_paths.len(), 1, "one closed box path");
    assert_eq!(box_paths[0].attribute("fill"), Some("none"));
    assert_eq!(box_paths[0].attribute("stroke-width"), Some("1"));
    // The box has 4 corners: 1 move + 3 lines.
    assert_eq!(box_paths[0].attribute("d").unwrap().matches('L').count(), 3);

    let geom = &layout_geometry(&p, g).unwrap()[0];
    assert_eq!((geom.y.min, geom.y.max, geom.y.step), (-45.0, 45.0, 15.0));
    // Tight X axis: exactly the data range.
    assert!((geom.x.min + 0.2).abs() < 1e-12 && (geom.x.max - 0.6).abs() < 1e-12);

    // Splines are written as cubic Bezier segments.
    let plots = with_class(&d, "plots")[0];
    assert!(paths_in(plots)[0].attribute("d").unwrap().contains('C'));
    // Legend in the lower right corner, inside the frame.
    let t = texts(&d);
    assert!(t.contains(&"Scan 1".to_string()) && t.contains(&"Scan 2".to_string()));
}

#[test]
fn ticks_point_into_the_frame_for_the_cv_template() {
    let (x, y) = loop_data(50, 1.0);
    let (p, g) = project(Template::CyclicVoltammetry, x, vec![y]);
    let geom = &layout_geometry(&p, g).unwrap()[0];
    let scene = layout::build_scene(&p, g, &FixedMetrics).unwrap();
    let f = geom.frame;
    // Bottom tick path: all vertical lines from the axis line upwards (y decreasing).
    let ticks: Vec<&str> = scene.layers[0]
        .overlay
        .iter()
        .filter_map(|it| match it {
            scene::Item::Path { d, .. } if d.matches('M').count() > 2 => Some(d.as_str()),
            _ => None,
        })
        .collect();
    assert!(!ticks.is_empty());
    let first = ticks[0];
    let bottom = f.y + f.h;
    // Parse the first tick: "Mx y Lx y2": y2 < y for an inward bottom tick.
    let (m, l) = first.split_once('L').unwrap();
    let y0: f64 = m[1..].split(' ').nth(1).unwrap().parse().unwrap();
    let y1: f64 = l.split([' ', 'M']).nth(1).unwrap().parse().unwrap();
    assert!((y0 - bottom).abs() < 0.01);
    assert!(y1 < y0, "inward tick goes up");
    assert_eq!(geom.x.minor.len(), 0);
}

#[test]
fn labels_sit_outside_the_frame_without_overlap() {
    let (x, y) = loop_data(50, 1.0);
    let (p, g) = project(Template::Line, x, vec![y]);
    let scene = layout::build_scene(&p, g, &FixedMetrics).unwrap();
    let f = layout_geometry(&p, g).unwrap()[0].frame;
    let mut x_label_y = Vec::new();
    for it in &scene.layers[0].overlay {
        if let scene::Item::Text(t) = it {
            if t.anchor == scene::Anchor::End && t.rotate_ccw == 0.0 {
                assert!(t.x < f.x, "Y tick labels are left of the frame");
            }
            if t.rotate_ccw == 90.0 {
                assert!(t.x < f.x - 5.0);
            }
            if t.anchor == scene::Anchor::Middle && t.rotate_ccw == 0.0 {
                x_label_y.push(t.y);
            }
        }
    }
    // X tick labels (12 pt) and the X title (14 pt) are all below the frame, title lowest.
    assert!(x_label_y.iter().all(|y| *y > f.y + f.h));
    let title_y = *x_label_y.last().unwrap();
    assert!(x_label_y[..x_label_y.len() - 1].iter().all(|y| *y < title_y - 12.0));
}

#[test]
fn output_is_deterministic() {
    let (x, y) = loop_data(300, 3.0);
    let (p, g) = project(Template::CyclicVoltammetry, x, vec![y]);
    assert_eq!(svg_of(&p, g), svg_of(&p, g));
    assert_eq!(render_svg(&p, g).unwrap(), render_svg(&p, g).unwrap());
}

#[test]
fn missing_graph_is_an_error() {
    let p = Project::new();
    assert!(matches!(render_svg(&p, WindowId(99)), Err(PlotError::NoSuchGraph(_))));
    assert!(matches!(layout_geometry(&p, WindowId(99)), Err(PlotError::NoSuchGraph(_))));
}

#[test]
fn plots_with_missing_or_text_y_columns_are_skipped() {
    let (mut p, g) = project(Template::Line, vec![1.0, 2.0, 3.0], vec![vec![1.0, 2.0, 3.0]]);
    let book = p.workbooks().next().unwrap().id;
    let t = p.add_column(book, 0, Designation::Y, ColumnData::text(vec!["a".into(), "b".into(), "c".into()])).unwrap();
    p.add_plot(g, 0, t).unwrap();
    p.add_plot(g, 0, sheaf_core::ColumnId(999_999)).unwrap_err();
    let svg = svg_of(&p, g);
    let d = doc(&svg);
    assert_eq!(paths_in(with_class(&d, "plots")[0]).len(), 1);
    // And no legend row for it.
    assert!(!texts(&d).iter().any(|s| s == "(missing column)"));
}

#[test]
fn hidden_plots_and_row_ranges_are_respected() {
    let x: Vec<f64> = (0..100).map(f64::from).collect();
    let (mut p, g) = project(Template::Line, x.clone(), vec![x.clone(), x]);
    {
        let layer = &mut p.graph_mut(g).unwrap().layers[0];
        layer.plots[0].visible = false;
        layer.plots[1].rows = Some((10, 20));
    }
    let geom = &layout_geometry(&p, g).unwrap()[0];
    assert_eq!((geom.x.min, geom.x.max), (10.0, 20.0));
    let svg = svg_of(&p, g);
    let d = doc(&svg);
    assert_eq!(paths_in(with_class(&d, "plots")[0]).len(), 1);
}

#[test]
fn alpha_dash_and_width_reach_the_svg() {
    let (mut p, g) = project(Template::Line, vec![0.0, 1.0, 2.0], vec![vec![0.0, 1.0, 0.5]]);
    {
        let line = &mut p.graph_mut(g).unwrap().layers[0].plots[0].line;
        line.color = Color::rgba(255, 0, 0, 128);
        line.width_pt = 2.0;
        line.dash = sheaf_core::graph::Dash::Dash;
    }
    let svg = svg_of(&p, g);
    let d = doc(&svg);
    let path = paths_in(with_class(&d, "plots")[0])[0];
    assert_eq!(path.attribute("stroke"), Some("#FF0000"));
    assert_eq!(path.attribute("stroke-opacity"), Some("0.502"));
    assert_eq!(path.attribute("stroke-width"), Some("2"));
    assert_eq!(path.attribute("stroke-dasharray"), Some("12 6"));
    assert_eq!(path.attribute("stroke-linecap"), None, "dashed lines have butt caps");
}

#[test]
fn step_and_spline_connections() {
    let (mut p, g) = project(Template::Line, vec![0.0, 1.0, 2.0], vec![vec![0.0, 1.0, 0.0]]);
    let set = |p: &mut Project, c| p.graph_mut(g).unwrap().layers[0].plots[0].line.connect = c;
    set(&mut p, Connect::StepHorizontal);
    let d1 = svg_of(&p, g);
    let path = paths_in(with_class(&doc(&d1), "plots")[0])[0].attribute("d").unwrap().to_string();
    assert_eq!(path.matches('L').count(), 4, "{path}");
    set(&mut p, Connect::Spline);
    let d2 = svg_of(&p, g);
    let path = paths_in(with_class(&doc(&d2), "plots")[0])[0].attribute("d").unwrap().to_string();
    assert_eq!(path.matches('C').count(), 2, "{path}");
}

#[test]
fn spline_control_points_follow_catmull_rom() {
    let mut d = String::new();
    let pts = [(0.0, 0.0), (6.0, 6.0), (12.0, 0.0), (18.0, 6.0)];
    plots::polyline_path_for_test(&mut d, &pts);
    // First segment: P0 duplicated: c1 = P1 + (P2-P0)/6 = (6,6)? no: segment P0->P1,
    // c1 = P0 + (P1 - P0)/6 = (1,1); c2 = P1 - (P2 - P0)/6 = (4,6).
    assert!(d.starts_with("M0 0C1,1 4,6 6,6"), "{d}");
}

#[test]
fn symbols_every_nth_and_error_bars() {
    let x: Vec<f64> = (0..10).map(f64::from).collect();
    let (mut p, g) = project(Template::Scatter, x.clone(), vec![x.clone()]);
    let book = p.workbooks().next().unwrap().id;
    let err = p.add_column(book, 0, Designation::YError, ColumnData::numeric(vec![0.5; 10])).unwrap();
    {
        let plot = &mut p.graph_mut(g).unwrap().layers[0].plots[0];
        plot.symbol.every = 3;
        plot.y_error = Some(err);
    }
    let svg = svg_of(&p, g);
    let d = doc(&svg);
    let plots = paths_in(with_class(&d, "plots")[0]);
    // Error bars first, then symbols (no line for a scatter plot).
    assert_eq!(plots.len(), 2);
    let bars = plots[0].attribute("d").unwrap();
    assert_eq!(bars.matches('M').count(), 3 * 10);
    assert_eq!(plots[0].attribute("stroke-width"), Some("0.75")); // half the 1.5 pt line width
    let symbols = plots[1].attribute("d").unwrap();
    assert_eq!(symbols.matches('Z').count(), 4, "points 0, 3, 6, 9");
    assert_eq!(plots[1].attribute("stroke"), None);
    {
        let plot = &mut p.graph_mut(g).unwrap().layers[0].plots[0];
        plot.symbol.shape = SymbolShape::Plus;
    }
    let svg = svg_of(&p, g);
    let d = doc(&svg);
    assert!(paths_in(with_class(&d, "plots")[0])[1].attribute("stroke").is_some(), "plus is stroked");
}

#[test]
fn log_axis_places_decades_evenly() {
    let (mut p, g) = project(Template::Line, vec![1.0, 10.0, 100.0, 1000.0], vec![vec![1.0, 2.0, 3.0, 4.0]]);
    p.graph_mut(g).unwrap().layers[0].x_axis.scale = ScaleKind::Log10;
    let geom = &layout_geometry(&p, g).unwrap()[0];
    assert_eq!((geom.x.min, geom.x.max), (1.0, 1000.0));
    let (a, _) = geom.data_to_page(10.0, 1.0);
    let (b, _) = geom.data_to_page(100.0, 1.0);
    let (c, _) = geom.data_to_page(1000.0, 1.0);
    assert!(((b - a) - (c - b)).abs() < 1e-9);
    // Labels are powers of ten with a raised exponent: separate text elements.
    let svg = svg_of(&p, g);
    let d = doc(&svg);
    let t = texts(&d);
    assert!(t.iter().filter(|s| *s == "10").count() >= 4, "{t:?}");
    assert!(t.contains(&"3".to_string()));
}

#[test]
fn geometry_maps_between_data_and_page() {
    let (p, g) = project(Template::Line, vec![0.0, 10.0], vec![vec![0.0, 100.0]]);
    let geom = &layout_geometry(&p, g).unwrap()[0];
    let f = geom.frame;
    assert_eq!(geom.data_to_page(geom.x.min, geom.y.min), (f.x, f.y + f.h));
    let (px, py) = geom.data_to_page(geom.x.max, geom.y.max);
    assert!((px - (f.x + f.w)).abs() < 1e-9 && (py - f.y).abs() < 1e-9);
    let (x, y) = geom.page_to_data(f.x + f.w / 2.0, f.y + f.h / 2.0);
    assert!((x - 5.0).abs() < 1e-9 && (y - 50.0).abs() < 1e-9);
}

#[test]
fn custom_titles_and_markup() {
    let (mut p, g) = project(Template::Line, vec![0.0, 1.0], vec![vec![0.0, 1.0]]);
    {
        let layer = &mut p.graph_mut(g).unwrap().layers[0];
        layer.y_axis.title = Some("j (mA cm\\+(-2))".into());
        layer.x_axis.show_title = false;
    }
    let svg = svg_of(&p, g);
    let d = doc(&svg);
    let t = texts(&d);
    assert!(
        t.contains(&"j (mA cm".to_string()) && t.contains(&"-2".to_string()) && t.contains(&")".to_string()),
        "{t:?}"
    );
    assert!(!t.contains(&"Potential (V)".to_string()));
    // The superscript is smaller than the main text.
    let sizes: Vec<_> = d
        .descendants()
        .filter(|n| n.has_tag_name("text") && n.text() == Some("-2"))
        .map(|n| n.attribute("font-size").unwrap().to_string())
        .collect();
    assert_eq!(sizes, vec!["9.1"]); // 14 * 0.65
}

#[test]
fn text_is_xml_escaped() {
    let (mut p, g) = project(Template::Line, vec![0.0, 1.0], vec![vec![0.0, 1.0]]);
    p.graph_mut(g).unwrap().layers[0].plots[0].label = Some("a < b & \"c\"".into());
    let svg = svg_of(&p, g);
    assert!(svg.contains("a &lt; b &amp; \"c\""));
    doc(&svg);
}

#[test]
fn free_labels_and_legend_frame() {
    let (mut p, g) = project(Template::Line, vec![0.0, 1.0], vec![vec![0.0, 1.0]]);
    {
        let layer = &mut p.graph_mut(g).unwrap().layers[0];
        layer.legend.frame = true;
        layer.legend.background = Some(Color::WHITE);
        layer.labels.push(sheaf_core::graph::TextLabel {
            text: "peak".into(),
            x: 0.5,
            y: 0.5,
            font: sheaf_core::graph::Font::new("Arial", 10.0),
            rotation_deg: 45.0,
        });
    }
    let svg = svg_of(&p, g);
    assert!(svg.contains(">peak</text>") && svg.contains("rotate(-45 "));
    let d = doc(&svg);
    assert!(
        d.descendants().any(|n| n.has_tag_name("rect")
            && n.attribute("fill") == Some("#FFFFFF")
            && n.attribute("stroke").is_some())
    );
}

#[test]
fn legend_stays_inside_the_frame_in_every_corner() {
    use sheaf_core::graph::LegendPosition::*;
    let (mut p, g) = project(Template::Line, vec![0.0, 1.0], vec![vec![0.0, 1.0], vec![1.0, 0.0]]);
    let f = layout_geometry(&p, g).unwrap()[0].frame;
    for pos in [TopLeft, TopRight, BottomLeft, BottomRight, Manual { x: 0.5, y: 0.5 }] {
        p.graph_mut(g).unwrap().layers[0].legend.position = pos;
        let scene = layout::build_scene(&p, g, &FixedMetrics).unwrap();
        for it in &scene.layers[0].overlay {
            if let scene::Item::Text(t) = it
                && ["Scan 1", "Scan 2", "B", "C"].contains(&t.text.as_str())
            {
                assert!(t.x > f.x && t.x < f.x + f.w && t.y > f.y && t.y < f.y + f.h, "{pos:?}: {t:?}");
            }
        }
    }
}

#[test]
fn tick_direction_none_draws_no_ticks() {
    let (mut p, g) = project(Template::Line, vec![0.0, 1.0], vec![vec![0.0, 1.0]]);
    {
        let l = &mut p.graph_mut(g).unwrap().layers[0];
        for a in [&mut l.x_axis, &mut l.y_axis] {
            a.major_tick = TickDirection::None;
            a.minor_tick = TickDirection::None;
        }
    }
    let scene = layout::build_scene(&p, g, &FixedMetrics).unwrap();
    // Only the two axis lines remain as paths.
    let n = scene.layers[0].overlay.iter().filter(|i| matches!(i, scene::Item::Path { .. })).count();
    assert_eq!(n, 2 + 1 /* legend sample */);
}

// ----- raster, PNG, PDF ------------------------------------------------------------------------

fn red_line_project() -> (Project, WindowId) {
    let x: Vec<f64> = (0..=10).map(f64::from).collect();
    let (mut p, g) = project(Template::Line, x.clone(), vec![x]);
    let plot = &mut p.graph_mut(g).unwrap().layers[0].plots[0];
    plot.line.color = Color::RED;
    plot.line.width_pt = 12.0;
    plot.line.connect = Connect::Straight;
    plot.kind = PlotKind::Line;
    p.graph_mut(g).unwrap().layers[0].legend.visible = false;
    (p, g)
}

fn pixel(r: &Raster, x: u32, y: u32) -> [u8; 4] {
    let i = ((y * r.width + x) * 4) as usize;
    [r.rgba[i], r.rgba[i + 1], r.rgba[i + 2], r.rgba[i + 3]]
}

#[test]
fn raster_has_a_red_line_on_white() {
    let (p, g) = red_line_project();
    let r = render_raster(&p, g, 2.0).unwrap();
    assert_eq!((r.width, r.height), (922, 692)); // 460.8 x 345.6 pt at 2 px/pt
    assert_eq!(r.rgba.len(), (r.width * r.height * 4) as usize);
    let geom = &layout_geometry(&p, g).unwrap()[0];
    let (px, py) = geom.data_to_page(5.0, 5.0);
    assert_eq!(pixel(&r, (px * 2.0) as u32, (py * 2.0) as u32), [255, 0, 0, 255]);
    assert_eq!(pixel(&r, 5, 5), [255, 255, 255, 255]);
    // Inside the frame but far from the line.
    let (qx, qy) = geom.data_to_page(9.0, 1.0);
    assert_eq!(pixel(&r, (qx * 2.0) as u32, (qy * 2.0) as u32), [255, 255, 255, 255]);
}

#[test]
fn png_dimensions_and_resolution() {
    let (x, y) = loop_data(100, 1.0);
    let (p, g) = project(Template::CyclicVoltammetry, x, vec![y]);
    let bytes = render_png(&p, g, 300.0).unwrap();
    assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
    let decoder = png::Decoder::new(std::io::Cursor::new(&bytes));
    let mut reader = decoder.read_info().unwrap();
    let info = reader.info();
    assert_eq!((info.width, info.height), (1500, 1080));
    let dims = info.pixel_dims.expect("pHYs chunk");
    assert_eq!(dims.unit, png::Unit::Meter);
    assert_eq!(dims.xppu, 11811); // 300 / 0.0254
    assert_eq!(dims.yppu, 11811);
    let mut buf = vec![0; reader.output_buffer_size().unwrap()];
    reader.next_frame(&mut buf).unwrap();
    assert_eq!(&buf[..4], &[255, 255, 255, 255], "white page background");
}

#[test]
fn bad_resolution_is_an_error() {
    let (p, g) = red_line_project();
    assert!(matches!(render_png(&p, g, 0.0), Err(PlotError::Png(_))));
    assert!(matches!(render_png(&p, g, f64::NAN), Err(PlotError::Png(_))));
    assert!(matches!(render_png(&p, g, 1.0e6), Err(PlotError::Png(_))));
}

#[test]
fn pdf_is_a_real_pdf() {
    let (x, y) = loop_data(100, 1.0);
    let (p, g) = project(Template::CyclicVoltammetry, x, vec![y]);
    let pdf = render_pdf(&p, g).unwrap();
    assert!(pdf.starts_with(b"%PDF"));
    assert!(pdf.len() > 1500, "{} bytes", pdf.len());
    let tail = String::from_utf8_lossy(&pdf[pdf.len().saturating_sub(64)..]).to_string();
    assert!(tail.contains("%%EOF"), "{tail}");
}

#[test]
fn export_by_extension() {
    let dir = std::env::temp_dir().join(format!("sheaf-plot-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let (p, g) = red_line_project();
    for (name, magic) in [("a.svg", &b"<svg"[..]), ("b.PNG", b"\x89PNG"), ("c.pdf", b"%PDF")] {
        let path = dir.join(name);
        export_file(&p, g, &path, 72.0).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        assert!(bytes.starts_with(magic), "{name}");
    }
    let err = export_file(&p, g, &dir.join("d.jpg"), 72.0).unwrap_err();
    assert!(matches!(&err, PlotError::UnknownFormat(e) if e == "jpg"));
    assert!(err.to_string().contains(".svg"));
    let err = export_file(&p, g, &dir.join("no-such-folder").join("e.svg"), 72.0).unwrap_err();
    assert!(matches!(err, PlotError::Io { .. }));
    std::fs::remove_dir_all(&dir).unwrap();
}

// ----- performance and decimation ---------------------------------------------------------------

#[test]
fn a_million_points_are_decimated() {
    let n = 1_000_000;
    let x: Vec<f64> = (0..n).map(|i| i as f64 / n as f64).collect();
    let y: Vec<f64> = x.iter().map(|v| (v * 20.0).sin()).collect();
    let (mut p, g) = project(Template::Line, x, vec![y]);
    p.graph_mut(g).unwrap().layers[0].plots[0].line.connect = Connect::Spline; // must fall back to a polyline
    let t = std::time::Instant::now();
    let svg = svg_of(&p, g);
    let elapsed = t.elapsed();
    let segments = svg.matches('L').count();
    assert!(segments < 50_000, "{segments} segments");
    assert!(segments > 100);
    assert!(!svg.contains('C') || svg.matches('C').count() < 10, "no spline at this density");
    if !cfg!(debug_assertions) {
        assert!(elapsed.as_secs_f64() < 2.0, "took {elapsed:?}");
    }
}

#[test]
fn decimation_keeps_first_and_last_points() {
    // Three points within 0.05 pt of each other: first and last stay.
    let (p, g) = project(Template::Line, vec![0.0, 0.0001, 0.0002, 10.0], vec![vec![0.0, 0.0, 0.0, 10.0]]);
    let svg = svg_of(&p, g);
    let d = doc(&svg);
    let path = paths_in(with_class(&d, "plots")[0])[0].attribute("d").unwrap().to_string();
    assert_eq!(path.matches('L').count(), 1, "{path}");
}

#[test]
fn a_million_noisy_points_keep_their_envelope_with_few_segments() {
    // Every point jumps far from the previous one, so thinning by distance alone keeps them all.
    let n = 1_000_000;
    let x: Vec<f64> = (0..n).map(|i| i as f64 / n as f64).collect();
    let y: Vec<f64> = (0..n).map(|i| ((i as u64 * 7919) % 1000) as f64 / 100.0).collect();
    let (p, g) = project(Template::Line, x, vec![y]);
    let svg = svg_of(&p, g);
    let segments = svg.matches('L').count();
    assert!(segments < 40_000, "{segments} segments");
    // The lowest (0) and highest (9.99) values are still drawn: the path spans the data range.
    let geom = &layout_geometry(&p, g).unwrap()[0];
    let d = doc(&svg);
    let path = paths_in(with_class(&d, "plots")[0])[0].attribute("d").unwrap().to_string();
    let ys: Vec<f64> = path
        .split(['M', 'L'])
        .filter_map(|t| t.split_once(' ').and_then(|(_, y)| y.trim().parse::<f64>().ok()))
        .collect();
    let (top, bottom) = (geom.data_to_page(0.5, 9.99).1, geom.data_to_page(0.5, 0.0).1);
    let (lo, hi) = ys.iter().fold((f64::MAX, f64::MIN), |(lo, hi), v| (lo.min(*v), hi.max(*v)));
    assert!((lo - top).abs() < 0.02 && (hi - bottom).abs() < 0.02, "path y {lo}..{hi}, data {top}..{bottom}");
}

#[test]
fn a_million_symbols_on_one_curve_are_thinned() {
    let n = 1_000_000;
    let x: Vec<f64> = (0..n).map(|i| i as f64 / n as f64).collect();
    let y: Vec<f64> = x.iter().map(|v| v * (1.0 - v)).collect();
    let (p, g) = project(Template::Scatter, x, vec![y]);
    let svg = svg_of(&p, g);
    let symbols = svg.matches('Z').count();
    assert!(symbols > 300 && symbols < 5_000, "{symbols} symbols");
}

#[test]
fn thinning_leaves_sparse_and_reversing_curves_alone() {
    // A loop that goes out and back over the same X range (like a voltammogram): all 400 points stay.
    let n = 200;
    let mut x: Vec<f64> = (0..n).map(|i| i as f64 / n as f64).collect();
    x.extend((0..n).map(|i| 1.0 - i as f64 / n as f64));
    let y: Vec<f64> = (0..2 * n).map(|i| if i < n { 1.0 } else { -1.0 }).collect();
    let (p, g) = project(Template::Line, x, vec![y]);
    let svg = svg_of(&p, g);
    let d = doc(&svg);
    let path = paths_in(with_class(&d, "plots")[0])[0].attribute("d").unwrap().to_string();
    assert_eq!(path.matches('L').count(), 2 * n - 1);
}

// ----- golden images -------------------------------------------------------

/// Compares the SVG of a built-in template with the stored one in
/// `tests/golden/`. Text is measured with [`FixedMetrics`] and the data uses
/// only plain arithmetic, so the output does not depend on installed fonts or
/// on the platform's math library. After an intended change of the rendering,
/// run the tests once with the environment variable `SHEAF_UPDATE_GOLDEN=1`
/// to rewrite the stored files, and look at them before committing.
fn check_golden(name: &str, template: Template) {
    let n = 41;
    let x: Vec<f64> = (0..n).map(|i| -0.2 + 0.03 * i as f64).collect();
    let curve = |amp: f64| -> Vec<f64> {
        x.iter()
            .map(|v| {
                let t = (v + 0.2) / 1.2;
                amp * (t * (1.0 - t) * (1.0 - 2.0 * t) * 20.0)
            })
            .collect()
    };
    let (p, g) = project(template, x.clone(), vec![curve(30.0), curve(21.5), curve(12.25)]);
    let svg = svg_of(&p, g);
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("golden").join(format!("{name}.svg"));
    if std::env::var_os("SHEAF_UPDATE_GOLDEN").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &svg).unwrap();
        return;
    }
    let stored = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}; run with SHEAF_UPDATE_GOLDEN=1 to create it", path.display()));
    // Git may check the file out with Windows line ends.
    let stored = stored.replace("\r\n", "\n");
    assert!(
        stored == svg,
        "the rendering of the '{name}' template changed; if that is intended, run the tests with \
         SHEAF_UPDATE_GOLDEN=1 and look at {}",
        path.display()
    );
}

#[test]
fn golden_line_template() {
    check_golden("line", Template::Line);
}

#[test]
fn golden_scatter_template() {
    check_golden("scatter", Template::Scatter);
}

#[test]
fn golden_line_symbol_template() {
    check_golden("line-symbol", Template::LineSymbol);
}

#[test]
fn golden_cv_template() {
    check_golden("cv", Template::CyclicVoltammetry);
}

// ----- labels stay on the page, reversed axes ------------------------------

#[test]
fn y_title_stays_inside_the_page_with_wide_tick_labels() {
    let x: Vec<f64> = (0..10).map(f64::from).collect();
    let y: Vec<f64> = x.iter().map(|v| -467338.0 + v * 1000.0).collect();
    let (p, g) = project(Template::CyclicVoltammetry, x, vec![y]);
    let svg = svg_of(&p, g);
    let d = doc(&svg);
    let title = d
        .descendants()
        .find(|n| n.has_tag_name("text") && n.text().is_some_and(|t| t.starts_with("Current 1")))
        .expect("the Y title");
    // The title is rotated about its anchor; its glyphs reach 0.72 * size left of the anchor x.
    let transform = title.attribute("transform").expect("rotated");
    let nums: Vec<f64> = transform
        .trim_start_matches("rotate(")
        .trim_end_matches(')')
        .split(' ')
        .filter_map(|s| s.parse().ok())
        .collect();
    let size: f64 = title.attribute("font-size").unwrap().parse().unwrap();
    let left = nums[1] - 0.72 * size;
    assert!(left >= 2.99, "the title reaches x = {left}");
}

#[test]
fn a_frame_that_fits_is_left_alone() {
    let (x, y) = loop_data(20, 1.0);
    let (p, g) = project(Template::Line, x, vec![y]);
    let gr = p.graph(g).unwrap();
    let nominal = gr.layers[0].frame;
    let page = gr.page.clone();
    let f = layout_geometry(&p, g).unwrap()[0].frame;
    assert!((f.x - nominal.left * page.width_pt).abs() < 1e-9);
    assert!((f.w - nominal.width * page.width_pt).abs() < 1e-9);
    assert!((f.h - nominal.height * page.height_pt).abs() < 1e-9);
}

#[test]
fn reversed_limits_draw_a_reversed_axis() {
    let (x, y) = loop_data(20, 1.0);
    let (mut p, g) = project(Template::Line, x, vec![y]);
    let layer = &mut p.graph_mut(g).unwrap().layers[0];
    layer.x_axis.min = Some(5.0);
    layer.x_axis.max = Some(1.0);
    let geom = &layout_geometry(&p, g).unwrap()[0];
    assert!(geom.x.reversed);
    assert_eq!((geom.x.min, geom.x.max), (1.0, 5.0));
    assert_eq!(geom.x.major, vec![1.0, 2.0, 3.0, 4.0, 5.0]);
    // 5 is at the left end of the frame and 1 at the right end.
    let (left, _) = geom.data_to_page(5.0, 0.0);
    let (right, _) = geom.data_to_page(1.0, 0.0);
    assert!((left - geom.frame.x).abs() < 1e-9);
    assert!((right - (geom.frame.x + geom.frame.w)).abs() < 1e-9);
    let (back, _) = geom.page_to_data(left, 0.0);
    assert!((back - 5.0).abs() < 1e-9);
}

#[test]
fn column_plots_draw_one_filled_bar_per_value() {
    let x: Vec<f64> = (1..=4).map(f64::from).collect();
    let (mut p, g) = project(Template::Scatter, x.clone(), vec![vec![1.0, 2.0, 3.0, 2.0]]);
    {
        let plot = &mut p.graph_mut(g).unwrap().layers[0].plots[0];
        plot.kind = sheaf_core::graph::PlotKind::Column;
        plot.symbol.color = sheaf_core::color::Color::rgb(0, 107, 172);
    }
    let svg = svg_of(&p, g);
    assert_eq!(svg.matches("fill=\"#006BAC\"").count(), 4, "{svg}");
}
