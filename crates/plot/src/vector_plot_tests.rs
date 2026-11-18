//! Vector XYAM and XYXY (T107, graphing spec "Vector plots").

use super::*;
use crate::scene::{Item, ObjectKind};
use sheaf_core::template::Template;
use sheaf_core::{ColumnData, Designation};

fn geometry(p: &Project, g: WindowId) -> LayerGeometry {
    let graph = p.graph(g).unwrap();
    let pds: Vec<data::LayerData> = graph.layers.iter().map(|l| data::collect(p, l)).collect();
    layout::graph_geometry(p, graph, &pds, &FixedMetrics).remove(0)
}

#[test]
fn xyxy_axes_take_in_the_end_points() {
    // Origin's probe: (1,1) -> (2,1.5) and (2,2) -> (2.5,3); its axes went to 2.7 and 3.25.
    let mut p = Project::new();
    let b = p.add_workbook("");
    p.add_column(b, 0, Designation::X, ColumnData::numeric(vec![1.0, 2.0])).unwrap();
    let y1 = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![1.0, 2.0])).unwrap();
    p.add_column(b, 0, Designation::X, ColumnData::numeric(vec![2.0, 2.5])).unwrap();
    let y2 = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![1.5, 3.0])).unwrap();
    let g = p.plot_columns(Template::VectorXyxy, &[y1, y2]).unwrap();
    let geom = geometry(&p, g);
    assert!(geom.x.max >= 2.5 && geom.y.max >= 3.0, "x to {}, y to {}", geom.x.max, geom.y.max);
}

#[test]
fn an_xyam_arrow_runs_magnitude_times_multiplier_points_at_its_angle() {
    let mut p = Project::new();
    let b = p.add_workbook("");
    p.add_column(b, 0, Designation::X, ColumnData::numeric(vec![1.0, 2.0])).unwrap();
    let y = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![1.0, 2.0])).unwrap();
    let a = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![0.0, std::f64::consts::FRAC_PI_2])).unwrap();
    let m = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![100.0, 50.0])).unwrap();
    let g = p.plot_columns(Template::VectorXyam, &[y, a, m]).unwrap();
    let geom = geometry(&p, g);
    let s = layout::build_scene(&p, g, &FixedMetrics).unwrap();
    let shafts: Vec<Vec<f64>> = s.layers[0]
        .plots
        .iter()
        .filter_map(|it| match it {
            scene::Item::Path { d, fill: None, stroke: Some(st) } if st.width == 0.5 => {
                Some(d.split(['M', 'L', ' ']).filter_map(|t| t.parse().ok()).collect())
            }
            _ => None,
        })
        .collect();
    assert_eq!(shafts.len(), 2, "{shafts:?}");
    // Row 1: angle 0, 100 x 0.16 = 16 pt to the right (the shaft stops at the 4.5 pt head).
    let (x0, y0) = geom.data_to_page(1.0, 1.0);
    let r1 = &shafts[0];
    assert!((r1[0] - x0).abs() < 0.01 && (r1[1] - y0).abs() < 0.01, "{r1:?}");
    assert!((r1[2] - (x0 + 16.0 - 4.5)).abs() < 0.01 && (r1[3] - y0).abs() < 0.01, "{r1:?}");
    // Row 2: pi/2 radians points straight up, 8 pt.
    let (x1, y1) = geom.data_to_page(2.0, 2.0);
    let r2 = &shafts[1];
    assert!((r2[2] - x1).abs() < 0.01 && (r2[3] - (y1 - 8.0 + 4.5)).abs() < 0.01, "{r2:?}");
}

fn sample_graph(template: Template) -> (Project, WindowId) {
    let mut p = Project::new();
    let book = p.add_workbook("");
    let add =
        |p: &mut Project, designation, values| p.add_column(book, 0, designation, ColumnData::numeric(values)).unwrap();
    add(&mut p, Designation::X, vec![1.0, 2.0]);
    let y = add(&mut p, Designation::Y, vec![1.0, 2.0]);
    let columns = if template == Template::VectorXyam {
        let angle = add(&mut p, Designation::Y, vec![0.0, 1.0]);
        let magnitude = add(&mut p, Designation::Y, vec![100.0, 50.0]);
        vec![y, angle, magnitude]
    } else {
        add(&mut p, Designation::X, vec![2.0, 2.5]);
        let end_y = add(&mut p, Designation::Y, vec![1.5, 3.0]);
        vec![y, end_y]
    };
    let graph = p.plot_columns(template, &columns).unwrap();
    (p, graph)
}

fn legend_items(scene: &scene::Scene) -> &[Item] {
    let layer = &scene.layers[0];
    let legend = layer.objects.iter().find(|object| object.kind == ObjectKind::Legend).unwrap();
    &layer.overlay[legend.items.clone()]
}

#[test]
fn both_vector_legends_draw_the_plot_style_as_a_right_pointing_arrow() {
    for (template, head_length) in [(Template::VectorXyam, 4.5), (Template::VectorXyxy, 6.5)] {
        let (mut p, graph) = sample_graph(template);
        let layer = &mut p.graph_mut(graph).unwrap().layers[0];
        let color = sheaf_core::Color::rgb(40, 70, 120);
        layer.plots[0].line.color = color;
        layer.plots[0].line.width_pt = 1.25;
        let size = layer.legend.font.size_pt;
        let scene = layout::build_scene(&p, graph, &FixedMetrics).unwrap();
        let [
            Item::Rect { .. },
            Item::Path { d: shaft, fill: None, stroke: Some(pen) },
            Item::Path { d: head, fill: Some(fill), stroke: None },
            Item::Text(label),
        ] = legend_items(&scene)
        else {
            panic!("expected a shaft, solid head and label for {template:?}");
        };
        assert_eq!((*fill, pen.color, pen.width), (color, color, 1.25));
        let shaft = crate::hit::path_points(shaft);
        let points = crate::hit::path_points(head);
        let (tail, base, tip) = (shaft[0], shaft[1], points[0]);
        assert_eq!(tail.1, tip.1);
        assert_eq!(base.0, points[1].0);
        assert_eq!(base.0, points[2].0);
        assert!((tip.0 - base.0 - head_length).abs() < 0.01);
        assert!((tip.0 - tail.0 - 2.2 * size).abs() < 0.01);
        assert!((label.x - tip.0 - 0.5 * size).abs() < 0.01);
        assert!(crate::svg::write_svg(&scene).contains(&format!("d=\"{head}\"")));
    }
}

#[test]
fn oversized_vector_legend_heads_fit_the_row_and_leave_a_shaft() {
    let (mut p, graph) = sample_graph(Template::VectorXyxy);
    let layer = &mut p.graph_mut(graph).unwrap().layers[0];
    layer.legend.font.size_pt = 8.0;
    let spec = layer.plots[0].decor.vector.as_mut().unwrap();
    spec.head_pt = 600.0;
    spec.head_angle_deg = 150.0;
    let scene = layout::build_scene(&p, graph, &FixedMetrics).unwrap();
    let [Item::Rect { rect, .. }, Item::Path { d: shaft, .. }, Item::Path { d: head, .. }, Item::Text(label)] =
        legend_items(&scene)
    else {
        panic!("expected a framed vector legend");
    };
    let shaft = crate::hit::path_points(shaft);
    let head = crate::hit::path_points(head);
    assert!(shaft[1].0 - shaft[0].0 >= 8.8);
    assert!(head.iter().all(|&(x, y)| x < label.x && y > rect.y && y < rect.y + rect.h));
    assert!((head[1].1 - head[2].1 - 8.0).abs() < 0.01);
}
