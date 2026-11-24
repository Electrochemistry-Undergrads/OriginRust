use crate::Rect;
use crate::fonts::{FixedMetrics, TextLineMetrics, TextMeasurer};
use crate::scene::{Anchor, Item, LayerScene, ObjectKind, Scene, Stroke};
use sheaf_core::graph::{Font, PlotKind, TextLabel};
use sheaf_core::{Color, ColumnData, Designation, Project};

#[test]
fn underline_draws_a_line_in_addition_to_the_glyphs() {
    let mut items = Vec::new();
    crate::text::place(
        &mut items,
        &FixedMetrics,
        r"\u(abc)",
        &Font::new("Arial", 20.0),
        20.0,
        (100.0, 60.0),
        Anchor::Start,
        0.0,
    );
    assert_eq!(items.iter().filter(|i| matches!(i, Item::Text(_))).count(), 1);
    assert_eq!(items.iter().filter(|i| matches!(i, Item::Path { .. })).count(), 1);
}

#[test]
fn an_underlined_font_underlines_every_run() {
    // T293: Font::underline draws the line under plain text and under each run of styled text.
    let font = Font { underline: true, ..Font::new("Arial", 20.0) };
    let lines = |text: &str, font: &Font| {
        let mut items = Vec::new();
        crate::text::place(&mut items, &FixedMetrics, text, font, 20.0, (100.0, 60.0), Anchor::Middle, 0.0);
        items.iter().filter(|i| matches!(i, Item::Path { .. })).count()
    };
    assert_eq!(lines("abc", &Font::new("Arial", 20.0)), 0);
    assert_eq!(lines("abc", &font), 1);
    assert_eq!(lines(r"a\b(b)c", &font), 3, "one line per run");
    assert_eq!(lines(r"\u(abc)", &font), 1, "markup underline is not doubled");
}

fn placed(text: &str, anchor: Anchor, angle: f64) -> Vec<Item> {
    let mut items = Vec::new();
    crate::text::place(&mut items, &FixedMetrics, text, &Font::new("Arial", 20.0), 20.0, (100.0, 60.0), anchor, angle);
    items
}

#[test]
fn free_label_rows_keep_inline_styles_and_add_separator_height() {
    let mut items = Vec::new();
    crate::text::place_label(
        &mut items,
        &FixedMetrics,
        "\\b(First\nSecond)\r\n \t\\sep:50 \r\\u(Third)",
        &Font::new("Arial", 20.0),
        (100.0, 60.0),
        0.0,
        &|_| None,
    );
    let rows: Vec<_> = items
        .iter()
        .filter_map(|item| match item {
            Item::Text(t) => Some((t.text.as_str(), t.x, t.y, t.bold)),
            _ => None,
        })
        .collect();
    assert_eq!(rows, [("First", 100.0, 60.0, true), ("Second", 100.0, 86.0, true), ("Third", 100.0, 122.0, false)]);
    assert_eq!(paths(&items).len(), 1);
}

#[test]
fn free_label_separators_rotate_with_rows_and_leave_literal_text_visible() {
    let mut items = Vec::new();
    let font = Font::new("Arial", 20.0);
    crate::text::place_label(
        &mut items,
        &FixedMetrics,
        "\\sep:900\nA\n\\sep:50\n\\sep:25\nB\n\\sep:1000\n",
        &font,
        (100.0, 60.0),
        90.0,
        &|_| None,
    );
    let [Item::Text(a), Item::Text(b)] = items.as_slice() else { panic!("expected just two label rows") };
    assert_eq!((a.text.as_str(), a.x, a.y, b.text.as_str()), ("A", 100.0, 60.0, "B"));
    assert!((b.x - 141.0).abs() < 1e-12 && (b.y - 60.0).abs() < 1e-12);
    assert_eq!(b.rotate_ccw, 90.0);

    items.clear();
    crate::text::place_label(
        &mut items,
        &FixedMetrics,
        "\\v(A\n\\sep:50\nB)\n\\sep:-1\nC",
        &font,
        (100.0, 60.0),
        0.0,
        &|_| None,
    );
    let rows: Vec<_> = items
        .iter()
        .filter_map(|item| match item {
            Item::Text(t) => Some((t.text.as_str(), t.y)),
            _ => None,
        })
        .collect();
    assert_eq!(rows, [("A", 60.0), ("\\sep:50", 86.0), ("B", 112.0), ("\\sep:-1", 138.0), ("C", 164.0)]);
}

#[test]
fn free_label_separator_rows_share_one_pickable_scene_object_and_export() {
    use crate::hit::{Hit, hit_test};

    let mut p = Project::new();
    let graph = p.add_graph(sheaf_core::template::Template::Line);
    let label = TextLabel::new("Upper\n\\sep:50\nLower", 0.4, 0.3, Font::new("Arial", 20.0));
    p.graph_mut(graph).unwrap().layers[0].labels.push(label);
    let s = crate::layout::build_scene(&p, graph, &FixedMetrics).unwrap();
    let layer = &s.layers[0];
    let label = layer.objects.iter().find(|o| o.kind == ObjectKind::Label(0)).unwrap();
    let [Item::Text(upper), Item::Text(lower)] = &layer.overlay[label.items.clone()] else {
        panic!("expected the two rendered rows");
    };
    assert_eq!((upper.text.as_str(), lower.text.as_str()), ("Upper", "Lower"));
    assert_eq!(lower.y - upper.y, 36.0);
    assert_eq!(label.boxes.len(), 2);
    for b in &label.boxes {
        let picked = hit_test(&s, (b.x + b.w / 2.0, b.y + b.h / 2.0), 0.0);
        assert_eq!(picked, Hit::Object { layer: 0, kind: ObjectKind::Label(0) });
    }
    let svg = crate::svg::write_svg(&s);
    assert!(svg.contains(">Upper</text>") && svg.contains(">Lower</text>"));
    assert!(!svg.contains("\\sep:50"));
}

#[test]
fn plot_color_escape_follows_the_label_layer_and_saved_plot_slot() {
    let mut project = Project::new();
    let book = project.add_workbook("");
    project.add_column(book, 0, Designation::X, ColumnData::numeric(vec![0.0, 1.0])).unwrap();
    let first = project.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![1.0, 2.0])).unwrap();
    let second = project.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![2.0, 3.0])).unwrap();
    let graph = project.add_graph(sheaf_core::template::Template::Line);
    project.add_plot(graph, 0, first).unwrap();
    project.add_plot(graph, 0, second).unwrap();
    let graph_model = project.graph_mut(graph).unwrap();
    let layer = &mut graph_model.layers[0];
    layer.plots[0].visible = false;
    layer.plots[1].line.color = Color::rgb(21, 44, 109);
    layer.labels.push(TextLabel::new(r"\cp2(\u(First layer))", 0.5, 0.3, Font::new("Arial", 20.0)));
    let mut other = layer.clone();
    other.plots[1].kind = PlotKind::Scatter;
    other.plots[1].symbol.color = Color::rgb(107, 83, 41);
    other.labels[0].text = r"\cp2(\u(Second layer))".into();
    graph_model.layers.push(other);

    let scene = crate::layout::build_scene(&project, graph, &FixedMetrics).unwrap();
    let svg = crate::svg::write_svg(&scene);
    for (layer, text, color) in [
        (&scene.layers[0], "First layer", Color::rgb(21, 44, 109)),
        (&scene.layers[1], "Second layer", Color::rgb(107, 83, 41)),
    ] {
        let label = layer.objects.iter().find(|object| object.kind == ObjectKind::Label(0)).unwrap();
        let items = &layer.overlay[label.items.clone()];
        assert!(items.iter().any(|item| matches!(item, Item::Text(t) if t.text == text && t.color == color)));
        assert_eq!(paths(items).iter().map(|(_, stroke)| stroke.color).collect::<Vec<_>>(), [color]);
        let svg_text = svg.lines().find(|line| line.ends_with(&format!(">{text}</text>"))).unwrap();
        assert!(svg_text.contains(&format!("fill=\"{}\"", color.hex_rgb())));
    }
}

#[test]
fn plot_color_escape_nests_and_keeps_the_surrounding_color_for_missing_indices() {
    let mut items = Vec::new();
    let mut font = Font::new("Arial", 20.0);
    font.color = Color::rgb(78, 34, 12);
    let plot_color = Color::rgb(10, 63, 99);
    crate::text::place_label(
        &mut items,
        &FixedMetrics,
        "\\b(\\cp1(Alpha\nBeta \\cp3(missing) \\c2(red) return) outside) \\v(\\cp1(literal)) \\cp0(zero)",
        &font,
        (100.0, 60.0),
        0.0,
        &|index| (index == 0).then_some(plot_color),
    );
    let runs: Vec<_> = items
        .iter()
        .filter_map(|item| match item {
            Item::Text(t) => Some((t.text.as_str(), t.color, t.bold)),
            _ => None,
        })
        .collect();
    assert_eq!(
        runs,
        [
            ("Alpha", plot_color, true),
            ("Beta missing ", plot_color, true),
            ("red", Color::RED, true),
            (" return", plot_color, true),
            (" outside", font.color, true),
            (r" \cp1(literal) zero", font.color, false),
        ]
    );
    assert_eq!(crate::markup::plain(r"\cp1(context-free)"), r"\cp1(context-free)");
}

#[test]
fn plot_colored_cartesian_titles_match_explicit_color_on_each_edge() {
    use sheaf_core::graph::AxisSide;
    use sheaf_core::template::Template;

    let mut p = Project::new();
    let book = p.add_workbook("");
    p.add_column(book, 0, Designation::X, ColumnData::numeric(vec![0.0, 1.0])).unwrap();
    let a = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![1.0, 2.0])).unwrap();
    let b = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![2.0, 3.0])).unwrap();
    for (template, side) in [
        (Template::Line, AxisSide::Normal),
        (Template::Line, AxisSide::Opposite),
        (Template::ScatterCentral, AxisSide::Normal),
    ] {
        let graph = p.add_graph(template);
        p.add_plot(graph, 0, a).unwrap();
        p.add_plot(graph, 0, b).unwrap();
        let layer = &mut p.graph_mut(graph).unwrap().layers[0];
        for (plot, color) in layer.plots.iter_mut().zip([Color::BLUE, Color::RED]) {
            plot.line.color = color;
            plot.symbol.color = color;
        }
        for (axis, text) in [(&mut layer.x_axis, "X field"), (&mut layer.y_axis, "Y field")] {
            axis.side = side;
            axis.title_color_follows_plot = true;
            axis.title = Some(format!(r"\cp2(\u({text}))"));
        }
        let actual = crate::layout::build_scene(&p, graph, &FixedMetrics).unwrap();
        let layer = &mut p.graph_mut(graph).unwrap().layers[0];
        layer.x_axis.title = Some(r"\c2(\u(X field))".into());
        layer.y_axis.title = Some(r"\c2(\u(Y field))".into());
        let expected = crate::layout::build_scene(&p, graph, &FixedMetrics).unwrap();
        assert_eq!(crate::svg::write_svg(&actual), crate::svg::write_svg(&expected));
        for (kind, title) in [(ObjectKind::XTitle, "X field"), (ObjectKind::YTitle, "Y field")] {
            let layer = &actual.layers[0];
            let object = layer.objects.iter().find(|object| object.kind == kind).unwrap();
            let items = &layer.overlay[object.items.clone()];
            assert!(items.iter().any(|item| matches!(item, Item::Text(t) if t.text == title && t.color == Color::RED)));
            assert!(paths(items).iter().any(|(_, stroke)| stroke.color == Color::RED));
        }
    }
}

fn paths(items: &[Item]) -> Vec<(&str, &Stroke)> {
    items
        .iter()
        .filter_map(|i| match i {
            Item::Path { d, stroke: Some(stroke), .. } => Some((d.as_str(), stroke)),
            _ => None,
        })
        .collect()
}

fn scene(items: Vec<Item>) -> Scene {
    Scene {
        width: 220.0,
        height: 140.0,
        background: Color::WHITE,
        layers: vec![LayerScene {
            clip: Rect { x: 0.0, y: 0.0, w: 220.0, h: 140.0 },
            grid: vec![],
            plots: vec![],
            overlay: items,
            underlay_len: 0,
            data_on_top: false,
            objects: vec![],
        }],
    }
}

#[test]
fn nested_decoration_styles_end_at_their_own_closing_parentheses() {
    let runs = crate::markup::parse(r"a\u(b\s(c)d)e");
    assert_eq!(
        runs.iter().map(|r| (r.text.as_str(), r.underline, r.strikeout)).collect::<Vec<_>>(),
        [("a", false, false), ("b", true, false), ("c", true, true), ("d", true, false), ("e", false, false)]
    );
    assert_eq!(crate::markup::plain(r"\u(µ\s(A))"), "µA");
    assert!(crate::markup::parse(r"\u(\s(\b(\i(x))))")[0].bold);
    let deeply_nested = r"\u(".repeat(1000) + "x" + &")".repeat(1000);
    assert!(!crate::markup::parse(&deeply_nested).is_empty());
}

#[test]
fn decorations_keep_the_glyph_anchor_and_rotation() {
    for anchor in [Anchor::Start, Anchor::Middle, Anchor::End] {
        for angle in [0.0, 90.0, 37.0] {
            let plain = placed("abc", anchor, angle);
            let decorated = placed(r"\u(abc)", anchor, angle);
            assert_eq!(plain[0], decorated[0], "{anchor:?} {angle}");
            assert_eq!(paths(&decorated).len(), 1);
        }
    }
}

#[test]
fn decorations_rotate_and_center_with_the_text() {
    let items = placed(r"\u(abc)", Anchor::Middle, 90.0);
    let lines = paths(&items);
    assert_eq!(lines[0].0, "M102.8 76.5L102.8 43.5");
    assert!((lines[0].1.width - 1.4).abs() < 1e-12);
    let items = placed(r"\s(abc)", Anchor::End, 0.0);
    assert_eq!(paths(&items)[0].0, "M67 55.4L100 55.4");
}

#[test]
fn overbar_spans_nested_text_and_paired_scripts_without_adding_advance() {
    let body = r"P\=(i,xy)";
    let decorated = r"\ab(P\=(i,xy))";
    let font = Font::new("Arial", 20.0);
    let width = crate::text::width(&FixedMetrics, decorated, &font, 20.0);
    assert_eq!(width, crate::text::width(&FixedMetrics, body, &font, 20.0));
    assert!((width - 24.2).abs() < 1e-12);
    assert_eq!(crate::markup::parse(decorated), crate::markup::parse(body));
    let items = placed(decorated, Anchor::Start, 0.0);
    assert_eq!(&items[..3], &placed(body, Anchor::Start, 0.0));
    assert_eq!(paths(&items)[0].0, "M100 39.16L124.2 39.16");

    let items = placed(r"\c2(\ab(a\b(b)\c4(c)))d", Anchor::Start, 0.0);
    let bars = paths(&items);
    assert_eq!(bars.len(), 1);
    assert_eq!(bars[0].0, "M100 43.2L133 43.2");
    assert_eq!(bars[0].1.color, Color::RED);
    assert!(matches!(items.last(), Some(Item::Text(item)) if item.text == "d" && item.color == Color::BLACK));
}

#[test]
fn overbars_keep_the_single_run_anchor_rotate_and_stack() {
    let items = placed(r"\ab(abc)", Anchor::Middle, 90.0);
    assert_eq!(items[0], placed("abc", Anchor::Middle, 90.0)[0]);
    assert_eq!(paths(&items)[0].0, "M83.2 76.5L83.2 43.5");
    let nested = placed(r"\ab(a\ab(b)c)", Anchor::Start, 0.0);
    assert_eq!(paths(&nested).iter().map(|p| p.0).collect::<Vec<_>>(), ["M111 43.2L122 43.2", "M100 40.4L133 40.4"]);
    let svg = crate::svg::write_svg(&scene(nested));
    assert!(svg.contains("M100 40.4L133 40.4"));

    assert!(placed(r"\ab()", Anchor::Start, 0.0).is_empty());
    assert_eq!(crate::markup::plain(r"\v(\ab(x))"), r"\ab(x)");
    assert_eq!(crate::markup::plain(r"\abc(x)"), r"\abc(x)");
}

#[test]
fn dot_and_tilde_are_centered_over_text_and_keep_glyph_placement() {
    let dot = placed(r"\ad(abc)", Anchor::Middle, 90.0);
    assert_eq!(dot[0], placed("abc", Anchor::Middle, 90.0)[0]);
    assert!(matches!(&dot[1], Item::Path { d, fill: Some(Color::BLACK), stroke: None }
        if d.starts_with("M82.4 58.8") && d.ends_with('Z')));

    let tilde = placed(r"\at(abc)", Anchor::Start, 0.0);
    assert_eq!(tilde[0], placed("abc", Anchor::Start, 0.0)[0]);
    let lines = paths(&tilde);
    assert_eq!(lines.len(), 1);
    assert!(lines[0].0.starts_with("M111 41.9") && lines[0].0.ends_with("L122 41.9"));
    assert!((lines[0].1.width - 0.8).abs() < 1e-12);
}

#[test]
fn mixed_accents_stack_and_export_without_changing_text_or_script_advance() {
    let source = r"\c2(\ab(\ad(x)))\at(P\=(i,xy))";
    let plain = r"\c2(x)P\=(i,xy)";
    let font = Font::new("Arial", 20.0);
    assert_eq!(crate::markup::parse(source), crate::markup::parse(plain));
    assert_eq!(
        crate::text::width(&FixedMetrics, source, &font, 20.0),
        crate::text::width(&FixedMetrics, plain, &font, 20.0),
    );
    let items = placed(source, Anchor::Start, 0.0);
    let bars = paths(&items);
    assert_eq!(bars.len(), 2);
    assert_eq!(bars[0].0, "M100 38.8L111 38.8");
    assert_eq!(bars[0].1.color, Color::RED);
    assert!(bars[1].0.starts_with("M117.6 37.86"));
    let svg = crate::svg::write_svg(&scene(items));
    assert!(svg.contains("M100 38.8L111 38.8"));
    assert!(svg.contains("M117.6 37.86"));
    assert_eq!(crate::markup::plain(r"\v(\ad(x)\at(y))"), r"\ad(x)\at(y)");
    assert!(placed(r"\ad()\at()", Anchor::Start, 0.0).is_empty());
}

#[test]
fn decorated_scripts_move_and_shrink_their_lines_with_the_glyphs() {
    let items = placed(r"\u(a\+(2)b)", Anchor::Start, 0.0);
    let lines = paths(&items);
    assert_eq!(
        lines.iter().map(|l| l.0).collect::<Vec<_>>(),
        ["M100 62.8L111 62.8", "M111 54.82L118.15 54.82", "M118.15 62.8L129.15 62.8"]
    );
    assert!((lines[1].1.width - 0.91).abs() < 1e-12);
}

#[test]
fn decorated_bold_runs_use_their_own_font_metrics_and_colour() {
    struct Metrics;
    impl TextMeasurer for Metrics {
        fn width(&self, s: &str, f: &Font, size: f64) -> f64 {
            FixedMetrics.width(s, f, size)
        }
        fn underline_metrics(&self, f: &Font) -> TextLineMetrics {
            TextLineMetrics { center: if f.bold { 0.3 } else { 0.1 }, thickness: 0.05 }
        }
    }
    let mut font = Font::new("Arial", 20.0);
    font.color = Color::RED;
    let mut items = Vec::new();
    crate::text::place(&mut items, &Metrics, r"\u(a\b(b))", &font, 20.0, (0.0, 50.0), Anchor::Start, 0.0);
    let lines = paths(&items);
    assert_eq!(lines.iter().map(|l| l.0).collect::<Vec<_>>(), ["M0 52L11 52", "M11 56L22 56"]);
    assert!(lines.iter().all(|(_, s)| s.color == Color::RED && s.width == 1.0));
}

fn decorated_scene() -> Scene {
    let mut items = placed(r"plain \u(under) \s(strike)", Anchor::Middle, 0.0);
    crate::text::place(
        &mut items,
        &FixedMetrics,
        r"\u(\s(both))",
        &Font::new("Arial", 16.0),
        16.0,
        (40.0, 110.0),
        Anchor::Start,
        0.0,
    );
    crate::text::place(
        &mut items,
        &FixedMetrics,
        r"\u(x\+(2))",
        &Font::new("Arial", 16.0),
        16.0,
        (185.0, 110.0),
        Anchor::Start,
        90.0,
    );
    scene(items)
}

#[test]
fn decoration_svg_and_raster_geometry_match_reviewed_goldens() {
    let mut scene = decorated_scene();
    crate::tests::compare_golden("text-decorations", &crate::svg::write_svg(&scene));
    // The PNG golden contains only the decoration strokes, so installed glyph fonts cannot
    // change this cross-platform geometry check. The SVG golden includes all text runs.
    scene.layers[0].overlay.retain(|i| !matches!(i, Item::Text(_)));
    let raster = crate::render_scene_raster(&scene, 2.0).unwrap();
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/text-decorations.png");
    if std::env::var_os("SHEAF_UPDATE_GOLDEN").is_some() {
        std::fs::write(&path, crate::raster::encode_png(&raster, 144.0).unwrap()).unwrap();
    }
    let mut decoder = png::Decoder::new(std::io::Cursor::new(std::fs::read(&path).unwrap()));
    decoder.set_transformations(png::Transformations::EXPAND);
    let mut reader = decoder.read_info().unwrap();
    let mut bytes = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut bytes).unwrap();
    assert_eq!((info.width, info.height), (raster.width, raster.height));
    assert_eq!(&bytes[..info.buffer_size()], raster.rgba);
}

#[test]
fn decoration_strokes_reach_eps_emf_pdf_and_the_screen_raster() {
    let scene = decorated_scene();
    let count = paths(&scene.layers[0].overlay).len();
    assert_eq!(count, 6);
    let eps = String::from_utf8(crate::eps::write_eps(&scene, 1.0).bytes).unwrap();
    assert_eq!(eps.split_whitespace().filter(|s| *s == "stroke").count(), count);
    let emf = crate::emf::write_emf(&scene, 1.0, &FixedMetrics);
    let mut at = 0;
    let mut begins = 0;
    while at + 8 <= emf.len() {
        let kind = u32::from_le_bytes(emf[at..at + 4].try_into().unwrap());
        let len = u32::from_le_bytes(emf[at + 4..at + 8].try_into().unwrap()) as usize;
        assert!(len >= 8 && at + len <= emf.len());
        begins += usize::from(kind == 59); // EMR_BEGINPATH
        at += len;
    }
    assert_eq!(begins, count);
    let svg = crate::svg::write_svg(&scene);
    let pdf = crate::raster::to_pdf(&svg, crate::SystemFonts::shared()).unwrap();
    assert!(pdf.starts_with(b"%PDF-") && pdf.trim_ascii_end().ends_with(b"%%EOF"));
    let raster = crate::render_scene_raster(&self::scene(placed(r"\u(abc)", Anchor::Start, 0.0)), 4.0).unwrap();
    let plain = crate::render_scene_raster(&self::scene(placed("abc", Anchor::Start, 0.0)), 4.0).unwrap();
    let dark = |r: &crate::Raster| (412..500).filter(|x| r.rgba[(252 * r.width as usize + x) * 4] < 100).count();
    assert!(dark(&raster) > 70);
    assert_eq!(dark(&plain), 0);
}

#[test]
fn titles_legend_labels_and_tick_prefixes_all_use_decorated_runs() {
    let mut p = Project::new();
    let book = p.add_workbook("");
    p.add_column(book, 0, Designation::X, ColumnData::numeric(vec![0.0, 1.0])).unwrap();
    let y = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![1.0, 2.0])).unwrap();
    let g = p.add_graph(sheaf_core::template::Template::Line);
    p.add_plot(g, 0, y).unwrap();
    let layer = &mut p.graph_mut(g).unwrap().layers[0];
    layer.x_axis.title = Some(r"\u(X title)".into());
    layer.y_axis.title = Some(r"\s(Y title)".into());
    layer.x_axis.tick_prefix = r"\u(T)".into();
    layer.legend.text = Some(r"\u(legend)".into());
    let colour = Color::rgb(111, 0, 111);
    layer.x_axis.title_font.color = colour;
    layer.y_axis.title_font.color = colour;
    layer.x_axis.tick_font.color = colour;
    layer.legend.font.color = colour;
    let mut font = Font::new("Arial", 20.0);
    font.color = colour;
    layer.labels.push(TextLabel::new(r"\u(\s(label))", 0.5, 0.5, font));
    let s = crate::layout::build_scene(&p, g, &FixedMetrics).unwrap();
    for (kind, minimum) in [
        (ObjectKind::XTitle, 1),
        (ObjectKind::YTitle, 1),
        (ObjectKind::Legend, 1),
        (ObjectKind::Label(0), 2),
        (ObjectKind::XAxis, 2),
    ] {
        // Axis lines and tick labels have separate pickable records of the same kind.
        let count = s.layers[0]
            .objects
            .iter()
            .filter(|o| o.kind == kind)
            .map(|object| {
                paths(&s.layers[0].overlay[object.items.clone()])
                    .iter()
                    .filter(|(_, stroke)| stroke.color == colour)
                    .count()
            })
            .sum::<usize>();
        assert!(count >= minimum, "{kind:?}: {count}");
    }
}

#[test]
fn legend_protected_text_exports_literal_references_and_expands_the_remaining_rows() {
    let mut p = Project::new();
    let book = p.add_workbook("");
    p.add_column(book, 0, Designation::X, ColumnData::numeric(vec![0.0, 1.0])).unwrap();
    let y = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![1.0, 2.0])).unwrap();
    p.column_mut(y).unwrap().long_name = "Series".into();
    let graph = p.add_graph(sheaf_core::template::Template::Line);
    p.add_plot(graph, 0, y).unwrap();
    let legend = &mut p.graph_mut(graph).unwrap().layers[0].legend;
    legend.text = Some(
        r"\b(\h(\l(1) %(1))) \v(\L(1) %(1))
\l(1) %(1)"
            .into(),
    );
    let s = crate::layout::build_scene(&p, graph, &FixedMetrics).unwrap();
    let layer = &s.layers[0];
    let legend = layer.objects.iter().find(|o| o.kind == ObjectKind::Legend).unwrap();
    let labels: Vec<_> = layer.overlay[legend.items.clone()]
        .iter()
        .filter_map(|item| match item {
            Item::Text(t) => Some((t.text.as_str(), t.bold)),
            _ => None,
        })
        .collect();
    assert_eq!(labels, [(r"\l(1) %(1)", true), (r" \L(1) %(1)", false), ("Series", false)]);
    let svg = crate::svg::write_svg(&s);
    assert!(svg.contains(r">\l(1) %(1)</text>"));
    assert!(svg.contains(r"> \L(1) %(1)</text>"));
    assert!(svg.contains(">Series</text>"));
}

#[test]
fn legend_conditional_spaces_follow_expanded_metadata_in_text_and_frame_width() {
    let mut project = Project::new();
    let book = project.add_workbook("");
    project.add_column(book, 0, Designation::X, ColumnData::numeric(vec![0.0, 1.0])).unwrap();
    let y = project.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![1.0, 2.0])).unwrap();
    let column = project.column_mut(y).unwrap();
    column.long_name = "Series".into();
    column.units = "mA".into();
    let graph = project.add_graph(sheaf_core::template::Template::Line);
    project.add_plot(graph, 0, y).unwrap();

    let svg = |project: &mut Project, text: &str| {
        let legend = &mut project.graph_mut(graph).unwrap().layers[0].legend;
        legend.frame = true;
        legend.text = Some(text.into());
        let scene = crate::layout::build_scene(project, graph, &FixedMetrics).unwrap();
        crate::svg::write_svg(&scene)
    };
    let conditional = r"\l(1) \b(%(1,@LL)\sc%(1,@LC)\sc%(1,@LP)\sc(%(1,@LU)))";
    for (comments, expected) in [("", r"\l(1) \b(Series mA)"), ("Sample", r"\l(1) \b(Series Sample mA)")] {
        project.column_mut(y).unwrap().comments = comments.into();
        assert_eq!(svg(&mut project, conditional), svg(&mut project, expected));
    }
    project.column_mut(y).unwrap().units.clear();
    assert_eq!(svg(&mut project, conditional), svg(&mut project, r"\l(1) \b(Series Sample)"));
}
