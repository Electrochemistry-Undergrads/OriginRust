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

/// The X values and the curve shape used by the golden images.
fn golden_data() -> (Vec<f64>, impl Fn(f64) -> Vec<f64>) {
    let n = 41;
    let x: Vec<f64> = (0..n).map(|i| -0.2 + 0.03 * i as f64).collect();
    let xs = x.clone();
    let curve = move |amp: f64| -> Vec<f64> {
        xs.iter()
            .map(|v| {
                let t = (v + 0.2) / 1.2;
                amp * (t * (1.0 - t) * (1.0 - 2.0 * t) * 20.0)
            })
            .collect()
    };
    (x, curve)
}

/// Compares the SVG of a built-in template with the stored one in
/// `tests/golden/`. Text is measured with [`FixedMetrics`] and the data uses
/// only plain arithmetic, so the output does not depend on installed fonts or
/// on the platform's math library. After an intended change of the rendering,
/// run the tests once with the environment variable `SHEAF_UPDATE_GOLDEN=1`
/// to rewrite the stored files, and look at them before committing.
fn check_golden(name: &str, template: Template) {
    let (x, curve) = golden_data();
    let (p, g) = project(template, x, vec![curve(30.0), curve(21.5), curve(12.25)]);
    compare_golden(name, &svg_of(&p, g));
}

fn compare_golden(name: &str, svg: &str) {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("golden").join(format!("{name}.svg"));
    if std::env::var_os("SHEAF_UPDATE_GOLDEN").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, svg).unwrap();
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

// ----- double-Y graphs: linked layers, axis sides, graph legend ------------

/// One curve for a double-Y test: its column's long name, unit, X values and Y values.
struct Curve {
    name: &'static str,
    unit: &'static str,
    x: Vec<f64>,
    y: Vec<f64>,
}

fn curve(name: &'static str, unit: &'static str, x: &[f64], f: impl Fn(f64) -> f64) -> Curve {
    Curve { name, unit, x: x.to_vec(), y: x.iter().map(|v| f(*v)).collect() }
}

/// A graph of `template` with the `left` curves in layer 0 and the `right` curves in the layer
/// that carries the right Y axis (made with `add_right_y_layer` unless the template has one).
/// Every curve has its own X column, called "Time" in s. The legend text of a curve is its name.
fn double_y(template: Template, left: Vec<Curve>, right: Vec<Curve>) -> (Project, WindowId) {
    let mut p = Project::new();
    let book = p.add_workbook("data");
    let g = p.add_graph(template);
    let r = p.add_right_y_layer(g, 0).unwrap();
    for (layer, curves) in [(0, left), (r, right)] {
        for c in curves {
            let x = p.add_column(book, 0, Designation::X, ColumnData::numeric(c.x)).unwrap();
            {
                let col = p.column_mut(x).unwrap();
                col.long_name = "Time".into();
                col.units = "s".into();
            }
            let y = p.add_column(book, 0, Designation::Y, ColumnData::numeric(c.y)).unwrap();
            let col = p.column_mut(y).unwrap();
            col.long_name = c.name.into();
            col.units = c.unit.into();
            p.add_plot_xy(g, layer, Some(x), y, None).unwrap();
        }
    }
    (p, g)
}

/// The spec's example: X = 0..10, left Y = x², right Y = (x - 5)/100.
fn squares_and_line() -> (Project, WindowId) {
    let x: Vec<f64> = (0..=10).map(f64::from).collect();
    double_y(
        Template::DoubleY,
        vec![curve("Squares", "mm", &x, |v| v * v)],
        vec![curve("Line", "mA", &x, |v| (v - 5.0) / 100.0)],
    )
}

/// Geometry with the fixed text metrics of the SVG tests (`layout_geometry` measures with the
/// installed fonts).
fn fixed_geometry(p: &Project, g: WindowId) -> Vec<LayerGeometry> {
    let graph = p.graph(g).unwrap();
    let pds: Vec<data::LayerData> = graph.layers.iter().map(|l| data::collect(p, l)).collect();
    layout::graph_geometry(graph, &pds, &FixedMetrics)
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

fn texts_of(items: &[scene::Item]) -> Vec<&scene::TextItem> {
    items
        .iter()
        .filter_map(|it| match it {
            scene::Item::Text(t) => Some(t),
            _ => None,
        })
        .collect()
}

/// Straight lines (one move, one line) drawn with this stroke width: axis lines, not ticks.
fn single_lines(items: &[scene::Item], width: f64) -> Vec<(String, Color)> {
    items
        .iter()
        .filter_map(|it| match it {
            scene::Item::Path { d, stroke: Some(s), .. }
                if s.width == width && d.matches('M').count() == 1 && d.matches('L').count() == 1 =>
            {
                Some((d.clone(), s.color))
            }
            _ => None,
        })
        .collect()
}

#[test]
fn double_y_layout_puts_the_right_axis_labels_and_title_beside_the_frame() {
    let (p, g) = squares_and_line();
    let geoms = fixed_geometry(&p, g);
    assert_eq!(geoms.len(), 2);
    for geom in &geoms {
        let f = geom.frame;
        assert!(near(f.x, 73.728) && near(f.y, 24.192) && near(f.w, 313.344) && near(f.h, 259.2), "{f:?}");
        assert_eq!((geom.x.min, geom.x.max, geom.x.step), (0.0, 10.0, 2.0));
    }
    assert_eq!((geoms[0].y.min, geoms[0].y.max, geoms[0].y.step), (0.0, 100.0, 20.0));
    let right = &geoms[1].y;
    assert!(near(right.min, -0.06) && near(right.max, 0.06) && near(right.step, 0.02), "{right:?}");
    assert_eq!(right.major.len(), 7);

    let scene = layout::build_scene(&p, g, &FixedMetrics).unwrap();
    let texts = texts_of(&scene.layers[1].overlay);
    let tick_labels: Vec<_> = texts.iter().filter(|t| t.rotate_ccw == 0.0 && t.x > 387.0).collect();
    assert_eq!(tick_labels.len(), 7);
    assert!(tick_labels.iter().all(|t| t.anchor == scene::Anchor::Start && near(t.x, 396.272)), "{tick_labels:?}");
    let titles: Vec<_> = texts.iter().filter(|t| t.rotate_ccw == 90.0).collect();
    assert_eq!(titles.len(), 1);
    let t = titles[0];
    assert_eq!(t.text, "Line (mA)");
    assert_eq!(t.anchor, scene::Anchor::Middle);
    assert!(near(t.x, 447.752) && near(t.y, 153.792), "{t:?}");
    // Layer 1 draws no X axis: no X tick labels and no X title.
    assert!(!texts.iter().any(|t| t.text == "Time (s)" || (t.anchor == scene::Anchor::Middle && t.rotate_ccw == 0.0)));
}

#[test]
fn right_labels_move_the_right_side_of_the_frame_to_stay_on_the_page() {
    let (mut p, g) = squares_and_line();
    p.graph_mut(g).unwrap().layers[0].frame.width = 0.78;
    for geom in fixed_geometry(&p, g) {
        let f = geom.frame;
        // 359.424 nominal width; the right title would reach 496.772, 38.972 past 457.8.
        assert!(near(f.x, 73.728) && near(f.w, 359.424 - 38.972), "{f:?}");
        assert!(near(f.y, 24.192) && near(f.h, 259.2), "{f:?}");
    }
    // The right title now ends 3 pt inside the page.
    let scene = layout::build_scene(&p, g, &FixedMetrics).unwrap();
    let title = texts_of(&scene.layers[1].overlay).into_iter().find(|t| t.rotate_ccw == 90.0).unwrap();
    assert!(near(title.x + 0.21 * title.size, 460.8 - 3.0), "{title:?}");
}

#[test]
fn a_linked_x_axis_covers_the_data_of_both_layers_and_follows_the_parent() {
    let a: Vec<f64> = (0..=10).map(f64::from).collect();
    let b: Vec<f64> = (5..=25).map(f64::from).collect();
    let (mut p, g) =
        double_y(Template::DoubleY, vec![curve("Left", "V", &a, |v| v)], vec![curve("Right", "A", &b, |v| 1000.0 + v)]);
    let geoms = fixed_geometry(&p, g);
    for geom in &geoms {
        assert_eq!((geom.x.min, geom.x.max, geom.x.step), (0.0, 25.0, 5.0));
    }
    assert_eq!((geoms[0].y.min, geoms[0].y.max), (0.0, 10.0), "the left Y axis sees only its own data");
    assert_eq!((geoms[1].y.min, geoms[1].y.max), (1005.0, 1025.0), "the right Y axis sees only its own data");
    // The linked layer's own X settings are not used while it is linked.
    p.graph_mut(g).unwrap().layers[1].x_axis.min = Some(100.0);
    assert_eq!(fixed_geometry(&p, g)[1].x, geoms[1].x);
    // Limits set on the parent apply to both.
    {
        let x = &mut p.graph_mut(g).unwrap().layers[0].x_axis;
        x.min = Some(2.0);
        x.max = Some(6.0);
    }
    for geom in fixed_geometry(&p, g) {
        assert_eq!((geom.x.min, geom.x.max), (2.0, 6.0));
    }
}

#[test]
fn y_axes_take_the_color_of_their_first_curve_in_the_double_y_template() {
    let (mut p, g) = squares_and_line();
    let blue = Color::rgb(30, 80, 220);
    let scene = layout::build_scene(&p, g, &FixedMetrics).unwrap();
    // Right axis: line and ticks (width 1) and tick labels in the right curve's blue; title black.
    let right_strokes: Vec<Color> = scene.layers[1]
        .overlay
        .iter()
        .filter_map(|it| match it {
            scene::Item::Path { stroke: Some(s), .. } if s.width == 1.0 => Some(s.color),
            _ => None,
        })
        .collect();
    assert_eq!(right_strokes.len(), 2, "the right line and its ticks");
    assert!(right_strokes.iter().all(|c| *c == blue), "{right_strokes:?}");
    let texts = texts_of(&scene.layers[1].overlay);
    assert!(texts.iter().filter(|t| t.x > 387.0 && t.rotate_ccw == 0.0).all(|t| t.color == blue));
    assert!(texts.iter().filter(|t| t.rotate_ccw == 90.0).all(|t| t.color == Color::BLACK));
    // Left axis: its curve is black, so everything there is black.
    let left = &scene.layers[0].overlay;
    assert!(texts_of(left).iter().all(|t| t.color == Color::BLACK));
    let svg = svg_of(&p, g);
    assert!(svg.contains("stroke=\"#1E50DC\""));

    // A left curve colored by hand: the left axis follows it, the X axis stays black.
    {
        let plot = &mut p.graph_mut(g).unwrap().layers[0].plots[0];
        plot.line.color = Color::RED;
        plot.auto_color = false;
    }
    let scene = layout::build_scene(&p, g, &FixedMetrics).unwrap();
    let lines = single_lines(&scene.layers[0].overlay, 1.0);
    let left_line = lines.iter().find(|(d, _)| d == "M73.728 24.192L73.728 283.392").expect("left line");
    assert_eq!(left_line.1, Color::RED);
    let bottom = lines.iter().find(|(d, _)| d == "M73.728 283.392L387.072 283.392").expect("bottom line");
    assert_eq!(bottom.1, Color::BLACK);
    let left_labels: Vec<_> =
        texts_of(&scene.layers[0].overlay).into_iter().filter(|t| t.anchor == scene::Anchor::End).collect();
    assert!(!left_labels.is_empty() && left_labels.iter().all(|t| t.color == Color::RED));

    // Fixed colors: every axis black.
    for layer in &mut p.graph_mut(g).unwrap().layers {
        layer.y_axis.color_follows_plot = false;
    }
    let scene = layout::build_scene(&p, g, &FixedMetrics).unwrap();
    for layer in &scene.layers {
        for (_, color) in single_lines(&layer.overlay, 1.0) {
            assert_eq!(color, Color::BLACK);
        }
    }
    // With the title following too, the right title is blue.
    p.graph_mut(g).unwrap().layers[1].y_axis.title_color_follows_plot = true;
    let scene = layout::build_scene(&p, g, &FixedMetrics).unwrap();
    let title = texts_of(&scene.layers[1].overlay).into_iter().find(|t| t.rotate_ccw == 90.0).unwrap();
    assert_eq!(title.color, blue);
}

#[test]
fn each_layer_draws_only_the_edges_of_its_own_axes() {
    let (p, g) = squares_and_line();
    let scene = layout::build_scene(&p, g, &FixedMetrics).unwrap();
    let (l, r, t, b) = (73.728, 387.072, 24.192, 283.392);
    let edges = |items: &[scene::Item]| -> Vec<&'static str> {
        let mut out: Vec<&'static str> = single_lines(items, 1.0)
            .into_iter()
            .map(|(d, _)| {
                let n: Vec<f64> = d[1..].split(['L', ' ']).map(|s| s.parse().unwrap()).collect();
                match () {
                    _ if near(n[1], b) && near(n[3], b) => "bottom",
                    _ if near(n[1], t) && near(n[3], t) => "top",
                    _ if near(n[0], l) && near(n[2], l) => "left",
                    _ if near(n[0], r) && near(n[2], r) => "right",
                    _ => "other",
                }
            })
            .collect();
        out.sort_unstable();
        out
    };
    assert_eq!(edges(&scene.layers[0].overlay), vec!["bottom", "left", "top"]);
    assert_eq!(edges(&scene.layers[1].overlay), vec!["right"]);
    // Layer 1 has no left ticks of its own (the Origin file read today draws them on the left).
    let layer1_ticks_left = scene.layers[1].overlay.iter().any(|it| {
        matches!(it, scene::Item::Path { d, stroke: Some(s), .. }
            if s.width == 1.0 && d.matches('M').count() > 1 && d.starts_with(&format!("M{l}")))
    });
    assert!(!layer1_ticks_left);
}

#[test]
fn one_legend_lists_the_curves_of_both_axes_on_top_of_everything() {
    let x: Vec<f64> = (0..=10).map(f64::from).collect();
    let (p, g) = double_y(
        Template::DoubleY,
        vec![curve("B", "V", &x, |v| v), curve("D", "V", &x, |v| 10.0 - v)],
        vec![curve("C", "mA", &x, |v| v * 0.01)],
    );
    let svg = svg_of(&p, g);
    let pos = |text: &str| -> Vec<usize> { svg.match_indices(&format!(">{text}</text>")).map(|(i, _)| i).collect() };
    let (b, d, c) = (pos("B"), pos("D"), pos("C"));
    assert_eq!((b.len(), d.len(), c.len()), (1, 1, 1), "each curve is listed once");
    assert!(b[0] < d[0] && d[0] < c[0], "left curves first, then the right one");
    let layer1_plots = svg.find("clip-path=\"url(#clip1)\"").unwrap();
    assert!(b[0] > layer1_plots, "the legend comes after the right layer's curves");
    // Rows of one box: equal spacing, one column.
    let scene = layout::build_scene(&p, g, &FixedMetrics).unwrap();
    let rows: Vec<_> =
        texts_of(&scene.layers[1].overlay).into_iter().filter(|t| ["B", "D", "C"].contains(&t.text.as_str())).collect();
    assert!(near(rows[1].y - rows[0].y, rows[2].y - rows[1].y) && near(rows[0].x, rows[2].x));
}

#[test]
fn an_empty_right_axis_is_drawn_from_0_to_1_without_a_title() {
    let x: Vec<f64> = (0..=10).map(f64::from).collect();
    let (p, g) = double_y(Template::DoubleY, vec![curve("Left", "V", &x, |v| v)], vec![]);
    let geoms = fixed_geometry(&p, g);
    assert_eq!((geoms[1].y.min, geoms[1].y.max), (0.0, 1.0));
    let scene = layout::build_scene(&p, g, &FixedMetrics).unwrap();
    assert!(!texts_of(&scene.layers[1].overlay).iter().any(|t| t.rotate_ccw == 90.0));
    let svg = svg_of(&p, g);
    assert_eq!(svg.matches(">Left</text>").count(), 1);
}

#[test]
fn invalid_links_draw_independent_layers() {
    let x: Vec<f64> = (0..=10).map(f64::from).collect();
    for parent in [1, 5] {
        let (mut p, g) =
            double_y(Template::DoubleY, vec![curve("Left", "V", &x, |v| v)], vec![curve("Right", "A", &x, |v| -v)]);
        p.graph_mut(g).unwrap().layers[1].link.as_mut().unwrap().parent = parent;
        let geoms = fixed_geometry(&p, g);
        assert_eq!(geoms.len(), 2);
        // The frame {0, 0, 1, 1} is read as the whole page; only its right side moves in.
        let f = geoms[1].frame;
        assert!(f.x == 0.0 && f.y == 0.0 && near(f.h, 345.6) && f.w < 460.8, "{f:?}");
        assert_eq!((geoms[1].x.min, geoms[1].x.max), (0.0, 10.0), "its own X range");
        svg_of(&p, g);
        layout_geometry(&p, g).unwrap();
    }
    // A chain: layer 2 links to layer 1, which links to layer 0. Layer 2 is drawn on its own.
    let (mut p, g) = squares_and_line();
    let third = sheaf_core::template::right_y_layer(&p.graph(g).unwrap().layers[1], 1);
    p.graph_mut(g).unwrap().layers.push(third);
    let geoms = fixed_geometry(&p, g);
    assert_eq!(geoms.len(), 3);
    assert_eq!(geoms[1].frame, geoms[0].frame);
    assert_eq!(geoms[2].frame.x, 0.0);
    svg_of(&p, g);
}

#[test]
fn golden_double_y_template() {
    // Two curves on the left, a charge-like curve with a different scale on the right.
    let (x, shape) = golden_data();
    let mk = |name: &'static str, unit: &'static str, y: Vec<f64>| Curve { name, unit, x: x.clone(), y };
    let mut q = 0.0;
    let charge: Vec<f64> = shape(30.0)
        .iter()
        .map(|i| {
            q += i * 0.03;
            q
        })
        .collect();
    let (p, g) = double_y(
        Template::DoubleY,
        vec![mk("Current 1", "mA", shape(30.0)), mk("Current 2", "mA", shape(12.25))],
        vec![mk("Charge", "mC", charge)],
    );
    compare_golden("double-y", &svg_of(&p, g));
}

#[test]
fn a_right_axis_on_the_cv_template_keeps_the_closed_black_box() {
    let (x, y) = loop_data(100, 3.0);
    let (p, g) = double_y(
        Template::CyclicVoltammetry,
        vec![Curve { name: "Current", unit: "mA", x: x.clone(), y: y.clone() }],
        vec![Curve { name: "Charge", unit: "mC", x, y: y.iter().map(|v| v * 0.4 + 1.0).collect() }],
    );
    let graph = p.graph(g).unwrap();
    assert_eq!(graph.layers[1].y_axis.major_tick, TickDirection::In);
    let geoms = fixed_geometry(&p, g);
    // Both Y axes are symmetric about zero, each for its own data.
    for geom in &geoms {
        assert!(geom.y.max > 0.0 && geom.y.min == -geom.y.max, "{:?}", geom.y);
    }
    assert!(geoms[1].y.max > geoms[0].y.max / 3.0 && geoms[1].y.max < geoms[0].y.max, "its own range");
    let scene = layout::build_scene(&p, g, &FixedMetrics).unwrap();
    let mut colors: Vec<Color> = Vec::new();
    for layer in &scene.layers {
        colors.extend(single_lines(&layer.overlay, 1.0).into_iter().map(|(_, c)| c));
    }
    assert_eq!(colors, vec![Color::BLACK; 4], "four black edges");
}

// ----- stacked curves and waterfall (graphing spec section 13) --------------

use sheaf_core::graph::Stack;

const AUTO10: Stack = Stack::Auto { gap_percent: 10.0 };

/// A curve given as a function of X.
type Of = &'static dyn Fn(f64) -> f64;

/// Origin's first check data: x/2, 1000 + 5x, -5 + x/2 and x/20.
const ORIGIN_CHECK: [Of; 4] = [&|x| x / 2.0, &|x| 1000.0 + 5.0 * x, &|x| -5.0 + x / 2.0, &|x| x / 20.0];

fn x0_20() -> Vec<f64> {
    (0..=20).map(f64::from).collect()
}

/// A line graph of the curves `fs` over X = 0, 1, ..., 20 whose layer has the stack mode `stack`.
fn stacked(stack: Stack, fs: &[Of]) -> (Project, WindowId) {
    let x = x0_20();
    let ys = fs.iter().map(|f| x.iter().map(|v| f(*v)).collect()).collect();
    let (mut p, g) = project(Template::Line, x, ys);
    p.graph_mut(g).unwrap().layers[0].stack = stack;
    (p, g)
}

fn y_offsets(p: &Project, g: WindowId) -> Vec<f64> {
    let offsets = stack_offsets(p, g, 0).unwrap();
    assert!(offsets.iter().all(|(ox, _)| *ox == 0.0), "{offsets:?}");
    offsets.into_iter().map(|(_, oy)| oy).collect()
}

fn assert_offsets(p: &Project, g: WindowId, want: &[f64]) {
    let got = y_offsets(p, g);
    assert!(got.len() == want.len() && got.iter().zip(want).all(|(a, b)| near(*a, *b)), "{got:?}, want {want:?}");
}

/// The data range of the drawn Y values of the first layer (before rounding to ticks).
fn y_range(p: &Project, g: WindowId) -> Option<(f64, f64)> {
    let layer = &p.graph(g).unwrap().layers[0];
    data::axis_range(data::collect(p, layer).plots.iter(), data::Dim::Y, layer.y_axis.scale)
}

fn plot_column(p: &Project, g: WindowId, index: usize) -> sheaf_core::ColumnId {
    p.graph(g).unwrap().layers[0].plots[index].y
}

fn set_values(p: &mut Project, column: sheaf_core::ColumnId, values: Vec<f64>) {
    p.column_mut(column).unwrap().data = ColumnData::numeric(values);
}

#[test]
fn auto_offsets_stack_like_origin_2016_and_follow_the_data() {
    let (mut p, g) = stacked(AUTO10, &ORIGIN_CHECK);
    assert_offsets(&p, g, &[0.0, -980.0, 135.0, 150.0]);
    assert_eq!(y_range(&p, g), Some((0.0, 151.0)), "the axis range covers the moved curves");
    // The offsets are computed again from the data: the second curve becomes 1000 + 10x.
    let c = plot_column(&p, g, 1);
    set_values(&mut p, c, x0_20().iter().map(|x| 1000.0 + 10.0 * x).collect());
    assert_offsets(&p, g, &[0.0, -970.0, 255.0, 280.0]);
}

#[test]
fn auto_offsets_use_the_tallest_curve_for_the_gap() {
    let (p, g) = stacked(AUTO10, &[&|x| x, &|x| 50.0 - x / 2.0, &|x| -5.0 + x / 4.0, &|x| 2.0 * x - 10.0]);
    assert_offsets(&p, g, &[0.0, -16.0, 43.0, 57.0]);
}

#[test]
fn auto_offsets_ignore_missing_values() {
    let (mut p, g) = stacked(AUTO10, &ORIGIN_CHECK);
    let c = plot_column(&p, g, 2);
    let mut values = p.numeric(c).unwrap().to_vec();
    values[20] = f64::NAN;
    set_values(&mut p, c, values);
    assert_offsets(&p, g, &[0.0, -980.0, 135.0, 149.5]);
}

#[test]
fn auto_offsets_of_flat_curves() {
    let (p, g) = stacked(AUTO10, &[&|_| 7.0, &|x| x / 2.0, &|_| 7.0]);
    assert_offsets(&p, g, &[0.0, 8.0, 12.0]);
    // All flat: the height counts as 10, so the gap is 1.
    let (p, g) = stacked(AUTO10, &[&|_| 7.0, &|_| 7.0, &|_| 7.0]);
    assert_offsets(&p, g, &[0.0, 1.0, 2.0]);
}

#[test]
fn auto_offsets_are_added_in_data_units_on_a_log_axis() {
    let (mut p, g) = stacked(AUTO10, &[&|x| 10.0 + x, &|x| 1.0 + x / 20.0]);
    p.graph_mut(g).unwrap().layers[0].y_axis.scale = ScaleKind::Log10;
    assert_offsets(&p, g, &[0.0, 31.0]);
    let layer = &p.graph(g).unwrap().layers[0];
    let d = data::collect(&p, layer);
    assert_eq!(data::axis_range(d.plots[1..].iter(), data::Dim::Y, ScaleKind::Log10), Some((32.0, 33.0)));
}

#[test]
fn hidden_plots_keep_their_place_in_the_stack() {
    let (mut p, g) = stacked(AUTO10, &ORIGIN_CHECK);
    p.graph_mut(g).unwrap().layers[0].plots[1].visible = false;
    assert_offsets(&p, g, &[0.0, -980.0, 135.0, 150.0]);
    // The automatic range comes from the visible curves: 0..10, 130..140 and 150..151.
    assert_eq!(y_range(&p, g), Some((0.0, 151.0)));
    let svg = svg_of(&p, g);
    assert_eq!(paths_in(with_class(&doc(&svg), "plots")[0]).len(), 3);

    let (mut p, g) = stacked(Stack::Constant { step: 500.0 }, &ORIGIN_CHECK);
    p.graph_mut(g).unwrap().layers[0].plots[1].visible = false;
    assert_offsets(&p, g, &[0.0, 500.0, 1000.0, 1500.0]);
}

#[test]
fn constant_steps_may_be_negative_and_numbers_that_are_not_finite_count_as_0() {
    let three: [Of; 3] = [&|x| x, &|x| x, &|x| x];
    let (p, g) = stacked(Stack::Constant { step: -2.0 }, &three);
    assert_offsets(&p, g, &[0.0, -2.0, -4.0]);
    let (p, g) = stacked(Stack::Constant { step: f64::NAN }, &three);
    assert_offsets(&p, g, &[0.0, 0.0, 0.0]);
    // A gap that is not finite is no gap: each curve starts where the one before it ends.
    let (p, g) = stacked(Stack::Auto { gap_percent: f64::INFINITY }, &ORIGIN_CHECK);
    assert_offsets(&p, g, &[0.0, -990.0, 115.0, 120.0]);
}

#[test]
fn a_curve_without_numbers_has_no_offset_and_the_next_one_stacks_on_the_one_before() {
    let (mut p, g) = stacked(AUTO10, &[&|x| x, &|_| f64::NAN, &|x| 50.0 + x]);
    assert_offsets(&p, g, &[0.0, 0.0, -28.0]);
    // A curve of a text column is not a member either.
    let book = p.workbooks().next().unwrap().id;
    let text = p.add_column(book, 0, Designation::Y, ColumnData::text(vec!["a".into(); 21])).unwrap();
    p.add_plot(g, 0, text).unwrap();
    let layer = &mut p.graph_mut(g).unwrap().layers[0];
    let last = layer.plots.pop().unwrap();
    layer.plots.insert(1, last);
    assert_offsets(&p, g, &[0.0, 0.0, 0.0, -28.0]);
}

#[test]
fn individual_offsets_move_each_curve_and_are_ignored_in_other_modes() {
    let (mut p, g) = stacked(Stack::None, &[&|x| x, &|x| 2.0 * x]);
    let plain = svg_of(&p, g);
    let b = plot_column(&p, g, 1);
    {
        let entry = p.graph_mut(g).unwrap().layers[0].plot_offset_mut(b);
        entry.x = 1.5;
        entry.y = 100.0;
    }
    assert_eq!(svg_of(&p, g), plain, "stored offsets do nothing without the individual mode");
    p.graph_mut(g).unwrap().layers[0].stack = Stack::Individual;
    assert_eq!(stack_offsets(&p, g, 0).unwrap(), vec![(0.0, 0.0), (1.5, 100.0)]);
    let d = data::collect(&p, &p.graph(g).unwrap().layers[0]);
    assert_eq!((d.plots[1].x_at(0), d.plots[1].y_at(0)), (1.5, 100.0), "the first point (0, 0) is moved");
    // The drawn curve starts there.
    let geom = &fixed_geometry(&p, g)[0];
    let (px, py) = geom.data_to_page(1.5, 100.0);
    let mut start = String::new();
    scene::move_to(&mut start, px, py);
    let svg = svg_of(&p, g);
    let path = paths_in(with_class(&doc(&svg), "plots")[0])[1].attribute("d").unwrap().to_string();
    assert!(path.starts_with(&start), "{path} should start with {start}");
    // An offset that is not finite counts as 0.
    p.graph_mut(g).unwrap().layers[0].plot_offset_mut(b).y = f64::INFINITY;
    assert_eq!(stack_offsets(&p, g, 0).unwrap()[1], (1.5, 0.0));
}

#[test]
fn a_fitted_curve_moves_with_its_data_and_takes_no_place_of_its_own() {
    // Curves: B, A, the fit of A, C.
    let fs: [Of; 4] = [&|x| x / 20.0, &|x| x / 2.0, &|x| x / 2.0 + 0.25, &|x| 1000.0 + 5.0 * x];
    let (mut p, g) = stacked(AUTO10, &fs);
    let (a, fit) = (plot_column(&p, g, 1), plot_column(&p, g, 2));
    p.graph_mut(g).unwrap().layers[0].plot_offset_mut(fit).follows = Some(a);
    // B 0..1, A moved to 11..21, C moved so that it starts 10 above: 31 - 1000.
    assert_offsets(&p, g, &[0.0, 11.0, 11.0, -969.0]);
    let (q, h) = stacked(AUTO10, &[fs[0], fs[1], fs[3]]);
    assert_offsets(&q, h, &[0.0, 11.0, -969.0]);
    let d = data::collect(&p, &p.graph(g).unwrap().layers[0]);
    let members: Vec<Option<usize>> = d.plots.iter().map(|pd| pd.member).collect();
    assert_eq!(members, vec![Some(0), Some(1), Some(1), Some(2)]);
    assert_eq!(d.members, 3);
    // A curve that follows a column the layer does not plot stays where it is.
    p.graph_mut(g).unwrap().layers[0].plot_offset_mut(fit).follows = Some(sheaf_core::ColumnId(999_999));
    assert_eq!(y_offsets(&p, g)[2], 0.0);
}

#[test]
fn a_stacked_graph_draws_exactly_like_data_that_were_shifted_beforehand() {
    let (p, g) = stacked(AUTO10, &ORIGIN_CHECK);
    let offsets = y_offsets(&p, g);
    let x = x0_20();
    let shifted: Vec<Vec<f64>> = ORIGIN_CHECK
        .iter()
        .zip(&offsets)
        .map(|(f, o)| x.iter().map(|v| if *o == 0.0 { f(*v) } else { f(*v) + o }).collect())
        .collect();
    let (q, h) = project(Template::Line, x, shifted);
    assert_eq!(svg_of(&p, g), svg_of(&q, h));
}

#[test]
fn bars_and_error_bars_start_from_the_offset() {
    let (mut p, g) = project(Template::Scatter, vec![1.0, 2.0], vec![vec![5.0, 6.0]]);
    let book = p.workbooks().next().unwrap().id;
    let err = p.add_column(book, 0, Designation::YError, ColumnData::numeric(vec![1.0, 1.0])).unwrap();
    {
        let layer = &mut p.graph_mut(g).unwrap().layers[0];
        layer.stack = Stack::Individual;
        let y = layer.plots[0].y;
        layer.plot_offset_mut(y).y = 100.0;
        layer.plots[0].kind = PlotKind::Column;
        layer.plots[0].y_error = Some(err);
    }
    let geom = &fixed_geometry(&p, g)[0];
    let scene = layout::build_scene(&p, g, &FixedMetrics).unwrap();
    let bars: Vec<Rect> = scene.layers[0]
        .plots
        .iter()
        .filter_map(|it| match it {
            scene::Item::Rect { rect, .. } => Some(*rect),
            _ => None,
        })
        .collect();
    assert_eq!(bars.len(), 2);
    let (top, base) = (geom.data_to_page(1.0, 105.0).1, geom.data_to_page(1.0, 100.0).1);
    assert!(near(bars[0].y, top) && near(bars[0].y + bars[0].h, base), "{:?}", bars[0]);
    // The error bar of the first point runs from 106 down to 104.
    let error_bars = scene.layers[0]
        .plots
        .iter()
        .find_map(|it| match it {
            scene::Item::Path { d, .. } => Some(d.clone()),
            _ => None,
        })
        .unwrap();
    let mut want = String::new();
    let (bx, high) = geom.data_to_page(1.0, 106.0);
    scene::move_to(&mut want, bx, high);
    scene::line_to(&mut want, bx, geom.data_to_page(1.0, 104.0).1);
    assert!(error_bars.starts_with(&want), "{error_bars} should start with {want}");
}

#[test]
fn stack_offsets_need_a_graph_and_a_layer() {
    let (p, g) = stacked(AUTO10, &[&|x| x]);
    assert_eq!(stack_offsets(&p, g, 0).unwrap(), vec![(0.0, 0.0)]);
    assert!(matches!(stack_offsets(&p, g, 1), Err(PlotError::NoSuchLayer(1))));
    assert!(matches!(stack_offsets(&p, WindowId(999), 0), Err(PlotError::NoSuchGraph(_))));
}

fn near3(a: f64, b: f64) -> bool {
    (a - b).abs() < 5e-4
}

#[test]
fn the_waterfall_front_frame_follows_the_spec() {
    let f = Rect { x: 100.0, y: 50.0, w: 300.0, h: 200.0 };
    let b = stack::front_frame(f, 3, 0.05, 0.1);
    assert!(near3(b.x, 100.0) && near3(b.y, 83.333) && near3(b.w, 272.727) && near3(b.h, 166.667), "{b:?}");
    // The third curve is moved by two shifts and ends at the top right corner of f.
    assert!(near3(2.0 * 0.05 * b.w, 27.273) && near3(2.0 * 0.1 * b.h, 33.333));
    assert!(near(b.x + 2.0 * 0.05 * b.w + b.w, f.x + f.w) && near(b.y - 2.0 * 0.1 * b.h, f.y));
    // Moving left: the front frame sits at the right side.
    assert!(near3(stack::front_frame(f, 3, -0.05, 0.1).x, 127.273));
    // Moving down: the front frame sits at the top.
    assert_eq!(stack::front_frame(f, 3, 0.05, -0.1).y, f.y);
    assert_eq!(stack::front_frame(f, 1, 0.05, 0.1), f);
    assert_eq!(stack::front_frame(f, 0, 0.05, 0.1), f);
}

#[test]
fn a_waterfall_draws_later_curves_moved_and_the_first_on_top() {
    let three: [Of; 3] = [&|x| x, &|x| x, &|x| x];
    let (mut p, g) = stacked(Stack::Waterfall { dx_percent: 5.0, dy_percent: 10.0 }, &three);
    {
        let layer = &mut p.graph_mut(g).unwrap().layers[0];
        for axis in [&mut layer.x_axis, &mut layer.y_axis] {
            axis.show_opposite = true;
            axis.opposite_ticks = true;
            axis.major_grid = Some(sheaf_core::graph::LineStyle::solid(Color::rgb(200, 200, 200), 0.5));
        }
        layer.legend.visible = false;
    }
    let geom = fixed_geometry(&p, g)[0].clone();
    let mut plain = p.clone();
    plain.graph_mut(g).unwrap().layers[0].stack = Stack::None;
    let f = fixed_geometry(&plain, g)[0].frame;
    assert_eq!(geom.clip, f, "data are clipped to the whole frame");
    let b = stack::front_frame(f, 3, 0.05, 0.1);
    assert_eq!(geom.frame, b, "the axes are on the front frame");
    assert_eq!((geom.y.min, geom.y.max), (0.0, 20.0), "limits from the unmoved data");

    let scene = layout::build_scene(&p, g, &FixedMetrics).unwrap();
    let layer = &scene.layers[0];
    assert_eq!(layer.clip, f);
    assert!(layer.grid.is_empty(), "no grid");
    let curves: Vec<(&str, Color)> = layer
        .plots
        .iter()
        .filter_map(|it| match it {
            scene::Item::Path { d, stroke: Some(s), .. } => Some((d.as_str(), s.color)),
            _ => None,
        })
        .collect();
    assert_eq!(curves.len(), 3);
    let (x0, y0) = geom.data_to_page(0.0, 0.0);
    for (i, (d, _)) in curves.iter().enumerate() {
        // Drawn back to front: the third curve (two shifts) first.
        let k = (2 - i) as f64;
        let mut start = String::new();
        scene::move_to(&mut start, x0 + k * 0.05 * b.w, y0 - k * 0.1 * b.h);
        assert!(d.starts_with(&start), "curve {i}: {d} should start with {start}");
    }
    let first_color = p.graph(g).unwrap().layers[0].plots[0].line.color;
    assert_eq!(curves[2].1, first_color, "the first curve is drawn last, on top");
    // Only the bottom and left axis lines: no opposite lines, no closed box.
    assert_eq!(single_lines(&layer.overlay, 1.0).len(), 2);
    assert!(!layer.overlay.iter().any(|it| matches!(it, scene::Item::Path { d, .. } if d.ends_with('Z'))));
    // No opposite ticks: every tick path starts on the bottom or the left edge.
    for it in &layer.overlay {
        if let scene::Item::Path { d, stroke: Some(s), .. } = it
            && s.width == 1.0
            && d.matches('M').count() > 1
        {
            let n: Vec<f64> = d[1..].split(['L', ' ', 'M']).take(2).map(|v| v.parse().unwrap()).collect();
            assert!(near3(n[0], b.x) || near3(n[1], b.y + b.h), "{d}");
        }
    }
}

#[test]
fn a_waterfall_in_a_graph_with_a_right_axis_is_drawn_without_shifts() {
    let x: Vec<f64> = (0..=10).map(f64::from).collect();
    let (p, g) = double_y(
        Template::Waterfall,
        vec![curve("A", "V", &x, |v| v), curve("B", "V", &x, |v| v + 1.0)],
        vec![curve("C", "mA", &x, |v| -v)],
    );
    assert!(matches!(p.graph(g).unwrap().layers[0].stack, Stack::Waterfall { .. }));
    for geom in fixed_geometry(&p, g) {
        assert_eq!(geom.frame, geom.clip);
    }
    let mut plain = p.clone();
    for layer in &mut plain.graph_mut(g).unwrap().layers {
        layer.stack = Stack::None;
    }
    assert_eq!(svg_of(&p, g), svg_of(&plain, g));
}

#[test]
fn golden_stacked_template() {
    check_golden("stacked", Template::StackedLines);
}

#[test]
fn golden_waterfall_template() {
    check_golden("waterfall", Template::Waterfall);
}
