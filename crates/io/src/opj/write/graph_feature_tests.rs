//! Round-trip tests of the graph features that the file holds since spec section 13.12: stacked
//! curve offsets, the right Y axis of a double-Y graph, bar gaps, axis sides and automatic axis
//! colours. Origin showed each of them the way the model draws them (spec 13.12).

use super::*;
use crate::opj::frame::Frames;
use crate::opj::{OpjImport, dataset, read_opj_from, window};
use sheaf_core::color::Color;
use sheaf_core::graph::*;
use sheaf_core::ids::{ColumnId, WindowId};
use sheaf_core::stack::places;
use sheaf_core::template::Template;
use std::io::Cursor;
use std::path::Path;

const TIME: f64 = 2_461_000.5;

fn source() -> Project {
    let x: Vec<f64> = (0..21).map(|i| f64::from(i) * 0.5).collect();
    let nums = |v: Vec<f64>| ColumnData::numeric(v);
    let mut s = Worksheet::new("Sheet1");
    s.columns = vec![
        Column::new(ColumnId(0), "A", Designation::X, nums(x.clone())).with_labels("Time", "s", ""),
        Column::new(ColumnId(0), "B", Designation::Y, nums(x.iter().map(|v| v * v).collect())),
        Column::new(ColumnId(0), "C", Designation::Y, nums(x.iter().map(|v| 100.0 - 5.0 * v).collect())),
        Column::new(ColumnId(0), "D", Designation::Y, nums(x.iter().map(|v| v.sin() * 0.05).collect()))
            .with_labels("Wave", "A", ""),
        Column::new(ColumnId(0), "E", Designation::Y, nums(x.iter().map(|v| 5.0 + v.cos()).collect())),
        Column::new(ColumnId(0), "F", Designation::Y, nums(vec![3.0, 5.0, 2.0, 6.0, 4.0, 7.0])),
        Column::new(ColumnId(0), "G", Designation::Y, nums(vec![4.0, 2.0, 5.0, 3.0, 6.0, 2.0])),
    ];
    let mut b = Workbook::new(WindowId(0), "Src");
    b.sheets = vec![s];
    let mut p = Project::new();
    p.adopt_named_workbook(b);
    p
}

fn ids(p: &Project) -> Vec<ColumnId> {
    p.workbooks().next().unwrap().sheets[0].columns.iter().map(|c| c.id).collect()
}

fn round_trip(p: &Project) -> (OpjImport, OpjWrite) {
    let out = write_named(p, "g", TIME).unwrap();
    (read_opj_from(Cursor::new(out.bytes.clone()), Path::new("g.opj")).unwrap(), out)
}

fn stacked(mode: Stack) -> Project {
    let mut p = source();
    let c = ids(&p);
    let id = p.add_graph(Template::StackedLines);
    for y in [1, 2, 3, 4] {
        p.add_plot_xy(id, 0, Some(c[0]), c[y], None).unwrap();
    }
    let l = &mut p.graph_mut(id).unwrap().layers[0];
    l.stack = mode;
    if mode == Stack::Individual {
        for (k, y) in [1, 2, 3, 4].into_iter().enumerate() {
            l.plot_offset_mut(c[y]).y = 30.0 * k as f64 + 0.5;
        }
    }
    p
}

#[test]
fn stacked_curves_keep_the_offsets_they_are_drawn_with_in_every_mode() {
    for mode in [Stack::Auto { gap_percent: 10.0 }, Stack::Constant { step: 40.0 }, Stack::Individual] {
        let p = stacked(mode);
        let layer = &p.graphs().next().unwrap().layers[0];
        let expected: Vec<f64> = places(&p, layer).0.iter().map(|pl| pl.oy).collect();
        assert!(expected.iter().any(|o| *o != 0.0), "{mode:?}");
        let (imp, out) = round_trip(&p);
        assert!(out.warnings.is_empty(), "{mode:?}: {:?}", out.warnings);
        let read = &imp.graphs[0].layers[0];
        assert_eq!(read.stack, Stack::Individual, "{mode:?}");
        let got: Vec<f64> = read.plots.iter().map(|pl| read.plot_offset(pl.y).map_or(f64::NAN, |o| o.y)).collect();
        for (g, e) in got.iter().zip(&expected) {
            assert!((g - e).abs() < 1e-9, "{mode:?}: {got:?} against {expected:?}");
        }
        // the Y range written holds the moved curves
        let top =
            layer.plots.iter().zip(&expected).fold(f64::MIN, |m, (pl, o)| {
                m.max(p.numeric(pl.y).unwrap().iter().copied().fold(f64::MIN, f64::max) + o)
            });
        assert!(read.y_axis.max.unwrap() >= top, "{mode:?}: {:?} below {top}", read.y_axis.max);
    }
}

#[test]
fn x_offsets_of_stacked_curves_are_written_and_read_back() {
    let mut p = stacked(Stack::Individual);
    let c = ids(&p);
    let id = p.graphs().next().unwrap().id;
    {
        let l = &mut p.graph_mut(id).unwrap().layers[0];
        l.plot_offset_mut(c[2]).x = 1.5;
        l.plot_offset_mut(c[3]).x = -0.5;
    }
    let (imp, out) = round_trip(&p);
    assert!(!out.warnings.iter().any(|w| w.contains("X offset")), "{:?}", out.warnings);
    let read = &imp.graphs[0].layers[0];
    let got: Vec<(f64, f64)> =
        read.plots.iter().map(|pl| read.plot_offset(pl.y).map_or((f64::NAN, f64::NAN), |o| (o.x, o.y))).collect();
    assert_eq!(got.iter().map(|o| o.0).collect::<Vec<_>>(), vec![0.0, 1.5, -0.5, 0.0]);
    assert_eq!(got.iter().map(|o| o.1).collect::<Vec<_>>(), vec![0.5, 30.5, 60.5, 90.5]);
}

#[test]
fn a_waterfall_is_written_as_offsets_that_put_the_curves_where_sheaf_draws_them() {
    let (imp, out) = round_trip(&stacked(Stack::Waterfall { dx_percent: 5.0, dy_percent: 10.0 }));
    assert!(out.warnings.iter().any(|w| w.contains("waterfall")), "{:?}", out.warnings);
    let read = &imp.graphs[0].layers[0];
    assert_eq!(read.stack, Stack::Individual);
    let at = |f: fn(&PlotOffset) -> f64| -> Vec<f64> {
        read.plots.iter().map(|pl| read.plot_offset(pl.y).map_or(f64::NAN, f)).collect()
    };
    let (xs, ys) = (at(|o| o.x), at(|o| o.y));
    assert_eq!(xs.len(), 4);
    assert_eq!(xs[0], 0.0);
    assert_eq!(ys[0], 0.0);
    // member k is moved by k times a fixed step, and the axes reach the last curve
    for k in 1..4 {
        assert!((xs[k] - k as f64 * xs[1]).abs() < 1e-9 && xs[1] > 0.0, "{xs:?}");
        assert!((ys[k] - k as f64 * ys[1]).abs() < 1e-9 && ys[1] > 0.0, "{ys:?}");
    }
    assert!(read.y_axis.max.unwrap() >= ys[3]);

    let (imp, out) = round_trip(&stacked(Stack::None));
    assert!(out.warnings.is_empty(), "{:?}", out.warnings);
    assert_eq!(imp.graphs[0].layers[0].stack, Stack::None);
}

#[test]
fn a_cumulative_stack_is_written_as_plain_curves_with_a_warning() {
    let (imp, out) = round_trip(&stacked(Stack::Cumulative));
    assert!(out.warnings.iter().any(|w| w.contains("cumulative")), "{:?}", out.warnings);
    assert_eq!(imp.graphs[0].layers[0].stack, Stack::None);
}

#[test]
fn a_colour_mapped_waterfall_warns_that_the_mapping_was_not_written() {
    let mut p = stacked(Stack::Waterfall { dx_percent: 5.0, dy_percent: 10.0 });
    let id = p.graphs().next().unwrap().id;
    p.graph_mut(id).unwrap().layers[0].waterfall_color = sheaf_core::graph::WaterfallColor::Y;
    let (_, out) = round_trip(&p);
    assert!(out.warnings.iter().any(|w| w.contains("colour mapping")), "{:?}", out.warnings);
}

fn double_y() -> Project {
    let mut p = source();
    let c = ids(&p);
    let g = p.plot_columns_double_y(Template::DoubleY, &[c[1], c[2]], &[c[3], c[4]]).unwrap();
    let graph = p.graph_mut(g).unwrap();
    graph.layers[1].y_axis.title = Some("Wave (A)".into());
    graph.layers[0].plots[0].label = Some("Squares".into());
    p
}

#[test]
fn a_double_y_graph_is_written_as_linked_layers_and_comes_back_the_same() {
    let p = double_y();
    let before = p.graphs().next().unwrap().clone();
    let (imp, out) = round_trip(&p);
    assert!(out.warnings.is_empty(), "{:?}", out.warnings);
    assert!(imp.warnings.is_empty(), "{:?}", imp.warnings);
    let g = &imp.graphs[0];
    assert_eq!(g.layers.len(), 2);
    let (left, right) = (&g.layers[0], &g.layers[1]);
    assert_eq!(right.link, Some(LayerLink { parent: 0, x: ScaleLink::Straight, y: ScaleLink::None }));
    assert_eq!(g.right_y_member(0), Some(1));
    assert_eq!(right.frame, Frame::WHOLE);
    assert_eq!(right.y_axis.side, AxisSide::Opposite);
    assert_eq!(right.x_axis.side, AxisSide::Hidden);
    assert_eq!(left.y_axis.side, AxisSide::Normal);
    // titles: the left one is made from the columns, the right one was typed
    assert_eq!(right.y_axis.title.as_deref(), Some("Wave (A)"));
    assert!(right.y_axis.show_title && left.y_axis.show_title);
    // the box: the top line is the right layer's, the parent has no right line of its own
    assert_eq!(left.x_axis.show_opposite, before.layers[0].x_axis.show_opposite);
    assert!(!left.y_axis.show_opposite);
    // every Y axis follows the colour of its plot, as in the template
    assert!(left.y_axis.color_follows_plot && right.y_axis.color_follows_plot);
    // one legend for both layers
    assert!(left.legend.visible && left.legend.scope == LegendScope::Graph && !right.legend.visible);
    // both layers show the same X range, each its own Y range
    assert_eq!((left.x_axis.min, left.x_axis.max), (right.x_axis.min, right.x_axis.max));
    assert_ne!((left.y_axis.min, left.y_axis.max), (right.y_axis.min, right.y_axis.max));
    assert_eq!((left.plots.len(), right.plots.len()), (2, 2));
    assert!(right.y_axis.max.unwrap() < 10.0 && left.y_axis.max.unwrap() > 50.0);
}

#[test]
fn the_legend_of_a_double_y_graph_lists_the_plots_of_both_layers() {
    let (_, out) = round_trip(&double_y());
    let has = |needle: &[u8]| out.bytes.windows(needle.len()).any(|w| w == needle);
    assert!(has(b"\\l(1.1) Squares"), "the custom label of the first plot");
    for needle in [&b"\\l(1.2) %(1.2)"[..], b"\\l(2.1) %(2.1)", b"\\l(2.2) %(2.2)"] {
        assert!(has(needle), "{}", String::from_utf8_lossy(needle));
    }
}

#[test]
fn a_right_layer_with_its_own_x_scale_and_other_links_are_handled_apart() {
    // a link whose X is not straight is still a right layer in the file; the model comes back linked
    let mut p = double_y();
    let id = p.graphs().next().unwrap().id;
    p.graph_mut(id).unwrap().layers[1].link = Some(LayerLink { parent: 0, x: ScaleLink::None, y: ScaleLink::None });
    let (imp, out) = round_trip(&p);
    assert!(out.warnings.iter().any(|w| w.contains("not linked to its parent")), "{:?}", out.warnings);
    assert_eq!(imp.graphs[0].layers.len(), 2);
}

#[test]
fn the_gap_of_the_bars_comes_back_and_the_overlap_is_named() {
    for gap in [0.0, 20.0, 35.0, 60.0, 100.0] {
        let mut p = source();
        let c = ids(&p);
        let id = p.add_graph(Template::Column);
        p.graph_mut(id).unwrap().layers[0].legend.position = LegendPosition::TopRight;
        p.graph_mut(id).unwrap().layers[0].legend.background = None;
        for y in [5, 6] {
            p.add_plot_xy(id, 0, None, c[y], None).unwrap();
        }
        p.graph_mut(id).unwrap().layers[0].bars = BarSpacing { gap_pct: gap, overlap_pct: 0.0, width_pct: None };
        let (imp, out) = round_trip(&p);
        assert!(out.warnings.is_empty(), "gap {gap}: {:?}", out.warnings);
        assert!((imp.graphs[0].layers[0].bars.gap_pct - gap).abs() < 0.02, "gap {gap}");
    }
    let mut p = source();
    let c = ids(&p);
    let id = p.add_graph(Template::Column);
    p.graph_mut(id).unwrap().layers[0].legend.position = LegendPosition::TopRight;
    p.graph_mut(id).unwrap().layers[0].legend.background = None;
    p.add_plot_xy(id, 0, None, c[5], None).unwrap();
    p.graph_mut(id).unwrap().layers[0].bars = BarSpacing { gap_pct: 30.0, overlap_pct: 40.0, width_pct: None };
    let (imp, out) = round_trip(&p);
    assert!(out.warnings.iter().any(|w| w.contains("overlap")), "{:?}", out.warnings);
    assert!(!out.warnings.iter().any(|w| w.contains("gap")), "{:?}", out.warnings);
    assert!((imp.graphs[0].layers[0].bars.gap_pct - 30.0).abs() < 0.02);
}

fn one_layer(f: impl FnOnce(&mut Layer)) -> Project {
    let mut p = source();
    let c = ids(&p);
    let id = p.add_graph(Template::Line);
    p.add_plot_xy(id, 0, Some(c[0]), c[1], None).unwrap();
    // These feature fixtures use the original fixed-position legend. Automatic retention is
    // checked separately and intentionally produces a companion warning.
    let layer = &mut p.graph_mut(id).unwrap().layers[0];
    layer.legend.position = LegendPosition::TopRight;
    layer.legend.background = None;
    f(layer);
    p
}

#[test]
fn the_display_frame_round_trips_independently_of_opposite_axis_lines() {
    let (imp, out) = round_trip(&one_layer(|l| {
        l.display.show_frame = true;
        l.x_axis.show_opposite = true;
    }));
    assert!(out.warnings.is_empty(), "{:?}", out.warnings);
    let layer = &imp.graphs[0].layers[0];
    assert!(layer.display.show_frame);
    assert!(layer.x_axis.show_opposite);
    assert!(!layer.y_axis.show_opposite);
}

#[test]
fn axes_on_the_opposite_side_or_hidden_come_back() {
    let (imp, out) = round_trip(&one_layer(|l| {
        l.x_axis.side = AxisSide::Opposite;
        l.x_axis.title = Some("Top".into());
        l.y_axis.side = AxisSide::Opposite;
        l.y_axis.title = Some("Right".into());
        l.y_axis.show_tick_labels = false;
        l.y_axis.major_tick = TickDirection::In;
        l.y_axis.line_width_pt = 2.0;
    }));
    assert!(out.warnings.is_empty(), "{:?}", out.warnings);
    let l = &imp.graphs[0].layers[0];
    assert_eq!((l.x_axis.side, l.y_axis.side), (AxisSide::Opposite, AxisSide::Opposite));
    assert_eq!((l.x_axis.title.as_deref(), l.y_axis.title.as_deref()), (Some("Top"), Some("Right")));
    assert!(l.x_axis.show_tick_labels && !l.y_axis.show_tick_labels);
    assert_eq!(l.y_axis.major_tick, TickDirection::In);
    assert!((l.y_axis.line_width_pt - 2.0).abs() < 1e-9);

    let (imp, out) = round_trip(&one_layer(|l| {
        l.x_axis.side = AxisSide::Hidden;
        l.y_axis.side = AxisSide::Hidden;
    }));
    assert!(out.warnings.is_empty(), "{:?}", out.warnings);
    let l = &imp.graphs[0].layers[0];
    assert_eq!((l.x_axis.side, l.y_axis.side), (AxisSide::Hidden, AxisSide::Hidden));

    // an ordinary axis stays on its edge, with or without a box and with its labels off
    let (imp, _) = round_trip(&one_layer(|l| {
        l.x_axis.show_opposite = true;
        l.y_axis.show_tick_labels = false;
    }));
    let l = &imp.graphs[0].layers[0];
    assert_eq!((l.x_axis.side, l.y_axis.side), (AxisSide::Normal, AxisSide::Normal));
    assert!(l.x_axis.show_opposite && !l.y_axis.show_opposite && !l.y_axis.show_tick_labels);
}

#[test]
fn axis_colours_that_follow_the_plot_come_back_and_title_colours_are_named() {
    let (imp, out) = round_trip(&one_layer(|l| l.y_axis.color_follows_plot = true));
    assert!(out.warnings.is_empty(), "{:?}", out.warnings);
    let l = &imp.graphs[0].layers[0];
    assert!(l.y_axis.color_follows_plot && !l.x_axis.color_follows_plot);

    let (_, out) = round_trip(&one_layer(|l| l.y_axis.title_color_follows_plot = true));
    assert!(out.warnings.iter().any(|w| w.contains("title colours")), "{:?}", out.warnings);
}

#[test]
fn tick_label_number_formats_are_written_and_read_back() {
    let formats = [
        (TickFormat::Auto, TickFormat::Auto),
        (TickFormat::Decimal(0), TickFormat::Decimal(0)),
        (TickFormat::Decimal(3), TickFormat::Decimal(3)),
        (TickFormat::Scientific(2), TickFormat::Scientific(2)),
        (TickFormat::Decimal(40), TickFormat::Decimal(15)),
    ];
    for (put, want) in formats {
        let mut p = source();
        let c = ids(&p);
        let id = p.add_graph(Template::Line);
        p.graph_mut(id).unwrap().layers[0].legend.position = LegendPosition::TopRight;
        p.graph_mut(id).unwrap().layers[0].legend.background = None;
        p.add_plot_xy(id, 0, Some(c[0]), c[1], None).unwrap();
        let l = &mut p.graph_mut(id).unwrap().layers[0];
        l.x_axis.tick_format = put;
        l.y_axis.tick_format = TickFormat::Scientific(1);
        let (imp, out) = round_trip(&p);
        assert!(out.warnings.is_empty(), "{put:?}: {:?}", out.warnings);
        let l = &imp.graphs[0].layers[0];
        assert_eq!(l.x_axis.tick_format, want, "{put:?}");
        assert_eq!(l.y_axis.tick_format, TickFormat::Scientific(1));
    }
}

#[test]
fn a_number_format_on_a_column_plot_comes_back() {
    let mut p = source();
    let c = ids(&p);
    let id = p.add_graph(Template::Column);
    p.graph_mut(id).unwrap().layers[0].legend.position = LegendPosition::TopRight;
    p.graph_mut(id).unwrap().layers[0].legend.background = None;
    p.add_plot_xy(id, 0, None, c[5], None).unwrap();
    p.graph_mut(id).unwrap().layers[0].y_axis.tick_format = TickFormat::Decimal(2);
    let (imp, out) = round_trip(&p);
    assert!(out.warnings.is_empty(), "{:?}", out.warnings);
    assert_eq!(imp.graphs[0].layers[0].y_axis.tick_format, TickFormat::Decimal(2));
}

/// The plot records of every graph in a written project, in file order.
fn curve_records(bytes: &[u8]) -> Vec<Vec<u8>> {
    let mut f = Frames::new(Cursor::new(bytes.to_vec())).unwrap();
    f.line(200).unwrap();
    f.block().unwrap();
    f.block().unwrap();
    let _ = dataset::scan(&mut f);
    let (windows, _) = window::scan(&mut f);
    windows
        .iter()
        .flat_map(|w| w.layers.iter().flat_map(|l| l.columns.iter().map(|c| c.record.clone())))
        .filter(|r| r.len() == 515 && (200..=204).contains(&u16::from_le_bytes([r[0x4c], r[0x4d]])))
        .collect()
}

fn columns_graph(ys: &[usize], f: impl FnOnce(&mut Layer)) -> Project {
    let mut p = source();
    let c = ids(&p);
    let id = p.add_graph(Template::Column);
    for y in ys {
        p.add_plot_xy(id, 0, None, c[*y], None).unwrap();
    }
    // These feature fixtures use the original fixed-position legend. Automatic retention is
    // checked separately and intentionally produces a companion warning.
    let layer = &mut p.graph_mut(id).unwrap().layers[0];
    layer.legend.position = LegendPosition::TopRight;
    layer.legend.background = None;
    f(layer);
    p
}

#[test]
fn column_plots_of_a_layer_are_written_as_a_plot_group() {
    // the group byte: 0x29 for the first member, 0x19 for the others, 0x09 for a plot outside any group
    let flags = |p: &Project| -> Vec<u8> {
        curve_records(&write_named(p, "g", TIME).unwrap().bytes).iter().map(|r| r[0x06]).collect()
    };
    assert_eq!(flags(&columns_graph(&[5, 6], |_| {})), [0x29, 0x19]);
    assert_eq!(flags(&columns_graph(&[5, 6, 5], |_| {})), [0x29, 0x19, 0x19]);
    assert_eq!(flags(&columns_graph(&[5], |_| {})), [0x09], "one column plot is not a group");
    // a line next to two column plots stays outside the group
    let mut p = columns_graph(&[5, 6], |_| {});
    let c = ids(&p);
    let id = p.graphs().next().unwrap().id;
    p.add_plot_xy(id, 0, None, c[1], None).unwrap();
    p.graph_mut(id).unwrap().layers[0].plots[2].kind = PlotKind::Line;
    assert_eq!(flags(&p), [0x29, 0x19, 0x09]);
}

#[test]
fn colours_that_a_plot_group_would_replace_are_named() {
    let (_, out) = round_trip(&columns_graph(&[5, 6], |_| {}));
    assert!(out.warnings.is_empty(), "{:?}", out.warnings);
    let (imp, out) = round_trip(&columns_graph(&[5, 6], |l| {
        l.plots[1].symbol.color = Color::rgb(12, 34, 56);
        l.plots[1].auto_color = false;
    }));
    assert!(out.warnings.iter().any(|w| w.contains("colours of plots in a plot group")), "{:?}", out.warnings);
    assert_eq!(imp.graphs[0].layers[0].plots.len(), 2);
}

fn lines_graph(ys: &[usize], f: impl FnOnce(&mut Layer)) -> Project {
    let mut p = source();
    let c = ids(&p);
    let id = p.add_graph(Template::Line);
    for y in ys {
        p.add_plot_xy(id, 0, None, c[*y], None).unwrap();
    }
    // These feature fixtures use the original fixed-position legend. Automatic retention is
    // checked separately and intentionally produces a companion warning.
    let layer = &mut p.graph_mut(id).unwrap().layers[0];
    layer.legend.position = LegendPosition::TopRight;
    layer.legend.background = None;
    f(layer);
    p
}

#[test]
fn plots_with_a_group_number_are_written_as_a_group_and_read_back() {
    let flags = |p: &Project| -> Vec<u8> {
        curve_records(&write_named(p, "g", TIME).unwrap().bytes).iter().map(|r| r[0x06]).collect()
    };
    let grouped = |ids: [Option<u32>; 4]| {
        lines_graph(&[5, 6, 5, 6], |l| {
            for (p, g) in l.plots.iter_mut().zip(ids) {
                p.group = g;
            }
        })
    };
    // two groups, side by side in the file
    let p = grouped([Some(1), Some(1), Some(2), Some(2)]);
    assert_eq!(flags(&p), [0x29, 0x19, 0x29, 0x19]);
    let (imp, out) = round_trip(&p);
    assert!(out.warnings.is_empty(), "{:?}", out.warnings);
    let plots = &imp.graphs[0].layers[0].plots;
    assert_eq!(plots.iter().map(|p| p.group.is_some()).collect::<Vec<_>>(), [true; 4]);
    assert_eq!((plots[0].group, plots[2].group), (plots[1].group, plots[3].group));
    assert_ne!(plots[0].group, plots[2].group);
    // members that are not next to each other, and a group of one, are no group in the file
    assert_eq!(flags(&grouped([Some(1), None, Some(1), None])), [0x09; 4]);
    assert_eq!(flags(&grouped([Some(1), None, None, None])), [0x09; 4]);
    // no group, no flag
    assert_eq!(flags(&grouped([None; 4])), [0x09; 4]);
}

#[test]
fn a_group_of_lines_names_hand_chosen_colours() {
    let (_, out) = round_trip(&lines_graph(&[5, 6], |l| {
        l.plots[0].group = Some(1);
        l.plots[1].group = Some(1);
        l.plots[1].line.color = Color::rgb(12, 34, 56);
        l.plots[1].auto_color = false;
    }));
    assert!(out.warnings.iter().any(|w| w.contains("colours of plots in a plot group")), "{:?}", out.warnings);
}

#[test]
fn the_legend_frame_survives_a_save() {
    for frame in [true, false] {
        let (imp, out) = round_trip(&lines_graph(&[5], |l| l.legend.frame = frame));
        assert!(out.warnings.is_empty(), "{:?}", out.warnings);
        assert_eq!(imp.graphs[0].layers[0].legend.frame, frame);
    }
}

#[test]
fn error_bars_on_grouped_columns_are_named_and_still_read_back() {
    let mut p = columns_graph(&[5, 6], |_| {});
    let c = ids(&p);
    let id = p.graphs().next().unwrap().id;
    p.graph_mut(id).unwrap().layers[0].plots[0].y_error = Some(c[3]);
    let (imp, out) = round_trip(&p);
    assert!(out.warnings.iter().any(|w| w.contains("error bars on column plots")), "{:?}", out.warnings);
    let l = &imp.graphs[0].layers[0];
    assert_eq!(l.plots.len(), 2);
    assert!(l.plots[0].y_error.is_some() && l.plots[1].y_error.is_none());
}

/// A graph of `n` panels (layer 0 and layers linked to it), each with a plot, in the frames of a
/// vertical stack: fractions of the first panel's frame, the panels above going up by one height.
fn panels(n: usize, x: ScaleLink, y: ScaleLink) -> Project {
    let mut p = source();
    let c = ids(&p);
    let id = p.add_graph(Template::Line);
    p.add_plot_xy(id, 0, Some(c[0]), c[1], None).unwrap();
    p.graph_mut(id).unwrap().layers[0].frame = Frame { left: 0.15, top: 0.65, width: 0.75, height: 0.25 };
    for k in 1..n {
        let mut panel = p.graph(id).unwrap().layers[0].clone();
        panel.plots.clear();
        panel.frame = Frame { left: 0.0, top: -1.2 * k as f64, width: 1.0, height: 1.0 };
        panel.link = Some(LayerLink { parent: 0, x: x.clone(), y: y.clone() });
        p.graph_mut(id).unwrap().layers.push(panel);
        p.add_plot_xy(id, k, Some(c[0]), c[1 + k], None).unwrap();
    }
    p
}

#[test]
fn panels_keep_their_link_their_frames_and_their_linked_scales() {
    for (x, y) in [
        (ScaleLink::Straight, ScaleLink::None),
        (ScaleLink::None, ScaleLink::Straight),
        (ScaleLink::Straight, ScaleLink::Straight),
        (ScaleLink::None, ScaleLink::None),
    ] {
        let mut p = panels(3, x.clone(), y.clone());
        let graph_ids: Vec<_> = p.graphs().map(|g| g.id).collect();
        for id in graph_ids {
            for layer in &mut p.graph_mut(id).unwrap().layers {
                layer.legend.position = LegendPosition::TopRight;
                layer.legend.background = None;
            }
        }
        let (imp, out) = round_trip(&p);
        assert!(out.warnings.is_empty(), "{x:?} {y:?}: {:?}", out.warnings);
        let want = &p.graphs().next().unwrap().layers;
        let got = &imp.graphs[0].layers;
        assert_eq!(got.len(), 3);
        assert_eq!(got[0].link, None);
        for k in 1..3 {
            assert_eq!(got[k].link, Some(LayerLink { parent: 0, x: x.clone(), y: y.clone() }), "{x:?} {y:?} layer {k}");
            let (g, w) = (got[k].frame, want[k].frame);
            assert!(
                (g.left - w.left).abs() < 0.01
                    && (g.top - w.top).abs() < 0.01
                    && (g.width - w.width).abs() < 0.01
                    && (g.height - w.height).abs() < 0.01,
                "{g:?} against {w:?}"
            );
        }
    }
}

#[test]
fn linked_scales_cover_the_data_of_every_panel() {
    let (imp, _) = round_trip(&panels(3, ScaleLink::Straight, ScaleLink::Straight));
    let l = &imp.graphs[0].layers;
    // B is 0 to 100, C is 100 to 0 and D is about -0.05 to 0.05: the Y scale of the stack holds all of them
    for layer in l {
        assert_eq!((layer.y_axis.min, layer.y_axis.max), (l[0].y_axis.min, l[0].y_axis.max));
    }
    assert!(l[0].y_axis.min.unwrap() <= 0.0 && l[0].y_axis.max.unwrap() >= 100.0);
    for layer in l {
        assert_eq!((layer.x_axis.min, layer.x_axis.max), (l[0].x_axis.min, l[0].x_axis.max));
    }
}

#[test]
fn a_panel_header_names_its_parent_and_the_unit_percent_of_the_linked_layer() {
    let headers = |p: &Project| -> Vec<Vec<u8>> {
        let mut f = Frames::new(Cursor::new(write_named(p, "g", TIME).unwrap().bytes)).unwrap();
        f.line(200).unwrap();
        f.block().unwrap();
        f.block().unwrap();
        let _ = dataset::scan(&mut f);
        let (windows, _) = window::scan(&mut f);
        windows.iter().filter(|w| w.name != "Src").flat_map(|w| w.layers.iter().map(|l| l.header.clone())).collect()
    };
    let h = headers(&panels(3, ScaleLink::Straight, ScaleLink::None));
    assert_eq!(h.len(), 3);
    assert_eq!([h[0][0xAB], h[1][0xAB], h[2][0xAB]], [0, 1, 1]);
    assert_eq!([h[0][0x67], h[1][0x67], h[2][0x67]], [0, 6, 6]);
    // the right Y layer of a double-Y graph keeps its page-percent unit
    let h = headers(&double_y());
    assert_eq!([h[1][0xAB], h[1][0x67]], [1, 0]);
}

#[test]
fn the_legend_text_and_its_columns_are_handled() {
    let mut p = double_y();
    let id = p.graphs().next().unwrap().id;
    p.graph_mut(id).unwrap().layers[0].legend.text = Some("\\l(1.1) T = 5 K\n\\l(2.1) %(2.1)".into());
    let (imp, out) = round_trip(&p);
    let has = |needle: &[u8]| out.bytes.windows(needle.len()).any(|w| w == needle);
    assert!(has(b"\\l(1.1) T = 5 K\r\n\\l(2.1) %(2.1)"), "the custom text is written as it is");
    assert!(!has(b"\\l(1.2)"), "and not the automatic one");
    assert_eq!(imp.graphs[0].layers[0].legend.text.as_deref(), Some("\\l(1.1) T = 5 K\n\\l(2.1) %(2.1)"));
    assert!(!out.warnings.iter().any(|w| w.contains("legend")), "{:?}", out.warnings);

    // The number of columns has no place in the file: it is named, so the companion keeps it.
    p.graph_mut(id).unwrap().layers[0].legend.columns = 2;
    let (_, out) = round_trip(&p);
    assert!(out.warnings.iter().any(|w| w.contains("columns of the legend")), "{:?}", out.warnings);
}

fn line_graph(p: &mut Project, f: impl FnOnce(&mut Layer)) {
    let c = ids(p);
    let id = p.add_graph(Template::Line);
    p.add_plot_xy(id, 0, Some(c[0]), c[1], None).unwrap();
    let layer = &mut p.graph_mut(id).unwrap().layers[0];
    layer.legend.position = LegendPosition::TopRight;
    layer.legend.background = None;
    f(layer);
}

#[test]
fn date_tick_labels_are_written_as_origins_date_type() {
    use sheaf_core::datetime::DatePattern;
    let mut p = source();
    line_graph(&mut p, |l| l.x_axis.tick_format = TickFormat::DateTime(DatePattern::Mdy));
    let (imp, out) = round_trip(&p);
    assert!(out.warnings.is_empty(), "{:?}", out.warnings);
    let l = &imp.graphs[0].layers[0];
    assert_eq!(l.x_axis.tick_format, TickFormat::DateTime(DatePattern::Mdy));
    assert_eq!(l.y_axis.tick_format, TickFormat::Auto);

    // a pattern that Origin's list lacks is written as month/day/year and named
    let mut p = source();
    line_graph(&mut p, |l| l.y_axis.tick_format = TickFormat::DateTime(DatePattern::Iso));
    let (imp, out) = round_trip(&p);
    assert!(out.warnings.iter().any(|w| w.contains("yyyy-MM-dd")), "{:?}", out.warnings);
    assert_eq!(imp.graphs[0].layers[0].y_axis.tick_format, TickFormat::DateTime(DatePattern::Mdy));
}

/// The window headers of a written project, in file order.
fn window_headers(bytes: &[u8]) -> Vec<Vec<u8>> {
    let mut f = Frames::new(Cursor::new(bytes.to_vec())).unwrap();
    f.line(200).unwrap();
    f.block().unwrap();
    f.block().unwrap();
    let _ = dataset::scan(&mut f);
    window::scan(&mut f).0.into_iter().map(|w| w.header).collect()
}

#[test]
fn window_places_are_written_as_the_window_rectangle() {
    use sheaf_core::project::{WindowPlace, WindowState};
    let mut p = stacked(Stack::None);
    let ids: Vec<WindowId> = p.windows.iter().map(|w| w.id()).collect();
    assert_eq!(ids.len(), 2);
    let rect =
        |h: &[u8]| -> Vec<i16> { (0..4).map(|i| i16::from_le_bytes([h[0x1B + 2 * i], h[0x1C + 2 * i]])).collect() };
    // unknown places: the cascade
    let before = window_headers(&write_named(&p, "g", TIME).unwrap().bytes);
    assert_eq!(rect(&before[1]), vec![72, 72, 579, 421]);
    p.window_places = vec![
        WindowPlace { window: ids[0], x: 30.4, y: 40.0, width: 600.0, height: 300.0, state: WindowState::Normal },
        WindowPlace { window: ids[1], x: 100.0, y: 120.0, width: 500.0, height: 400.0, state: WindowState::Maximised },
    ];
    let after = window_headers(&write_named(&p, "g", TIME).unwrap().bytes);
    assert_eq!(rect(&after[0]), vec![30, 40, 630, 340]);
    assert_eq!(rect(&after[1]), vec![100, 120, 600, 520]);
    assert_eq!(after[0][0x32] & 3, 0);
    assert_eq!(after[1][0x32] & 3, 2);
}

#[test]
fn a_polar_layer_is_written_as_a_line_plot_and_named() {
    let p = one_layer(|l| l.coords = Coords::Polar(PolarCoords::default()));
    let (imp, out) = round_trip(&p);
    assert!(out.warnings.iter().any(|w| w.contains("polar")), "{:?}", out.warnings);
    assert_eq!(imp.graphs[0].layers[0].plots[0].kind, PlotKind::Line);
    assert!(imp.graphs[0].layers[0].coords.is_cartesian());
}

#[test]
fn a_radar_layer_is_written_as_a_line_symbol_plot_and_named() {
    let p = one_layer(|l| l.coords = Coords::Polar(PolarCoords { radar: true, ..PolarCoords::default() }));
    let (imp, out) = round_trip(&p);
    assert!(out.warnings.iter().any(|w| w.contains("radar")), "{:?}", out.warnings);
    assert!(imp.graphs[0].layers[0].coords.is_cartesian());
}

#[test]
fn a_ternary_layer_is_written_as_a_scatter_plot_and_named() {
    let p = one_layer(|l| l.coords = Coords::Ternary(TernaryCoords::default()));
    let (imp, out) = round_trip(&p);
    assert!(out.warnings.iter().any(|w| w.contains("ternary")), "{:?}", out.warnings);
    assert!(imp.graphs[0].layers[0].coords.is_cartesian());
}

#[test]
fn plot_decorations_and_a_layer_background_are_named_in_the_save_warnings() {
    let mut p = source();
    line_graph(&mut p, |l| {
        l.background = Some(sheaf_core::Color::WHITE);
        l.plots[0].decor.drop.vertical.on = true;
    });
    let (_, out) = round_trip(&p);
    assert!(out.warnings.iter().any(|w| w.contains("layer background")), "{:?}", out.warnings);
    assert!(out.warnings.iter().any(|w| w.contains("drop lines")), "{:?}", out.warnings);
}

#[test]
fn lollipop_drop_settings_are_named_in_the_save_warnings() {
    let mut p = source();
    line_graph(&mut p, |l| {
        let d = &mut l.plots[0].decor.drop.vertical;
        d.on = true;
        d.target = sheaf_core::graph::DropTarget::NextDataPlot;
        d.color_mode = sheaf_core::graph::DropColorMode::PlusMinus;
    });
    let (_, out) = round_trip(&p);
    assert!(out.warnings.iter().any(|w| w.contains("Plus-Minus")), "{:?}", out.warnings);
}

#[test]
fn an_axis_rug_is_named_in_the_save_warnings() {
    let mut p = source();
    line_graph(&mut p, |l| l.y_axis.rug = sheaf_core::graph::Rug::shown());
    let (_, out) = round_trip(&p);
    assert!(out.warnings.iter().any(|w| w.contains("axis rug")), "{:?}", out.warnings);
    let (_, out) = round_trip(&source());
    assert!(!out.warnings.iter().any(|w| w.contains("axis rug")), "{:?}", out.warnings);
}

#[test]
fn ridgeline_charts_are_written_as_plain_columns_with_a_save_warning() {
    let mut p = source();
    line_graph(&mut p, |l| l.plots[0].kind = sheaf_core::graph::PlotKind::Ridgeline);
    let (_, out) = round_trip(&p);
    assert!(
        out.warnings.iter().any(|w| w.contains("ridgeline charts were written as plain columns")),
        "{:?}",
        out.warnings
    );
    assert!(!out.warnings.iter().any(|w| w.contains("violin plots")), "{:?}", out.warnings);
}

#[test]
fn hexbin_plots_are_written_as_scatter_plots_with_a_save_warning() {
    let mut p = source();
    line_graph(&mut p, |l| l.plots[0].kind = sheaf_core::graph::PlotKind::Hexbin);
    let (_, out) = round_trip(&p);
    assert!(
        out.warnings.iter().any(|w| w.contains("hexbin plots were written as scatter plots")),
        "{:?}",
        out.warnings
    );
    assert!(!out.warnings.iter().any(|w| w.contains("ridgeline")), "{:?}", out.warnings);
}

#[test]
fn column_scatter_plots_are_written_as_scatter_plots_with_a_save_warning() {
    let mut p = source();
    line_graph(&mut p, |l| l.plots[0].kind = sheaf_core::graph::PlotKind::ColumnScatter);
    let (_, out) = round_trip(&p);
    assert!(
        out.warnings.iter().any(|w| w.contains("column scatter plots were written as scatter plots")),
        "{:?}",
        out.warnings
    );
}

#[test]
fn parallel_plots_are_written_as_line_plots_with_a_save_warning() {
    for kind in [sheaf_core::graph::PlotKind::Parallel, sheaf_core::graph::PlotKind::ParallelIndex] {
        let mut p = source();
        line_graph(&mut p, |l| l.plots[0].kind = kind);
        let (_, out) = round_trip(&p);
        assert!(
            out.warnings.iter().any(|w| w.contains("parallel plots and parallel indexes were written as line plots")),
            "{:?}",
            out.warnings
        );
        assert!(!out.warnings.iter().any(|w| w.contains("hexbin")), "{:?}", out.warnings);
    }
}

#[test]
fn treemaps_are_written_as_column_plots_with_a_save_warning() {
    let mut p = source();
    line_graph(&mut p, |l| l.plots[0].kind = sheaf_core::graph::PlotKind::Treemap);
    let (_, out) = round_trip(&p);
    assert!(
        out.warnings.iter().any(|w| w.contains("treemaps were written as column plots of their value column")),
        "{:?}",
        out.warnings
    );
    assert!(!out.warnings.iter().any(|w| w.contains("hexbin") || w.contains("parallel")), "{:?}", out.warnings);
}

#[test]
fn sunbursts_are_written_as_column_plots_with_a_save_warning() {
    let mut p = source();
    line_graph(&mut p, |l| l.plots[0].kind = sheaf_core::graph::PlotKind::Sunburst);
    let (_, out) = round_trip(&p);
    assert!(
        out.warnings.iter().any(|w| w.contains("sunburst plots were written as column plots of their value column")),
        "{:?}",
        out.warnings
    );
    assert!(!out.warnings.iter().any(|w| w.contains("treemap") || w.contains("parallel")), "{:?}", out.warnings);
}

#[test]
fn grouped_indexed_plots_are_written_as_plain_plots_with_a_save_warning() {
    let mut p = source();
    line_graph(&mut p, |l| {
        l.plots[0].kind = sheaf_core::graph::PlotKind::Column;
        l.plots[0].indexed =
            Some(sheaf_core::graph::Indexed { group: l.plots[0].y, subset: None, error: Default::default() });
    });
    let (_, out) = round_trip(&p);
    assert!(
        out.warnings.iter().any(|w| w.contains("grouped columns and grouped box charts of indexed data were written")),
        "{:?}",
        out.warnings
    );
}

#[test]
fn interval_plots_and_pareto_charts_are_written_as_plain_plots_with_a_save_warning() {
    let mut p = source();
    line_graph(&mut p, |l| {
        l.plots[0].stat = Some(sheaf_core::graph::StatPlot::Pareto);
    });
    let (_, out) = round_trip(&p);
    assert!(
        out.warnings.iter().any(|w| w.contains("interval plots and Pareto charts were written")),
        "{:?}",
        out.warnings
    );
}

#[test]
fn violin_plots_are_written_as_plain_columns_with_a_save_warning() {
    for kind in [sheaf_core::graph::PlotKind::Violin, sheaf_core::graph::PlotKind::HalfViolin] {
        let mut p = source();
        line_graph(&mut p, |l| l.plots[0].kind = kind);
        let (_, out) = round_trip(&p);
        assert!(
            out.warnings.iter().any(|w| w.contains("violin plots were written as plain columns")),
            "{:?}",
            out.warnings
        );
        assert!(!out.warnings.iter().any(|w| w.contains("histogram, box and pie")), "{:?}", out.warnings);
    }
}

#[test]
fn the_plot_details_settings_the_file_cannot_hold_are_named_in_the_save_warnings() {
    let mut p = one_layer(|l| {
        l.bars.width_pct = Some(70.0);
        l.group_style.subgrouping = Subgrouping::BySize { size: 2 };
        l.display.show_x_axes = false;
        l.background_style.gradient = Some(Gradient::new(Color::WHITE));
        let d = &mut l.plots[0].decor;
        d.fill.pattern = FillPattern::Cross;
        d.fill_under = Some(Color::RED);
        d.labels.enable = true;
        d.labels.form = LabelForm::Y;
    });
    let id = p.graphs().next().unwrap().id;
    let style = &mut p.graph_mut(id).unwrap().page.style;
    style.border = Some(BorderStyle::solid(Color::BLACK, 1.0));
    style.layer_drawing_order = LayerDrawingOrder::Composite;
    let (_, out) = round_trip(&p);
    for word in [
        "pattern, transparency and gradient",
        "width of column bars",
        "Group tab",
        "Display tab",
        "gradient and border",
        "page's border",
        "layer drawing order flag",
    ] {
        assert!(out.warnings.iter().any(|w| w.contains(word)), "{word}: {:?}", out.warnings);
    }
    // The notes say only what is true: the Group tab is not written at all (the colours of grouped
    // plots have a warning of their own), and a column width reopens with the gap setting, not
    // with the spacing the width gave on screen.
    let note = |word: &str| out.warnings.iter().find(|w| w.contains(word)).cloned().unwrap();
    assert!(!note("Group tab").contains("colours were"), "{}", note("Group tab"));
    assert!(!note("width of column bars").contains("gap between bars was"), "{}", note("width of column bars"));
    assert!(note("width of column bars").contains("gap setting"));
    // A graph without them warns about none of it.
    let (_, out) = round_trip(&one_layer(|_| {}));
    assert!(out.warnings.is_empty(), "{:?}", out.warnings);
}

#[test]
fn automatic_legend_opj_rectangle_keeps_the_measured_upper_left_anchor() {
    for count in [1, 4] {
        let p = lines_graph(&[1, 2, 3, 4][..count], |l| {
            l.legend.position = LegendPosition::Automatic;
            for plot in &mut l.plots {
                plot.label = Some("B".into());
            }
        });
        let (imported, out) = round_trip(&p);
        assert!(out.warnings.iter().any(|w| w.contains("automatic legend anchor")));
        let source = p.graphs().next().unwrap();
        let LegendPosition::Manual { x, y } = imported.graphs[0].layers[0].legend.position else {
            panic!("raw OPJ imports a fixed rectangle centre");
        };
        // The annotation is stored as a centre plus a rectangle. The writer's published
        // character/line metrics for a one-character legend give this rectangle in points.
        let width = (0.58 + 2.6) * 22.0;
        let height = 1.2 * 22.0 * count as f64;
        let fw = source.layers[0].frame.width * source.page.width_pt;
        let fh = source.layers[0].frame.height * source.page.height_pt;
        assert!((x - width / (2.0 * fw) - 4.933_850_365 / 6.0).abs() < 0.001);
        assert!((y - height / (2.0 * fh) - (1.0 - (12.043_552_036 - 1.0) / 10.0)).abs() < 0.001);
    }
}

#[test]
fn multi_y_layers_are_written_as_ordinary_layers_and_the_axis_offsets_are_named() {
    let mut p = source();
    let c = ids(&p);
    let ys = [c[1], c[2], c[3]];
    let id = p.plot_columns(Template::Multi3YYy, &ys).unwrap();
    let (imp, out) = round_trip(&p);
    assert_eq!(imp.graphs[0].layers.len(), 3, "one ordinary layer per Y axis");
    assert!(out.warnings.iter().any(|w| w.contains("position of a Y axis away from the frame")), "{:?}", out.warnings);
    // Without a displaced axis nothing is named.
    p.graph_mut(id).unwrap().layers.iter_mut().for_each(|l| l.y_axis.offset_pct = 0.0);
    let (_, out) = round_trip(&p);
    assert!(!out.warnings.iter().any(|w| w.contains("away from the frame")), "{:?}", out.warnings);
}

#[test]
fn an_x_axis_offset_is_named_in_the_notes() {
    let mut p = source();
    let c = ids(&p);
    let id = p.plot_columns(Template::ScatterCentral, &[c[1]]).unwrap();
    let (_, out) = round_trip(&p);
    assert!(out.warnings.iter().any(|w| w.contains("position of an X axis away from the frame")), "{:?}", out.warnings);
    p.graph_mut(id).unwrap().layers.iter_mut().for_each(|l| l.x_axis.offset_pct = 0.0);
    let (_, out) = round_trip(&p);
    assert!(!out.warnings.iter().any(|w| w.contains("position of an X axis")), "{:?}", out.warnings);
}

#[test]
fn a_paired_axis_is_named_in_the_notes() {
    let mut p = source();
    let c = ids(&p);
    let id = p.plot_columns(Template::Line, &[c[1]]).unwrap();
    let axis = &mut p.graph_mut(id).unwrap().layers[0].x_axis;
    axis.show_opposite = true;
    axis.opposite_ticks = true;
    axis.opposite_offset_pct = Some(3.0);
    let (imp, out) = round_trip(&p);
    assert!(out.warnings.iter().any(|w| w.contains("paired X axis' tick labels, title and offset")));
    let axis = &imp.graphs[0].layers[0].x_axis;
    assert!(axis.show_opposite && axis.opposite_offset_pct.is_none());
}

#[test]
fn an_axis_at_a_position_is_named_in_the_notes() {
    // T243: Origin's "At Position =" is not written; the companion keeps it.
    let mut p = source();
    let c = ids(&p);
    let id = p.plot_columns(Template::Scatter, &[c[1]]).unwrap();
    p.graph_mut(id).unwrap().layers[0].y_axis.at_position = Some(0.0);
    let (_, out) = round_trip(&p);
    assert!(out.warnings.iter().any(|w| w.contains("Y axis' position at a value")), "{:?}", out.warnings);
    p.graph_mut(id).unwrap().layers[0].y_axis.at_position = None;
    let (_, out) = round_trip(&p);
    assert!(!out.warnings.iter().any(|w| w.contains("At Position")), "{:?}", out.warnings);
}

#[test]
fn axis_arrows_are_named_in_the_notes() {
    // T245: Arrow at Begin / End are not written; the companion keeps them.
    let mut p = source();
    let c = ids(&p);
    let id = p.plot_columns(Template::Scatter, &[c[1]]).unwrap();
    p.graph_mut(id).unwrap().layers[0].x_axis.arrow_end = true;
    let (_, out) = round_trip(&p);
    assert!(out.warnings.iter().any(|w| w.contains("arrows at the ends of the X axis")), "{:?}", out.warnings);
}
