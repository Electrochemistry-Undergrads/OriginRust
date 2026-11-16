use crate::fonts::FixedMetrics;
use crate::scene::{Anchor, Item, LayerScene, Scene, TextItem};
use sheaf_core::Color;
use sheaf_core::graph::Font;

// Independent Origin 2016 EMR_SETTEXTCOLOR records. The first ten are Color(r,g,b)
// return values used verbatim; the remaining controls establish the one-based offset
// and the tag boundaries. Scratch: text-run-rgb/rgb[-boundaries].emf.
const CONTROLS: [(u32, (u8, u8, u8)); 22] = [
    (16777216, (0, 0, 0)),
    (16777471, (254, 0, 0)),
    (16842496, (255, 254, 0)),
    (33488896, (255, 255, 254)),
    (33554431, (254, 255, 255)),
    (25249549, (12, 71, 129)),
    (22426642, (17, 52, 86)),
    (22352368, (239, 17, 85)),
    (16974337, (0, 2, 3)),
    (18890880, (127, 64, 32)),
    (16777215, (0, 0, 0)),
    (16777216, (0, 0, 0)),
    (16777217, (0, 0, 0)),
    (16777218, (1, 0, 0)),
    (16842497, (0, 255, 0)),
    (33488897, (0, 0, 255)),
    (33554431, (254, 255, 255)),
    (33554432, (255, 255, 255)),
    // The next tag is not implemented; its native black is recorded in the spec.
    (25249550, (13, 71, 129)),
    (22426643, (18, 52, 86)),
    (22352369, (240, 17, 85)),
    (2, (255, 0, 0)), // nested palette control
];

fn placed(source: &str, font: &Font, anchor: Anchor, angle: f64) -> Vec<Item> {
    let mut items = Vec::new();
    crate::text::place(&mut items, &FixedMetrics, source, font, font.size_pt, (50.0, 60.0), anchor, angle);
    items
}

fn texts(items: &[Item]) -> Vec<&TextItem> {
    items.iter().filter_map(|i| if let Item::Text(t) = i { Some(t) } else { None }).collect()
}

#[test]
fn tagged_rgb_colour_matches_the_origin_vector_control() {
    let mut font = Font::new("Arial", 20.0);
    font.color = Color::rgb(200, 100, 20);
    let items = placed(r"\c25249549(MMMM)", &font, Anchor::Start, 0.0);
    assert_eq!((texts(&items)[0].text.as_str(), texts(&items)[0].color), ("MMMM", Color::rgb(12, 71, 129)));
}

#[test]
fn every_native_rgb_and_boundary_control_retains_its_exact_colour() {
    let mut font = Font::new("Arial", 20.0);
    font.color = Color::rgb(200, 100, 20);
    for (code, (r, g, b)) in CONTROLS {
        let source = format!(r"\c{code}(MMMM)");
        let items = placed(&source, &font, Anchor::Start, 0.0);
        assert_eq!(texts(&items).len(), 1, "{code}");
        assert_eq!((texts(&items)[0].text.as_str(), texts(&items)[0].color), ("MMMM", Color::rgb(r, g, b)), "{code}");
        assert_eq!(crate::markup::plain(&source), "MMMM");
    }
}

#[test]
fn rgb_and_palette_nesting_restore_the_measured_surrounding_colours() {
    let mut font = Font::new("Arial", 20.0);
    font.color = Color::rgba(200, 100, 20, 80);
    let items = placed(r"\c25249549(a\c2(b)c)d", &font, Anchor::Start, 0.0);
    assert_eq!(
        texts(&items).iter().map(|t| (t.text.as_str(), t.color)).collect::<Vec<_>>(),
        [("a", Color::rgb(12, 71, 129)), ("b", Color::RED), ("c", Color::rgb(12, 71, 129)), ("d", font.color)]
    );
    let items = placed(r"\u(\c25249549(a\c2(b)c)d)", &font, Anchor::Start, 90.0);
    let strokes = items
        .iter()
        .filter_map(|i| match i {
            Item::Path { stroke: Some(s), .. } => Some(s.color),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(strokes, [Color::rgb(12, 71, 129), Color::RED, Color::rgb(12, 71, 129), font.color]);
    let items = placed(r"\c25249550(\b(A)\i(B)\+(2)\g(m))", &font, Anchor::Start, 0.0);
    let runs = texts(&items);
    assert!(runs.iter().all(|t| t.color == Color::rgb(13, 71, 129)));
    assert!(runs[0].bold && runs[1].italic);
    assert_eq!((runs[2].size, runs[3].text.as_str()), (13.0, "\u{03bc}"));
}

#[test]
fn rgb_spans_preserve_glyph_anchor_rotation_and_measured_width() {
    let font = Font::new("Arial", 20.0);
    for anchor in [Anchor::Start, Anchor::Middle, Anchor::End] {
        for angle in [0.0, 37.0, 90.0] {
            let plain = placed("abc", &font, anchor, angle);
            let rgb = placed(r"\c25249550(abc)", &font, anchor, angle);
            let mut expected = texts(&plain)[0].clone();
            expected.color = Color::rgb(13, 71, 129);
            assert_eq!(*texts(&rgb)[0], expected);
        }
    }
    assert_eq!(
        crate::text::width(&FixedMetrics, r"\c25249550(abc)", &font, 20.0),
        crate::text::width(&FixedMetrics, "abc", &font, 20.0)
    );
}

#[test]
fn other_tags_overflow_and_literal_wrapping_keep_their_text_boundary() {
    for source in [
        r"\c33554433(text)",
        r"\c4294967295(text)",
        r"\c4294967296(text)",
        r"\c-16777471(text)",
        r"\c00025249550(text)",
    ] {
        assert_eq!(crate::markup::plain(source), source);
    }
    assert_eq!(crate::markup::plain(r"\v(\c25249550(text))"), r"\c25249550(text)");
    assert_eq!(crate::markup::parse(r"\c025249550(x)")[0].color, Some(Color::rgb(13, 71, 129)));
}

fn rgb_scene() -> Scene {
    let mut items = Vec::new();
    let font = Font::new("Arial", 20.0);
    for (i, (code, _)) in CONTROLS[..10].iter().enumerate() {
        crate::text::place(
            &mut items,
            &FixedMetrics,
            &format!(r"\c{code}(\u(MM))"),
            &font,
            20.0,
            (10.0 + (i % 5) as f64 * 55.0, 25.0 + (i / 5) as f64 * 35.0),
            Anchor::Start,
            0.0,
        );
    }
    Scene {
        width: 290.0,
        height: 85.0,
        background: Color::rgb(238, 238, 238),
        layers: vec![LayerScene {
            clip: crate::Rect { x: 0.0, y: 0.0, w: 290.0, h: 85.0 },
            grid: vec![],
            plots: vec![],
            overlay: items,
            underlay_len: 0,
            objects: vec![],
        }],
    }
}

#[test]
fn rgb_svg_and_stroke_png_goldens_preserve_native_fixture_colours() {
    let mut scene = rgb_scene();
    crate::tests::compare_golden("text-rgb", &crate::svg::write_svg(&scene));
    scene.layers[0].overlay.retain(|i| !matches!(i, Item::Text(_)));
    let raster = crate::render_scene_raster(&scene, 3.0).unwrap();
    for (i, (_, (r, g, b))) in CONTROLS[..10].iter().enumerate() {
        let x = (20 + (i % 5) * 55) * 3;
        let y = ((25.0 + (i / 5) as f64 * 35.0 + 2.8) * 3.0).floor() as usize;
        let at = (y * raster.width as usize + x) * 4;
        assert_eq!(&raster.rgba[at..at + 4], &[*r, *g, *b, 255], "control {i}");
    }
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/text-rgb.png");
    if std::env::var_os("SHEAF_UPDATE_GOLDEN").is_some() {
        std::fs::write(&path, crate::raster::encode_png(&raster, 216.0).unwrap()).unwrap();
    }
    let decoder = png::Decoder::new(std::io::Cursor::new(std::fs::read(path).unwrap()));
    let mut reader = decoder.read_info().unwrap();
    let mut bytes = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut bytes).unwrap();
    assert_eq!((info.width, info.height), (raster.width, raster.height));
    assert_eq!(&bytes[..info.buffer_size()], raster.rgba);
}

#[test]
fn vector_exports_use_the_same_rgb_for_glyphs_and_decorations() {
    let scene = rgb_scene();
    let emf = crate::emf::write_emf(&scene, 1.0, &FixedMetrics);
    let mut at = 0;
    let mut colours = Vec::new();
    while at + 8 <= emf.len() {
        let kind = u32::from_le_bytes(emf[at..at + 4].try_into().unwrap());
        let size = u32::from_le_bytes(emf[at + 4..at + 8].try_into().unwrap()) as usize;
        assert!(size >= 8 && at + size <= emf.len());
        if kind == 24 {
            colours.push((emf[at + 8], emf[at + 9], emf[at + 10]));
        }
        at += size;
    }
    assert_eq!(colours, CONTROLS[..10].iter().map(|(_, rgb)| *rgb).collect::<Vec<_>>());
    let svg = crate::svg::write_svg(&scene);
    assert!(svg.contains("#0C4781") && svg.contains("#FFFE00"));
    let eps = String::from_utf8(crate::eps::write_eps(&scene, 1.0).bytes).unwrap();
    assert!(eps.contains("0.047 0.278 0.506 rg") && eps.contains("1 0.996 0 rg"));
    let pdf = crate::raster::to_pdf(&svg, crate::SystemFonts::shared()).unwrap();
    assert!(pdf.starts_with(b"%PDF-") && pdf.trim_ascii_end().ends_with(b"%%EOF"));
}
