use crate::fonts::FixedMetrics;
use crate::scene::{Anchor, Item, LayerScene, Scene, TextItem};
use sheaf_core::Color;
use sheaf_core::graph::Font;

#[test]
fn palette_four_overrides_the_parent_font_with_blue() {
    let mut font = Font::new("Arial", 20.0);
    font.color = Color::RED;
    let mut items = Vec::new();
    crate::text::place(&mut items, &FixedMetrics, r"\c4(blue)", &font, 20.0, (10.0, 30.0), Anchor::Start, 0.0);
    let Item::Text(text) = &items[0] else { panic!("expected text") };
    assert_eq!(text.color, Color::BLUE);
}

// Origin 2016 EMR_SETTEXTCOLOR captures, scratch/text-run-colors/palette.emf.
const RGB: [(u8, u8, u8); 24] = [
    (0, 0, 0),
    (255, 0, 0),
    (0, 255, 0),
    (0, 0, 255),
    (0, 255, 255),
    (255, 0, 255),
    (255, 255, 0),
    (128, 128, 0),
    (0, 0, 128),
    (128, 0, 128),
    (128, 0, 0),
    (0, 128, 0),
    (0, 128, 128),
    (0, 0, 160),
    (255, 128, 0),
    (128, 0, 255),
    (255, 0, 128),
    (255, 255, 255),
    (192, 192, 192),
    (128, 128, 128),
    (255, 255, 128),
    (128, 255, 255),
    (255, 128, 255),
    (64, 64, 64),
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
fn every_classic_colour_matches_the_origin_vector_fixture() {
    let mut font = Font::new("Arial", 20.0);
    font.color = Color::rgb(13, 71, 129);
    for (i, &(r, g, b)) in RGB.iter().enumerate() {
        let source = format!(r"\c{}(MMMM)", i + 1);
        let items = placed(&source, &font, Anchor::Start, 0.0);
        assert_eq!(
            texts(&items).iter().map(|t| (t.text.as_str(), t.color)).collect::<Vec<_>>(),
            [("MMMM", Color::rgb(r, g, b))],
            "{source}"
        );
    }
}

#[test]
fn positive_indices_follow_the_measured_byte_selection_and_black_fallback() {
    let font = Font::new("Arial", 20.0);
    // Independently captured boundary controls, not PNG's quantized colours.
    for (code, expected) in [
        ("25", Color::BLACK),
        ("26", Color::BLACK),
        ("99", Color::BLACK),
        ("255", Color::BLACK),
        ("256", Color::BLACK),
        ("257", Color::BLACK),
        ("258", Color::RED),
        ("280", Color::rgb(64, 64, 64)),
        ("281", Color::BLACK),
        ("512", Color::BLACK),
        ("1000", Color::BLACK),
        ("9999", Color::rgb(255, 128, 0)),
        ("65535", Color::BLACK),
        ("0002", Color::RED),
    ] {
        let items = placed(&format!(r"\c{code}(MMMM)"), &font, Anchor::Start, 0.0);
        assert_eq!(texts(&items)[0].color, expected, "{code}");
        assert_eq!(texts(&items)[0].text, "MMMM");
    }
}

#[test]
fn nested_colours_restore_the_surrounding_font_and_span_style() {
    let mut font = Font::new("Arial", 20.0);
    font.color = Color::rgba(13, 71, 129, 80);
    let items = placed(r"a\c2(b\c4(c)d)e", &font, Anchor::Start, 0.0);
    let values = texts(&items).iter().map(|t| (t.text.as_str(), t.color)).collect::<Vec<_>>();
    assert_eq!(
        values,
        [("a", font.color), ("b", Color::RED), ("c", Color::BLUE), ("d", Color::RED), ("e", font.color)]
    );
    assert_eq!(crate::markup::plain(r"a\c2(b\c4(c)d)e"), "abcde");
    let items = placed(r"\c4(\b(A)\i(B)\+(2)\g(m))", &font, Anchor::Start, 0.0);
    let runs = texts(&items);
    assert!(runs.iter().all(|r| r.color == Color::BLUE));
    assert!(runs[0].bold && runs[1].italic);
    assert_eq!(runs[2].size, 13.0);
    assert_eq!(runs[3].text, "μ");
}

#[test]
fn zero_and_malformed_codes_preserve_the_existing_text_boundary() {
    let mut font = Font::new("Arial", 20.0);
    font.color = Color::RED;
    for source in [r"\c0(text)", r"\c00(text)"] {
        let items = placed(source, &font, Anchor::Start, 0.0);
        assert_eq!(texts(&items)[0].text, "text");
        assert_eq!(texts(&items)[0].color, Color::RED);
    }
    for source in [r"\c-1(text)", r"\c(text)", r"\c33554433(text)", r"\c4294967296(text)", r"\c00000000002(text)"] {
        assert_eq!(crate::markup::plain(source), source);
    }
    assert_eq!(crate::markup::plain(r"\v(\c4(x))"), r"\c4(x)");
    let nested = r"\c2(".repeat(1000) + "x" + &")".repeat(1000);
    assert!(!crate::markup::parse(&nested).is_empty());
}

#[test]
fn a_single_coloured_run_preserves_anchor_size_and_rotation() {
    let font = Font::new("Arial", 20.0);
    for anchor in [Anchor::Start, Anchor::Middle, Anchor::End] {
        for angle in [0.0, 37.0, 90.0] {
            let plain = placed("abc", &font, anchor, angle);
            let coloured = placed(r"\c4(abc)", &font, anchor, angle);
            let mut expected = texts(&plain)[0].clone();
            expected.color = Color::BLUE;
            assert_eq!(*texts(&coloured)[0], expected);
        }
    }
}

#[test]
fn decoration_strokes_follow_their_own_nested_colour() {
    let mut font = Font::new("Arial", 20.0);
    font.color = Color::rgb(13, 71, 129);
    let items = placed(r"\u(a\c2(b\c4(\s(c))d)e)", &font, Anchor::Start, 90.0);
    let colors = items
        .iter()
        .filter_map(|i| match i {
            Item::Path { stroke: Some(s), .. } => Some(s.color),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(colors, [font.color, Color::RED, Color::BLUE, Color::BLUE, Color::RED, font.color]);
}

fn palette_scene() -> Scene {
    let mut items = Vec::new();
    let font = Font::new("Arial", 20.0);
    for i in 0..24 {
        crate::text::place(
            &mut items,
            &FixedMetrics,
            &format!(r"\c{}(\u(MM))", i + 1),
            &font,
            20.0,
            (10.0 + (i % 6) as f64 * 55.0, 25.0 + (i / 6) as f64 * 35.0),
            Anchor::Start,
            0.0,
        );
    }
    Scene {
        width: 350.0,
        height: 155.0,
        background: Color::rgb(238, 238, 238),
        layers: vec![LayerScene {
            clip: crate::Rect { x: 0.0, y: 0.0, w: 350.0, h: 155.0 },
            grid: vec![],
            plots: vec![],
            overlay: items,
            underlay_len: 0,
            objects: vec![],
        }],
    }
}

#[test]
fn palette_svg_and_stroke_png_goldens_preserve_all_twenty_four_colours() {
    let mut scene = palette_scene();
    crate::tests::compare_golden("text-colors", &crate::svg::write_svg(&scene));
    scene.layers[0].overlay.retain(|i| !matches!(i, Item::Text(_)));
    let raster = crate::render_scene_raster(&scene, 3.0).unwrap();
    for (i, &(r, g, b)) in RGB.iter().enumerate() {
        let x = (20 + (i % 6) * 55) * 3;
        let y = ((25.0 + (i / 6) as f64 * 35.0 + 2.8) * 3.0).floor() as usize;
        let at = (y * raster.width as usize + x) * 4;
        assert_eq!(&raster.rgba[at..at + 4], &[r, g, b, 255], "palette {}", i + 1);
    }
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/text-colors.png");
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
fn vector_exports_retain_explicit_text_and_decoration_rgb_values() {
    let scene = palette_scene();
    let eps = String::from_utf8(crate::eps::write_eps(&scene, 1.0).bytes).unwrap();
    assert!(eps.contains("1 0 0 rg") && eps.contains("0 0 1 rg"));
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
    assert_eq!(colours, RGB);
    let svg = crate::svg::write_svg(&scene);
    let pdf = crate::raster::to_pdf(&svg, crate::SystemFonts::shared()).unwrap();
    assert!(pdf.starts_with(b"%PDF-") && pdf.trim_ascii_end().ends_with(b"%%EOF"));
}
