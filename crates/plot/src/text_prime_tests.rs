//! Documented prime notation; Unicode controls test Sheaf layout, not Native glyph geometry.
use crate::fonts::FixedMetrics;
use crate::scene::{Anchor, Item, LayerScene, Scene, TextItem};
use sheaf_core::{Color, graph::Font};

fn placed(source: &str, anchor: Anchor, angle: f64) -> Vec<Item> {
    let mut items = Vec::new();
    crate::text::place(&mut items, &FixedMetrics, source, &Font::new("Arial", 20.0), 20.0, (50.0, 60.0), anchor, angle);
    items
}

fn texts(items: &[Item]) -> Vec<&TextItem> {
    items.iter().filter_map(|i| if let Item::Text(t) = i { Some(t) } else { None }).collect()
}

#[test]
fn prime_escapes_produce_distinct_full_size_characters_without_changing_quotes() {
    let runs = crate::markup::parse(r#"Z\y(') Z\y(") ' ""#);
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].text, "Z\u{2032} Z\u{2033} ' \"");
    assert_eq!((runs[0].scale, runs[0].rise), (1.0, 0.0));
    assert_eq!(crate::markup::plain(r#"\y(')\y(")\x(2032)\(8243)"#), "\u{2032}\u{2033}\u{2032}\u{2033}");
}

#[test]
fn prime_styles_fonts_greek_and_colours_nest_and_restore_like_explicit_unicode() {
    let source = r#"\b(\f:Times New Roman(\u(\c2(\g(m\y('))))))x\i(\y("))z"#;
    let expected = "\\b(\\f:Times New Roman(\\u(\\c2(\\g(m\u{2032})))))x\\i(\u{2033})z";
    let runs = crate::markup::parse(source);
    assert_eq!(runs, crate::markup::parse(expected));
    assert_eq!(runs[0].text, "\u{03bc}\u{2032}");
    assert!(runs[0].bold && runs[0].underline);
    assert_eq!(runs[0].family.as_deref(), Some("Times New Roman"));
    assert_eq!(runs[0].color, Some(Color::RED));
    assert_eq!(runs[1].text, "x");
    assert!(!runs[1].bold && !runs[1].underline);
    assert_eq!(runs[1].family, None);
    assert_eq!(runs[1].color, None);
    assert!(runs[2].italic);
    assert!(!runs[3].italic);
}

#[test]
fn malformed_literal_and_depth_limited_prime_text_retains_existing_rules() {
    for source in [r"\y()", r"\y(x)", r"\y('')", r"\y( ')", r"\y(' )", r"\Y(')", r"\y('", r"\y"] {
        assert_eq!(crate::markup::plain(source), source);
    }
    assert_eq!(crate::markup::plain(r#"\v(\y(')\y("))"#), r#"\y(')\y(")"#);
    for (depth, expected) in [(23, "\u{2032}"), (24, r"\y(')")] {
        let source = r"\b(".repeat(depth) + r"\y(')" + &")".repeat(depth);
        assert!(crate::markup::plain(&source).starts_with(expected));
    }
}

#[test]
fn primes_use_existing_script_columns_anchors_rotation_and_glyph_size() {
    for anchor in [Anchor::Start, Anchor::Middle, Anchor::End] {
        for angle in [0.0, 37.0, 90.0] {
            let actual = placed(r#"Z\y(')\+(\y("))\=(0,\y('))Q"#, anchor, angle);
            let expected = placed("Z\u{2032}\\+(\u{2033})\\=(0,\u{2032})Q", anchor, angle);
            assert_eq!(actual, expected);
        }
    }
    let items = placed(r#"Z\i(\y('))\+(\y("))Q"#, Anchor::Start, 0.0);
    let ts = texts(&items);
    assert_eq!((ts[1].text.as_str(), ts[1].size, ts[1].y), ("\u{2032}", 20.0, 60.0));
    assert_eq!((ts[2].text.as_str(), ts[2].size, ts[2].y), ("\u{2033}", 13.0, 53.0));
}

fn scene(source: &str) -> Scene {
    Scene {
        width: 240.0,
        height: 100.0,
        background: Color::WHITE,
        layers: vec![LayerScene {
            clip: crate::Rect { x: 0.0, y: 0.0, w: 240.0, h: 100.0 },
            grid: vec![],
            plots: vec![],
            overlay: placed(source, Anchor::Start, 0.0),
            underlay_len: 0,
            objects: vec![],
        }],
    }
}

#[test]
fn vector_and_raster_exports_match_explicit_unicode_prime_controls() {
    let actual = scene(r#"\u(Z\y(') \c2(\+(Z\y("))))"#);
    let expected = scene("\\u(Z\u{2032} \\c2(\\+(Z\u{2033})))");
    let svg = crate::svg::write_svg(&actual);
    assert!(svg.contains('\u{2032}') && svg.contains('\u{2033}'));
    assert!(!svg.contains(r"\y("));
    assert_eq!(svg, crate::svg::write_svg(&expected));
    assert_eq!(crate::eps::write_eps(&actual, 1.0).bytes, crate::eps::write_eps(&expected, 1.0).bytes);
    assert_eq!(
        crate::emf::write_emf(&actual, 1.0, &FixedMetrics),
        crate::emf::write_emf(&expected, 1.0, &FixedMetrics)
    );
    assert_eq!(
        crate::render_scene_raster(&actual, 2.0).unwrap().rgba,
        crate::render_scene_raster(&expected, 2.0).unwrap().rgba
    );
    let pdf = crate::raster::to_pdf(&svg, crate::SystemFonts::shared()).unwrap();
    assert!(pdf.starts_with(b"%PDF-") && pdf.trim_ascii_end().ends_with(b"%%EOF"));
}

#[test]
fn actual_legend_titles_and_tick_labels_share_the_prime_parser_and_geometry() {
    let (mut project, graph) =
        crate::tests::project(sheaf_core::template::Template::Line, vec![0.0, 1.0], vec![vec![0.0, 1.0]]);
    let layer = &mut project.graph_mut(graph).unwrap().layers[0];
    layer.legend.text = Some(r#"Z\y(') and Z\y(")"#.into());
    layer.x_axis.tick_prefix = r"\y(')".into();
    layer.x_axis.title = Some(r"Z\+(\y('))".into());
    layer.y_axis.title = Some(r#"\u(Z\y("))"#.into());
    let actual = crate::layout::build_scene(&project, graph, &FixedMetrics).unwrap();
    let layer = &mut project.graph_mut(graph).unwrap().layers[0];
    layer.legend.text = Some("Z\u{2032} and Z\u{2033}".into());
    layer.x_axis.tick_prefix = "\u{2032}".into();
    layer.x_axis.title = Some("Z\\+(\u{2032})".into());
    layer.y_axis.title = Some("\\u(Z\u{2033})".into());
    let expected = crate::layout::build_scene(&project, graph, &FixedMetrics).unwrap();
    assert_eq!(actual, expected);
    assert!(texts(&actual.layers[0].overlay).iter().any(|t| t.text.contains('\u{2033}') && t.rotate_ccw == 90.0));
}
