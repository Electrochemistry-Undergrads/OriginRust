use super::*;
use crate::scene::ObjectKind;
use sheaf_core::template::Template;

const TEMPLATES: [Template; 4] = [Template::Multi3YYy, Template::Multi3YYY, Template::Multi4YYyy, Template::Multi4YyYy];

fn project(template: Template) -> (Project, WindowId) {
    let mut p = Project::new();
    let book = p.add_workbook("synthetic");
    p.add_column(book, 0, sheaf_core::Designation::X, sheaf_core::ColumnData::numeric(vec![0.0, 1.0, 2.0])).unwrap();
    let mut ys = Vec::new();
    for (i, name) in ["Alpha", "Beta", "Gamma", "Delta"].iter().enumerate().take(template.multi_y().unwrap().layers()) {
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
