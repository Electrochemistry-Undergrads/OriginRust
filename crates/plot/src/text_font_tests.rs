use crate::fonts::{FixedMetrics, TextLineMetrics, TextMeasurer};
use crate::scene::{Anchor, Item, LayerScene, Scene, TextItem};
use sheaf_core::Color;
use sheaf_core::graph::Font;

fn placed(source: &str, font: &Font, anchor: Anchor, angle: f64) -> Vec<Item> {
    let mut items = Vec::new();
    crate::text::place(&mut items, &FamilyMetrics, source, font, font.size_pt, (50.0, 60.0), anchor, angle);
    items
}

fn texts(items: &[Item]) -> Vec<&TextItem> {
    items.iter().filter_map(|i| if let Item::Text(t) = i { Some(t) } else { None }).collect()
}

#[test]
fn inline_font_family_reaches_the_placed_glyphs() {
    let font = Font::new("Arial", 20.0);
    let items = placed(r"\f:Times New Roman(MMMM)", &font, Anchor::Start, 0.0);
    assert_eq!((texts(&items)[0].text.as_str(), texts(&items)[0].family.as_str()), ("MMMM", "Times New Roman"));
}

#[test]
fn named_families_and_nested_restoration_match_origin_vector_controls() {
    let font = Font::new("Arial", 24.0);
    // Origin 2016 fonts.emf: full names plus a nested Times/Courier/Times/Arial label.
    for family in ["Times New Roman", "Courier New", "Arial"] {
        let items = placed(&format!(r"\f:{family}(MMMMiiii)"), &font, Anchor::Start, 0.0);
        assert_eq!((texts(&items)[0].text.as_str(), texts(&items)[0].family.as_str()), ("MMMMiiii", family));
    }
    let source = r"\f:Times New Roman(a\f:Courier New(b)c)d";
    let items = placed(source, &font, Anchor::Start, 0.0);
    assert_eq!(
        texts(&items).iter().map(|t| (t.text.as_str(), t.family.as_str())).collect::<Vec<_>>(),
        [("a", "Times New Roman"), ("b", "Courier New"), ("c", "Times New Roman"), ("d", "Arial")]
    );
    assert_eq!(crate::markup::plain(source), "abcd");
    let runs = crate::markup::parse(r"\f:Times New Roman(a)\f:Times New Roman(b)");
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].text, "ab");
}

// Deliberately distinct widths/metrics make accidental use of the parent font visible,
// without depending on fonts installed on Windows or Linux. These are test values.
struct FamilyMetrics;
impl TextMeasurer for FamilyMetrics {
    fn width(&self, text: &str, font: &Font, size: f64) -> f64 {
        text.chars().count() as f64
            * size
            * match font.family.as_str() {
                "Times New Roman" => 0.4,
                "Courier New" => 0.8,
                _ => 0.55,
            }
    }
    fn underline_metrics(&self, font: &Font) -> TextLineMetrics {
        match font.family.as_str() {
            "Times New Roman" => TextLineMetrics { center: 0.1, thickness: 0.03 },
            "Courier New" => TextLineMetrics { center: 0.2, thickness: 0.04 },
            _ => FixedMetrics.underline_metrics(font),
        }
    }
}

#[test]
fn inline_family_controls_measurement_anchors_and_decoration_geometry() {
    let font = Font::new("Arial", 20.0);
    let source = r"\u(a\f:Times New Roman(b)c)";
    assert_eq!(crate::text::width(&FamilyMetrics, source, &font, 20.0), 30.0);
    let items = placed(source, &font, Anchor::Middle, 0.0);
    assert_eq!(texts(&items).iter().map(|t| t.x).collect::<Vec<_>>(), [35.0, 46.0, 54.0]);
    let strokes = items
        .iter()
        .filter_map(|i| match i {
            Item::Path { d, stroke: Some(s), .. } => Some((d.as_str(), s.width)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(strokes.len(), 3);
    assert_eq!(strokes[1], ("M46 62L54 62", 0.6));
    assert!((strokes[0].1 - 1.4).abs() < 1e-12);
    let rotated = placed(source, &font, Anchor::Middle, 90.0);
    for (t, y) in texts(&rotated).iter().zip([75.0, 64.0, 56.0]) {
        assert!((t.x - 50.0).abs() < 1e-10 && (t.y - y).abs() < 1e-10);
    }
}

#[test]
fn family_composes_with_styles_scripts_greek_and_explicit_colour() {
    let mut font = Font::new("Arial", 20.0);
    font.bold = true;
    font.color = Color::rgb(13, 71, 129);
    let items = placed(r"\f:Times New Roman(\i(A)\+(2)\c2(B)\g(m))", &font, Anchor::Start, 0.0);
    let ts = texts(&items);
    assert!(ts.iter().all(|t| t.family == "Times New Roman" && t.bold));
    assert!(ts[0].italic);
    assert_eq!(ts[1].size, 13.0);
    assert_eq!(ts[2].color, Color::RED);
    assert_eq!((ts[3].text.as_str(), ts[3].color), ("\u{03bc}", font.color));
    let items = placed(r"\f:Symbol(m\f:Arial(x)m)y", &font, Anchor::Start, 0.0);
    assert_eq!(
        texts(&items).iter().map(|t| (t.text.as_str(), t.family.as_str())).collect::<Vec<_>>(),
        [("\u{03bc}", "Symbol"), ("x", "Arial"), ("\u{03bc}", "Symbol"), ("y", "Arial")]
    );
}

#[test]
fn a_single_family_run_retains_the_original_glyph_anchor_and_rotation() {
    let font = Font::new("Arial", 20.0);
    for anchor in [Anchor::Start, Anchor::Middle, Anchor::End] {
        for angle in [0.0, 37.0, 90.0] {
            let plain = placed("abc", &font, anchor, angle);
            let switched = placed(r"\f:Times New Roman(abc)", &font, anchor, angle);
            let mut expected = texts(&plain)[0].clone();
            expected.family = "Times New Roman".to_owned();
            assert_eq!(*texts(&switched)[0], expected);
        }
    }
}

#[test]
fn literal_unicode_and_bounded_names_keep_their_existing_text_rules() {
    let runs = crate::markup::parse(r"\f:Times New Roman(\v(\b(x))\x(03bc)\(65))");
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].text, "\\b(x)\u{03bc}A");
    assert_eq!(runs[0].family.as_deref(), Some("Times New Roman"));
    let runs = crate::markup::parse(r"\f: Times New Roman (x)");
    assert_eq!(runs[0].text, "x");
    assert_eq!(runs[0].family, None);
    let name = "n".repeat(64);
    assert_eq!(crate::markup::parse(&format!(r"\f:{name}(x)"))[0].family.as_deref(), Some(name.as_str()));
    let source = format!(r"\f:{}(x)", "n".repeat(65));
    assert_eq!(crate::markup::plain(&source), source);
    let nested = r"\f:Times New Roman(".repeat(1000) + "x" + &")".repeat(1000);
    assert!(!crate::markup::parse(&nested).is_empty());
    let fonts = crate::SystemFonts::from_database(fontdb::Database::new());
    let font = Font::new("Arial", 20.0);
    assert_eq!(crate::text::width(&fonts, r"\f:Missing Family(ab)", &font, 20.0), 22.0);
}

#[test]
fn empty_font_names_retain_the_literal_origin_control() {
    // Accepted Origin 2016 N6: one Arial run containing all seven characters,
    // at (1694,1047), advance 317, rather than the two-character MM run (166).
    let source = r"\f:(MM)";
    let runs = crate::markup::parse(source);
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].text, source);
    assert_eq!(runs[0].family, None);
    assert_eq!(crate::markup::plain(source), source);
    assert_eq!(crate::markup::plain(r"left\f:()right"), r"left\f:()right");
    assert_eq!(crate::markup::plain(r"\f:(MM"), r"\f:(MM");
}

#[test]
fn empty_font_literals_keep_the_surrounding_family_styles_and_measured_width() {
    let source = r"\u(\f:Times New Roman(a\f:(MM)b))x";
    let runs = crate::markup::parse(source);
    assert_eq!(runs.len(), 2);
    assert_eq!(runs[0].text, r"a\f:(MM)b");
    assert_eq!(runs[0].family.as_deref(), Some("Times New Roman"));
    assert!(runs[0].underline);
    assert_eq!(runs[1].text, "x");
    assert_eq!(runs[1].family, None);
    assert!(!runs[1].underline);
    let font = Font::new("Arial", 20.0);
    let mut inner = font.clone();
    inner.family = "Times New Roman".into();
    let expected = FamilyMetrics.width(r"a\f:(MM)b", &inner, 20.0) + FamilyMetrics.width("x", &font, 20.0);
    assert_eq!(crate::text::width(&FamilyMetrics, source, &font, 20.0), expected);
    let items = placed(source, &font, Anchor::Middle, 37.0);
    assert_eq!(texts(&items).iter().map(|t| t.text.as_str()).collect::<String>(), r"a\f:(MM)bx");
}

#[test]
fn empty_font_literals_export_like_explicit_literal_text() {
    let scene = |source| Scene {
        width: 155.0,
        height: 80.0,
        background: Color::WHITE,
        layers: vec![LayerScene {
            clip: crate::Rect { x: 0.0, y: 0.0, w: 155.0, h: 80.0 },
            grid: vec![],
            plots: vec![],
            overlay: placed(source, &Font::new("Arial", 20.0), Anchor::Start, 0.0),
            underlay_len: 0,
            objects: vec![],
        }],
    };
    let actual = scene(r"\f:(MM)");
    let literal = scene(r"\v(\f:(MM))");
    let svg = crate::svg::write_svg(&actual);
    assert!(svg.contains(r"\f:(MM)"));
    assert_eq!(svg, crate::svg::write_svg(&literal));
    assert_eq!(crate::eps::write_eps(&actual, 1.0).bytes, crate::eps::write_eps(&literal, 1.0).bytes);
    assert_eq!(crate::emf::write_emf(&actual, 1.0, &FixedMetrics), crate::emf::write_emf(&literal, 1.0, &FixedMetrics));
    assert_eq!(
        crate::render_scene_raster(&actual, 2.0).unwrap().rgba,
        crate::render_scene_raster(&literal, 2.0).unwrap().rgba
    );
}

fn family_scene() -> Scene {
    let mut items = Vec::new();
    let mut font = Font::new("Arial", 20.0);
    font.color = Color::rgb(13, 71, 129);
    for (i, name) in ["Arial", "Times New Roman", "Courier New"].iter().enumerate() {
        crate::text::place(
            &mut items,
            &FamilyMetrics,
            &format!(r"\f:{name}(\u(MMMMiiii))"),
            &font,
            20.0,
            (10.0, 25.0 + i as f64 * 35.0),
            Anchor::Start,
            0.0,
        );
    }
    Scene {
        width: 155.0,
        height: 120.0,
        background: Color::WHITE,
        layers: vec![LayerScene {
            clip: crate::Rect { x: 0.0, y: 0.0, w: 155.0, h: 120.0 },
            grid: vec![],
            plots: vec![],
            overlay: items,
            underlay_len: 0,
            objects: vec![],
        }],
    }
}

#[test]
fn family_svg_and_stroke_png_goldens_use_each_runs_measurement() {
    let mut scene = family_scene();
    crate::tests::compare_golden("text-families", &crate::svg::write_svg(&scene));
    scene.layers[0].overlay.retain(|i| !matches!(i, Item::Text(_)));
    let raster = crate::render_scene_raster(&scene, 3.0).unwrap();
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/text-families.png");
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
fn vector_exports_preserve_the_requested_family_and_native_standard_faces() {
    let scene = family_scene();
    let svg = crate::svg::write_svg(&scene);
    for name in ["Arial", "Times New Roman", "Courier New"] {
        assert!(svg.contains(name));
    }
    let eps = String::from_utf8(crate::eps::write_eps(&scene, 1.0).bytes).unwrap();
    for face in ["Helvetica", "Times-Roman", "Courier"] {
        assert!(eps.contains(face), "{face}");
    }
    let emf = crate::emf::write_emf(&scene, 1.0, &FixedMetrics);
    let mut at = 0;
    let mut families = Vec::new();
    while at + 8 <= emf.len() {
        let kind = u32::from_le_bytes(emf[at..at + 4].try_into().unwrap());
        let size = u32::from_le_bytes(emf[at + 4..at + 8].try_into().unwrap()) as usize;
        assert!(size >= 8 && at + size <= emf.len());
        if kind == 82 {
            let wide: Vec<_> = emf[at + 40..at + 104]
                .as_chunks::<2>()
                .0
                .iter()
                .map(|b| u16::from_le_bytes([b[0], b[1]]))
                .take_while(|c| *c != 0)
                .collect();
            families.push(String::from_utf16(&wide).unwrap());
        }
        at += size;
    }
    assert_eq!(families, ["Arial", "Times New Roman", "Courier New"]);
    let pdf = crate::raster::to_pdf(&svg, crate::SystemFonts::shared()).unwrap();
    assert!(pdf.starts_with(b"%PDF-") && pdf.trim_ascii_end().ends_with(b"%%EOF"));
}
