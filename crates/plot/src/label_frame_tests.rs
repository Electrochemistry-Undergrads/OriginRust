use super::*;
use crate::scene::{Item, Scene};
use sheaf_core::graph::{LabelForm, LabelFrame, LabelPosition};
use sheaf_core::template::Template;
use sheaf_core::{Color, ColumnData, Designation};

fn project() -> (Project, WindowId) {
    let mut p = Project::new();
    let book = p.add_workbook("Labels");
    p.add_column(book, 0, Designation::X, ColumnData::numeric(vec![0.0, 1.0, 2.0])).unwrap();
    let y = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![12345.0; 3])).unwrap();
    let g = p.add_graph(Template::LineSymbol);
    p.add_plot(g, 0, y).unwrap();
    let layer = &mut p.graph_mut(g).unwrap().layers[0];
    layer.background = Some(Color::rgb(220, 220, 220));
    layer.legend.visible = false;
    layer.x_axis.min = Some(0.0);
    layer.x_axis.max = Some(2.0);
    layer.y_axis.min = Some(0.0);
    layer.y_axis.max = Some(24690.0);
    let plot = &mut layer.plots[0];
    plot.line.color = Color::RED;
    plot.line.width_pt = 6.0;
    plot.symbol.size_pt = 0.0;
    let labels = &mut plot.decor.labels;
    labels.enable = true;
    labels.only_rows = "2".into();
    labels.font.size_pt = 30.0;
    labels.position = LabelPosition::Center;
    labels.frame = LabelFrame::Box;
    labels.margin_percent = 20.0;
    (p, g)
}

fn labels(p: &mut Project, g: WindowId) -> &mut sheaf_core::graph::DataLabels {
    &mut p.graph_mut(g).unwrap().layers[0].plots[0].decor.labels
}

fn rectangles(scene: &Scene) -> Vec<Rect> {
    scene.layers[0]
        .plots
        .iter()
        .filter_map(|i| match i {
            Item::Rect { rect, .. } => Some(*rect),
            _ => None,
        })
        .collect()
}

#[test]
fn label_frame_and_whiteout_follow_origins_paint_order_and_unscaled_whiteout_box() {
    // Origin 2016 exports box0, box20, shadow20white, box0white, none20white:
    // black fill precedes White Out; margin grows only the frame, not the white text box.
    let (mut p, g) = project();
    let font = &p.graph(g).unwrap().layers[0].plots[0].decor.labels.font;
    let width = SystemFonts::shared().width("12345", font, 30.0);
    let (px, py) = layout_geometry(&p, g).unwrap()[0].data_to_page(1.0, 12345.0);
    let sample = |p: &Project, x: f64, y: f64| {
        let r = render_raster(p, g, 2.0).unwrap();
        let i = (((y * 2.0) as u32 * r.width + (x * 2.0) as u32) * 4) as usize;
        r.rgba[i..i + 4].to_vec()
    };
    let text_edge = px - width / 2.0 - 0.75;
    assert_eq!(sample(&p, text_edge, py), [0, 0, 0, 255]);
    labels(&mut p, g).white_out = true;
    assert_eq!(sample(&p, text_edge, py), [255, 255, 255, 255]);
    assert_eq!(sample(&p, px - width / 2.0 - 4.0, py), [0, 0, 0, 255], "black margin remains outside White Out");
    let edge = rectangles(&graph_scene(&p, g).unwrap())[0];
    let just_right = edge.x + edge.w + 2.0;
    assert_eq!(sample(&p, just_right, py), [255, 0, 0, 255]);
    labels(&mut p, g).frame = LabelFrame::Shadow;
    assert_eq!(sample(&p, just_right, py), [0, 0, 0, 255], "shadow is behind the box");
}

#[test]
fn label_frames_enclose_rotated_padded_text_and_shadows_stay_aligned_to_the_page() {
    // Origin shadow20rot45 uses a page-aligned square enclosing the rotated padded box.
    // Arial 15/30 exports both move the shadow about 6-7 pixels, independent of font size.
    let (mut p, g) = project();
    for size in [15.0, 30.0] {
        for angle in [0.0_f64, 45.0, 90.0, -30.0] {
            for position in [LabelPosition::Left, LabelPosition::Center, LabelPosition::Right] {
                let l = labels(&mut p, g);
                l.frame = LabelFrame::Shadow;
                l.white_out = true;
                l.frame_border = Color::BLUE;
                l.frame_fill = Color::WHITE;
                l.font.size_pt = size;
                l.rotate_deg = angle;
                l.position = position;
                l.dx_pt = 7.0;
                l.dy_pt = -3.0;
                let s = layout::build_scene(&p, g, &FixedMetrics).unwrap();
                let r = rectangles(&s);
                assert_eq!(r.len(), 2);
                let (shadow, frame) = (r[0], r[1]);
                let pad = size * (4.0 / 3.0) * 0.2;
                let (w, h) = (5.0 * 0.55 * size + 2.0 * pad, 1.15 * size + 2.0 * pad);
                let (sin, cos) = angle.to_radians().sin_cos();
                assert!((frame.w + 0.5 - (w * cos.abs() + h * sin.abs())).abs() < 1e-8);
                assert!((frame.h + 0.5 - (w * sin.abs() + h * cos.abs())).abs() < 1e-8);
                assert!((shadow.x - (frame.x - 0.25) - 6.0).abs() < 1e-8);
                assert!((shadow.y - (frame.y - 0.25) - 6.0).abs() < 1e-8);
                let items = &s.layers[0].plots;
                let start = items.iter().position(|i| matches!(i, Item::Rect { .. })).unwrap();
                assert!(
                    matches!(&items[start+1],Item::Rect{fill:Some(c),stroke:Some(st),..} if *c==Color::WHITE && st.color==Color::BLUE)
                );
                assert!(matches!(&items[start+2],Item::Path{fill:Some(c),stroke:None,..} if *c==Color::WHITE));
                assert!(matches!(&items[start+3],Item::Text(t) if t.text=="12345" && t.rotate_ccw==angle));
            }
        }
    }
}

#[test]
fn frame_none_ignores_margin_and_empty_or_skipped_labels_get_no_frame() {
    let (mut p, g) = project();
    let l = labels(&mut p, g);
    l.frame = LabelFrame::None;
    l.white_out = true;
    l.margin_percent = 0.0;
    let before = render_svg(&p, g).unwrap();
    labels(&mut p, g).margin_percent = 20.0;
    assert_eq!(render_svg(&p, g).unwrap(), before);
    let book = p.workbooks().next().unwrap().id;
    let c = p
        .add_column(book, 0, Designation::Label, ColumnData::text(vec!["".into(), "middle".into(), "".into()]))
        .unwrap();
    let l = labels(&mut p, g);
    l.form = LabelForm::Column { column: c };
    l.frame = LabelFrame::Box;
    l.only_rows.clear();
    assert_eq!(rectangles(&graph_scene(&p, g).unwrap()).len(), 1);
    p.graph_mut(g).unwrap().layers[0].plots[0].symbol.every = 2;
    assert!(rectangles(&graph_scene(&p, g).unwrap()).is_empty());
}

#[test]
fn label_frames_export_through_screen_and_each_vector_backend() {
    let (mut p, g) = project();
    let l = labels(&mut p, g);
    l.frame = LabelFrame::Shadow;
    l.frame_fill = Color::WHITE;
    l.frame_border = Color::rgb(11, 22, 33);
    l.rotate_deg = 45.0;
    assert!(render_svg(&p, g).unwrap().contains("#0B1621"));
    assert!(render_png(&p, g, 72.0).unwrap().starts_with(b"\x89PNG"));
    assert!(render_pdf(&p, g).unwrap().starts_with(b"%PDF"));
    assert!(render_eps(&p, g).unwrap().starts_with(b"%!PS"));
    assert!(!render_emf(&p, g).unwrap().is_empty());
}

#[test]
fn point_label_escapes_use_the_owning_layers_saved_plot_slots() {
    let (mut p, g) = project();
    let book = p.workbooks().next().unwrap().id;
    let column = p
        .add_column(
            book,
            0,
            Designation::Label,
            ColumnData::text(vec!["".into(), r"\cp1(\b(H)\-(2))\cp9(O)\v(\cp2(Q))".into(), "".into()]),
        )
        .unwrap();
    let layer = &mut p.graph_mut(g).unwrap().layers[0];
    let mut second = layer.clone();
    let plot = &mut second.plots[0];
    plot.decor.labels.form = LabelForm::Column { column };
    plot.decor.labels.rotate_deg = 25.0;
    plot.decor.labels.font.color = Color::rgb(40, 60, 80);
    let mut hidden = layer.plots[0].clone();
    hidden.visible = false;
    hidden.line.color = Color::rgb(17, 131, 213);
    second.plots.insert(0, hidden);
    p.graph_mut(g).unwrap().layers.push(second);

    let scene = graph_scene(&p, g).unwrap();
    let runs: Vec<_> = scene.layers[1]
        .plots
        .iter()
        .filter_map(|item| if let Item::Text(run) = item { Some(run) } else { None })
        .collect();
    assert_eq!(runs.iter().map(|r| r.text.as_str()).collect::<Vec<_>>(), ["H", "2", r"O\cp2(Q)"]);
    assert!(runs[0].bold && runs[1].size < runs[0].size);
    assert!(runs.iter().all(|r| r.rotate_ccw == 25.0));
    assert_eq!(runs[0].color, Color::rgb(17, 131, 213));
    assert_eq!(runs[1].color, runs[0].color);
    assert_eq!(runs[2].color, Color::rgb(40, 60, 80));
    let svg = render_svg(&p, g).unwrap();
    assert!(svg.contains("#1183D5") && svg.contains(r"O\cp2(Q)"));
}

#[test]
fn point_label_frame_and_whiteout_measure_displayed_text() {
    let (mut p, g) = project();
    let book = p.workbooks().next().unwrap().id;
    let column = p
        .add_column(book, 0, Designation::Label, ColumnData::text(vec!["".into(), "Water".into(), "".into()]))
        .unwrap();
    let l = labels(&mut p, g);
    l.form = LabelForm::Column { column };
    l.rotate_deg = 40.0;
    l.white_out = true;
    l.font.bold = true;
    let plain = graph_scene(&p, g).unwrap();
    labels(&mut p, g).font.bold = false;
    p.column_mut(column).unwrap().data = ColumnData::text(vec!["".into(), r"\cp1(\b(Water))".into(), "".into()]);
    let decorated = graph_scene(&p, g).unwrap();
    assert_eq!(rectangles(&decorated), rectangles(&plain));
    let white_paths = |scene: &Scene| {
        scene.layers[0]
            .plots
            .iter()
            .filter_map(|item| match item {
                Item::Path { d, fill: Some(Color::WHITE), .. } => Some(d.clone()),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    assert!(!white_paths(&plain).is_empty());
    assert_eq!(white_paths(&decorated), white_paths(&plain));
    p.column_mut(column).unwrap().data = ColumnData::text(vec!["".into(), r"\b()".into(), "".into()]);
    assert!(rectangles(&graph_scene(&p, g).unwrap()).is_empty());
}
