use super::*;
use crate::scene::ObjectKind;
use sheaf_core::template::Template;

const TEMPLATES: [Template; 4] = [Template::Multi3YYy, Template::Multi3YYY, Template::Multi4YYyy, Template::Multi4YyYy];

fn project(template: Template) -> (Project, WindowId) {
    let names = ["Alpha", "Beta", "Gamma", "Delta"];
    project_with_names(template, &names[..template.multi_y().unwrap().layers()])
}

fn project_with_names(template: Template, names: &[&str]) -> (Project, WindowId) {
    let mut p = Project::new();
    let book = p.add_workbook("synthetic");
    p.add_column(book, 0, sheaf_core::Designation::X, sheaf_core::ColumnData::numeric(vec![0.0, 1.0, 2.0])).unwrap();
    let mut ys = Vec::new();
    for (i, name) in names.iter().enumerate() {
        let id = p
            .add_column(
                book,
                0,
                sheaf_core::Designation::Y,
                sheaf_core::ColumnData::numeric(vec![1.0 + i as f64, 2.0 + i as f64, 3.0 + i as f64]),
            )
            .unwrap();
        p.column_mut(id).unwrap().long_name = (*name).into();
        ys.push(id);
    }
    let graph = p.plot_columns(template, &ys).unwrap();
    (p, graph)
}

#[test]
fn native_stock_legend_membership_and_spacing_follow_the_eight_plot_exports() {
    let names = ["Series01", "Series02", "Series03", "Series04", "Series05", "Series06", "Series07", "Series08"];
    for template in TEMPLATES {
        let (p, g) = project_with_names(template, &names);
        let sizes = p.graph(g).unwrap().layers[0].legend.font.size_pt;
        let scene = layout::build_scene(&p, g, &FixedMetrics).unwrap();
        let (layer, object) = scene
            .layers
            .iter()
            .find_map(|l| l.objects.iter().find(|o| o.kind == ObjectKind::Legend).map(|o| (l, o)))
            .unwrap();
        let rows = &layer.overlay[object.items.clone()];
        let text: Vec<_> =
            rows.iter().filter_map(|item| if let scene::Item::Text(t) = item { Some(t) } else { None }).collect();
        let expected = [
            names[0],
            names[1],
            names[2],
            names[3],
            if template.multi_y().unwrap().layers() == 3 { names[6] } else { names[4] },
        ];
        assert_eq!(text.iter().map(|t| t.text.as_str()).collect::<Vec<_>>(), expected, "{template:?}");
        assert!(text.windows(2).all(|pair| (pair[1].y - pair[0].y - 1.22 * sizes).abs() < 1e-6));
        let (sample_start, sample_end) = rows
            .iter()
            .find_map(|item| match item {
                scene::Item::Path { d, .. } => {
                    let points = hit::path_points(d);
                    (points.len() == 2).then(|| (points[0].0, points[1].0))
                }
                _ => None,
            })
            .unwrap();
        assert!((sample_end - sample_start - 2.22 * sizes).abs() < 0.002);
        assert!((text[0].x - sample_end - 0.275 * sizes).abs() < 0.002);
    }
}

#[test]
fn custom_stock_legend_text_can_reference_an_extra_plot_on_a_later_layer() {
    let (mut p, g) = project_with_names(Template::Multi3YYy, &["Alpha", "Beta", "Gamma", "Delta", "Extra"]);
    let svg = tests::svg_of(&p, g);
    assert!(!tests::texts(&tests::doc(&svg)).iter().any(|s| s == "Extra"));
    p.graph_mut(g).unwrap().layers[0].legend.text = Some("\\l(2.2) %(2.2)".to_owned());
    let svg = tests::svg_of(&p, g);
    assert!(tests::texts(&tests::doc(&svg)).iter().any(|s| s == "Extra"));
}

#[test]
fn hiding_a_linked_layers_first_plot_keeps_its_remaining_legend_slots() {
    let (mut p, g) = project_with_names(Template::Multi3YYy, &["Alpha", "Beta", "Gamma", "Delta", "Extra"]);
    p.graph_mut(g).unwrap().layers[1].plots[0].visible = false;
    let svg = tests::svg_of(&p, g);
    let doc = tests::doc(&svg);
    let names = tests::texts(&doc);
    assert!(!names.iter().any(|s| s == "Beta" || s == "Extra"));
    assert!(!legend_automatic_text(&p, g, 0).unwrap().contains("2.1"));

    p.graph_mut(g).unwrap().layers[1].plots.remove(0);
    let svg = tests::svg_of(&p, g);
    assert!(tests::texts(&tests::doc(&svg)).iter().any(|s| s == "Extra"));
    assert!(legend_automatic_text(&p, g, 0).unwrap().contains("2.1"));
}

#[test]
fn stock_multi_y_legends_have_origin_defaults_above_the_frame() {
    for (i, template) in TEMPLATES.into_iter().enumerate() {
        let (p, g) = project(template);
        let graph = p.graph(g).unwrap();
        let legend = &graph.layers[0].legend;
        let scene = layout::build_scene(&p, g, &FixedMetrics).unwrap();
        let object =
            scene.layers.iter().flat_map(|layer| &layer.objects).find(|o| o.kind == ObjectKind::Legend).unwrap();
        let frame = layout_geometry(&p, g).unwrap()[0].frame;
        assert!(object.bounds.y < frame.y, "{template:?}: legend must begin above the frame");
        assert_eq!(legend.font.size_pt, if i == 3 { 20.0 } else { 22.0 });
        assert_eq!(legend.background, None);
        assert!(!legend.frame, "{template:?}: native background=0 has no border");
        let empty = template.new_graph(WindowId(999), "empty");
        assert_eq!(&empty.layers[0].legend, legend, "both named-template construction paths must agree");
    }
}

fn bounds(p: &Project, g: WindowId) -> Rect {
    let scene = layout::build_scene(p, g, &FixedMetrics).unwrap();
    scene.layers.iter().flat_map(|l| &l.objects).find(|o| o.kind == ObjectKind::Legend).unwrap().bounds
}

#[test]
fn native_top_left_points_are_retained_for_all_four_templates() {
    // Origin 2016 post-properties.json: x1/y1, converted from axis units to frame fractions.
    let native = [
        (0.12482036542804353, -0.21129770992366412),
        (0.15074229158736202, -0.19563293006786664),
        (1.002754820936639, -0.3123018389346861),
        (0.05832955061280075, -0.27712),
    ];
    for (template, (nx, ny)) in TEMPLATES.into_iter().zip(native) {
        let (p, g) = project(template);
        let sheaf_core::graph::LegendPosition::ManualTopLeft { x, y } = p.graph(g).unwrap().layers[0].legend.position
        else {
            panic!("{template:?}")
        };
        assert!((x - nx).abs() < 1e-12 && (y - ny).abs() < 1e-12);
    }
}

#[test]
fn multi_y_legend_text_grows_right_from_the_same_anchor() {
    for template in TEMPLATES {
        let (mut p, g) = project(template);
        let before = bounds(&p, g);
        let y = p.graph(g).unwrap().layers[0].plots[0].y;
        p.column_mut(y).unwrap().long_name = "Alpha with a considerably longer title".into();
        let after = bounds(&p, g);
        assert!((before.x - after.x).abs() < 1e-9 && (before.y - after.y).abs() < 1e-9, "{template:?}");
        assert!(after.w > before.w + 100.0);
        let svg = tests::svg_of(&p, g);
        let doc = tests::doc(&svg);
        assert!(tests::texts(&doc).iter().any(|t| t == "Alpha with a considerably longer title"));
    }
}

#[test]
fn legacy_manual_centres_and_dialog_plan_defaults_remain_unchanged() {
    use sheaf_core::graph::LegendPosition;
    let (mut p, g) = project(Template::Multi3YYy);
    p.graph_mut(g).unwrap().layers[0].legend.position = LegendPosition::Manual { x: 0.5, y: 0.5 };
    let before = bounds(&p, g);
    let y = p.graph(g).unwrap().layers[0].plots[0].y;
    p.column_mut(y).unwrap().long_name = "Alpha with a considerably longer title".into();
    let after = bounds(&p, g);
    assert!((before.x + before.w / 2.0 - after.x - after.w / 2.0).abs() < 1e-9);
    assert_eq!(sheaf_core::multi_y::MultiYOptions::default().plan(3).unwrap().legend, LegendPosition::TopRight);
}

#[test]
fn multi_y_legend_visible_bounds_match_the_native_anchor_and_clear_the_frame() {
    use sheaf_core::graph::LegendPosition;
    for template in TEMPLATES {
        let (p, g) = project(template);
        let frame = layout_geometry(&p, g).unwrap()[0].frame;
        let LegendPosition::ManualTopLeft { x, y } = p.graph(g).unwrap().layers[0].legend.position else { panic!() };
        let visible = bounds(&p, g);
        // Path coordinates are emitted to three decimals in each paint pass.
        assert!(
            (visible.x - frame.x - x * frame.w).abs() <= 0.001,
            "{template:?}: visible={visible:?}, frame={frame:?}, anchor={x},{y}"
        );
        assert!(
            (visible.y - frame.y - y * frame.h).abs() < 1e-9,
            "{template:?}: visible={visible:?}, frame={frame:?}, anchor={x},{y}"
        );
        assert!(visible.y + visible.h < frame.y, "{template:?}: legend rows must stay above the plot frame");
    }
}
