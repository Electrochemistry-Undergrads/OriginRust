use super::*;
use crate::scene::{Item, Scene};
use sheaf_core::graph::{PlotKind, SymbolShape};
use sheaf_core::template::Template;
use sheaf_core::{Color, ColumnData, ColumnId, Designation};

fn project() -> (Project, WindowId, ColumnId) {
    let mut p = Project::new();
    let b = p.add_workbook("Data");
    p.add_column(b, 0, Designation::X, ColumnData::numeric(vec![2.0, 6.0, 10.0])).unwrap();
    let y = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![1.0; 3])).unwrap();
    let g = p.add_graph(Template::LineSymbol);
    p.add_plot(g, 0, y).unwrap();
    let layer = &mut p.graph_mut(g).unwrap().layers[0];
    layer.legend.visible = false;
    layer.x_axis.min = Some(0.0);
    layer.x_axis.max = Some(12.0);
    layer.y_axis.min = Some(0.0);
    layer.y_axis.max = Some(2.0);
    let plot = &mut layer.plots[0];
    plot.line.color = Color::RED;
    plot.line.width_pt = 6.0;
    plot.symbol.color = Color::BLUE;
    plot.symbol.shape = SymbolShape::Square;
    plot.symbol.filled = true;
    plot.symbol.size_pt = 20.0;
    (p, g, y)
}

fn front(p: &mut Project, g: WindowId, on: bool) {
    p.graph_mut(g).unwrap().layers[0].plots[0].decor.symbol_line.draw_in_front = on;
}

fn sample(p: &Project, g: WindowId, point: (f64, f64)) -> Vec<u8> {
    let r = render_raster(p, g, 2.0).unwrap();
    let i = (((point.1 * 2.0) as u32 * r.width + (point.0 * 2.0) as u32) * 4) as usize;
    r.rgba[i..i + 4].to_vec()
}

fn order(s: &Scene) -> (usize, usize) {
    let items = &s.layers[0].plots;
    let line = items
        .iter()
        .position(|i| matches!(i, Item::Path { fill:None,stroke:Some(st),.. } if st.color==Color::RED && st.width==6.0))
        .unwrap();
    let symbol = items.iter().position(|i| matches!(i, Item::Path { fill:Some(c),.. } if *c==Color::BLUE)).unwrap();
    (line, symbol)
}

#[test]
fn symbol_line_order_changes_marker_overlap_and_gaps_override_the_saved_front_choice() {
    let (mut p, g, y) = project();
    let geometry = layout_geometry(&p, g).unwrap();
    let middle = geometry[0].data_to_page(6.0, 1.0);
    let behind = render_svg(&p, g).unwrap();
    assert_eq!(sample(&p, g, middle), [0, 0, 255, 255]);
    let (line, symbol) = order(&graph_scene(&p, g).unwrap());
    assert!(line < symbol);
    front(&mut p, g, true);
    assert_eq!(sample(&p, g, middle), [255, 0, 0, 255]);
    let (line, symbol) = order(&graph_scene(&p, g).unwrap());
    assert!(line > symbol);
    assert_eq!(layout_geometry(&p, g).unwrap(), geometry);
    assert_eq!(p.column(y).unwrap().numeric().unwrap(), [1.0; 3]);
    front(&mut p, g, false);
    assert_eq!(render_svg(&p, g).unwrap(), behind);
    p.graph_mut(g).unwrap().layers[0].plots[0].decor.symbol_line.gap_to_symbol = true;
    let gaps = render_svg(&p, g).unwrap();
    front(&mut p, g, true);
    assert_eq!(
        render_svg(&p, g).unwrap(),
        gaps,
        "Gap to Symbol controls the interface while ordering radios are disabled"
    );
    p.graph_mut(g).unwrap().layers[0].plots[0].kind = PlotKind::Line;
    front(&mut p, g, false);
    let line = render_svg(&p, g).unwrap();
    front(&mut p, g, true);
    assert_eq!(render_svg(&p, g).unwrap(), line, "other plot kinds ignore the saved choice");
}

#[test]
fn symbol_line_order_is_present_in_the_shared_scene_and_each_vector_backend() {
    let (mut p, g, _) = project();
    front(&mut p, g, true);
    let svg = render_svg(&p, g).unwrap();
    assert!(svg.find("#0000FF").unwrap() < svg.find("#FF0000").unwrap());
    assert!(render_png(&p, g, 72.0).unwrap().starts_with(b"\x89PNG"));
    assert!(render_pdf(&p, g).unwrap().starts_with(b"%PDF"));
    assert!(render_eps(&p, g).unwrap().starts_with(b"%!PS"));
    assert!(!render_emf(&p, g).unwrap().is_empty());
}

#[test]
fn symbol_line_order_also_changes_the_legend_sample_as_in_origin() {
    // Origin 2016 behind_verified/front_verified: blue versus red at both the data marker
    // centre (468,327) and the legend marker centre (694,42), in 900 px exports.
    let (mut p, g, _) = project();
    p.graph_mut(g).unwrap().layers[0].legend.visible = true;
    let scene = graph_scene(&p, g).unwrap();
    let symbol = scene.layers[0]
        .overlay
        .iter()
        .find_map(|i| match i {
            Item::Path { d, fill: Some(c), .. } if *c == Color::BLUE => Some(d),
            _ => None,
        })
        .unwrap();
    let bounds = hit::path_points(symbol)
        .iter()
        .fold((f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY), |(x0, y0, x1, y1), &(x, y)| {
            (x0.min(x), y0.min(y), x1.max(x), y1.max(y))
        });
    let centre = ((bounds.0 + bounds.2) / 2.0, (bounds.1 + bounds.3) / 2.0);
    assert_eq!(sample(&p, g, centre), [0, 0, 255, 255]);
    front(&mut p, g, true);
    assert_eq!(sample(&p, g, centre), [255, 0, 0, 255]);
    p.graph_mut(g).unwrap().layers[0].plots[0].decor.symbol_line.gap_to_symbol = true;
    assert_eq!(sample(&p, g, centre), [0, 0, 255, 255], "gaps override the inactive front choice in the sample too");
}
