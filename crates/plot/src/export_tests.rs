//! Tests of the EPS and EMF writers.

use super::*;
use crate::scene::{Anchor, Item, LayerScene, Scene, Stroke, TextItem};
use sheaf_core::template::Template;
use sheaf_core::{Color, ColumnData, Designation};

fn project() -> (Project, WindowId) {
    let mut p = Project::new();
    let book = p.add_workbook("data");
    let x: Vec<f64> = (0..=10).map(f64::from).collect();
    let xc = p.add_column(book, 0, Designation::X, ColumnData::numeric(x.clone())).unwrap();
    p.column_mut(xc).unwrap().long_name = "Potential".into();
    let g = p.add_graph(Template::Line);
    let yc = p.add_column(book, 0, Designation::Y, ColumnData::numeric(x.iter().map(|v| v * v).collect())).unwrap();
    p.add_plot(g, 0, yc).unwrap();
    (p, g)
}

fn text(s: &str, anchor: Anchor) -> TextItem {
    TextItem {
        x: 50.0,
        y: 40.0,
        text: s.into(),
        family: "Arial".into(),
        size: 10.0,
        bold: false,
        italic: false,
        color: Color::BLACK,
        anchor,
        rotate_ccw: 0.0,
    }
}

fn scene_with(items: Vec<Item>) -> Scene {
    Scene {
        width: 100.0,
        height: 80.0,
        background: Color::WHITE,
        layers: vec![LayerScene {
            clip: Rect { x: 10.0, y: 10.0, w: 60.0, h: 50.0 },
            grid: vec![],
            plots: items,
            overlay: vec![],
            underlay_len: 0,
            objects: vec![],
        }],
    }
}

fn eps_text(scene: &Scene) -> String {
    String::from_utf8(eps::write_eps(scene, 1.0).bytes).unwrap()
}

#[test]
fn data_on_top_keeps_axes_outside_the_data_clip_and_annotations_above_it_in_vector_exports() {
    let mut scene = scene_with(vec![Item::Text(text("DATA", Anchor::Start))]);
    scene.layers[0].overlay = vec![Item::Text(text("AXIS", Anchor::Start)), Item::Text(text("LEGEND", Anchor::Start))];
    for enabled in [false, true] {
        scene.layers[0].underlay_len = usize::from(enabled);
        let svg = svg::write_svg(&scene);
        let axis = svg.find(">AXIS<").unwrap();
        let data = svg.find(">DATA<").unwrap();
        let legend = svg.find(">LEGEND<").unwrap();
        assert_eq!(axis < data, enabled, "{svg}");
        assert!(legend > axis && legend > data);
        let doc = roxmltree::Document::parse(&svg).unwrap();
        let group = |label: &str| {
            doc.descendants().find(|n| n.has_tag_name("text") && n.text() == Some(label)).unwrap().parent().unwrap()
        };
        assert!(group("AXIS").attribute("clip-path").is_none());
        assert!(group("LEGEND").attribute("clip-path").is_none());
        assert!(group("DATA").attribute("clip-path").is_some());

        let eps = eps_text(&scene);
        let axis = eps.find("(AXIS)").unwrap();
        let data = eps.find("(DATA)").unwrap();
        let legend = eps.find("(LEGEND)").unwrap();
        let clip = eps.find("rectclip\n").unwrap();
        assert_eq!(axis < clip, enabled);
        assert_eq!(axis < data, enabled);
        assert!(legend > axis && legend > data);

        let bytes = emf::write_emf(&scene, 1.0, &FixedMetrics);
        let records = emf_records(&bytes);
        let labels: Vec<(usize, String)> = records
            .iter()
            .enumerate()
            .filter_map(|(i, (kind, r))| {
                if *kind != 84 {
                    return None;
                }
                let len = i32_at(r, 44) as usize;
                let offset = i32_at(r, 48) as usize;
                let units: Vec<u16> =
                    r[offset..offset + len * 2].as_chunks::<2>().0.iter().copied().map(u16::from_le_bytes).collect();
                Some((i, String::from_utf16(&units).unwrap()))
            })
            .collect();
        let at = |label: &str| labels.iter().find(|(_, s)| s == label).unwrap().0;
        let clip = records.iter().position(|(kind, _)| *kind == 30).unwrap();
        let restore = records.iter().position(|(kind, _)| *kind == 34).unwrap();
        assert_eq!(at("AXIS") < clip, enabled);
        assert_eq!(at("AXIS") < at("DATA"), enabled);
        assert!(clip < at("DATA") && at("DATA") < restore);
        assert!(at("LEGEND") > restore && at("LEGEND") > at("AXIS"));
    }
}

#[test]
fn eps_has_a_bounding_box_that_matches_the_page() {
    let (p, g) = project();
    let eps = String::from_utf8(render_eps(&p, g).unwrap()).unwrap();
    let page = &p.graph(g).unwrap().page;
    assert!(eps.starts_with("%!PS-Adobe-3.0 EPSF-3.0\n"));
    assert!(
        eps.contains(&format!("%%BoundingBox: 0 0 {} {}\n", page.width_pt.ceil(), page.height_pt.ceil())),
        "{}",
        &eps[..200]
    );
    let hires = eps.lines().find(|l| l.starts_with("%%HiResBoundingBox")).unwrap();
    let nums: Vec<f64> = hires.split_whitespace().skip(1).map(|v| v.parse().unwrap()).collect();
    assert!((nums[2] - page.width_pt).abs() < 0.001 && (nums[3] - page.height_pt).abs() < 0.001, "{hires}");
    assert!(eps.ends_with("%%EOF\n") && eps.contains("showpage"));
    assert!(eps.is_ascii());
    // Saves and restores balance, and the figure is drawn with paths and text.
    let toks = |t: &str| eps.split_whitespace().filter(|w| *w == t).count();
    assert_eq!(toks("q"), toks("Q"));
    assert!(toks("rectclip") >= 1 && toks("stroke") > 5 && toks("show") > 3, "{}", eps.len());
}

#[test]
fn eps_scales_with_the_export_size() {
    let (p, g) = project();
    let page = &p.graph(g).unwrap().page;
    let size = ExportSize { width_in: Some(page.width_pt / 72.0 / 2.0), height_in: None, ratio: None };
    let eps = String::from_utf8(export_bytes(&p, g, "eps", 300.0, size).unwrap()).unwrap();
    assert!(eps.contains(&format!(
        "%%BoundingBox: 0 0 {} {}",
        (page.width_pt / 2.0).ceil(),
        (page.height_pt / 2.0).ceil()
    )));
    assert!(eps.contains("0.5 0.5 scale"));
}

#[test]
fn eps_text_uses_latin_1_and_standard_fonts() {
    let mut t = text("20 \u{b0}C, 5 \u{b5}m, m\u{b2}", Anchor::Middle);
    t.family = "Times New Roman".into();
    t.bold = true;
    let out = eps::write_eps(&scene_with(vec![Item::Text(t)]), 1.0);
    let s = String::from_utf8(out.bytes).unwrap();
    assert!(s.contains("20 \\260C, 5 \\265m, m\\262"), "{s}");
    assert!(s.contains("/Times-Bold /Times-Bold-L1 reenc") && s.contains("/Times-Bold-L1 10 sf"), "{s}");
    assert!(s.contains("stringwidth pop add") && s.contains("2 div neg 0 m"));
    assert!(out.warnings.is_empty());
}

#[test]
fn eps_sets_greek_in_symbol_and_warns_about_what_it_cannot_set() {
    let out = eps::write_eps(
        &scene_with(vec![Item::Text(text("\u{394}E (\u{3b1}) \u{2212}1 \u{4e2d} \u{2014}", Anchor::Start))]),
        1.0,
    );
    let s = String::from_utf8(out.bytes).unwrap();
    assert!(s.contains("/Symbol 10 sf (D) show") && s.contains("/Symbol 10 sf (a) show"), "{s}");
    assert!(s.contains("/Symbol 10 sf (-) show"), "the minus sign comes from Symbol: {s}");
    assert_eq!(out.warnings.len(), 2, "{:?}", out.warnings);
    assert!(out.warnings[0].contains('\u{2014}') && out.warnings[1].contains('\u{4e2d}'));
}

#[test]
fn eps_text_is_escaped_and_rotated() {
    let mut t = text("f(x) \\ y", Anchor::End);
    t.rotate_ccw = 90.0;
    let s = eps_text(&scene_with(vec![Item::Text(t)]));
    assert!(s.contains("(f\\(x\\) \\\\ y)"), "{s}");
    assert!(s.contains("90 rotate") && s.contains("neg 0 m"));
}

#[test]
fn eps_draws_dashes_caps_fills_and_clips() {
    let stroke = Stroke { dash: vec![4.0, 2.0, 1.0], cap: scene::Cap::Round, ..Stroke::solid(Color::RED, 1.5) };
    let items = vec![
        Item::Path {
            d: "M10 10L50 10C60 10 60 30 50 30Z".into(),
            fill: Some(Color::rgba(0, 0, 255, 128)),
            stroke: Some(stroke),
        },
        Item::Rect { rect: Rect { x: 1.0, y: 2.0, w: 3.0, h: 4.0 }, fill: Some(Color::BLACK), stroke: None },
    ];
    let s = eps_text(&scene_with(items));
    assert!(s.contains("10 70 m 50 70 l 60 70 60 50 50 50 c h"), "{s}");
    assert!(
        s.contains("[4 2 1 4 2 1 ] 0 setdash") && s.contains("1 setlinecap"),
        "dash pattern repeated to even length: {s}"
    );
    // 50 % blue on white: (127, 127, 255) -> 0.498 0.498 1.
    assert!(s.contains("0.498 0.498 1 rg fill"), "{s}");
    assert!(s.contains("q 10 20 60 50 rectclip"), "clip is in PostScript coordinates: {s}");
}

#[test]
fn eps_runs_through_ghostscript_when_it_is_installed() {
    let Some(gs) = ["gswin64c", "gs"]
        .into_iter()
        .find(|n| std::process::Command::new(n).arg("--version").output().map(|o| o.status.success()).unwrap_or(false))
    else {
        eprintln!("ghostscript not installed: skipped");
        return;
    };
    let (p, g) = project();
    let dir = std::env::temp_dir().join(format!("sheaf-eps-gs-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("fig.eps");
    export_file(&p, g, &path, 72.0).unwrap();
    let png = dir.join("fig.png");
    let out = std::process::Command::new(gs)
        .args(["-dBATCH", "-dNOPAUSE", "-dSAFER", "-dEPSCrop", "-sDEVICE=png16m", "-r72"])
        .arg(format!("-sOutputFile={}", png.display()))
        .arg(&path)
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    assert!(std::fs::metadata(&png).unwrap().len() > 1000);
    let _ = std::fs::remove_dir_all(&dir);
}

// ----- EMF ---------------------------------------------------------------------------------------

/// The records of an EMF file: (type, bytes), checking the framing as it goes.
fn emf_records(b: &[u8]) -> Vec<(u32, &[u8])> {
    let u32_at = |i: usize| u32::from_le_bytes(b[i..i + 4].try_into().unwrap());
    let mut out = Vec::new();
    let mut at = 0;
    while at < b.len() {
        let (ty, size) = (u32_at(at), u32_at(at + 4) as usize);
        assert!(size >= 8 && size % 4 == 0 && at + size <= b.len(), "record {ty} at {at} has size {size}");
        out.push((ty, &b[at..at + size]));
        at += size;
    }
    assert_eq!(at, b.len());
    out
}

fn i32_at(r: &[u8], i: usize) -> i32 {
    i32::from_le_bytes(r[i..i + 4].try_into().unwrap())
}

#[test]
fn emf_header_and_record_stream_are_consistent() {
    let (p, g) = project();
    let bytes = render_emf(&p, g).unwrap();
    let recs = emf_records(&bytes);
    let (ty, head) = recs[0];
    assert_eq!(ty, 1);
    assert_eq!(&head[40..44], b" EMF");
    assert_eq!(u32::from_le_bytes(head[44..48].try_into().unwrap()), 0x10000);
    assert_eq!(u32::from_le_bytes(head[48..52].try_into().unwrap()) as usize, bytes.len());
    assert_eq!(u32::from_le_bytes(head[52..56].try_into().unwrap()) as usize, recs.len());
    let last = recs.last().unwrap();
    assert_eq!(last.0, 14);
    assert_eq!(last.1.len(), 20);
    // The frame is the page in 0.01 mm.
    let page = &p.graph(g).unwrap().page;
    let mm100 = |pt: f64| (pt * 2540.0 / 72.0).round() as i32;
    assert_eq!((i32_at(head, 32), i32_at(head, 36)), (mm100(page.width_pt), mm100(page.height_pt)));
    assert_eq!((i32_at(head, 16), i32_at(head, 20)), (mm100(page.width_pt) - 1, mm100(page.height_pt) - 1));
    // The description names the producer.
    let n_desc = u32::from_le_bytes(head[60..64].try_into().unwrap()) as usize;
    let off = u32::from_le_bytes(head[64..68].try_into().unwrap()) as usize;
    let desc: Vec<u16> = head[off..off + n_desc * 2].chunks(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
    assert!(String::from_utf16_lossy(&desc).starts_with("Sheaf"));
    // Every object is created before it is selected, and the saved device contexts balance.
    let n_handles = u16::from_le_bytes(head[56..58].try_into().unwrap()) as u32;
    let mut created = std::collections::HashSet::new();
    let (mut saves, mut restores) = (0, 0);
    for (ty, r) in &recs {
        match ty {
            39 | 82 | 95 => {
                let h = u32::from_le_bytes(r[8..12].try_into().unwrap());
                assert!(h >= 1 && h < n_handles, "handle {h} of {n_handles}");
                created.insert(h);
            }
            37 => {
                let h = u32::from_le_bytes(r[8..12].try_into().unwrap());
                assert!(h & 0x8000_0000 != 0 || created.contains(&h), "handle {h} selected before it exists");
            }
            33 => saves += 1,
            34 => restores += 1,
            _ => {}
        }
    }
    assert_eq!(saves, restores);
    assert_eq!(created.len() as u32, n_handles - 1);
    let count = |t: u32| recs.iter().filter(|r| r.0 == t).count();
    assert!(count(84) > 3, "text records");
    assert!(count(59) == count(60) && count(64) > 3, "paths: {} begin, {} stroke", count(59), count(64));
    assert!(count(30) >= 1, "layer clip");
}

#[test]
fn emf_text_records_carry_the_string_font_and_spacing() {
    let mut t = text("A\u{b0}C \u{3b1}", Anchor::End);
    t.rotate_ccw = 90.0;
    t.bold = true;
    let bytes = emf::write_emf(&scene_with(vec![Item::Text(t)]), 1.0, &FixedMetrics);
    let recs = emf_records(&bytes);
    let font = recs.iter().find(|r| r.0 == 82).expect("font record").1;
    assert_eq!(font.len(), 104);
    assert_eq!(i32_at(font, 12), -353, "10 pt is 353 units high, negative for character height");
    assert_eq!(i32_at(font, 20), 900, "escapement in tenths of a degree");
    assert_eq!(i32_at(font, 28), 700, "bold");
    let face: Vec<u16> = font[12 + 28..12 + 28 + 10].chunks(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
    assert_eq!(String::from_utf16_lossy(&face), "Arial");
    assert!(
        recs.iter().any(|r| r.0 == 22 && u32::from_le_bytes(r.1[8..12].try_into().unwrap()) == 24 | 2),
        "right + baseline"
    );
    let t = recs.iter().find(|r| r.0 == 84).expect("text record").1;
    let n = u32::from_le_bytes(t[44..48].try_into().unwrap()) as usize;
    assert_eq!(n, 5);
    let off_s = u32::from_le_bytes(t[48..52].try_into().unwrap()) as usize;
    let off_dx = u32::from_le_bytes(t[72..76].try_into().unwrap()) as usize;
    let s: Vec<u16> = t[off_s..off_s + n * 2].chunks(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
    assert_eq!(String::from_utf16_lossy(&s), "A\u{b0}C \u{3b1}");
    let dx: Vec<i32> = (0..n).map(|k| i32_at(t, off_dx + 4 * k)).collect();
    assert_eq!(dx, vec![194; 5], "0.55 em of 353 units each");
    assert_eq!(t.len(), off_dx + 4 * n);
    assert_eq!((i32_at(t, 36), i32_at(t, 40)), (1764, 1411), "reference point (50, 40) pt in 0.01 mm");
}

#[test]
fn emf_pens_carry_width_dashes_and_colour() {
    let stroke = Stroke { dash: vec![4.0, 2.0], ..Stroke::solid(Color::rgb(1, 2, 3), 2.0) };
    let items = vec![Item::Path { d: "M10 10L50 10L50 30".into(), fill: None, stroke: Some(stroke) }];
    let bytes = emf::write_emf(&scene_with(items), 1.0, &FixedMetrics);
    let recs = emf_records(&bytes);
    let pen = recs.iter().find(|r| r.0 == 95).expect("pen").1;
    let u = |k: usize| u32::from_le_bytes(pen[8 + 4 * k..12 + 4 * k].try_into().unwrap());
    assert_eq!(u(5), 0x10000 | 0x200 | 0x2000 | 7, "geometric, flat caps, mitre joins, user style");
    assert_eq!(u(6), 71, "2 pt wide");
    assert_eq!(u(8), 0x0003_0201, "COLORREF is 0x00BBGGRR");
    assert_eq!(u(10), 2);
    assert_eq!((u(11), u(12)), (141, 71));
    // One path: begin, move, polyline, end, stroke.
    let kinds: Vec<u32> =
        recs.iter().map(|r| r.0).filter(|t| (59..=64).contains(t) || [5, 6, 27].contains(t)).collect();
    assert_eq!(kinds, vec![59, 27, 6, 60, 64]);
}

#[test]
fn emf_flattens_translucent_fills_and_scales() {
    let items =
        vec![Item::Path { d: "M0 0L10 0L10 10Z".into(), fill: Some(Color::rgba(255, 0, 0, 128)), stroke: None }];
    let scene = scene_with(items);
    let a = emf::write_emf(&scene, 1.0, &FixedMetrics);
    let b = emf::write_emf(&scene, 2.0, &FixedMetrics);
    let brush = emf_records(&a).into_iter().filter(|r| r.0 == 39).nth(1).unwrap().1; // after the white page
    assert_eq!(u32::from_le_bytes(brush[16..20].try_into().unwrap()), 0x007F_7FFF, "blended onto white");
    let width = |bytes: &[u8]| i32_at(emf_records(bytes)[0].1, 32);
    assert_eq!(width(&b), 2 * width(&a));
}

#[test]
fn eps_and_emf_are_exported_by_extension() {
    let dir = std::env::temp_dir().join(format!("sheaf-plot-vector-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let (p, g) = project();
    export_file(&p, g, &dir.join("a.EPS"), 72.0).unwrap();
    export_file(&p, g, &dir.join("b.emf"), 72.0).unwrap();
    assert!(std::fs::read(dir.join("a.EPS")).unwrap().starts_with(b"%!PS-Adobe-3.0 EPSF-3.0"));
    assert_eq!(std::fs::read(dir.join("b.emf")).unwrap()[..4], [1, 0, 0, 0]);
    assert!(matches!(export_bytes(&p, g, "xyz", 72.0, ExportSize::default()), Err(PlotError::UnknownFormat(_))));
    let _ = std::fs::remove_dir_all(&dir);
}

fn raster_bytes(ext: &str, opts: &RasterOptions) -> Vec<u8> {
    let (p, g) = project();
    export_bytes_opts(&p, g, ext, 100.0, ExportSize::default(), ExportLayout::default(), opts).unwrap()
}

#[test]
fn jpeg_bmp_and_gif_are_written_with_their_headers() {
    let o = RasterOptions::default();
    let jpg = raster_bytes("jpg", &o);
    assert_eq!(&jpg[..3], &[0xFF, 0xD8, 0xFF], "JPEG start of image");
    assert_eq!(&jpg[jpg.len() - 2..], &[0xFF, 0xD9], "JPEG end of image");
    let bmp = raster_bytes("bmp", &o);
    assert_eq!(&bmp[..2], b"BM");
    assert_eq!(u32::from_le_bytes(bmp[2..6].try_into().unwrap()) as usize, bmp.len(), "file size field");
    assert_eq!(u16::from_le_bytes(bmp[28..30].try_into().unwrap()), 24, "24 bits per pixel");
    let gif = raster_bytes("gif", &o);
    assert_eq!(&gif[..6], b"GIF89a");
}

#[test]
fn tga_is_written_with_its_header_in_each_depth() {
    for (depth, image_type, bpp, map) in
        [(ColorDepth::Bit24, 2, 24, 0usize), (ColorDepth::Grey8, 3, 8, 0), (ColorDepth::Colors256, 1, 8, 768)]
    {
        let tga = raster_bytes("tga", &RasterOptions { depth, ..RasterOptions::default() });
        assert_eq!((tga[1], tga[2], tga[16], tga[17]), (u8::from(map > 0), image_type, bpp, 0x20), "{depth:?}");
        let (w, h) = (u16::from_le_bytes([tga[12], tga[13]]) as usize, u16::from_le_bytes([tga[14], tga[15]]) as usize);
        assert_eq!(tga.len(), 18 + map + w * h * usize::from(bpp / 8), "{depth:?}: header, palette and pixels");
    }
}

#[test]
fn pcx_is_written_with_its_header_and_palette() {
    let rgb = raster_bytes("pcx", &RasterOptions { depth: ColorDepth::Bit24, ..RasterOptions::default() });
    assert_eq!(&rgb[..4], &[0x0A, 5, 1, 8], "maker, version 5, run-length, 8 bits");
    assert_eq!(rgb[65], 3, "three colour planes");
    let pal = raster_bytes("pcx", &RasterOptions { depth: ColorDepth::Colors256, ..RasterOptions::default() });
    assert_eq!(pal[65], 1, "one plane");
    assert_eq!(pal[pal.len() - 769], 0x0C, "256-colour palette marker before the last 768 bytes");
    let grey = raster_bytes("pcx", &RasterOptions { depth: ColorDepth::Grey8, ..RasterOptions::default() });
    assert_eq!(u16::from_le_bytes([grey[68], grey[69]]), 2, "grey palette info");
}

#[test]
fn psd_graph_exports_have_the_requested_colour_mode_and_complete_pixels() {
    for (depth, mode, channels, palette_len) in
        [(ColorDepth::Bit24, 3, 3, 0), (ColorDepth::Grey8, 1, 1, 0), (ColorDepth::Colors256, 2, 1, 768)]
    {
        let psd = raster_bytes("psd", &RasterOptions { depth, ..RasterOptions::default() });
        let word = |at| u16::from_be_bytes(psd[at..at + 2].try_into().unwrap());
        let long = |at| u32::from_be_bytes(psd[at..at + 4].try_into().unwrap());
        assert_eq!(&psd[..4], b"8BPS");
        assert_eq!((word(4), word(12), word(22), word(24)), (1, channels, 8, mode));
        assert!(long(14) > 0 && long(18) > 0);
        assert_eq!(long(26), palette_len);
        assert_eq!(psd.len(), 68 + palette_len as usize + long(14) as usize * long(18) as usize * channels as usize);
        assert!(psd[68 + palette_len as usize..].iter().any(|&b| b < 200), "graph pixels, not a blank image");
    }
}

#[test]
fn psd_composites_transparency_on_white_before_rgb_or_grey_encoding() {
    let raster = Raster { width: 2, height: 1, rgba: vec![255, 0, 0, 128, 0, 0, 255, 0] };
    let rgb = raster::encode_raster(&raster, "psd", 150.5, &RasterOptions::default()).unwrap();
    assert_eq!(&rgb[68..], &[255, 255, 127, 255, 127, 255], "planar RGB, composited on white");
    let grey = raster::encode_raster(
        &raster,
        "psd",
        150.5,
        &RasterOptions { depth: ColorDepth::Grey8, ..RasterOptions::default() },
    )
    .unwrap();
    assert_eq!(&grey[68..], &[165, 255]);
}

#[test]
fn oversized_psd_exports_are_refused_before_allocating_the_image() {
    let (p, g) = project();
    let size = ExportSize { width_in: Some(30_001.0 / 72.0), ..ExportSize::default() };
    let error = export_bytes(&p, g, "psd", 72.0, size).unwrap_err();
    assert!(matches!(error, PlotError::Image(_)), "{error}");
    assert!(error.to_string().contains("30000 pixels per dimension"), "{error}");
}

#[test]
fn colour_depth_changes_png_bmp_and_tiff() {
    let colours = RasterOptions { depth: ColorDepth::Colors256, ..RasterOptions::default() };
    let png = raster_bytes("png", &colours);
    assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
    assert_eq!(png[25], 3, "PNG colour type 3 is indexed");
    assert_eq!(raster_bytes("png", &RasterOptions::default())[25], 6, "24-bit keeps alpha (RGBA)");
    let bmp = raster_bytes("bmp", &colours);
    assert_eq!(u16::from_le_bytes(bmp[28..30].try_into().unwrap()), 8, "palette bitmap");
    assert_eq!(u32::from_le_bytes(bmp[2..6].try_into().unwrap()) as usize, bmp.len());
    let grey = RasterOptions { depth: ColorDepth::Grey8, ..RasterOptions::default() };
    assert_eq!(raster_bytes("png", &grey)[25], 0, "PNG colour type 0 is grey");
}

#[test]
fn indexed_tiff_preserves_palette_indices_resolution_and_each_compression() {
    use std::io::Cursor;
    use tiff::tags::Tag;

    // Odd width and transparency expose row-padding and white-compositing mistakes.
    let raster = Raster {
        width: 3,
        height: 2,
        rgba: vec![255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 0, 0, 0, 0, 255, 0, 0, 128, 40, 50, 60, 255],
    };
    let options = RasterOptions { depth: ColorDepth::Colors256, ..RasterOptions::default() };
    let png = raster::encode_raster(&raster, "png", 150.5, &options).unwrap();
    let mut png = png::Decoder::new(Cursor::new(png)).read_info().unwrap();
    let expected_palette = png.info().palette.as_ref().unwrap().to_vec();
    let mut expected_indices = vec![0; png.output_buffer_size().unwrap()];
    let frame = png.next_frame(&mut expected_indices).unwrap();
    expected_indices.truncate(frame.buffer_size());
    for (compression, tag) in
        [(TiffCompression::None, 1), (TiffCompression::PackBits, 32773), (TiffCompression::Lzw, 5)]
    {
        let options = RasterOptions { tiff_compression: compression, ..options };
        let bytes = raster::encode_raster(&raster, "tif", 150.5, &options).unwrap();
        let mut decoder = tiff::decoder::Decoder::new(Cursor::new(&bytes)).unwrap();
        assert_eq!(decoder.dimensions().unwrap(), (3, 2));
        assert_eq!(decoder.get_tag_u32(Tag::PhotometricInterpretation).unwrap(), 3);
        assert_eq!(decoder.get_tag_u32(Tag::SamplesPerPixel).unwrap(), 1);
        assert_eq!(decoder.get_tag_u16_vec(Tag::BitsPerSample).unwrap(), [8]);
        assert_eq!(decoder.get_tag_u32(Tag::Compression).unwrap(), tag);
        assert_eq!(decoder.get_tag_u32(Tag::ResolutionUnit).unwrap(), 2);
        for tag in [Tag::XResolution, Tag::YResolution] {
            assert_eq!(decoder.get_tag_u32_vec(tag).unwrap(), [15050, 100]);
        }
        let palette = decoder.get_tag_u16_vec(Tag::ColorMap).unwrap();
        assert_eq!(palette.len(), 768);
        for channel in 0..3 {
            for index in 0..256 {
                assert_eq!(palette[channel * 256 + index], u16::from(expected_palette[index * 3 + channel]) * 257);
            }
        }
        // This tiff crate can decompress 8-bit samples but cannot expand a palette. In a
        // test-only copy, interpret the samples as gray to read the original index plane.
        let mut indices_only = bytes;
        let ifd = u32::from_le_bytes(indices_only[4..8].try_into().unwrap()) as usize;
        let count = u16::from_le_bytes(indices_only[ifd..ifd + 2].try_into().unwrap()) as usize;
        let entry = (0..count)
            .map(|i| ifd + 2 + 12 * i)
            .find(|&at| u16::from_le_bytes(indices_only[at..at + 2].try_into().unwrap()) == 262)
            .unwrap();
        indices_only[entry + 8..entry + 10].copy_from_slice(&1u16.to_le_bytes());
        let mut decoder = tiff::decoder::Decoder::new(Cursor::new(indices_only)).unwrap();
        let tiff::decoder::DecodingResult::U8(indices) = decoder.read_image().unwrap() else { panic!("8-bit indices") };
        assert_eq!(indices, expected_indices, "{compression:?}");
    }
}

#[test]
fn tiff_compression_choice_is_written_in_the_file() {
    // The compression tag (259) of the first image file directory.
    fn compression(b: &[u8]) -> u16 {
        assert_eq!(&b[..4], b"II*\0");
        let ifd = u32::from_le_bytes(b[4..8].try_into().unwrap()) as usize;
        let n = u16::from_le_bytes(b[ifd..ifd + 2].try_into().unwrap()) as usize;
        for i in 0..n {
            let e = &b[ifd + 2 + i * 12..ifd + 14 + i * 12];
            if u16::from_le_bytes(e[..2].try_into().unwrap()) == 259 {
                return u16::from_le_bytes(e[8..10].try_into().unwrap());
            }
        }
        panic!("no compression tag");
    }
    let with =
        |c| compression(&raster_bytes("tif", &RasterOptions { tiff_compression: c, ..RasterOptions::default() }));
    assert_eq!(with(TiffCompression::None), 1);
    assert_eq!(with(TiffCompression::Lzw), 5);
    assert_eq!(with(TiffCompression::PackBits), 32773);
    let none =
        raster_bytes("tif", &RasterOptions { tiff_compression: TiffCompression::None, ..RasterOptions::default() });
    let lzw = raster_bytes("tif", &RasterOptions::default());
    assert!(lzw.len() < none.len(), "LZW is smaller than no compression");
}
