use crate::fonts::{FixedMetrics, TextLineMetrics, TextMeasurer};
use crate::scene::{Anchor, Item, LayerScene, Scene, TextItem};
use sheaf_core::Color;
use sheaf_core::graph::Font;

fn placed(source: &str, anchor: Anchor, angle: f64) -> Vec<Item> {
    let mut items = Vec::new();
    crate::text::place(&mut items, &FixedMetrics, source, &Font::new("Arial", 20.0), 20.0, (10.0, 30.0), anchor, angle);
    items
}
fn texts(items: &[Item]) -> Vec<&TextItem> {
    items.iter().filter_map(|i| if let Item::Text(t) = i { Some(t) } else { None }).collect()
}
fn near(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-10, "{a} != {b}");
}

#[test]
fn paired_scripts_share_a_column_and_advance_by_the_wider_branch() {
    // Origin 2016 pairs-quoted.emf: P, upper, lower, Q. Both branches start
    // after P; Q advances by 132 units for either (1234,5) or (1,2345).
    for (lower, upper) in [("1234", "5"), ("1", "2345"), ("12", "34")] {
        let source = format!(r"P\=({lower},{upper})Q");
        let items = placed(&source, Anchor::Start, 0.0);
        let ts = texts(&items);
        assert_eq!(ts.iter().map(|t| t.text.as_str()).collect::<Vec<_>>(), ["P", upper, lower, "Q"]);
        near(ts[1].x, 21.0);
        near(ts[2].x, 21.0);
        near(ts[1].y, 20.2);
        near(ts[2].y, 37.0);
        near(ts[1].size, 12.0);
        near(ts[2].size, 12.0);
        let column = lower.len().max(upper.len()) as f64 * 6.6;
        near(ts[3].x, 21.0 + column);
        near(crate::text::width(&FixedMetrics, &source, &Font::new("Arial", 20.0), 20.0), 22.0 + column);
    }
}

#[test]
fn native_pair_sizes_and_nested_baselines_scale_with_the_surrounding_run() {
    // Native 12/24/48/72pt: base heights 50/100/200/300, upper shifts
    // -25/-49/-98/-147 and lower +18/+35/+70/+105 (integer EMF rounding).
    let source = r"P\=(\=(1,2),3)Q";
    let items = placed(source, Anchor::Start, 0.0);
    let ts = texts(&items);
    assert_eq!(ts.iter().map(|t| t.text.as_str()).collect::<Vec<_>>(), ["P", "3", "2", "1", "Q"]);
    near(ts[1].y, 20.2);
    near(ts[2].y, 31.12);
    near(ts[3].y, 41.2);
    near(ts[2].size, 7.2);
    near(ts[3].size, 7.2);
    near(ts[4].x, 27.6);
    let items = placed(r"\p150(P\=(1,\=(2,3))Q)", Anchor::Start, 0.0);
    let ts = texts(&items);
    near(ts[0].size, 30.0);
    near(ts[1].size, 10.8);
    near(ts[1].y, 30.0 - 30.0 * (0.49 + 0.6 * 0.49));
    near(ts[2].y, 30.0 - 30.0 * (0.49 - 0.6 * 0.35));
    near(ts[3].y, 40.5);
}

#[test]
fn first_outer_comma_separates_branches_and_literal_commas_stay_in_their_branch() {
    for (source, expected) in [
        (r"P\=(\v(1,2),3)Q", vec!["P", "3", "1,2", "Q"]),
        (r"P\=(1,2,3)Q", vec!["P", "2,3", "1", "Q"]),
        (r"P\=(123)Q", vec!["P", "123", "Q"]),
        (r"P\=(,12)Q", vec!["P", "12", "Q"]),
        (r"P\=(12,)Q", vec!["P", "12", "Q"]),
        (r"P\=(,)Q", vec!["PQ"]),
        (r"P\=(\(44),\x(002c))Q", vec!["P", ",", ",", "Q"]),
        (r"P\=(a(b,c),d)Q", vec!["P", "d", "a(b,c)", "Q"]),
    ] {
        let items = placed(source, Anchor::Start, 0.0);
        assert_eq!(texts(&items).iter().map(|t| t.text.as_str()).collect::<Vec<_>>(), expected, "{source}");
    }
    assert_eq!(crate::markup::plain(r"P\=(\v(1,2),3)Q"), "P1,23Q");
    assert_eq!(crate::markup::plain(r"\v(\=(1,2))"), r"\=(1,2)");
}

#[test]
fn pairs_inherit_and_restore_font_colour_greek_and_decorations() {
    let items = placed(r"\b(P\=(\f:Courier New(\u(12)),\i(\c2(\g(a))))Q)R", Anchor::Start, 0.0);
    let ts = texts(&items);
    assert_eq!(ts.iter().map(|t| t.text.as_str()).collect::<Vec<_>>(), ["P", "\u{03b1}", "12", "Q", "R"]);
    assert!(ts[..4].iter().all(|t| t.bold));
    assert!(!ts[4].bold);
    assert!(ts[1].italic);
    assert_eq!(ts[1].color, Color::RED);
    assert_eq!(ts[2].family, "Courier New");
    assert_eq!(ts[3].family, "Arial");
    assert_eq!(ts[3].color, Color::BLACK);
    near(ts[3].size, 20.0);
    assert_eq!(items.iter().filter(|i| matches!(i, Item::Path { .. })).count(), 1);
    let runs = crate::markup::parse(r"P\=(\g(a),\c4(b))Q");
    assert_eq!(runs.iter().map(|r| r.text.as_str()).collect::<Vec<_>>(), ["P", "\u{03b1}", "b", "Q"]);
    assert_eq!(runs[2].color, Some(Color::BLUE));
}

#[test]
fn shared_column_width_controls_all_anchors_and_rotations() {
    let source = r"P\=(12,345)Q";
    let total = 41.8;
    let start = placed(source, Anchor::Start, 0.0);
    for (anchor, shift) in [(Anchor::Start, 0.0), (Anchor::Middle, -total / 2.0), (Anchor::End, -total)] {
        for angle in [0.0_f64, 37.0, 90.0, -45.0] {
            let items = placed(source, anchor, angle);
            let (sin, cos) = angle.to_radians().sin_cos();
            for (actual, base) in texts(&items).iter().zip(texts(&start)) {
                let dx = base.x - 10.0 + shift;
                let dy = base.y - 30.0;
                near(actual.x, 10.0 + dx * cos + dy * sin);
                near(actual.y, 30.0 - dx * sin + dy * cos);
                assert_eq!(actual.anchor, Anchor::Start);
                near(actual.rotate_ccw, angle);
            }
        }
    }
}

struct FamilyMetrics;
impl TextMeasurer for FamilyMetrics {
    fn width(&self, text: &str, font: &Font, size: f64) -> f64 {
        text.chars().count() as f64 * size * if font.family == "Courier New" { 0.8 } else { 0.4 }
    }
    fn underline_metrics(&self, _: &Font) -> TextLineMetrics {
        TextLineMetrics { center: 0.2, thickness: 0.04 }
    }
}
#[test]
fn wider_effective_family_and_script_decoration_geometry_reach_the_scene() {
    let mut items = Vec::new();
    let font = Font::new("Arial", 20.0);
    let source = r"P\=(\c2(\u(\f:Courier New(12))),\s(345))Q";
    crate::text::place(&mut items, &FamilyMetrics, source, &font, 20.0, (10.0, 30.0), Anchor::Start, 90.0);
    let ts = texts(&items);
    near(ts[3].y, 2.8); // P:8; Courier lower:19.2 wins over upper:14.4.
    near(crate::text::width(&FamilyMetrics, source, &font, 20.0), 35.2);
    let strokes: Vec<_> = items
        .iter()
        .filter_map(|i| if let Item::Path { d, stroke: Some(s), .. } = i { Some((d, s)) } else { None })
        .collect();
    assert_eq!(strokes.len(), 2);
    assert_eq!(strokes[1].0, "M19.4 22L19.4 2.8");
    assert_eq!(strokes[1].1.color, Color::RED);
    near(strokes[1].1.width, 0.48);
}

#[test]
fn empty_and_deep_pairs_remain_bounded_and_standalone_script_rules_are_preserved() {
    assert!(placed(r"\=(,)", Anchor::End, 37.0).is_empty());
    near(crate::text::width(&FixedMetrics, r"\=(,)", &Font::new("Arial", 20.0), 20.0), 0.0);
    let source = r"\=(1,".repeat(1000) + "2" + &")".repeat(1000);
    assert!(!crate::markup::plain(&source).is_empty());
    assert!(!placed(&source, Anchor::Start, 0.0).is_empty());
    let runs = crate::markup::parse(r"\+(a)\-(b)");
    assert_eq!((runs[0].scale, runs[0].rise), (0.65, 0.35));
    assert_eq!((runs[1].scale, runs[1].rise), (0.65, -0.15));
}

fn pair_scene() -> Scene {
    let mut items = Vec::new();
    let font = Font::new("Arial", 20.0);
    for (i, source) in
        [r"P\=(1234,5)Q", r"P\=(1,2345)Q", r"\u(P\=(12,\c2(345))Q)", r"P\=(\=(1,2),3)Q"].iter().enumerate()
    {
        crate::text::place(
            &mut items,
            &FixedMetrics,
            source,
            &font,
            20.0,
            (15.0, 25.0 + i as f64 * 35.0),
            Anchor::Start,
            0.0,
        );
    }
    Scene {
        width: 120.0,
        height: 150.0,
        background: Color::WHITE,
        layers: vec![LayerScene {
            clip: crate::Rect { x: 0.0, y: 0.0, w: 120.0, h: 150.0 },
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
fn paired_glyph_svg_and_font_independent_stroke_png_goldens() {
    let mut scene = pair_scene();
    crate::tests::compare_golden("text-script-pairs", &crate::svg::write_svg(&scene));
    scene.layers[0].overlay.retain(|i| !matches!(i, Item::Text(_)));
    let raster = crate::render_scene_raster(&scene, 3.0).unwrap();
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/text-script-pairs.png");
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
fn paired_positions_survive_vector_and_pdf_exports() {
    let scene = pair_scene();
    let svg = crate::svg::write_svg(&scene);
    assert!(!svg.contains(r"\=("));
    assert!(svg.contains("font-size=\"12\""));
    let eps = String::from_utf8(crate::eps::write_eps(&scene, 1.0).bytes).unwrap();
    assert!(eps.contains("(1234)") && eps.contains("(2345)"));
    let emf = crate::emf::write_emf(&scene, 1.0, &FixedMetrics);
    let mut at = 0;
    let mut positions = Vec::new();
    while at + 8 <= emf.len() {
        let kind = u32::from_le_bytes(emf[at..at + 4].try_into().unwrap());
        let len = u32::from_le_bytes(emf[at + 4..at + 8].try_into().unwrap()) as usize;
        assert!(len >= 8 && at + len <= emf.len());
        if kind == 84 {
            let x = i32::from_le_bytes(emf[at + 36..at + 40].try_into().unwrap());
            let y = i32::from_le_bytes(emf[at + 40..at + 44].try_into().unwrap());
            positions.push((x, y));
        }
        at += len;
    }
    assert!(positions.len() >= 8);
    assert_eq!(positions[1].0, positions[2].0);
    assert!(positions[1].1 < positions[0].1 && positions[2].1 > positions[0].1);
    assert_eq!(positions[3].0 - positions[0].0, positions[7].0 - positions[4].0);
    let pdf = crate::raster::to_pdf(&svg, crate::SystemFonts::shared()).unwrap();
    assert!(pdf.starts_with(b"%PDF-") && pdf.trim_ascii_end().ends_with(b"%%EOF"));
}

#[test]
fn legend_tick_labels_and_rotated_axis_title_use_the_shared_pair_columns() {
    let (mut project, graph) =
        crate::tests::project(sheaf_core::template::Template::Line, vec![0.0, 1.0], vec![vec![0.0, 1.0]]);
    let layer = &mut project.graph_mut(graph).unwrap().layers[0];
    layer.legend.text = Some(r"L\=(1234,5)R".into());
    layer.legend.font = Font::new("Arial", 20.0);
    layer.legend.frame = true;
    layer.x_axis.min = Some(0.0);
    layer.x_axis.max = Some(1.0);
    layer.x_axis.major_step = Some(1.0);
    layer.x_axis.tick_font = Font::new("Arial", 20.0);
    layer.x_axis.tick_prefix = r"X\=(i,j)Z".into();
    layer.y_axis.title = Some(r"Y\=(k,LM)N".into());
    layer.y_axis.title_font = Font::new("Arial", 20.0);
    let scene = crate::layout::build_scene(&project, graph, &FixedMetrics).unwrap();
    let ts = texts(&scene.layers[0].overlay);
    let at = ts.iter().position(|t| t.text == "L").unwrap();
    assert_eq!(ts[at..at + 4].iter().map(|t| t.text.as_str()).collect::<Vec<_>>(), ["L", "5", "1234", "R"]);
    near(ts[at + 1].x, ts[at + 2].x);
    near(ts[at + 3].x - ts[at + 1].x, 26.4);
    let geom = &crate::layout_geometry(&project, graph).unwrap()[0];
    let ticks: Vec<_> = ts.windows(4).filter(|t| t[0].text == "X").collect();
    assert_eq!(ticks.len(), 2);
    for (tick, value) in ticks.iter().zip([0.0, 1.0]) {
        assert_eq!((tick[1].text.as_str(), tick[2].text.as_str()), ("j", "i"));
        near(tick[1].x, tick[2].x);
        near(tick[3].x - tick[1].x, 6.6);
        assert!(tick[1].y < tick[0].y && tick[2].y > tick[0].y);
        let total = 11.0 + 6.6 + tick[3].text.chars().count() as f64 * 11.0;
        near(tick[0].x + total / 2.0, geom.data_to_page(value, 0.0).0);
    }
    let at = ts.iter().position(|t| t.text == "Y").unwrap();
    assert_eq!(ts[at..at + 4].iter().map(|t| t.text.as_str()).collect::<Vec<_>>(), ["Y", "LM", "k", "N"]);
    assert!(ts[at..at + 4].iter().all(|t| t.rotate_ccw == 90.0));
    near(ts[at + 1].y, ts[at + 2].y);
    near(ts[at + 1].y - ts[at + 3].y, 13.2);
    assert!(ts[at + 1].x < ts[at].x && ts[at + 2].x > ts[at].x);
    let svg = crate::svg::write_svg(&scene);
    assert!(!svg.contains(r"\=("));
    assert!(crate::tests::texts(&crate::tests::doc(&svg)).iter().any(|t| t == "1234"));
}
