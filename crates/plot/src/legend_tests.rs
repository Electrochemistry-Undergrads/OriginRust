//! Tests of the legend text, its substitutions and its columns.

use super::*;
use crate::scene::{Item, ObjectKind, Scene};
use sheaf_core::template::Template;
use sheaf_core::{Color, ColumnData, Designation};

fn project(n: usize) -> (Project, WindowId) {
    project_with_template(n, Template::Line)
}

fn project_with_template(n: usize, template: Template) -> (Project, WindowId) {
    let mut p = Project::new();
    let book = p.add_workbook("data");
    let x: Vec<f64> = (0..=10).map(f64::from).collect();
    p.add_column(book, 0, Designation::X, ColumnData::numeric(x.clone())).unwrap();
    let g = p.add_graph(template);
    for k in 0..n {
        let yc = p
            .add_column(book, 0, Designation::Y, ColumnData::numeric(x.iter().map(|v| v * (k + 1) as f64).collect()))
            .unwrap();
        p.column_mut(yc).unwrap().long_name = format!("Series {}", k + 1);
        p.add_plot(g, 0, yc).unwrap();
    }
    (p, g)
}

fn scene(p: &Project, g: WindowId) -> Scene {
    layout::build_scene(p, g, &FixedMetrics).unwrap()
}

/// The text items of the legend: (x, y, text), and the number of line samples.
fn legend_items(s: &Scene) -> (Vec<(f64, f64, String)>, usize) {
    let layer = &s.layers[0];
    let obj = layer.objects.iter().find(|o| o.kind == ObjectKind::Legend).expect("a legend");
    let items = &layer.overlay[obj.items.clone()];
    let texts = items
        .iter()
        .filter_map(|i| if let Item::Text(t) = i { Some((t.x, t.y, t.text.clone())) } else { None })
        .collect();
    let samples = items.iter().filter(|i| matches!(i, Item::Path { fill: None, .. })).count();
    (texts, samples)
}

#[test]
fn the_automatic_text_lists_every_plot_with_its_symbol() {
    let (p, g) = project(3);
    assert_eq!(legend_automatic_text(&p, g, 0).unwrap(), "\\l(1) %(1)\n\\l(2) %(2)\n\\l(3) %(3)");
    assert_eq!(legend_text(&p, g, 0).unwrap(), legend_automatic_text(&p, g, 0).unwrap());
    let (texts, samples) = legend_items(&scene(&p, g));
    let names: Vec<&str> = texts.iter().map(|t| t.2.as_str()).collect();
    assert_eq!(names, ["Series 1", "Series 2", "Series 3"]);
    assert_eq!(samples, 3);
    assert!(legend_text(&p, g, 5).is_err());
}

#[test]
fn custom_text_reorders_substitutes_and_adds_literal_text() {
    let (mut p, g) = project(2);
    p.graph_mut(g).unwrap().layers[0].legend.text = Some("\\l(2) T = 300 K, %(2)\r\nonly text\n\\l(1)".into());
    assert_eq!(legend_text(&p, g, 0).unwrap(), "\\l(2) T = 300 K, %(2)\r\nonly text\n\\l(1)");
    let (texts, samples) = legend_items(&scene(&p, g));
    let names: Vec<&str> = texts.iter().map(|t| t.2.as_str()).collect();
    assert_eq!(names, ["T = 300 K, Series 2", "only text"]);
    assert_eq!(samples, 2, "two symbol samples: plots 2 and 1");
    // The text after a sample starts one gap after it; a line without sample starts at the left.
    let fs = 22.0;
    assert!((texts[0].0 - texts[1].0 - (2.2 * fs + 0.5 * fs)).abs() < 1e-9, "{texts:?}");
    // Rows are one after the other.
    assert!((texts[1].1 - texts[0].1 - 1.3 * fs).abs() < 1e-9);
}

#[test]
fn legend_plot_color_styles_substituted_text_at_its_measured_size() {
    use sheaf_core::graph::LegendPosition;

    let (mut p, g) = project(2);
    let layer = &mut p.graph_mut(g).unwrap().layers[0];
    layer.plots[1].line.color = Color::RED;
    layer.legend.position = LegendPosition::ManualTopLeft { x: 0.4, y: 0.3 };
    layer.legend.text = Some(r"\l(1) \cp2(\u(\b(%(2))))   ".into());
    let initial = scene(&p, g);
    let reference_box = automatic_legend_box(&p, g);
    let legend = &mut p.graph_mut(g).unwrap().layers[0].legend;
    legend.text = Some(r"\l(1) \c2(\u(\b(%(2))))   ".into());
    assert_eq!(crate::svg::write_svg(&initial), crate::svg::write_svg(&scene(&p, g)));
    assert_eq!(reference_box, automatic_legend_box(&p, g));
}

#[test]
fn shared_frame_legend_plot_color_uses_the_owner_layer_and_follows_style_changes() {
    use sheaf_core::graph::LegendScope;

    let (mut p, original) = project(2);
    let original = p.graph(original).unwrap();
    let a = original.layers[0].plots[0].y;
    let b = original.layers[0].plots[1].y;
    let g = p.plot_columns_double_y(Template::DoubleY, &[a], &[b]).unwrap();
    let layers = &mut p.graph_mut(g).unwrap().layers;
    layers[0].plots[0].line.color = Color::RED;
    layers[1].plots[0].line.color = Color::BLUE;
    let legend = &mut layers[1].legend;
    legend.scope = LegendScope::Graph;
    legend.visible = true;
    legend.text = Some(r"\l(1.1) \cp1(%(1.1)) \cp2(missing) \v(\cp1(literal))".into());
    let assert_color = |p: &Project, expected: Color| {
        let s = scene(p, g);
        let layer = s.layers.last().unwrap();
        let o = layer.objects.iter().find(|o| o.kind == ObjectKind::Legend && o.layer == 1).unwrap();
        let texts: Vec<_> = layer.overlay[o.items.clone()]
            .iter()
            .filter_map(|i| if let Item::Text(t) = i { Some((t.text.as_str(), t.color)) } else { None })
            .collect();
        assert_eq!(texts, [("Series 1", expected), (r" missing \cp1(literal)", Color::BLACK)]);
    };
    assert_color(&p, Color::BLUE);
    let color = Color::rgb(71, 163, 87);
    p.graph_mut(g).unwrap().layers[1].plots[0].line.color = color;
    assert_color(&p, color);
}

#[test]
fn separators_add_only_the_requested_gap_between_legend_entries() {
    let (mut p, g) = project(3);
    let plain = scene(&p, g);
    let plain_box = automatic_legend_box(&p, g);
    let raw = "\\sep:99\r\\l(1) %(1)\r\n \\sep:50 \n\\sep:25\n\\l(2) %(2)\n\\l(3) %(3)\n\\sep:99";
    p.graph_mut(g).unwrap().layers[0].legend.text = Some(raw.into());
    assert_eq!(legend_text(&p, g, 0).unwrap(), raw);

    let separated = scene(&p, g);
    let (texts, samples) = legend_items(&separated);
    let (plain_texts, plain_samples) = legend_items(&plain);
    assert_eq!((texts.len(), samples), (3, plain_samples));
    for (i, (text, control)) in texts.iter().zip(&plain_texts).enumerate() {
        assert_eq!((text.0, &text.2), (control.0, &control.2));
        let delta = if i == 0 { 0.0 } else { 0.75 * 22.0 };
        assert!((text.1 - control.1 - delta).abs() < 1e-9);
    }
    let separated_box = automatic_legend_box(&p, g);
    assert_eq!((separated_box.x, separated_box.y, separated_box.w), (plain_box.x, plain_box.y, plain_box.w));
    assert!((separated_box.h - plain_box.h - 0.75 * 22.0).abs() < 1e-9);
    assert!(!crate::svg::write_svg(&separated).contains("sep:"));
}

#[test]
fn legend_separators_do_not_take_column_slots_or_offset_the_next_column() {
    let (mut p, g) = project(5);
    let legend = &mut p.graph_mut(g).unwrap().layers[0].legend;
    legend.columns = 2;
    legend.text = Some("one\n\\sep:50\ntwo\nthree\n\\sep:75\nfour\n\\sep:200\n\\sep:bad".into());

    let (texts, _) = legend_items(&scene(&p, g));
    let names: Vec<_> = texts.iter().map(|row| row.2.as_str()).collect();
    assert_eq!(names, ["one", "two", "three", "four", r"\sep:bad"]);
    assert_eq!(texts[0].1, texts[3].1);
    assert_eq!(texts[0].0, texts[2].0);
    assert!(texts[3].0 > texts[2].0);
    let fs = 22.0;
    assert!((texts[1].1 - texts[0].1 - 1.8 * fs).abs() < 1e-9);
    assert!((texts[2].1 - texts[1].1 - 1.3 * fs).abs() < 1e-9);
    assert!((texts[4].1 - texts[3].1 - 3.3 * fs).abs() < 1e-9);
    assert!((automatic_legend_box(&p, g).h - (0.8 + 2.0 * 1.3 + 2.0) * fs).abs() < 1e-9);
}

#[test]
fn an_unknown_plot_number_draws_nothing() {
    let (mut p, g) = project(1);
    p.graph_mut(g).unwrap().layers[0].legend.text = Some("\\l(7) %(7) x".into());
    let (texts, samples) = legend_items(&scene(&p, g));
    assert_eq!(samples, 0);
    assert_eq!(texts.len(), 1);
    assert_eq!(texts[0].2, "x");
}

#[test]
fn columns_fill_top_to_bottom_and_widen_the_legend() {
    let (mut p, g) = project(5);
    p.graph_mut(g).unwrap().layers[0].legend.position = sheaf_core::graph::LegendPosition::TopRight;
    let one = legend_items(&scene(&p, g));
    p.graph_mut(g).unwrap().layers[0].legend.columns = 2;
    let (two, samples) = legend_items(&scene(&p, g));
    assert_eq!(samples, 5);
    // Three rows in the first column, two in the second.
    let xs: Vec<f64> = two.iter().map(|t| t.0).collect();
    assert!(xs[0] == xs[1] && xs[1] == xs[2] && xs[3] == xs[4] && xs[3] > xs[0], "{xs:?}");
    assert_eq!(two[3].1, two[0].1, "the second column starts at the top");
    assert!(two[0].1 < two[2].1);
    // The legend is anchored at the top right: more width moves its left side left.
    assert!(two[0].0 < one.0[0].0);
    // More columns than entries is the same as one entry per column.
    p.graph_mut(g).unwrap().layers[0].legend.columns = 99;
    let (many, _) = legend_items(&scene(&p, g));
    assert!(many.iter().all(|t| t.1 == many[0].1), "one row");
}

#[test]
fn wrapped_legend_captions_share_the_sample_and_keep_their_column_slots() {
    let (mut p, g) = project(3);
    let graph = p.graph_mut(g).unwrap();
    let layer = &mut graph.layers[0];
    layer.frame.width = 440.0 / graph.page.width_pt;
    layer.legend.columns = 2;
    layer.legend.text = Some(
        r"\l(1) \ww(Alpha beta élan gamma delta %(2))
\sep:50
\l(2) %(2)
\l(3) %(3)"
            .into(),
    );

    let s = scene(&p, g);
    let (text, samples) = legend_items(&s);
    let captions: Vec<_> = text.iter().map(|t| t.2.as_str()).collect();
    assert_eq!(captions, ["Alpha beta", "élan gamma", "delta %(2)", "Series 2", "Series 3"]);
    assert_eq!(samples, 3);
    assert!(text[..4].iter().all(|t| t.0 == text[0].0));
    assert_eq!(text[4].1, text[0].1);
    assert!(text[4].0 > text[0].0);

    let row_h = 1.3 * 22.0;
    assert!((text[2].1 - text[0].1 - 2.0 * row_h).abs() < 1e-9);
    assert!((text[3].1 - text[2].1 - row_h - 11.0).abs() < 1e-9);
    let frame = automatic_legend_box(&p, g);
    assert!((frame.h - 4.0 * row_h - (0.8 + 0.5) * 22.0).abs() < 1e-9);
    assert!(frame.w < 440.0);
    assert!(crate::svg::write_svg(&s).contains(">delta %(2)</text>"));
}

#[test]
fn legend_line_break_tokens_match_raw_newlines_before_plot_substitution() {
    let (mut p, g) = project(2);
    let y = p.graph(g).unwrap().layers[0].plots[0].y;
    p.column_mut(y).unwrap().long_name = "Value%(CRLF)".into();
    let set = |p: &mut Project, text: &str| {
        let legend = &mut p.graph_mut(g).unwrap().layers[0].legend;
        legend.columns = 2;
        legend.text = Some(text.into());
    };

    let authored = r"\l(1) %(1,@LL)%(CRLF)\h(Keep %(CRLF))%(CR)%(LF)%(CRLF)\l(2) %(2)";
    set(&mut p, authored);
    assert_eq!(legend_text(&p, g, 0).unwrap(), authored);
    let substituted = scene(&p, g);
    let (items, samples) = legend_items(&substituted);
    let text: Vec<_> = items.iter().map(|item| item.2.as_str()).collect();
    assert_eq!(text, ["Value%(CRLF)", "Keep %(CRLF)", "Series 2"]);
    assert_eq!(samples, 2);

    set(&mut p, "\\l(1) %(1,@LL)\n\\h(Keep %(CRLF))\n\n\\l(2) %(2)");
    assert_eq!(crate::svg::write_svg(&substituted), crate::svg::write_svg(&scene(&p, g)));
}

#[test]
fn legend_tabs_place_plot_samples_at_row_local_stops_and_extend_the_frame() {
    let (mut p, g) = project(2);
    let legend = &mut p.graph_mut(g).unwrap().layers[0].legend;
    legend.text = Some(r"\l(1) A%(TAB)\l(2) B%(TAB)%(CRLF)MMMMMMMM%(TAB)C%(TAB)%(TAB)D".into());
    for size in [11.0, 22.0, 33.0] {
        p.graph_mut(g).unwrap().layers[0].legend.font.size_pt = size;
        let s = scene(&p, g);
        let (items, samples) = legend_items(&s);
        let text: Vec<_> = items.iter().map(|item| item.2.as_str()).collect();
        assert_eq!(text, ["A", "B", "MMMMMMMM", "C", "D"]);
        assert_eq!(samples, 2);
        let step = 3.5 * size;
        assert!((items[1].0 - items[0].0 - step).abs() < 1e-9);
        assert!((items[3].0 - items[2].0 - 2.0 * step).abs() < 1e-9);
        assert!((items[4].0 - items[3].0 - 2.0 * step).abs() < 1e-9);
        let svg = crate::svg::write_svg(&s);
        assert!(!svg.contains("%(TAB)"));
    }

    p.graph_mut(g).unwrap().layers[0].legend.text = Some("A\tB".into());
    let unpadded = automatic_legend_box(&p, g);
    p.graph_mut(g).unwrap().layers[0].legend.text = Some("A\tB%(tab)".into());
    let padded = automatic_legend_box(&p, g);
    assert_eq!((padded.x, padded.y, padded.h), (unpadded.x, unpadded.y, unpadded.h));
    assert!((padded.w - (0.8 + 7.0) * 33.0).abs() < 1e-9);
    assert!(padded.w > unpadded.w);
}

#[test]
fn legend_tabs_keep_inline_font_and_styles_across_the_stop() {
    let (mut p, g) = project(1);
    let legend = &mut p.graph_mut(g).unwrap().layers[0].legend;
    legend.text = Some(r"\f:Courier New(\b(A%(TAB)\i(B))) C".into());
    let s = scene(&p, g);
    let layer = &s.layers[0];
    let legend = layer.objects.iter().find(|object| object.kind == ObjectKind::Legend).unwrap();
    let runs: Vec<_> = layer.overlay[legend.items.clone()]
        .iter()
        .filter_map(|item| if let Item::Text(text) = item { Some(text) } else { None })
        .collect();
    assert_eq!(runs.len(), 3);
    assert_eq!((&*runs[0].text, &*runs[0].family, runs[0].bold, runs[0].italic), ("A", "Courier New", true, false));
    assert_eq!((&*runs[1].text, &*runs[1].family, runs[1].bold, runs[1].italic), ("B", "Courier New", true, true));
    assert_eq!(&runs[2].text, " C");
    assert!(!runs[2].bold && !runs[2].italic && runs[2].family != "Courier New");
    assert!((runs[1].x - runs[0].x - 77.0).abs() < 1e-9);
}

#[test]
fn the_legend_of_a_double_y_graph_names_plots_by_layer_and_plot() {
    let mut p = Project::new();
    let book = p.add_workbook("data");
    let x: Vec<f64> = (0..=10).map(f64::from).collect();
    p.add_column(book, 0, Designation::X, ColumnData::numeric(x.clone())).unwrap();
    let a = p.add_column(book, 0, Designation::Y, ColumnData::numeric(x.clone())).unwrap();
    let b = p.add_column(book, 0, Designation::Y, ColumnData::numeric(x.clone())).unwrap();
    let g = p.plot_columns_double_y(Template::DoubleY, &[a], &[b]).unwrap();
    p.graph_mut(g).unwrap().layers[0].legend.scope = sheaf_core::graph::LegendScope::Graph;
    assert_eq!(legend_automatic_text(&p, g, 0).unwrap(), "\\l(1.1) %(1.1)\n\\l(2.1) %(2.1)");
}

/// The frame rectangle, excluding its stroke, so this checks the placement anchor rather than
/// font-dependent hit bounds. Reference values are the 2016 stock Line object's x1/y1.
fn automatic_legend_box(p: &Project, g: WindowId) -> Rect {
    let s = scene(p, g);
    let l = &s.layers[0];
    let object = l.objects.iter().find(|o| o.kind == ObjectKind::Legend).unwrap();
    l.overlay[object.items.clone()]
        .iter()
        .find_map(|item| match item {
            Item::Rect { rect, fill, .. } => {
                assert_eq!(*fill, Some(sheaf_core::color::Color::WHITE), "opaque stock legend");
                Some(*rect)
            }
            _ => None,
        })
        .unwrap()
}

fn check_origin_anchor(p: &Project, g: WindowId) -> Rect {
    let rect = automatic_legend_box(p, g);
    let frame = layout_geometry(p, g).unwrap()[0].frame;
    // x=4.933850365 on 0..6, y=12.043552036 on 1..11; the tolerance includes
    // Origin's page/frame rounding and the subpixel border of its exported rectangle.
    let expected_x = frame.x + (4.933_850_365 / 6.0) * frame.w;
    let expected_y = frame.y + (1.0 - (12.043_552_036 - 1.0) / 10.0) * frame.h;
    assert!((rect.x - expected_x).abs() < 0.0005 * frame.w, "{rect:?}, {frame:?}");
    assert!((rect.y - expected_y).abs() < 0.0005 * frame.h, "{rect:?}, {frame:?}");
    rect
}

#[test]
fn automatic_legend_anchor_matches_the_four_measured_stock_templates() {
    use sheaf_core::graph::LegendPosition;
    for template in [Template::Line, Template::Scatter, Template::LineSymbol, Template::Column] {
        let (p, g) = project_with_template(2, template);
        assert_eq!(p.graph(g).unwrap().layers[0].legend.position, LegendPosition::Automatic);
        check_origin_anchor(&p, g);
    }
    assert_eq!(
        Template::CyclicVoltammetry.new_graph(WindowId(1), "CV").layers[0].legend.position,
        LegendPosition::BottomRight
    );
    assert_eq!(Template::DoubleY.new_graph(WindowId(1), "DY").layers[0].legend.position, LegendPosition::TopRight);
}

#[test]
fn automatic_legend_anchor_stays_fixed_as_content_and_geometry_change() {
    use sheaf_core::graph::{Frame, Page};
    let (mut p, g) = project(4);
    let initial = check_origin_anchor(&p, g);
    {
        let legend = &mut p.graph_mut(g).unwrap().layers[0].legend;
        legend.text = Some("A much longer literal legend row\nsecond\nthird\nfourth".into());
    }
    let longer = check_origin_anchor(&p, g);
    assert!(longer.w > initial.w);
    assert_eq!((longer.x, longer.y), (initial.x, initial.y));
    p.graph_mut(g).unwrap().layers[0].legend.columns = 2;
    let columns = check_origin_anchor(&p, g);
    assert!(columns.h < longer.h && columns.w > longer.w);
    assert_eq!((columns.x, columns.y), (initial.x, initial.y));
    p.graph_mut(g).unwrap().layers[0].legend.font.size_pt = 30.0;
    let larger_font = check_origin_anchor(&p, g);
    assert_eq!((larger_font.x, larger_font.y), (initial.x, initial.y));
    p.graph_mut(g).unwrap().layers[0].frame = Frame { left: 0.25, top: 0.20, width: 0.55, height: 0.55 };
    check_origin_anchor(&p, g);
    p.graph_mut(g).unwrap().page = Page::inches(15.0, 7.5);
    check_origin_anchor(&p, g);
}

/// The items and bounds of free label `k` of the first layer.
fn label_items(s: &Scene, k: usize) -> (Vec<(f64, f64, String)>, usize, crate::Rect) {
    let layer = &s.layers[0];
    let obj = layer.objects.iter().find(|o| o.kind == ObjectKind::Label(k)).expect("the label is drawn");
    let items = &layer.overlay[obj.items.clone()];
    let texts = items
        .iter()
        .filter_map(|i| if let Item::Text(t) = i { Some((t.x, t.y, t.text.clone())) } else { None })
        .collect();
    let samples = items.iter().filter(|i| matches!(i, Item::Path { fill: None, .. })).count();
    (texts, samples, obj.bounds)
}

#[test]
fn a_free_label_draws_plot_symbols_and_names_like_the_legend_without_its_frame() {
    use sheaf_core::graph::TextLabel;
    let (mut p, g) = project(2);
    let layer = &mut p.graph_mut(g).unwrap().layers[0];
    let font = layer.legend.font.clone();
    layer.labels.push(TextLabel::new(r"\l(2) %(2)".to_string() + "\nNote", 0.2, 0.3, font.clone()));
    layer.labels.push(TextLabel::new("plain 50% text", 0.2, 0.7, font.clone()));
    let mut turned = TextLabel::new(r"\l(1) %(1)", 0.6, 0.5, font);
    turned.rotation_deg = 90.0;
    layer.labels.push(turned);
    let s = scene(&p, g);
    let (texts, samples, bounds) = label_items(&s, 0);
    let names: Vec<&str> = texts.iter().map(|t| t.2.as_str()).collect();
    assert_eq!(names, ["Series 2", "Note"]);
    assert_eq!(samples, 1, "the line sample of plot 2");
    assert!(
        !s.layers[0].overlay
            [s.layers[0].objects.iter().find(|o| o.kind == ObjectKind::Label(0)).unwrap().items.clone()]
        .iter()
        .any(|i| matches!(i, Item::Rect { .. })),
        "no frame or fill"
    );
    // The top of the first row sits half a font size above the anchor.
    let f = crate::layout_geometry(&p, g).unwrap()[0].frame;
    let anchor_y = f.y + 0.3 * f.h;
    assert!(bounds.y >= anchor_y - 22.0 && bounds.y <= anchor_y, "{bounds:?} {anchor_y}");
    assert!((bounds.x - (f.x + 0.2 * f.w)).abs() < 1.0, "the left edge at the anchor: {bounds:?}");
    // Labels without plot references and turned labels keep the plain text path.
    assert_eq!(label_items(&s, 1).0.iter().map(|t| t.2.as_str()).collect::<Vec<_>>(), ["plain 50% text"]);
    let (turned_texts, turned_samples, _) = label_items(&s, 2);
    assert_eq!((turned_texts.len(), turned_samples), (1, 0));
    assert_eq!(turned_texts[0].2, r"\l(1) %(1)");
}
