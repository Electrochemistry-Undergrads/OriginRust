use crate::Rect;
use crate::fonts::{FixedMetrics, TextLineMetrics, TextMeasurer};
use crate::scene::{Anchor, Item, LayerScene, ObjectKind, Scene, Stroke};
use sheaf_core::graph::{Font, TextLabel};
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

fn placed(text: &str, anchor: Anchor, angle: f64) -> Vec<Item> {
    let mut items = Vec::new();
    crate::text::place(&mut items, &FixedMetrics, text, &Font::new("Arial", 20.0), 20.0, (100.0, 60.0), anchor, angle);
    items
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
