//! Plot > Specialized > Zoom (T112, graphing spec "Zoom graph").

use super::*;
use sheaf_core::template::Template;
use sheaf_core::{ColumnData, Designation};

#[test]
fn the_lower_layer_shows_the_region_of_the_rectangle_and_follows_it() {
    let mut p = Project::new();
    let b = p.add_workbook("");
    p.add_column(b, 0, Designation::X, ColumnData::numeric((1..=50).map(f64::from).collect())).unwrap();
    let y = p
        .add_column(
            b,
            0,
            Designation::Y,
            ColumnData::numeric((1..=50).map(|i| (f64::from(i) / 5.0).sin() * 10.0).collect()),
        )
        .unwrap();
    let g = p.plot_columns(Template::Zoom, &[y]).unwrap();
    let geoms = |p: &Project| {
        let graph = p.graph(g).unwrap();
        let pds: Vec<data::LayerData> = graph.layers.iter().map(|l| data::collect(p, l)).collect();
        layout::graph_geometry(p, graph, &pds, &FixedMetrics)
    };
    let check = |p: &Project| {
        let gs = geoms(p);
        let r = &p.graph(g).unwrap().layers[0].shapes[0];
        let (whole, zoomed) = (&gs[0], &gs[1]);
        let left = whole.x.from_unit(r.x1);
        let right = whole.x.from_unit(r.x2);
        let top = whole.y.from_unit(1.0 - r.y1);
        let bottom = whole.y.from_unit(1.0 - r.y2);
        assert!((zoomed.x.min - left).abs() < 1e-9 && (zoomed.x.max - right).abs() < 1e-9, "{:?}", zoomed.x);
        assert!((zoomed.y.min - bottom).abs() < 1e-9 && (zoomed.y.max - top).abs() < 1e-9, "{:?}", zoomed.y);
    };
    check(&p);
    // Placed on the resolved axes, the region is centred on the first point (1, 10 sin 0.2), as Origin's.
    place_zoom_regions(&mut p, g);
    let gs = geoms(&p);
    let r = &p.graph(g).unwrap().layers[0].shapes[0];
    let (cx, cy) = (gs[0].x.from_unit((r.x1 + r.x2) / 2.0), gs[0].y.from_unit(1.0 - (r.y1 + r.y2) / 2.0));
    assert!((cx - 1.0).abs() < 1e-9 && (cy - (0.2f64).sin() * 10.0).abs() < 1e-9, "{cx} {cy}");
    check(&p);
    // Moving the rectangle moves the zoom.
    let r = &mut p.graph_mut(g).unwrap().layers[0].shapes[0];
    r.x1 += 0.4;
    r.x2 += 0.4;
    r.y1 += 0.2;
    r.y2 += 0.2;
    check(&p);
}

/// Data > Analysis Markers (T135): Hide removes the markers, Show Partial Ranges keeps only markers that
/// leave rows out, and the size sets the length of the bar at the point (T390: Origin's red full-height lines).
#[test]
fn analysis_markers_hide_partial_and_size() {
    use sheaf_core::data_markers::{MarkerShow, MarkerSize};
    let mut p = Project::new();
    let b = p.add_workbook("");
    p.add_column(b, 0, Designation::X, ColumnData::numeric((0..10).map(f64::from).collect())).unwrap();
    let y = p.add_column(b, 0, Designation::Y, ColumnData::numeric((0..10).map(f64::from).collect())).unwrap();
    let g = p.plot_columns(Template::Line, &[y]).unwrap();
    let count = |p: &Project| layout::build_scene(p, g, &FixedMetrics).unwrap().layers[0].plots.len();
    let plain = count(&p);
    assert!(p.mark_data_range(g, 0, 0));
    let marked = count(&p);
    assert_eq!(marked, plain + 2, "two markers over the whole range");
    p.graph_mut(g).unwrap().set_marker_display(Some(MarkerShow::PartialRanges), None);
    assert_eq!(count(&p), plain, "markers over every row are not a partial range");
    p.graph_mut(g).unwrap().set_data_markers(0, 0, 2, 9);
    assert_eq!(count(&p), plain + 2);
    p.graph_mut(g).unwrap().set_marker_display(Some(MarkerShow::Hide), Some(MarkerSize::Large));
    assert_eq!(count(&p), plain);
    p.graph_mut(g).unwrap().set_marker_display(Some(MarkerShow::All), None);
    let s = layout::build_scene(&p, g, &FixedMetrics).unwrap();
    let beam = s.layers[0].plots.iter().rev().find_map(|it| match it {
        scene::Item::Path { d, fill: None, stroke: Some(st) }
            if st.width == 1.0 && st.color == sheaf_core::Color::rgb(255, 0, 0) =>
        {
            Some(d.split(['M', 'L', ' ']).filter_map(|t| t.parse::<f64>().ok()).collect::<Vec<_>>())
        }
        _ => None,
    });
    let beam = beam.unwrap();
    // M x top L x bottom M x-h y L x+h y: the line spans the frame, the bar is a third of Large's 36 pt.
    assert!(beam[0] == beam[2] && beam[3] - beam[1] > 300.0, "a vertical line across the layer: {beam:?}");
    assert!(((beam[6] - beam[4]) - 12.0).abs() < 1e-9, "Large's bar is 12 pt: {beam:?}");
}

/// Graph > New Table (T136): a table label is a grid whose rectangle is the clickable object.
#[test]
fn a_table_label_is_drawn_as_a_grid_and_picked_inside() {
    let mut p = Project::new();
    let g = p.add_graph(Template::Line);
    let font = p.graph(g).unwrap().layers[0].new_label_font();
    let cells = vec![vec!["a".to_string(), "bb".to_string()], vec!["1".to_string(), String::new()]];
    p.graph_mut(g).unwrap().layers[0].labels.push(sheaf_core::graph::TextLabel::table(&cells, 0.2, 0.2, font));
    let s = layout::build_scene(&p, g, &FixedMetrics).unwrap();
    let texts: Vec<String> = s.layers[0]
        .overlay
        .iter()
        .filter_map(|it| match it {
            scene::Item::Text(t) => Some(t.text.clone()),
            _ => None,
        })
        .collect();
    assert!(texts.contains(&"a".to_string()) && texts.contains(&"bb".to_string()) && texts.contains(&"1".to_string()));
    let obj = s.layers[0].objects.iter().find(|o| o.kind == scene::ObjectKind::Label(0)).unwrap();
    assert!(obj.boxes.iter().any(|b| b.w > 0.0 && b.h > 0.0), "the whole grid counts as the table");
}
