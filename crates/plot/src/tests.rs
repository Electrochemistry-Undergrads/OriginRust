//! Tests of the whole pipeline: project -> scene -> SVG / PNG / PDF.

use super::*;
use sheaf_core::graph::{Connect, PlotKind, ScaleKind, SymbolShape, TickDirection, TickLabelSource};
use sheaf_core::panels::{PanelLayout, PanelOrder};
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
    assert_eq!((geom.x.min, geom.x.max), (9.0, 20.0));
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
    assert_eq!((geom.x.min, geom.x.max), (0.5, 2000.0), "Origin pads 8 % of the log span, then rounds to a 1..9 tick");
    let (a, _) = geom.data_to_page(10.0, 1.0);
    let (b, _) = geom.data_to_page(100.0, 1.0);
    let (c, _) = geom.data_to_page(1000.0, 1.0);
    assert!(((b - a) - (c - b)).abs() < 1e-9);
    // Labels are plain numbers, as Origin writes them on a log axis.
    let svg = svg_of(&p, g);
    let d = doc(&svg);
    let t = texts(&d);
    for label in ["1", "10", "100", "1000"] {
        assert!(t.contains(&label.to_string()), "{label} in {t:?}");
    }
    // With the scientific format they are powers of ten with a raised exponent.
    p.graph_mut(g).unwrap().layers[0].x_axis.tick_format = sheaf_core::graph::TickFormat::Scientific(0);
    let t = texts(&doc(&svg_of(&p, g)));
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
    let d = doc(&svg);
    let plots = with_class(&d, "plots")[0];
    let bars = plots.children().filter(|n| n.attribute("fill") == Some("#006BAC")).count();
    assert_eq!(bars, 4, "{svg}");
    // The legend shows a small bar of the same color.
    assert_eq!(svg.matches("fill=\"#006BAC\"").count(), 5, "{svg}");
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
    layout::graph_geometry(p, graph, &pds, &FixedMetrics)
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
        assert_eq!((geom.x.min, geom.x.max, geom.x.step), (-2.5, 12.5, 5.0));
    }
    assert_eq!((geoms[0].y.min, geoms[0].y.max, geoms[0].y.step), (-25.0, 125.0, 50.0));
    let right = &geoms[1].y;
    assert!(near(right.min, -0.075) && near(right.max, 0.075) && near(right.step, 0.05), "{right:?}");
    assert_eq!(right.major.len(), 3);

    let scene = layout::build_scene(&p, g, &FixedMetrics).unwrap();
    let texts = texts_of(&scene.layers[1].overlay);
    let tick_labels: Vec<_> = texts.iter().filter(|t| t.rotate_ccw == 0.0 && t.x > 387.0).collect();
    assert_eq!(tick_labels.len(), 3);
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
        assert_eq!((geom.x.min, geom.x.max, geom.x.step), (-2.0, 27.0, 2.0));
    }
    assert_eq!((geoms[0].y.min, geoms[0].y.max), (-2.5, 12.5), "the left Y axis sees only its own data");
    assert_eq!((geoms[1].y.min, geoms[1].y.max), (1003.0, 1027.0), "the right Y axis sees only its own data");
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
        assert_eq!((geoms[1].x.min, geoms[1].x.max), (-2.5, 12.5), "its own X range");
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
    assert_eq!((geom.y.min, geom.y.max), (-2.0, 22.0), "limits from the unmoved data");

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

#[test]
fn a_date_column_as_x_gives_date_tick_labels() {
    use sheaf_core::datetime::{DatePattern, TimePattern, date_value, time_value};
    use sheaf_core::format::NumberFormat;
    let t0 = date_value(2026, 11, 11).unwrap() + time_value(13, 0, 0.0).unwrap();
    // Two hours, one point every 10 minutes.
    let x: Vec<f64> = (0..13).map(|i| t0 + f64::from(i) * 10.0 / 1440.0).collect();
    let y: Vec<f64> = (0..13).map(f64::from).collect();
    let (mut p, g) = project(Template::Line, x, vec![y]);
    let xid = p.graph(g).unwrap().layers[0].plots[0].x.unwrap();
    // Plain numbers: ordinary labels.
    let plain = svg_of(&p, g);
    assert!(!texts(&doc(&plain)).iter().any(|t| t.contains(':')));
    p.column_mut(xid).unwrap().format = NumberFormat::DateTime(DatePattern::Iso, TimePattern::Hms);
    let svg = svg_of(&p, g);
    let all = texts(&doc(&svg));
    assert!(all.iter().any(|t| t == "13:00"), "{all:?}");
    assert!(all.iter().any(|t| t == "14:00"), "{all:?}");
}

// ----- text tick labels and column charts (graphing spec section 14) --------

/// A workbook "icp" with a text X column "Sample" holding `names`, and one plot per Y column in
/// `ys` (long names Li, Na, Mn, K, ...), in a graph of `template`.
fn names_project(template: Template, names: &[&str], ys: Vec<Vec<f64>>) -> (Project, WindowId) {
    let mut p = Project::new();
    let book = p.add_workbook("icp");
    let cells = names.iter().map(|s| s.to_string()).collect();
    let x = p.add_column(book, 0, Designation::X, ColumnData::text(cells)).unwrap();
    p.column_mut(x).unwrap().long_name = "Sample".into();
    let g = p.add_graph(template);
    for (i, y) in ys.into_iter().enumerate() {
        let yc = p.add_column(book, 0, Designation::Y, ColumnData::numeric(y)).unwrap();
        p.column_mut(yc).unwrap().long_name = ["Li", "Na", "Mn", "K"][i % 4].into();
        p.add_plot(g, 0, yc).unwrap();
    }
    (p, g)
}

const FIVE: [&str; 5] = ["S1", "S2", "S3", "S4", "S5"];

fn five_values() -> Vec<f64> {
    vec![1.2, 2.5, 0.8, 3.1, 1.9]
}

fn strings(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

/// The filled rectangles of the plots of layer 0 (the bars), with their fill and outline.
fn bars_of(p: &Project, g: WindowId) -> Vec<(Rect, Option<Color>, Option<scene::Stroke>)> {
    let scene = layout::build_scene(p, g, &FixedMetrics).unwrap();
    scene.layers[0]
        .plots
        .iter()
        .filter_map(|it| match it {
            scene::Item::Rect { rect, fill, stroke } => Some((*rect, *fill, stroke.clone())),
            _ => None,
        })
        .collect()
}

/// A bar in data units: (left, right, bottom, top).
fn bar_in_data(geom: &LayerGeometry, r: Rect) -> (f64, f64, f64, f64) {
    let (x0, y0) = geom.page_to_data(r.x, r.y + r.h);
    let (x1, y1) = geom.page_to_data(r.x + r.w, r.y);
    (x0, x1, y0, y1)
}

fn close9(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

#[test]
fn a_text_x_column_gives_a_category_axis_with_one_label_per_row() {
    let (mut p, g) = names_project(Template::Line, &FIVE, vec![five_values()]);
    let geom = &fixed_geometry(&p, g)[0];
    assert_eq!((geom.x.min, geom.x.max, geom.x.step), (0.5, 5.5, 1.0));
    assert_eq!(geom.x.major, vec![1.0, 2.0, 3.0, 4.0, 5.0]);
    assert_eq!(geom.x.labels, strings(&FIVE));
    // The line template has one minor tick between majors: they mark the borders of the rows.
    assert_eq!(geom.x.minor, vec![0.5, 1.5, 2.5, 3.5, 4.5, 5.5]);
    // The points stand at the row numbers.
    let d = doc(&svg_of(&p, g))
        .descendants()
        .find(|n| n.has_tag_name("g") && n.attribute("class") == Some("plots"))
        .and_then(|n| n.children().find(|c| c.has_tag_name("path")).and_then(|c| c.attribute("d").map(str::to_string)))
        .unwrap();
    let mut want = String::new();
    let (px, py) = geom.data_to_page(1.0, 1.2);
    scene::move_to(&mut want, px, py);
    assert!(d.starts_with(&want), "{d} should start with {want}");
    // The X title is the text column's name, not "Row".
    let t = texts(&doc(&svg_of(&p, g)));
    assert!(t.contains(&"Sample".to_string()) && !t.contains(&"Row".to_string()), "{t:?}");

    // Numbers on the same category axis: the row numbers, same range.
    p.graph_mut(g).unwrap().layers[0].x_axis.tick_label_source = TickLabelSource::Numbers;
    let geom = &fixed_geometry(&p, g)[0];
    assert_eq!((geom.x.min, geom.x.max), (0.5, 5.5));
    assert_eq!(geom.x.labels, strings(&["1", "2", "3", "4", "5"]));
}

#[test]
fn fixed_limits_and_steps_on_a_category_axis_label_only_whole_rows() {
    let (mut p, g) = names_project(Template::Line, &FIVE, vec![five_values()]);
    {
        let x = &mut p.graph_mut(g).unwrap().layers[0].x_axis;
        x.min = Some(0.0);
        x.max = Some(7.0);
    }
    let geom = &fixed_geometry(&p, g)[0];
    assert_eq!(geom.x.major, (0..=7).map(f64::from).collect::<Vec<_>>());
    assert_eq!(geom.x.labels, strings(&["", "S1", "S2", "S3", "S4", "S5", "", ""]));
    // An empty label writes no text element.
    assert!(texts(&doc(&svg_of(&p, g))).iter().all(|t| !t.is_empty()));
    {
        let x = &mut p.graph_mut(g).unwrap().layers[0].x_axis;
        x.min = None;
        x.max = None;
        x.major_step = Some(0.5);
    }
    let geom = &fixed_geometry(&p, g)[0];
    assert_eq!(geom.x.major.len(), 11);
    assert_eq!(geom.x.labels, strings(&["", "S1", "", "S2", "", "S3", "", "S4", "", "S5", ""]));
}

#[test]
fn a_long_category_axis_falls_back_to_the_usual_step() {
    let axis_of = |n: usize| {
        let names: Vec<String> = (1..=n).map(|i| format!("P{i}")).collect();
        let refs: Vec<&str> = names.iter().map(String::as_str).collect();
        let (p, g) = names_project(Template::Line, &refs, vec![(1..=n).map(|i| i as f64).collect()]);
        fixed_geometry(&p, g)[0].x.clone()
    };
    let x = axis_of(450);
    assert_eq!((x.min, x.max, x.step), (0.5, 450.5, 100.0));
    assert_eq!(x.major, vec![100.0, 200.0, 300.0, 400.0]);
    assert_eq!(x.labels, strings(&["P100", "P200", "P300", "P400"]));
    let x = axis_of(200);
    assert_eq!((x.step, x.major.len()), (1.0, 200));
    assert_eq!(x.labels[199], "P200");
    let x = axis_of(201);
    assert_eq!(x.step, 50.0);
    assert_eq!(x.labels, strings(&["P50", "P100", "P150", "P200"]));
}

#[test]
fn an_empty_name_still_draws_its_row_but_has_no_label() {
    let (p, g) = names_project(Template::Column, &["S1", "", "S3", "S4", "S5"], vec![five_values()]);
    let geom = &fixed_geometry(&p, g)[0];
    assert_eq!(geom.x.labels[1], "");
    let bars = bars_of(&p, g);
    assert_eq!(bars.len(), 5);
    let (left, right, _, top) = bar_in_data(geom, bars[1].0);
    assert!(close9((left + right) / 2.0, 2.0) && close9(top, 2.5), "{left} {right} {top}");
}

#[test]
fn labels_from_a_chosen_column() {
    // No X column: the plot stands at the row numbers; a label column makes the axis a category axis.
    let mut p = Project::new();
    let book = p.add_workbook("b");
    let names = p.add_column(book, 0, Designation::Label, ColumnData::text(strings(&["n1", "n2", "n3", "n4"])));
    let names = names.unwrap();
    p.column_mut(names).unwrap().long_name = "Name".into();
    let y = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![3.0, 1.0, 4.0, 1.5])).unwrap();
    let g = p.add_graph(Template::Line);
    p.add_plot(g, 0, y).unwrap();
    assert_eq!(p.graph(g).unwrap().layers[0].plots[0].x, None);
    // Without a chosen column the row numbers show, as before.
    let geom = &fixed_geometry(&p, g)[0];
    assert_eq!((geom.x.min, geom.x.max), (0.5, 4.5));
    assert!(texts(&doc(&svg_of(&p, g))).contains(&"Row".to_string()));
    p.graph_mut(g).unwrap().layers[0].x_axis.tick_label_source = TickLabelSource::Column { column: names };
    let geom = &fixed_geometry(&p, g)[0];
    assert_eq!((geom.x.min, geom.x.max), (0.5, 4.5));
    assert_eq!(geom.x.labels, strings(&["n1", "n2", "n3", "n4"]));
    let t = texts(&doc(&svg_of(&p, g)));
    assert!(t.contains(&"Name".to_string()) && !t.contains(&"Row".to_string()), "{t:?}");
    // A chosen column that no longer exists gives numbers again.
    p.graph_mut(g).unwrap().layers[0].x_axis.tick_label_source =
        TickLabelSource::Column { column: sheaf_core::ColumnId(999_999) };
    let geom = &fixed_geometry(&p, g)[0];
    assert_eq!((geom.x.min, geom.x.max), (0.5, 4.5));
    assert_eq!(geom.x.labels[0], "1");

    // A numeric X axis keeps its range; whole-number ticks show the rows' cells.
    let (mut p, g) = project(Template::Line, vec![1.0, 2.0, 3.0, 4.0, 5.0], vec![five_values()]);
    let book = p.workbooks().next().unwrap().id;
    let label = p.add_column(book, 0, Designation::Label, ColumnData::text(strings(&FIVE))).unwrap();
    p.graph_mut(g).unwrap().layers[0].x_axis.tick_label_source = TickLabelSource::Column { column: label };
    let geom = &fixed_geometry(&p, g)[0];
    assert_eq!((geom.x.min, geom.x.max), (1.0, 5.0));
    assert_eq!(geom.x.labels, strings(&FIVE));
    assert!(texts(&doc(&svg_of(&p, g))).contains(&"Potential (V)".to_string()));
}

#[test]
fn one_column_plot_draws_bars_from_zero_over_80_percent_of_the_spacing() {
    let (p, g) = names_project(Template::Column, &FIVE, vec![five_values()]);
    let geom = &fixed_geometry(&p, g)[0];
    let bars = bars_of(&p, g);
    assert_eq!(bars.len(), 5);
    let (l, r, b, t) = bar_in_data(geom, bars[0].0);
    assert!(close9(l, 0.6) && close9(r, 1.4) && close9(b, 0.0) && close9(t, 1.2), "{l} {r} {b} {t}");
    assert!(bars.iter().all(|(_, fill, _)| *fill == Some(Color::BLACK)));
    assert_eq!((geom.y.min, geom.y.max), (0.0, 3.5), "the Y axis reaches down to 0");
    let svg = svg_of(&p, g);
    let d = doc(&svg);
    assert_eq!(paths_in(with_class(&d, "plots")[0]).len(), 0, "no line");
    let t = texts(&d);
    for name in FIVE.iter().chain(&["Sample"]) {
        assert!(t.contains(&name.to_string()), "{name} in {t:?}");
    }
}

#[test]
fn several_column_plots_stand_side_by_side() {
    let ys = vec![five_values(), vec![0.5, 0.9, 1.4, 0.7, 1.1], vec![0.2, 0.3, 0.25, 0.4, 0.15]];
    let (mut p, g) = names_project(Template::Column, &FIVE, ys);
    let geom = &fixed_geometry(&p, g)[0];
    let bars = bars_of(&p, g);
    assert_eq!(bars.len(), 15);
    let palette = &p.graph(g).unwrap().layers[0].palette.colors;
    let want = [(0.6, 0.6 + 0.8 / 3.0), (0.6 + 0.8 / 3.0, 1.4 - 0.8 / 3.0), (1.4 - 0.8 / 3.0, 1.4)];
    for (k, (lo, hi)) in want.iter().enumerate() {
        let (rect, fill, stroke) = &bars[5 * k];
        let (l, r, _, _) = bar_in_data(geom, *rect);
        assert!(close9(l, *lo) && close9(r, *hi), "plot {k}: {l}..{r}");
        assert_eq!(*fill, Some(palette[k]));
        let stroke = stroke.as_ref().expect("an outline");
        assert_eq!((stroke.color, stroke.width), (Color::BLACK, 0.5), "black outline");
    }
    // Spacing settings of the layer, in data units of the first row.
    let row1 = |p: &Project| -> Vec<(f64, f64)> {
        let geom = fixed_geometry(p, g)[0].clone();
        let bars = bars_of(p, g);
        let per_plot = bars.len() / p.graph(g).unwrap().layers[0].plots.iter().filter(|q| q.visible).count();
        bars.iter().step_by(per_plot).map(|(r, _, _)| bar_in_data(&geom, *r)).map(|(l, r, _, _)| (l, r)).collect()
    };
    let set = |p: &mut Project, gap: f64, overlap: f64| {
        let bars = &mut p.graph_mut(g).unwrap().layers[0].bars;
        bars.gap_pct = gap;
        bars.overlap_pct = overlap;
    };
    let near_all = |got: &[(f64, f64)], want: &[(f64, f64)]| {
        got.len() == want.len() && got.iter().zip(want).all(|(a, b)| close9(a.0, b.0) && close9(a.1, b.1))
    };
    set(&mut p, 50.0, 0.0);
    let got = row1(&p);
    assert!(
        near_all(&got, &[(0.75, 0.75 + 1.0 / 6.0), (0.75 + 1.0 / 6.0, 1.25 - 1.0 / 6.0), (1.25 - 1.0 / 6.0, 1.25)]),
        "{got:?}"
    );
    set(&mut p, 20.0, 100.0);
    let got = row1(&p);
    assert!(near_all(&got, &[(0.6, 1.4), (0.6, 1.4), (0.6, 1.4)]), "{got:?}");
    // Two plots with a negative overlap leave a gap between them.
    p.graph_mut(g).unwrap().layers[0].plots[2].visible = false;
    set(&mut p, 20.0, -50.0);
    let got = row1(&p);
    assert!(near_all(&got, &[(0.6, 0.92), (1.08, 1.4)]), "{got:?}");
    // Values out of range are clamped: gap 150 gives bars without width, overlap -300 is -100.
    set(&mut p, 150.0, 0.0);
    let got = row1(&p);
    assert!(near_all(&got, &[(1.0, 1.0), (1.0, 1.0)]), "{got:?}");
    set(&mut p, 20.0, -300.0);
    let got = row1(&p);
    assert!(near_all(&got, &[(0.6, 0.6 + 0.8 / 3.0), (1.4 - 0.8 / 3.0, 1.4)]), "{got:?}");
}

#[test]
fn column_plots_reach_down_to_zero_or_to_the_bottom_of_a_log_axis() {
    let (mut p, g) = project(Template::Line, vec![1.0, 2.0, 3.0, 4.0], vec![vec![5.0, 6.0, 7.0, 8.0]]);
    let geom = &fixed_geometry(&p, g)[0];
    assert_eq!((geom.y.min, geom.y.max), (4.5, 8.5), "a line keeps its range");
    p.graph_mut(g).unwrap().layers[0].plots[0].kind = PlotKind::Column;
    let geom = &fixed_geometry(&p, g)[0];
    assert_eq!((geom.y.min, geom.y.max), (0.0, 9.0));
    p.graph_mut(g).unwrap().layers[0].y_axis.scale = ScaleKind::Log10;
    let geom = &fixed_geometry(&p, g)[0];
    // 5..8 is under a decade: Origin gives 4.78..8.33 (ninths of the step 1) with linear ticks.
    assert!(
        (geom.y.min - 43.0 / 9.0).abs() < 1e-9 && (geom.y.max - 25.0 / 3.0).abs() < 1e-9,
        "{:?}",
        (geom.y.min, geom.y.max)
    );
    let bars = bars_of(&p, g);
    let f = geom.frame;
    assert!(bars.iter().all(|(r, _, _)| near(r.y + r.h, f.y + f.h)), "bars start at the bottom of the axis");
}

#[test]
fn column_plots_over_numeric_x_use_the_mean_spacing_and_get_room_at_both_ends() {
    let (mut p, g) = project(Template::Line, vec![1.0, 2.0, 4.0, 8.0, 9.0], vec![vec![1.0, 2.0, 3.0, 4.0, 5.0]]);
    p.graph_mut(g).unwrap().layers[0].plots[0].kind = PlotKind::Column;
    let geom = &fixed_geometry(&p, g)[0];
    assert_eq!((geom.x.min, geom.x.max), (0.0, 10.0), "s = 2: one spacing of room, not rounded");
    let (l, r, _, _) = bar_in_data(geom, bars_of(&p, g)[0].0);
    assert!(close9(l, 0.2) && close9(r, 1.8), "{l} {r}");
    // Fixed limits are kept.
    {
        let x = &mut p.graph_mut(g).unwrap().layers[0].x_axis;
        x.min = Some(-1.0);
        x.max = Some(12.0);
    }
    let geom = &fixed_geometry(&p, g)[0];
    assert_eq!((geom.x.min, geom.x.max), (-1.0, 12.0));

    let (mut p, g) = project(Template::Line, vec![1.0, 2.0, 3.0, 4.0, 5.0], vec![five_values()]);
    p.graph_mut(g).unwrap().layers[0].plots[0].kind = PlotKind::Column;
    let geom = &fixed_geometry(&p, g)[0];
    assert_eq!((geom.x.min, geom.x.max), (0.5, 5.5));

    // An empty row between two groups of bars does not count for the spacing (as in Origin):
    // s = (1.4 - 1.0) / 5, and with a gap of 60 percent each bar is 0.4 * 0.08 wide.
    let x = vec![1.0, 1.05, 1.1, f64::NAN, 1.3, 1.35, 1.4];
    let y = vec![1.0, 1.0, 1.0, f64::NAN, 2.0, 2.0, 2.0];
    let (mut p, g) = project(Template::Column, x, vec![y]);
    p.graph_mut(g).unwrap().layers[0].bars.gap_pct = 60.0;
    let geom = &fixed_geometry(&p, g)[0];
    let (l, r, _, _) = bar_in_data(geom, bars_of(&p, g)[0].0);
    assert!(close9(r - l, 0.032), "{}", r - l);
    // A row with only its X missing counts: s = (5 - 1) / 4.
    let (mut p, g) = project(Template::Column, vec![1.0, 2.0, f64::NAN, 4.0, 5.0], vec![vec![1.0; 5]]);
    p.graph_mut(g).unwrap().layers[0].bars.gap_pct = 20.0;
    let geom = &fixed_geometry(&p, g)[0];
    let (l, r, _, _) = bar_in_data(geom, bars_of(&p, g)[0].0);
    assert!(close9(l, 0.6) && close9(r, 1.4), "{l} {r}");
    assert_eq!(bars_of(&p, g).len(), 4);
}

#[test]
fn error_bars_of_column_plots_stand_in_the_middle_of_their_own_bars() {
    let mut p = Project::new();
    let book = p.add_workbook("icp");
    p.add_column(book, 0, Designation::X, ColumnData::text(strings(&FIVE))).unwrap();
    let g = p.add_graph(Template::Column);
    for values in [five_values(), vec![0.5, 0.9, 1.4, 0.7, 1.1]] {
        let y = p.add_column(book, 0, Designation::Y, ColumnData::numeric(values)).unwrap();
        p.add_column(book, 0, Designation::YError, ColumnData::numeric(vec![0.3; 5])).unwrap();
        p.add_plot(g, 0, y).unwrap();
    }
    assert!(p.graph(g).unwrap().layers[0].plots.iter().all(|q| q.y_error.is_some()));
    let geom = &fixed_geometry(&p, g)[0];
    let scene = layout::build_scene(&p, g, &FixedMetrics).unwrap();
    let starts: Vec<f64> = scene.layers[0]
        .plots
        .iter()
        .filter_map(|it| match it {
            scene::Item::Path { d, .. } => {
                let first = d.trim_start_matches('M').split(['L', 'M']).next().unwrap();
                let px: f64 = first.split(' ').next().unwrap().parse().unwrap();
                Some(geom.page_to_data(px, 0.0).0)
            }
            _ => None,
        })
        .collect();
    assert_eq!(starts.len(), 2, "one error bar path per plot");
    assert!((starts[0] - 0.8).abs() < 1e-3 && (starts[1] - 1.2).abs() < 1e-3, "{starts:?}");
}

#[test]
fn the_legend_shows_a_small_bar_for_each_column_plot() {
    let (p, g) = names_project(Template::Column, &FIVE, vec![five_values(), five_values()]);
    let scene = layout::build_scene(&p, g, &FixedMetrics).unwrap();
    let samples: Vec<(Rect, Option<Color>)> = scene.layers[0]
        .overlay
        .iter()
        .filter_map(|it| match it {
            scene::Item::Rect { rect, fill, stroke: Some(s) } if s.color == Color::BLACK && s.width == 0.5 => {
                Some((*rect, *fill))
            }
            _ => None,
        })
        .collect();
    assert_eq!(samples.len(), 2);
    let palette = &p.graph(g).unwrap().layers[0].palette.colors;
    for (k, (r, fill)) in samples.iter().enumerate() {
        assert!(near(r.w, 2.2 * 11.0) && near(r.h, 11.0), "{r:?}");
        assert_eq!(*fill, Some(palette[k]));
    }
    // Each sample is centred on its row, the label text beside it.
    let label = texts_of(&scene.layers[0].overlay).into_iter().find(|t| t.text == "Na").unwrap();
    let cy = samples[1].0.y + samples[1].0.h / 2.0;
    assert!(near(label.y, cy + (text::ASCENT - text::DESCENT) / 2.0 * 11.0));
}

/// The tick label `text` of the X axis, the X position of tick `v` and the top of the label block.
fn x_label(p: &Project, g: WindowId, text: &str, v: f64) -> (scene::TextItem, f64, f64) {
    let geom = &fixed_geometry(p, g)[0];
    let scene = layout::build_scene(p, g, &FixedMetrics).unwrap();
    let item = texts_of(&scene.layers[0].overlay).into_iter().find(|t| t.text == text).unwrap().clone();
    let axis = &p.graph(g).unwrap().layers[0].x_axis;
    let f = geom.frame;
    // The column template draws no X ticks, so the labels start one gap below the frame.
    let top = f.y + f.h + 0.35 * axis.tick_font.size_pt;
    (item, geom.data_to_page(v, 0.0).0, top)
}

#[test]
fn rotated_x_labels_hang_from_their_highest_corner_under_the_tick() {
    let (mut p, g) = names_project(Template::Column, &FIVE, vec![five_values()]);
    let set = |p: &mut Project, angle: f64| p.graph_mut(g).unwrap().layers[0].x_axis.tick_label_angle_deg = angle;
    // "S1" at 12 pt is 13.2 pt wide with the fixed metrics; ascent 8.64, descent 2.52.
    set(&mut p, 90.0);
    let (t, tick, top) = x_label(&p, g, "S1", 1.0);
    assert_eq!((t.anchor, t.rotate_ccw), (scene::Anchor::Start, 90.0));
    assert!(near3(t.x, tick + 3.06) && near3(t.y, top + 13.2), "{t:?} tick {tick} top {top}");
    // The X title moves below the turned labels.
    let title = texts_of(&layout::build_scene(&p, g, &FixedMetrics).unwrap().layers[0].overlay)
        .into_iter()
        .find(|t| t.text == "Sample")
        .unwrap()
        .clone();
    assert!(near3(title.y, top + 13.2 + 0.6 * 14.0 + 0.72 * 14.0), "{} vs {top}", title.y);
    let svg = svg_of(&p, g);
    assert!(svg.contains(">S1</text>") && svg.matches("transform=\"rotate(-90 ").count() >= 6, "{svg}");

    set(&mut p, 45.0);
    let (t, tick, top) = x_label(&p, g, "S1", 1.0);
    assert!(near3(t.x, tick - 3.2244) && near3(t.y, top + 15.4432), "{t:?} tick {tick} top {top}");
    let title = texts_of(&layout::build_scene(&p, g, &FixedMetrics).unwrap().layers[0].overlay)
        .into_iter()
        .find(|t| t.text == "Sample")
        .unwrap()
        .clone();
    assert!(near3(title.y, top + 17.2251 + 0.6 * 14.0 + 0.72 * 14.0), "{} vs {top}", title.y);

    set(&mut p, -45.0);
    let (t, tick, top) = x_label(&p, g, "S1", 1.0);
    assert!(near3(t.x, tick - 6.1094) && near3(t.y, top + 6.1094), "{t:?} tick {tick} top {top}");
    // -45 and 315 are the same angle.
    set(&mut p, 315.0);
    let (u, _, _) = x_label(&p, g, "S1", 1.0);
    assert!(near3(u.x, t.x) && near3(u.y, t.y) && u.rotate_ccw == -45.0);
}

#[test]
fn an_angle_of_0_draws_the_labels_exactly_as_before() {
    let (x, curve) = golden_data();
    let (mut p, g) = project(Template::Line, x, vec![curve(30.0)]);
    let before = svg_of(&p, g);
    p.graph_mut(g).unwrap().layers[0].x_axis.tick_label_angle_deg = 360.0;
    assert_eq!(svg_of(&p, g), before);
    p.graph_mut(g).unwrap().layers[0].x_axis.tick_label_angle_deg = f64::NAN;
    assert_eq!(svg_of(&p, g), before);
}

#[test]
fn golden_column_template() {
    let ys = vec![vec![12.4, 15.8, 18.2, 9.6, 0.3], vec![3.1, 2.7, 2.2, 3.4, 0.1], vec![0.82, 1.05, 0.61, 1.42, 0.02]];
    let (p, g) = names_project(Template::Column, &["LMO-1", "LMO-2", "Al-LMO", "Ni-LMO", "Blank"], ys);
    compare_golden("column", &svg_of(&p, g));
}

// ----- multi-panel graphs (graphing spec section 15) ---------------------------------------

/// A panel graph of `template` with one panel per curve. Each curve is in a workbook of its own
/// (named after the curve), with X "Potential (V)" and Y named after the curve.
fn panel_graph(template: Template, layout: &PanelLayout, curves: Vec<Curve>) -> (Project, WindowId) {
    let mut p = Project::new();
    let mut groups = Vec::new();
    for c in curves {
        let book = p.add_workbook(c.name);
        let x = p.add_column(book, 0, Designation::X, ColumnData::numeric(c.x)).unwrap();
        {
            let col = p.column_mut(x).unwrap();
            col.long_name = "Potential".into();
            col.units = "V".into();
        }
        let y = p.add_column(book, 0, Designation::Y, ColumnData::numeric(c.y)).unwrap();
        let col = p.column_mut(y).unwrap();
        col.long_name = c.name.into();
        col.units = c.unit.into();
        groups.push(vec![y]);
    }
    let g = p.plot_panels(template, &groups, layout).unwrap();
    (p, g)
}

/// The tick labels of a Y axis on the left (right-aligned texts) in one layer's drawing.
fn left_labels(scene: &scene::Scene, layer: usize) -> Vec<String> {
    texts_of(&scene.layers[layer].overlay)
        .into_iter()
        .filter(|t| t.anchor == scene::Anchor::End && t.rotate_ccw == 0.0)
        .map(|t| t.text.clone())
        .collect()
}

/// Centred, unturned texts in one layer's drawing: X tick labels and the X title.
fn centred_texts(scene: &scene::Scene, layer: usize) -> Vec<String> {
    texts_of(&scene.layers[layer].overlay)
        .into_iter()
        .filter(|t| t.anchor == scene::Anchor::Middle && t.rotate_ccw == 0.0)
        .map(|t| t.text.clone())
        .collect()
}

#[test]
fn the_panels_of_a_stack_share_one_x_axis_that_covers_every_panel() {
    let a: Vec<f64> = (0..=20).map(|i| -1.0 + 0.1 * f64::from(i)).collect();
    let b: Vec<f64> = (0..=20).map(|i| 0.1 * f64::from(i)).collect();
    let curves = vec![curve("A", "mA", &a, |v| v), curve("B", "mA", &b, |v| 10.0 * v)];
    let (mut p, g) = panel_graph(Template::Line, &PanelLayout::stack(2), curves);
    let geoms = fixed_geometry(&p, g);
    for geom in &geoms {
        assert_eq!((geom.x.min, geom.x.max), (-1.5, 2.5));
        assert_eq!(geom.x.major, vec![-1.0, 0.0, 1.0, 2.0]);
    }
    // Each panel has its own Y axis.
    assert_eq!((geoms[0].y.min, geoms[0].y.max), (-1.5, 1.5));
    assert_eq!((geoms[1].y.min, geoms[1].y.max), (-2.0, 22.0));
    // Panel 1 sits right on top of panel 0, as wide as it.
    let (f0, f1) = (geoms[0].frame, geoms[1].frame);
    assert!(near(f1.y + f1.h, f0.y) && near(f1.x, f0.x) && near(f1.w, f0.w) && near(f1.h, f0.h), "{f0:?} {f1:?}");
    // The Y labels and titles need at least the 15 % margin, and both panels lose the same width on
    // the left; the right side (90 %) and the top of the stack (10 %) stay.
    assert!(
        f0.x >= 0.15 * 345.6 - 1e-9 && near(f0.x + f0.w, 0.90 * 345.6) && near(f1.y, 0.10 * 460.8),
        "{f0:?} {f1:?}"
    );
    // Limits set on the first panel apply to both.
    p.graph_mut(g).unwrap().layers[0].x_axis.max = Some(3.0);
    assert!(fixed_geometry(&p, g).iter().all(|geom| geom.x.max == 3.0));
    p.graph_mut(g).unwrap().layers[0].x_axis.max = None;
    // Without the X link each panel has its own X range; the panel keeps its place.
    p.graph_mut(g).unwrap().layers[1].link.as_mut().unwrap().x = sheaf_core::graph::ScaleLink::None;
    let geoms = fixed_geometry(&p, g);
    assert_eq!((geoms[0].x.min, geoms[0].x.max), (-1.5, 1.5));
    assert_eq!((geoms[1].x.min, geoms[1].x.max), (-0.5, 2.5));
    assert!(near(geoms[1].frame.y + geoms[1].frame.h, geoms[0].frame.y));
}

#[test]
fn a_panel_with_an_invalid_link_is_drawn_on_its_own() {
    use sheaf_core::graph::{LayerLink, ScaleLink};
    let x: Vec<f64> = (0..=10).map(f64::from).collect();
    for (layer, parent) in [(0, 0), (1, 5), (1, 1)] {
        let curves = vec![curve("A", "mA", &x, |v| v), curve("B", "mA", &x, |v| -v)];
        let (mut p, g) = panel_graph(Template::Line, &PanelLayout::stack(2), curves);
        p.graph_mut(g).unwrap().layers[layer].link =
            Some(LayerLink { parent, x: ScaleLink::Straight, y: ScaleLink::None });
        let graph = p.graph(g).unwrap();
        assert_eq!(graph.link_parent(1), None, "layer {layer} -> {parent}");
        let geoms = fixed_geometry(&p, g);
        assert_eq!(geoms.len(), 2);
        svg_of(&p, g);
        layout_geometry(&p, g).unwrap();
    }
}

#[test]
fn a_cv_stack_hides_the_top_label_of_the_lower_panel_and_the_inner_x_axis_labels() {
    let x: Vec<f64> = (0..=10).map(|i| -0.2 + 0.08 * f64::from(i)).collect();
    let down = |v: f64| 0.9 - 0.18 * ((v + 0.2) / 0.08);
    let curves = vec![curve("Before", "mA", &x, down), curve("After", "mA", &x, |v| -down(v))];
    let (p, g) = panel_graph(Template::CyclicVoltammetry, &PanelLayout::stack(2), curves);
    let geoms = fixed_geometry(&p, g);
    for geom in &geoms {
        assert!(near(geom.y.min, -1.2) && near(geom.y.max, 1.2) && near(geom.y.step, 0.4), "{:?}", geom.y);
    }
    let scene = layout::build_scene(&p, g, &FixedMetrics).unwrap();
    let (lower, upper) = (left_labels(&scene, 0), left_labels(&scene, 1));
    assert_eq!(lower.len(), 6, "{lower:?}");
    assert!(!lower.contains(&"1.2".to_string()) && lower.contains(&"\u{2212}1.2".to_string()), "{lower:?}");
    assert_eq!(upper.len(), 7, "{upper:?}");
    assert!(upper.contains(&"1.2".to_string()) && upper.contains(&"\u{2212}1.2".to_string()), "{upper:?}");
    // Only the bottom panel shows X tick labels and the X title.
    assert!(centred_texts(&scene, 1).is_empty(), "{:?}", centred_texts(&scene, 1));
    assert!(centred_texts(&scene, 0).contains(&"Potential (V)".to_string()));
    // Each panel keeps its Y title and its legend.
    for (layer, name) in [(0, "Before"), (1, "After")] {
        let texts = texts_of(&scene.layers[layer].overlay);
        assert!(texts.iter().any(|t| t.rotate_ccw == 90.0 && t.text == format!("{name} (mA)")));
        assert!(texts.iter().any(|t| t.anchor == scene::Anchor::Start && t.text == name));
    }
}

#[test]
fn the_panels_of_a_stack_move_together_to_keep_wide_labels_on_the_page() {
    let x: Vec<f64> = (0..=10).map(f64::from).collect();
    let curves = vec![curve("Narrow", "V", &x, |v| v / 10.0), curve("Wide", "counts", &x, |v| 450000.0 + 1000.0 * v)];
    let layout = PanelLayout { margin_left: 1.0, ..PanelLayout::stack(2) };
    let (p, g) = panel_graph(Template::Line, &layout, curves);
    let geoms = fixed_geometry(&p, g);
    let nominal = 0.01 * 345.6;
    let (f0, f1) = (geoms[0].frame, geoms[1].frame);
    assert!(f0.x > nominal + 20.0, "the wide labels of panel 1 move both panels: {f0:?}");
    assert!(near(f0.x, f1.x) && near(f0.w, f1.w), "{f0:?} {f1:?}");
    assert!(near(f0.x + f0.w, nominal + 0.89 * 345.6), "the right side stays: {f0:?}");
    // The title of the wide panel now starts 3 pt inside the page.
    let scene = layout::build_scene(&p, g, &FixedMetrics).unwrap();
    let title = texts_of(&scene.layers[1].overlay).into_iter().find(|t| t.rotate_ccw == 90.0).unwrap();
    assert!(near(title.x - 0.72 * title.size, 3.0), "{title:?}");
}

#[test]
fn a_stack_without_room_below_shrinks_as_a_whole_to_keep_the_x_title_on_the_page() {
    let x: Vec<f64> = (0..=10).map(f64::from).collect();
    for order in [PanelOrder::BottomUp, PanelOrder::TopDown] {
        let curves = vec![curve("A", "V", &x, |v| v), curve("B", "V", &x, |v| -v), curve("C", "V", &x, |v| v * v)];
        let layout = PanelLayout { margin_bottom: 0.0, order, ..PanelLayout::stack(3) };
        let (p, g) = panel_graph(Template::Line, &layout, curves);
        let geoms = fixed_geometry(&p, g);
        let mut frames: Vec<Rect> = geoms.iter().map(|g| g.frame).collect();
        frames.sort_by(|a, b| a.y.total_cmp(&b.y));
        // Still touching, equally high, and the top stays where it was.
        assert!(near(frames[0].y, 0.10 * 460.8), "{frames:?}");
        for w in frames.windows(2) {
            assert!(near(w[0].y + w[0].h, w[1].y) && near(w[0].h, w[1].h), "{frames:?}");
        }
        let svg = svg_of(&p, g);
        let d = doc(&svg);
        let title = d.descendants().find(|n| n.has_tag_name("text") && n.text() == Some("Potential (V)")).unwrap();
        let y: f64 = title.attribute("y").unwrap().parse().unwrap();
        let size: f64 = title.attribute("font-size").unwrap().parse().unwrap();
        assert!((y + 0.21 * size - (460.8 - 3.0)).abs() < 1e-2, "{order:?}: the title ends at {}", y + 0.21 * size);
    }
}

#[test]
fn panels_side_by_side_that_share_y_hide_the_inner_y_labels_and_the_joint_x_label() {
    let x: Vec<f64> = (0..=10).map(f64::from).collect();
    let curves = vec![curve("A", "V", &x, |v| v), curve("B", "V", &x, |v| 2.0 * v)];
    let layout = PanelLayout { h_gap: 0.0, link_y: true, ..PanelLayout::arrange(1, 2) };
    let (mut p, g) = panel_graph(Template::Line, &layout, curves);
    let geoms = fixed_geometry(&p, g);
    assert_eq!((geoms[0].y.min, geoms[0].y.max), (-2.0, 22.0), "one Y range for both");
    // The joint label only matters when the axis ends on a tick: fix the X limits on 0..10.
    for l in &mut p.graph_mut(g).unwrap().layers {
        l.x_axis.min = Some(0.0);
        l.x_axis.max = Some(10.0);
    }
    assert_eq!(geoms[1].y, geoms[0].y);
    let scene = layout::build_scene(&p, g, &FixedMetrics).unwrap();
    assert!(left_labels(&scene, 1).is_empty());
    assert!(!texts_of(&scene.layers[1].overlay).iter().any(|t| t.rotate_ccw == 90.0), "no second Y title");
    // Panel 0 leaves out its right-most X label (10), which would meet panel 1's first one (0).
    let x0 = centred_texts(&scene, 0);
    assert!(x0.contains(&"8".to_string()) && !x0.contains(&"10".to_string()), "{x0:?}");
    assert!(centred_texts(&scene, 1).contains(&"10".to_string()));
}

#[test]
fn a_right_y_axis_in_a_panel_lists_only_that_panels_curves_and_follows_the_shared_x() {
    let a: Vec<f64> = (0..=10).map(f64::from).collect();
    let b: Vec<f64> = (5..=20).map(f64::from).collect();
    let curves = vec![curve("A", "V", &a, |v| v), curve("B", "V", &b, |v| v)];
    let (mut p, g) = panel_graph(Template::Line, &PanelLayout::stack(2), curves);
    let right = p.add_right_y_layer(g, 1).unwrap();
    let book = p.workbooks().nth(1).unwrap().id;
    let q = p.add_column(book, 0, Designation::Y, ColumnData::numeric(b.iter().map(|v| v * 0.01).collect())).unwrap();
    p.column_mut(q).unwrap().long_name = "Q".into();
    p.column_mut(q).unwrap().units = "C".into();
    p.add_plot(g, right, q).unwrap();
    let geoms = fixed_geometry(&p, g);
    assert_eq!(geoms[right].frame, geoms[1].frame);
    assert!(geoms.iter().all(|geom| geom.x == geoms[0].x), "one X axis for all");
    assert_eq!((geoms[0].x.min, geoms[0].x.max), (-2.0, 22.0));
    let svg = svg_of(&p, g);
    for name in ["A", "B", "Q"] {
        assert_eq!(svg.matches(&format!(">{name}</text>")).count(), 1, "{name} is listed once");
    }
    // Panel 1's legend (drawn last) lists B and Q; panel 0's lists A.
    let scene = layout::build_scene(&p, g, &FixedMetrics).unwrap();
    let last: Vec<String> = texts_of(&scene.layers[right].overlay)
        .into_iter()
        .filter(|t| t.anchor == scene::Anchor::Start)
        .map(|t| t.text.clone())
        .collect();
    assert!(last.ends_with(&["B".to_string(), "Q".to_string()]), "{last:?}");
}

#[test]
fn golden_cv_stack() {
    let (x, shape) = golden_data();
    let mk = |name: &'static str, amp: f64| Curve { name, unit: "mA", x: x.clone(), y: shape(amp) };
    let curves = vec![mk("Pristine", 30.0), mk("10 cycles", 21.5), mk("100 cycles", 12.25)];
    let (p, g) = panel_graph(Template::CyclicVoltammetry, &PanelLayout::stack(3), curves);
    compare_golden("cv-stack", &svg_of(&p, g));
}

#[test]
fn golden_grid_of_four_panels() {
    let (x, shape) = golden_data();
    let mk = |name: &'static str, amp: f64| Curve { name, unit: "mA", x: x.clone(), y: shape(amp) };
    let curves = vec![mk("A", 30.0), mk("B", 21.5), mk("C", 12.25), mk("D", 5.5)];
    let frame = Template::Line.new_graph(WindowId(0), "").layers[0].frame;
    let (p, g) = panel_graph(Template::Line, &PanelLayout::grid(2, 2, &frame), curves);
    compare_golden("grid-2x2", &svg_of(&p, g));
}

#[test]
fn a_point_of_the_page_is_traced_back_to_the_axis_plot_legend_or_layer_under_it() {
    use crate::hit::{Hit, hit_test};
    use crate::scene::ObjectKind;
    let x: Vec<f64> = (0..10).map(f64::from).collect();
    let low: Vec<f64> = x.iter().map(|v| v * 0.5).collect();
    let high: Vec<f64> = x.iter().map(|v| 20.0 - v).collect();
    let (p, g) = project(Template::Line, x, vec![low, high]);
    let scene = layout::build_scene(&p, g, &FixedMetrics).unwrap();
    // The frame as laid out with the same text metrics as the scene.
    let mut geom = layout_geometry(&p, g).unwrap().remove(0);
    geom.frame = scene.layers[0].clip;
    let f = geom.frame;
    let on = |x: f64, y: f64| hit_test(&scene, geom.data_to_page(x, y), 2.0);
    let object = |kind| Hit::Object { layer: 0, kind };
    // On each curve, away from where they cross.
    assert_eq!(on(2.0, 1.0), object(ObjectKind::Plot(0)));
    assert_eq!(on(2.0, 18.0), object(ObjectKind::Plot(1)));
    // In the frame but far from both curves.
    let (mx, my) = geom.data_to_page(8.0, 7.0);
    assert_eq!(hit_test(&scene, (mx, my), 2.0), Hit::Layer(0));
    // On the bottom and left axis lines, and on a tick label below the frame.
    assert_eq!(hit_test(&scene, (f.x + f.w * 0.3, f.y + f.h + 0.5), 2.0), object(ObjectKind::XAxis));
    assert_eq!(hit_test(&scene, (f.x - 0.5, f.y + f.h * 0.6), 2.0), object(ObjectKind::YAxis));
    let boxes = scene.layers[0].objects.iter().filter(|o| o.kind == ObjectKind::XAxis).flat_map(|o| &o.boxes);
    let tick_label = boxes.into_iter().find(|b| b.y > f.y + f.h + 1.0).unwrap();
    let c = (tick_label.x + tick_label.w / 2.0, tick_label.y + tick_label.h / 2.0);
    assert_eq!(hit_test(&scene, c, 1.0), object(ObjectKind::XAxis));
    // The legend and the titles.
    for kind in [ObjectKind::Legend, ObjectKind::XTitle, ObjectKind::YTitle] {
        let o = scene.layers[0].objects.iter().find(|o| o.kind == kind).unwrap();
        let c = (o.bounds.x + o.bounds.w / 2.0, o.bounds.y + o.bounds.h / 2.0);
        assert_eq!(hit_test(&scene, c, 1.0), object(kind), "{kind:?}");
    }
    // The top edge of a line graph has no axis line: inside the frame it is the layer, above it the page.
    assert_eq!(hit_test(&scene, (f.x + f.w * 0.5, f.y - 8.0), 1.0), Hit::Page);
}

#[test]
fn an_area_plot_fills_down_to_zero_and_breaks_at_gaps() {
    let x: Vec<f64> = (0..6).map(f64::from).collect();
    let y = vec![1.0, 2.0, f64::NAN, 2.0, 3.0, 1.0];
    let (mut p, g) = project(Template::Line, x, vec![y]);
    p.graph_mut(g).unwrap().layers[0].plots[0].kind = PlotKind::Area;
    let scene = layout::build_scene(&p, g, &FixedMetrics).unwrap();
    let fills: Vec<&str> = scene.layers[0]
        .plots
        .iter()
        .filter_map(|i| match i {
            crate::scene::Item::Path { d, fill: Some(_), .. } => Some(d.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(fills.len(), 1, "one path");
    assert_eq!(fills[0].matches('Z').count(), 2, "two closed shapes, split at the gap");
    // The fill starts on the page height of Y = 0.
    let mut geom = layout_geometry(&p, g).unwrap().remove(0);
    geom.frame = scene.layers[0].clip;
    let zero = geom.data_to_page(0.0, 0.0).1;
    let first = crate::hit::path_points(fills[0])[0];
    assert!((first.1 - zero).abs() < 0.01 || (first.1 - (geom.frame.y + geom.frame.h)).abs() < 0.01);
    // A line is drawn over the fill.
    assert!(
        scene.layers[0].plots.iter().any(|i| matches!(i, crate::scene::Item::Path { fill: None, stroke: Some(_), .. }))
    );
}

#[test]
fn export_size_scales_the_whole_page() {
    let none = ExportSize::default();
    assert_eq!(none.scale(720.0, 540.0).unwrap(), 1.0);
    // 5 in wide on a 10 in page: half size.
    let w = ExportSize { width_in: Some(5.0), height_in: None, ratio: None };
    assert!((w.scale(720.0, 540.0).unwrap() - 0.5).abs() < 1e-12);
    // Both given: the page fits inside both.
    let both = ExportSize { width_in: Some(5.0), height_in: Some(2.0), ratio: None };
    assert!((both.scale(720.0, 540.0).unwrap() - 2.0 * 72.0 / 540.0).abs() < 1e-12);
    for bad in [0.0, -1.0, f64::NAN, 1e9] {
        assert!(ExportSize { width_in: Some(bad), height_in: None, ratio: None }.scale(720.0, 540.0).is_err(), "{bad}");
    }
    // Only the root size of the SVG changes; the drawing keeps its units through the viewBox.
    let (p, g) = project(Template::Line, vec![0.0, 1.0, 2.0], vec![vec![0.0, 1.0, 4.0]]);
    let scene = crate::layout::build_scene(&p, g, &FixedMetrics).unwrap();
    let page = crate::Rect { x: 0.0, y: 0.0, w: scene.width, h: scene.height };
    let half = crate::svg::write_svg_view(&scene, page, 0.5);
    let (w, h) = (scene.width, scene.height);
    let (mut a, mut b, mut c, mut d) = (String::new(), String::new(), String::new(), String::new());
    crate::scene::push_num(&mut a, w / 2.0);
    crate::scene::push_num(&mut b, h / 2.0);
    crate::scene::push_num(&mut c, w);
    crate::scene::push_num(&mut d, h);
    let want = format!("width=\"{a}pt\" height=\"{b}pt\" viewBox=\"0 0 {c} {d}\"");
    assert!(half.contains(&want), "{half}");
}

#[test]
fn sized_exports_and_tiff() {
    let (p, g) = project(Template::Line, vec![0.0, 1.0, 2.0], vec![vec![0.0, 1.0, 4.0]]);
    let page = p.graph(g).unwrap().page.clone();
    let half = ExportSize { width_in: Some(page.width_pt / 72.0 / 2.0), height_in: None, ratio: None };
    // PNG: the pixel width follows the export width at the chosen resolution.
    let png = export_bytes(&p, g, "png", 100.0, half).unwrap();
    let width = u32::from_be_bytes([png[16], png[17], png[18], png[19]]);
    assert_eq!(f64::from(width), (page.width_pt / 72.0 / 2.0 * 100.0).ceil());
    // PDF: the page size follows it too.
    let pdf = String::from_utf8_lossy(&export_bytes(&p, g, "pdf", 300.0, half).unwrap()).into_owned();
    let media = pdf.split("MediaBox [0 0 ").nth(1).expect("a MediaBox");
    let pdf_width: f64 = media.split_whitespace().next().unwrap().parse().unwrap();
    assert!((pdf_width - page.width_pt / 2.0).abs() < 0.01, "the PDF page is half as wide: {pdf_width}");
    // TIFF: an RGB image on white with the resolution recorded.
    let tif = render_tiff(&p, g, 150.0).unwrap();
    let mut dec = tiff::decoder::Decoder::new(std::io::Cursor::new(tif)).unwrap();
    let (w, h) = dec.dimensions().unwrap();
    assert_eq!(f64::from(w), (page.width_pt / 72.0 * 150.0).ceil());
    assert!(h > 0);
    assert_eq!(dec.colortype().unwrap(), tiff::ColorType::RGB(8));
    let res = dec.get_tag_u32_vec(tiff::tags::Tag::XResolution).unwrap();
    assert_eq!(res, vec![15000, 100], "150 dpi");
    let tiff::decoder::DecodingResult::U8(pixels) = dec.read_image().unwrap() else { panic!("8-bit image") };
    assert_eq!(&pixels[..3], &[255, 255, 255], "the corner of the page is white");
    assert!(matches!(export_bytes(&p, g, "bmp", 72.0, ExportSize::default()), Err(PlotError::UnknownFormat(_))));
}
