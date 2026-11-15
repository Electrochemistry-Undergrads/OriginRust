use super::*;
use crate::scene::{Item, TextItem};
use sheaf_core::graph::{LabelForm, LabelPosition};
use sheaf_core::template::Template;
use sheaf_core::{Color, ColumnData, Designation};

fn project() -> (Project, WindowId) {
    let mut p = Project::new();
    let book = p.add_workbook("Labels");
    let x = p.add_column(book, 0, Designation::X, ColumnData::numeric(vec![0.0, 1.0, 2.0])).unwrap();
    let y = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![1.0; 3])).unwrap();
    let g = p.add_graph(Template::Line);
    p.add_plot_xy(g, 0, Some(x), y, None).unwrap();
    let layer = &mut p.graph_mut(g).unwrap().layers[0];
    layer.x_axis.min = Some(0.0);
    layer.x_axis.max = Some(2.0);
    layer.y_axis.min = Some(0.0);
    layer.y_axis.max = Some(2.0);
    layer.background = Some(Color::rgb(220, 220, 220));
    layer.legend.visible = false;
    let plot = &mut layer.plots[0];
    plot.line.color = Color::BLUE;
    plot.line.width_pt = 6.0;
    plot.decor.labels.enable = true;
    plot.decor.labels.position = LabelPosition::Center;
    plot.decor.labels.font.size_pt = 20.0;
    plot.decor.labels.font.color = Color::RED;
    (p, g)
}

fn texts(items: &[Item]) -> Vec<&TextItem> {
    items.iter().filter_map(|item| if let Item::Text(t) = item { Some(t) } else { None }).collect()
}

fn mask(item: &Item) -> bool {
    matches!(item, Item::Path { fill: Some(color), stroke: None, .. } if *color == Color::WHITE)
}

#[test]
fn label_whiteout_clears_the_curve_behind_the_label_in_the_screen_raster() {
    let (mut p, g) = project();
    let font = &p.graph(g).unwrap().layers[0].plots[0].decor.labels.font;
    let text_width = SystemFonts::shared().width("1", font, font.size_pt);
    let (px, py) = layout_geometry(&p, g).unwrap()[0].data_to_page(1.0, 1.0);
    // Inside the left background margin, outside the digit, on the original blue curve.
    let sample = ((px - text_width / 2.0 - 0.025 * font.size_pt) * 2.0, py * 2.0);
    let pixel = |p: &Project| {
        let r = render_raster(p, g, 2.0).unwrap();
        let offset = ((sample.1 as u32 * r.width + sample.0 as u32) * 4) as usize;
        r.rgba[offset..offset + 4].to_vec()
    };
    assert_eq!(pixel(&p), [0, 0, 255, 255]);
    p.graph_mut(g).unwrap().layers[0].plots[0].decor.labels.white_out = true;
    assert_eq!(pixel(&p), [255, 255, 255, 255], "opaque white covers the blue curve");
}

#[test]
fn label_whiteout_follows_anchors_offsets_and_rotation_without_changing_text_or_clipping() {
    let (mut p, g) = project();
    for position in
        [LabelPosition::Left, LabelPosition::Right, LabelPosition::Center, LabelPosition::Above, LabelPosition::Below]
    {
        for angle in [0.0, 45.0, 90.0, -30.0] {
            let labels = &mut p.graph_mut(g).unwrap().layers[0].plots[0].decor.labels;
            labels.position = position;
            labels.rotate_deg = angle;
            labels.dx_pt = 7.0;
            labels.dy_pt = -3.0;
            labels.white_out = false;
            let before = layout::build_scene(&p, g, &FixedMetrics).unwrap();
            p.graph_mut(g).unwrap().layers[0].plots[0].decor.labels.white_out = true;
            let after = layout::build_scene(&p, g, &FixedMetrics).unwrap();
            let (b, a) = (&before.layers[0], &after.layers[0]);
            assert_eq!(texts(&b.plots), texts(&a.plots));
            assert_eq!(a.clip, b.clip);
            assert_eq!(a.overlay, b.overlay);
            assert_eq!(a.plots.iter().filter(|item| mask(item)).count(), 3);
            for (index, item) in a.plots.iter().enumerate() {
                let Item::Text(t) = item else { continue };
                let backdrop = &a.plots[index - 1];
                assert!(mask(backdrop), "a label is preceded by its own white polygon");
                let rect = hit::item_bounds(backdrop, &FixedMetrics).unwrap();
                let text = hit::item_bounds(item, &FixedMetrics).unwrap();
                assert!(rect.x <= text.x + 0.002 && rect.y <= text.y + 0.002);
                assert!(rect.x + rect.w >= text.x + text.w - 0.002);
                assert!(rect.y + rect.h >= text.y + text.h - 0.002);
                if t.rotate_ccw == 45.0 {
                    let Item::Path { d, .. } = backdrop else { unreachable!() };
                    let pts: Vec<(f64, f64)> = d
                        .trim_end_matches('Z')
                        .split(['M', 'L'])
                        .skip(1)
                        .map(|s| {
                            let mut values = s.split_whitespace().map(|v| v.parse::<f64>().unwrap());
                            (values.next().unwrap(), values.next().unwrap())
                        })
                        .collect();
                    assert_eq!(pts.len(), 4);
                    assert!(pts[1].0 > pts[0].0 && pts[1].1 < pts[0].1, "the rectangle rotates with the text");
                }
            }
        }
    }
}

#[test]
fn label_whiteout_only_draws_for_nonempty_labels_after_row_and_skip_selection() {
    let (mut p, g) = project();
    let book = p.workbooks().next().unwrap().id;
    let names = p
        .add_column(book, 0, Designation::Label, ColumnData::text(vec!["".into(), "middle".into(), "".into()]))
        .unwrap();
    let plot = &mut p.graph_mut(g).unwrap().layers[0].plots[0];
    plot.decor.labels.white_out = true;
    plot.decor.labels.form = LabelForm::Column { column: names };
    let s = layout::build_scene(&p, g, &FixedMetrics).unwrap();
    assert_eq!(s.layers[0].plots.iter().filter(|item| mask(item)).count(), 1);
    p.graph_mut(g).unwrap().layers[0].plots[0].symbol.every = 2;
    let s = layout::build_scene(&p, g, &FixedMetrics).unwrap();
    assert!(!s.layers[0].plots.iter().any(mask), "the only nonempty label is skipped");
    p.graph_mut(g).unwrap().layers[0].plots[0].symbol.every = 1;
    p.graph_mut(g).unwrap().layers[0].plots[0].decor.labels.only_rows = "1".into();
    let s = layout::build_scene(&p, g, &FixedMetrics).unwrap();
    assert!(!s.layers[0].plots.iter().any(mask), "row selection excludes the only nonempty label");
    p.graph_mut(g).unwrap().layers[0].plots[0].decor.labels.only_rows.clear();
    p.graph_mut(g).unwrap().layers[0].plots[0].decor.labels.enable = false;
    let s = layout::build_scene(&p, g, &FixedMetrics).unwrap();
    assert!(!s.layers[0].plots.iter().any(mask), "disabled labels have no backgrounds");
}
