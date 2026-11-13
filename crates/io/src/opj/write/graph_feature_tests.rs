//! Round-trip tests of the graph features that the file holds since spec section 13.12: stacked
//! curve offsets, the right Y axis of a double-Y graph, bar gaps, axis sides and automatic axis
//! colours. Origin 2016 showed each of them the way the model draws them (spec 13.12).

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
fn x_offsets_and_waterfalls_are_still_named_and_a_plain_layer_has_no_offsets() {
    let mut p = stacked(Stack::Individual);
    let c = ids(&p);
    let id = p.graphs().next().unwrap().id;
    p.graph_mut(id).unwrap().layers[0].plot_offset_mut(c[2]).x = 1.5;
    let (_, out) = round_trip(&p);
    assert!(out.warnings.iter().any(|w| w.contains("X offset")), "{:?}", out.warnings);

    let (_, out) = round_trip(&stacked(Stack::Waterfall { dx_percent: 5.0, dy_percent: 10.0 }));
    assert!(out.warnings.iter().any(|w| w.contains("waterfall")), "{:?}", out.warnings);

    let (imp, out) = round_trip(&stacked(Stack::None));
    assert!(out.warnings.is_empty(), "{:?}", out.warnings);
    assert_eq!(imp.graphs[0].layers[0].stack, Stack::None);
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
        for y in [5, 6] {
            p.add_plot_xy(id, 0, None, c[y], None).unwrap();
        }
        p.graph_mut(id).unwrap().layers[0].bars = BarSpacing { gap_pct: gap, overlap_pct: 0.0 };
        let (imp, out) = round_trip(&p);
        assert!(out.warnings.is_empty(), "gap {gap}: {:?}", out.warnings);
        assert!((imp.graphs[0].layers[0].bars.gap_pct - gap).abs() < 0.02, "gap {gap}");
    }
    let mut p = source();
    let c = ids(&p);
    let id = p.add_graph(Template::Column);
    p.add_plot_xy(id, 0, None, c[5], None).unwrap();
    p.graph_mut(id).unwrap().layers[0].bars = BarSpacing { gap_pct: 30.0, overlap_pct: 40.0 };
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
    f(&mut p.graph_mut(id).unwrap().layers[0]);
    p
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
    f(&mut p.graph_mut(id).unwrap().layers[0]);
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
    assert!(out.warnings.iter().any(|w| w.contains("colours of column plots")), "{:?}", out.warnings);
    assert_eq!(imp.graphs[0].layers[0].plots.len(), 2);
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
        panel.link = Some(LayerLink { parent: 0, x, y });
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
        let p = panels(3, x, y);
        let (imp, out) = round_trip(&p);
        assert!(out.warnings.is_empty(), "{x:?} {y:?}: {:?}", out.warnings);
        let want = &p.graphs().next().unwrap().layers;
        let got = &imp.graphs[0].layers;
        assert_eq!(got.len(), 3);
        assert_eq!(got[0].link, None);
        for k in 1..3 {
            assert_eq!(got[k].link, Some(LayerLink { parent: 0, x, y }), "{x:?} {y:?} layer {k}");
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
