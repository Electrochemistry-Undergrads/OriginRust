//! Stock plots (T108, graphing spec "Stock plots").

use super::*;
use sheaf_core::template::Template;
use sheaf_core::{Color, ColumnData, ColumnId, Designation};

/// Origin's probe data: open, high, low, close over five days.
fn project() -> (Project, Vec<ColumnId>) {
    let mut p = Project::new();
    let b = p.add_workbook("");
    p.add_column(b, 0, Designation::X, ColumnData::numeric(vec![1.0, 2.0, 3.0, 4.0, 5.0])).unwrap();
    let mut ys = Vec::new();
    for v in [
        [10.0, 12.0, 11.0, 13.0, 12.0],
        [13.0, 14.0, 13.0, 15.0, 14.0],
        [9.0, 11.0, 9.0, 12.0, 10.0],
        [12.0, 11.0, 13.0, 12.0, 13.0],
    ] {
        ys.push(p.add_column(b, 0, Designation::Y, ColumnData::numeric(v.to_vec())).unwrap());
    }
    (p, ys)
}

fn geometry(p: &Project, g: WindowId) -> LayerGeometry {
    let graph = p.graph(g).unwrap();
    let pds: Vec<data::LayerData> = graph.layers.iter().map(|l| data::collect(p, l)).collect();
    layout::graph_geometry(p, graph, &pds, &FixedMetrics).remove(0)
}

#[test]
fn the_y_axis_takes_in_the_highs_and_lows() {
    let (mut p, ys) = project();
    let g = p.plot_columns(Template::OhlcBar, &ys).unwrap();
    let geom = geometry(&p, g);
    assert!(geom.y.min <= 9.0 && geom.y.max >= 15.0, "y {}..{}", geom.y.min, geom.y.max);
}

#[test]
fn falling_candles_are_filled_red_and_rising_ones_hollow() {
    let (mut p, ys) = project();
    let g = p.plot_columns(Template::Candlestick, &ys).unwrap();
    let geom = geometry(&p, g);
    let s = layout::build_scene(&p, g, &FixedMetrics).unwrap();
    let bodies: Vec<(Option<Color>, Vec<f64>)> = s.layers[0]
        .plots
        .iter()
        .filter_map(|it| match it {
            scene::Item::Path { d, fill, stroke: Some(_) } if d.ends_with('Z') => {
                Some((*fill, d.split(['M', 'L', ' ', 'Z']).filter_map(|t| t.parse().ok()).collect()))
            }
            _ => None,
        })
        .collect();
    assert_eq!(bodies.len(), 5, "{bodies:?}");
    // Days 2 and 4 close below their open.
    let fills: Vec<bool> = bodies.iter().map(|(f, _)| *f == Some(Color::RED)).collect();
    assert_eq!(fills, [false, true, false, true, false]);
    // A body is 0.4 of the X spacing wide and runs from open to close.
    let spacing = geom.data_to_page(2.0, 0.0).0 - geom.data_to_page(1.0, 0.0).0;
    let d = &bodies[1].1;
    assert!(((d[2] - d[0]) - 0.4 * spacing).abs() < 0.01, "{d:?}");
    let (top, bottom) = (geom.data_to_page(2.0, 12.0).1, geom.data_to_page(2.0, 11.0).1);
    assert!((d[1] - top).abs() < 0.01 && (d[5] - bottom).abs() < 0.01, "{d:?}");
}

#[test]
fn hlc_draws_a_high_low_line_and_a_close_tick_to_the_right() {
    let (mut p, ys) = project();
    let g = p.plot_columns(Template::HighLowClose, &ys[1..]).unwrap();
    let geom = geometry(&p, g);
    let s = layout::build_scene(&p, g, &FixedMetrics).unwrap();
    let lines: Vec<Vec<f64>> = s.layers[0]
        .plots
        .iter()
        .filter_map(|it| match it {
            scene::Item::Path { d, fill: None, stroke: Some(st) } if st.color == Color::BLACK => {
                Some(d.split(['M', 'L', ' ']).filter_map(|t| t.parse().ok()).collect())
            }
            _ => None,
        })
        .collect();
    assert_eq!(lines.len(), 10, "a line and a tick per day: {lines:?}");
    let (x, high) = geom.data_to_page(1.0, 13.0);
    let low = geom.data_to_page(1.0, 9.0).1;
    assert_eq!(lines[0].len(), 4);
    assert!((lines[0][0] - x).abs() < 0.01 && (lines[0][1] - high).abs() < 0.01 && (lines[0][3] - low).abs() < 0.01);
    let close = geom.data_to_page(1.0, 12.0).1;
    assert!((lines[1][0] - x).abs() < 0.01 && lines[1][2] > x && (lines[1][1] - close).abs() < 0.01, "{:?}", lines[1]);
}

#[test]
fn ohlc_volume_draws_candles_over_grey_volume_columns_on_the_same_x() {
    let (mut p, mut ys) = project();
    let b = p.workbooks().next().unwrap().id;
    ys.push(p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![100.0, 250.0, 180.0, 300.0, 120.0])).unwrap());
    let g = p.plot_columns(Template::OhlcVolume, &ys).unwrap();
    let graph = p.graph(g).unwrap();
    let pds: Vec<data::LayerData> = graph.layers.iter().map(|l| data::collect(&p, l)).collect();
    let geoms = layout::graph_geometry(&p, graph, &pds, &FixedMetrics);
    assert_eq!(geoms.len(), 2);
    // The volume layer sits right under the price layer and shares its X scale.
    let (price, volume) = (&geoms[0], &geoms[1]);
    assert!((volume.frame.y - (price.frame.y + price.frame.h)).abs() < 0.01, "{:?} {:?}", price.frame, volume.frame);
    assert_eq!((price.x.min, price.x.max), (volume.x.min, volume.x.max));
    assert!(volume.y.min <= 0.0 && volume.y.max >= 300.0);
    let s = layout::build_scene(&p, g, &FixedMetrics).unwrap();
    let grey = s.layers[1]
        .plots
        .iter()
        .filter(|it| {
            matches!(it, scene::Item::Path { fill: Some(c), .. } if *c == Color::rgb(128, 128, 128))
                || matches!(it, scene::Item::Rect { fill: Some(c), .. } if *c == Color::rgb(128, 128, 128))
        })
        .count();
    assert_eq!(grey, 5, "{:?}", s.layers[1].plots);
}
