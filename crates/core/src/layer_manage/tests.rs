use super::*;
use crate::graph::{AxisSide, LayerLink};
use crate::ids::WindowId;
use crate::template::{Template, right_y_layer};

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

fn assert_frame(f: Frame, left: f64, top: f64, width: f64, height: f64) {
    assert!(
        close(f.left, left) && close(f.top, top) && close(f.width, width) && close(f.height, height),
        "{f:?} is not ({left}, {top}, {width}, {height})"
    );
}

/// A graph of `n` independent layers, each at the stock frame.
fn graph(n: usize) -> Graph {
    let mut g = Template::Line.new_graph(WindowId(1), "G");
    let base = g.layers[0].clone();
    g.layers = (0..n).map(|_| base.clone()).collect();
    g
}

fn rows(n: usize) -> ArrangeOptions {
    ArrangeOptions { rows: n, ..ArrangeOptions::default() }
}

/// Layer 1 and 2 stand on the page; layers 3 and 4 are linked to layer 1 (frames of their own).
fn linked_graph() -> Graph {
    let mut g = graph(4);
    g.layers[0].frame = Frame { left: 0.1, top: 0.1, width: 0.4, height: 0.4 };
    g.layers[1].frame = Frame { left: 0.5, top: 0.5, width: 0.4, height: 0.4 };
    for k in [2, 3] {
        g.layers[k].link = Some(LayerLink { parent: 0, x: ScaleLink::Straight, y: ScaleLink::None });
        g.layers[k].frame = Frame { left: 0.0, top: 0.5 * (k - 2) as f64, width: 1.0, height: 0.5 };
    }
    g
}

/// A double-Y graph: layer 2 is drawn over layer 1 on the right.
fn double_y() -> Graph {
    let mut g = graph(1);
    let right = right_y_layer(&g.layers[0], 0);
    g.layers.push(right);
    g
}

#[test]
fn the_defaults_are_those_of_the_layer_management_window() {
    let o = ArrangeOptions::default();
    assert_eq!((o.rows, o.cols, o.h_gap, o.v_gap), (1, 1, 5.0, 5.0));
    assert_eq!((o.margin_left, o.margin_right, o.margin_top, o.margin_bottom), (15.0, 10.0, 10.0, 15.0));
    // The layarrange defaults stay at 15 on every side.
    assert_eq!(PanelLayout::arrange(2, 1).margin_right, 15.0);
}

#[test]
fn two_layers_in_two_rows() {
    let mut g = graph(2);
    g.arrange_layers(&[], &rows(2)).unwrap();
    // height (100 - 10 - 15 - 5) / 2 = 35, width 75
    assert_frame(g.layers[0].frame, 0.15, 0.10, 0.75, 0.35);
    assert_frame(g.layers[1].frame, 0.15, 0.50, 0.75, 0.35);
}

#[test]
fn only_the_selected_layers_are_arranged() {
    let mut g = graph(3);
    let before = g.layers[1].frame;
    g.arrange_layers(&[2, 0], &rows(2)).unwrap();
    assert_frame(g.layers[0].frame, 0.15, 0.10, 0.75, 0.35);
    assert_frame(g.layers[2].frame, 0.15, 0.50, 0.75, 0.35);
    assert_eq!(g.layers[1].frame, before);
}

#[test]
fn a_grid_fills_row_by_row() {
    let mut g = graph(4);
    g.arrange_layers(&[], &ArrangeOptions { rows: 2, cols: 2, ..ArrangeOptions::default() }).unwrap();
    let w = (100.0 - 15.0 - 10.0 - 5.0) / 200.0;
    let h = (100.0 - 10.0 - 15.0 - 5.0) / 200.0;
    assert_frame(g.layers[1].frame, 0.15 + w + 0.05, 0.10, w, h);
    assert_frame(g.layers[2].frame, 0.15, 0.10 + h + 0.05, w, h);
}

#[test]
fn current_position_order_sorts_top_to_bottom_then_left_to_right() {
    let mut g = graph(3);
    g.layers[0].frame.top = 0.7;
    g.layers[1].frame.top = 0.1;
    g.layers[2].frame.top = 0.1;
    g.layers[1].frame.left = 0.6;
    g.layers[2].frame.left = 0.1;
    let opts = ArrangeOptions { order: ArrangeOrder::CurrentPosition, rows: 3, ..ArrangeOptions::default() };
    g.arrange_layers(&[], &opts).unwrap();
    // layer 3 (top left) first, then layer 2, then layer 1 (lowest)
    assert!(g.layers[2].frame.top < g.layers[1].frame.top);
    assert!(g.layers[1].frame.top < g.layers[0].frame.top);
    let mut by_index = graph(3);
    by_index.layers[0].frame.top = 0.7;
    by_index.arrange_layers(&[], &rows(3)).unwrap();
    assert!(by_index.layers[0].frame.top < by_index.layers[2].frame.top);
}

#[test]
fn common_scale_follows_position_order_and_uses_the_root_of_a_selected_group() {
    let mut g = linked_graph();
    g.layers[0].frame.top = 0.7;
    g.layers[1].frame.top = 0.1;
    let before = g.clone();
    let options = ArrangeOptions {
        order: ArrangeOrder::CurrentPosition,
        group_linked: true,
        keep_aspect: true,
        common_x_spans: Some(vec![10.0, 30.0, 900.0, 900.0]),
        common_y_spans: Some(vec![20.0, 60.0, 900.0, 900.0]),
        ..rows(2)
    };
    g.arrange_layers(&[1, 3], &options).unwrap();
    assert_frame(g.page_frame(1), 0.15, 0.10, 0.75, 0.525);
    assert_frame(g.page_frame(0), 0.15, 0.675, 0.25, 0.175);
    assert_eq!(g.layers[3].frame, before.layers[3].frame);
    assert_eq!(g.layers[3].link, before.layers[3].link);

    let before = g.clone();
    let invalid = ArrangeOptions { common_x_spans: Some(vec![0.0, 30.0, 900.0, 900.0]), ..options };
    assert_eq!(g.arrange_layers(&[1, 3], &invalid), Err(LayerError::Panel(PanelError::CommonScale)));
    assert_eq!(g, before);
}

#[test]
fn more_layers_than_cells_are_refused_and_nothing_changes() {
    let mut g = graph(3);
    let before = g.clone();
    assert_eq!(g.arrange_layers(&[], &rows(2)), Err(LayerError::TooManyLayers { cells: 2, layers: 3 }));
    assert_eq!(g, before);
    let bad = ArrangeOptions { h_gap: 80.0, ..rows(3) };
    assert!(matches!(g.arrange_layers(&[], &bad), Err(LayerError::Panel(PanelError::Gap))));
    assert_eq!(g.arrange_layers(&[7], &rows(3)), Err(LayerError::NoSuchLayer(7)));
    assert_eq!(g, before);
}

#[test]
fn keep_aspect_ratio_fits_the_layer_in_its_cell() {
    let mut g = graph(2);
    g.layers[0].frame = Frame { left: 0.0, top: 0.0, width: 0.4, height: 0.2 };
    g.layers[1].frame = Frame { left: 0.0, top: 0.0, width: 0.2, height: 0.4 };
    g.arrange_layers(&[], &ArrangeOptions { keep_aspect: true, ..rows(2) }).unwrap();
    // cells are 75 x 35 percent: layer 1 (2:1) is limited by the height
    assert_frame(g.layers[0].frame, 0.15, 0.10, 0.70, 0.35);
    assert_frame(g.layers[1].frame, 0.15, 0.50, 0.175, 0.35);
}

#[test]
fn linked_layers_get_frames_relative_to_their_parent_when_not_grouped() {
    let mut g = linked_graph();
    g.arrange_layers(&[], &ArrangeOptions { rows: 2, cols: 2, ..ArrangeOptions::default() }).unwrap();
    for k in 0..4 {
        let cells = ArrangeOptions { rows: 2, cols: 2, ..ArrangeOptions::default() }.layout().frames().unwrap();
        let p = g.page_frame(k);
        assert_frame(p, cells[k].left, cells[k].top, cells[k].width, cells[k].height);
    }
    // Layer 3 is still linked to layer 1 and stores a fraction of it.
    assert_eq!(g.layers[2].link.unwrap().parent, 0);
    assert!(g.layers[2].frame.top > 1.0);
}

#[test]
fn grouped_linked_layers_count_as_one_unit_and_follow_their_root() {
    let mut g = linked_graph();
    let before: Vec<Frame> = (0..4).map(|i| g.layers[i].frame).collect();
    // Layers 1, 3, 4 are one group; layer 2 the other unit.
    g.arrange_layers(&[], &ArrangeOptions { group_linked: true, ..rows(2) }).unwrap();
    assert_frame(g.page_frame(0), 0.15, 0.10, 0.75, 0.35);
    assert_frame(g.page_frame(1), 0.15, 0.50, 0.75, 0.35);
    // The linked layers keep their stored frames and so follow the root's new frame.
    assert_eq!(g.layers[2].frame, before[2]);
    assert_eq!(g.layers[3].frame, before[3]);
    assert_frame(g.page_frame(3), 0.15, 0.10 + 0.175, 0.75, 0.175);
}

#[test]
fn a_double_y_graph_arranged_as_a_group_keeps_its_right_axis_over_the_left() {
    let mut g = double_y();
    g.arrange_layers(&[], &ArrangeOptions { group_linked: true, ..rows(1) }).unwrap();
    assert_frame(g.page_frame(0), 0.15, 0.10, 0.75, 0.75);
    assert_eq!(g.page_frame(0), g.page_frame(1));
    assert_eq!(g.layers[1].y_axis.side, AxisSide::Opposite);
    assert_eq!(g.layers[1].link.unwrap().parent, 0);
}

#[test]
fn a_double_y_graph_arranged_without_grouping_places_both_layers_in_cells() {
    let mut g = double_y();
    g.arrange_layers(&[], &rows(2)).unwrap();
    assert_frame(g.page_frame(0), 0.15, 0.10, 0.75, 0.35);
    assert_frame(g.page_frame(1), 0.15, 0.50, 0.75, 0.35);
}

#[test]
fn link_layers_in_the_arrangement_follow_the_first_with_no_shared_scale() {
    let mut g = graph(3);
    g.arrange_layers(&[], &ArrangeOptions { link: true, ..rows(3) }).unwrap();
    assert!(g.layers[0].link.is_none());
    for k in 1..3 {
        let link = g.layers[k].link.unwrap();
        assert_eq!((link.parent, link.x, link.y), (0, ScaleLink::None, ScaleLink::None));
        let cells = rows(3).layout().frames().unwrap();
        let p = g.page_frame(k);
        assert_frame(p, cells[k].left, cells[k].top, cells[k].width, cells[k].height);
    }
}

#[test]
fn hide_inner_axes_needs_no_gap() {
    let mut g = graph(2);
    g.arrange_layers(&[], &ArrangeOptions { hide_inner: true, v_gap: 0.0, ..rows(2) }).unwrap();
    assert!(!g.layers[0].x_axis.show_tick_labels);
    assert!(g.layers[1].x_axis.show_tick_labels);
    let mut g = graph(2);
    g.arrange_layers(&[], &ArrangeOptions { hide_inner: true, ..rows(2) }).unwrap();
    assert!(g.layers[0].x_axis.show_tick_labels);
}

#[test]
fn resize_in_each_unit() {
    let mut g = graph(2);
    g.layers[0].frame = Frame { left: 0.1, top: 0.2, width: 0.5, height: 0.4 };
    let (pw, ph) = (g.page.width_pt, g.page.height_pt);
    g.resize_layers(&[1], 0, SizeUnit::ReferencePercent, 50.0, 100.0).unwrap();
    assert_frame(g.layers[1].frame, g.layers[1].frame.left, g.layers[1].frame.top, 0.25, 0.4);
    g.resize_layers(&[1], 0, SizeUnit::PagePercent, 30.0, 20.0).unwrap();
    assert_frame(g.layers[1].frame, g.layers[1].frame.left, g.layers[1].frame.top, 0.3, 0.2);
    g.resize_layers(&[1], 0, SizeUnit::Inch, 2.0, 1.0).unwrap();
    assert!(close(g.layers[1].frame.width, 144.0 / pw) && close(g.layers[1].frame.height, 72.0 / ph));
    g.resize_layers(&[1], 0, SizeUnit::Cm, 2.54, 2.54).unwrap();
    assert!(close(g.layers[1].frame.width, 72.0 / pw));
    g.resize_layers(&[1], 0, SizeUnit::Mm, 25.4, 25.4).unwrap();
    assert!(close(g.layers[1].frame.height, 72.0 / ph));
    g.resize_layers(&[1], 0, SizeUnit::Pixel, 96.0, 96.0).unwrap();
    assert!(close(g.layers[1].frame.width, 72.0 / pw));
    g.resize_layers(&[1], 0, SizeUnit::Point, 36.0, 36.0).unwrap();
    assert!(close(g.layers[1].frame.width, 36.0 / pw));
}

#[test]
fn the_reference_layer_can_be_a_linked_layer() {
    let mut g = linked_graph();
    // Layer 3 is the lower half... its page frame is 0.4 x 0.2 at (0.1, 0.1).
    g.resize_layers(&[1], 2, SizeUnit::ReferencePercent, 100.0, 100.0).unwrap();
    assert_frame(g.layers[1].frame, 0.5, 0.5, 0.4, 0.2);
}

#[test]
fn resizing_a_linked_layer_is_stored_as_fractions_of_the_parent() {
    let mut g = linked_graph();
    g.resize_layers(&[2], 0, SizeUnit::PagePercent, 20.0, 10.0).unwrap();
    assert_frame(g.page_frame(2), 0.1, 0.1, 0.2, 0.1);
    assert_frame(g.layers[2].frame, 0.0, 0.0, 0.5, 0.25);
    // Resizing the parent carries the layers linked to it along.
    g.resize_layers(&[0], 0, SizeUnit::PagePercent, 80.0, 80.0).unwrap();
    assert_frame(g.page_frame(2), 0.1, 0.1, 0.4, 0.2);
}

#[test]
fn resize_refuses_bad_numbers_and_empty_selection() {
    let mut g = graph(2);
    let before = g.clone();
    assert_eq!(g.resize_layers(&[], 0, SizeUnit::PagePercent, 50.0, 50.0), Err(LayerError::NoSelection));
    assert_eq!(g.resize_layers(&[0], 5, SizeUnit::PagePercent, 50.0, 50.0), Err(LayerError::NoSuchLayer(5)));
    assert_eq!(g.resize_layers(&[0], 0, SizeUnit::PagePercent, 0.0, 50.0), Err(LayerError::Bounds(1)));
    assert_eq!(g.resize_layers(&[0], 0, SizeUnit::PagePercent, f64::NAN, 50.0), Err(LayerError::Number));
    assert_eq!(g.resize_layers(&[0, 1], 0, SizeUnit::PagePercent, 5000.0, 50.0), Err(LayerError::Bounds(1)));
    assert_eq!(g, before);
}

#[test]
fn move_to_page_and_reference_positions() {
    let mut g = graph(2);
    g.layers[0].frame = Frame { left: 0.2, top: 0.3, width: 0.4, height: 0.4 };
    g.move_layers(&[1], 0, SizeUnit::PagePercent, 10.0, 20.0).unwrap();
    assert!(close(g.layers[1].frame.left, 0.1) && close(g.layers[1].frame.top, 0.2));
    // 50 percent of the reference layer from its left and top: 0.2 + 0.2, 0.3 + 0.2
    g.move_layers(&[1], 0, SizeUnit::ReferencePercent, 50.0, 50.0).unwrap();
    assert!(close(g.layers[1].frame.left, 0.4) && close(g.layers[1].frame.top, 0.5));
    let pw = g.page.width_pt;
    g.move_layers(&[1], 0, SizeUnit::Inch, 1.0, 1.0).unwrap();
    assert!(close(g.layers[1].frame.left, 72.0 / pw));
    assert_eq!(g.move_layers(&[1], 0, SizeUnit::PagePercent, 5000.0, 0.0), Err(LayerError::Bounds(2)));
}

#[test]
fn moving_a_linked_layer_keeps_it_linked() {
    let mut g = linked_graph();
    g.move_layers(&[3], 0, SizeUnit::ReferencePercent, 25.0, 75.0).unwrap();
    assert_eq!(g.layers[3].link.unwrap().parent, 0);
    assert_frame(g.page_frame(3), 0.2, 0.4, 0.4, 0.2);
    assert_frame(g.layers[3].frame, 0.25, 0.75, 1.0, 0.5);
}

#[test]
fn align_moves_one_edge_to_the_reference_edge() {
    let mut g = graph(3);
    g.layers[0].frame = Frame { left: 0.1, top: 0.1, width: 0.5, height: 0.4 };
    g.layers[1].frame = Frame { left: 0.3, top: 0.5, width: 0.2, height: 0.2 };
    g.layers[2].frame = Frame { left: 0.7, top: 0.6, width: 0.1, height: 0.3 };
    g.align_layers(&[0, 1, 2], 0, AlignEdge::Left).unwrap();
    assert!(close(g.layers[1].frame.left, 0.1) && close(g.layers[2].frame.left, 0.1));
    g.align_layers(&[1, 2], 0, AlignEdge::Right).unwrap();
    assert!(close(g.layers[1].frame.left, 0.4) && close(g.layers[2].frame.left, 0.5));
    g.align_layers(&[1, 2], 0, AlignEdge::Top).unwrap();
    assert!(close(g.layers[1].frame.top, 0.1) && close(g.layers[2].frame.top, 0.1));
    g.align_layers(&[1, 2], 0, AlignEdge::Bottom).unwrap();
    assert!(close(g.layers[1].frame.top, 0.3) && close(g.layers[2].frame.top, 0.2));
    // the reference layer itself did not move
    assert_frame(g.layers[0].frame, 0.1, 0.1, 0.5, 0.4);
}

#[test]
fn align_a_linked_layer_by_its_page_frame() {
    let mut g = linked_graph();
    g.align_layers(&[3], 1, AlignEdge::Left).unwrap();
    assert_frame(g.page_frame(3), 0.5, 0.3, 0.4, 0.2);
    assert!(close(g.layers[3].frame.left, 1.0));
}

#[test]
fn swap_positions_only() {
    let mut g = graph(2);
    g.layers[0].frame = Frame { left: 0.1, top: 0.1, width: 0.3, height: 0.3 };
    g.layers[1].frame = Frame { left: 0.5, top: 0.6, width: 0.4, height: 0.2 };
    g.layers[1].labels.clear();
    g.layers[0].x_axis.title = Some("first".into());
    g.swap_layers(0, 1, false).unwrap();
    assert_frame(g.layers[0].frame, 0.5, 0.6, 0.4, 0.2);
    assert_frame(g.layers[1].frame, 0.1, 0.1, 0.3, 0.3);
    assert_eq!(g.layers[0].x_axis.title.as_deref(), Some("first"));
}

#[test]
fn swap_with_reorder_trades_list_slots_and_follows_links() {
    let mut g = graph(3);
    g.layers[0].frame = Frame { left: 0.1, top: 0.1, width: 0.3, height: 0.3 };
    g.layers[1].frame = Frame { left: 0.5, top: 0.6, width: 0.4, height: 0.2 };
    g.layers[2].x_axis.title = Some("third".into());
    g.layers[0].x_axis.title = Some("first".into());
    g.layers[1].x_axis.title = Some("second".into());
    g.swap_layers(0, 1, true).unwrap();
    assert_eq!(g.layers[0].x_axis.title.as_deref(), Some("second"));
    assert_eq!(g.layers[1].x_axis.title.as_deref(), Some("first"));
    // frames stay with the slots, the layers moved
    assert_frame(g.layers[0].frame, 0.1, 0.1, 0.3, 0.3);
    assert_frame(g.layers[1].frame, 0.5, 0.6, 0.4, 0.2);
}

#[test]
fn reorder_remaps_links_to_the_swapped_layers() {
    // layer 4 follows layer 2: after swapping 2 and 3 it follows slot 3 (the same layer)
    let mut g = graph(4);
    g.layers[3].link = Some(LayerLink { parent: 1, x: ScaleLink::None, y: ScaleLink::None });
    g.layers[3].frame = Frame { left: 0.0, top: 0.0, width: 0.5, height: 0.5 };
    g.layers[1].x_axis.title = Some("two".into());
    g.swap_layers(1, 2, true).unwrap();
    assert_eq!(g.layers[2].x_axis.title.as_deref(), Some("two"));
    assert_eq!(g.layers[3].link.unwrap().parent, 2);
    assert_eq!(g.link_parent(3), Some(2));
}

#[test]
fn reorder_that_would_put_a_parent_after_its_child_is_refused() {
    let mut g = linked_graph();
    let before = g.clone();
    assert_eq!(g.swap_layers(0, 2, true), Err(LayerError::ReorderLinks));
    assert_eq!(g, before);
    // Without reorder the same swap is fine (positions only).
    g.swap_layers(0, 2, false).unwrap();
    assert_eq!(g.layers[2].link.unwrap().parent, 0);
    // A double-Y graph: the left layer cannot move behind its right layer.
    let mut d = double_y();
    assert_eq!(d.swap_layers(0, 1, true), Err(LayerError::ReorderLinks));
}

#[test]
fn swap_validates_its_layers() {
    let mut g = graph(2);
    assert_eq!(g.swap_layers(0, 0, false), Err(LayerError::SameLayer));
    assert_eq!(g.swap_layers(0, 9, false), Err(LayerError::NoSuchLayer(9)));
}

#[test]
fn link_keeps_position_and_sets_scale_links() {
    let mut g = graph(3);
    g.layers[0].frame = Frame { left: 0.1, top: 0.1, width: 0.4, height: 0.4 };
    g.layers[1].frame = Frame { left: 0.5, top: 0.5, width: 0.2, height: 0.2 };
    let page = g.page_frame(1);
    g.link_layers(&[1, 2], Some(0), ScaleLink::Straight, ScaleLink::None).unwrap();
    assert_eq!(g.page_frame(1), page);
    let link = g.layers[1].link.unwrap();
    assert_eq!((link.parent, link.x, link.y), (0, ScaleLink::Straight, ScaleLink::None));
    assert_eq!(g.layers[2].link.unwrap().x, ScaleLink::Straight);
    // unlinking keeps the place on the page too
    g.link_layers(&[1], None, ScaleLink::None, ScaleLink::None).unwrap();
    assert!(g.layers[1].link.is_none());
    assert_frame(g.layers[1].frame, 0.5, 0.5, 0.2, 0.2);
}

#[test]
fn link_refuses_cycles_and_misplaced_parents() {
    let mut g = graph(3);
    let before = g.clone();
    // itself, or a later layer
    assert!(matches!(g.link_layers(&[0], Some(0), ScaleLink::None, ScaleLink::None), Err(LayerError::Link { .. })));
    assert!(matches!(g.link_layers(&[0], Some(2), ScaleLink::None, ScaleLink::None), Err(LayerError::Link { .. })));
    assert_eq!(g.link_layers(&[1], Some(8), ScaleLink::None, ScaleLink::None), Err(LayerError::NoSuchLayer(8)));
    assert_eq!(g, before);
    // layer 2 -> 1 and then layer 3 -> 2 (a linked parent) is refused
    g.link_layers(&[1], Some(0), ScaleLink::None, ScaleLink::None).unwrap();
    assert!(g.link_layers(&[2], Some(1), ScaleLink::None, ScaleLink::None).is_err());
    // a layer that others follow cannot be linked; a failure in the middle undoes the first
    let mut h = graph(3);
    h.link_layers(&[2], Some(1), ScaleLink::None, ScaleLink::None).unwrap();
    let before = h.clone();
    assert!(h.link_layers(&[1, 2], Some(0), ScaleLink::None, ScaleLink::None).is_err());
    assert_eq!(h, before);
}

#[test]
fn linking_the_layers_of_a_double_y_graph_to_a_new_layer_is_refused_for_the_parent() {
    let mut g = double_y();
    // layer 1 has a child (layer 2): it cannot be linked to anything
    assert!(g.has_children(0));
    assert!(g.link_layers(&[0], Some(0), ScaleLink::None, ScaleLink::None).is_err());
    let base = g.layers[0].clone();
    g.layers.insert(0, base);
    // the old links point at layer 1 (now the new layer's index is 0): fix the parent by hand
    g.layers[2].link = Some(LayerLink { parent: 1, x: ScaleLink::Straight, y: ScaleLink::None });
    assert!(g.link_layers(&[1], Some(0), ScaleLink::None, ScaleLink::None).is_err());
}

#[test]
fn a_reference_that_follows_a_selected_layer_is_refused() {
    // layer 3 follows layer 1: aligning layer 1 to it would move the reference too
    let mut g = linked_graph();
    let before = g.clone();
    assert_eq!(g.align_layers(&[0], 2, AlignEdge::Left), Err(LayerError::ReferenceFollows(2)));
    assert_eq!(g.move_layers(&[0], 2, SizeUnit::ReferencePercent, 0.0, 0.0), Err(LayerError::ReferenceFollows(2)));
    assert_eq!(g.resize_layers(&[0], 2, SizeUnit::ReferencePercent, 50.0, 50.0), Err(LayerError::ReferenceFollows(2)));
    assert_eq!(g, before);
    // in page units the reference does not matter, and an unrelated reference is fine
    g.move_layers(&[0], 2, SizeUnit::PagePercent, 10.0, 10.0).unwrap();
    g.align_layers(&[1], 2, AlignEdge::Left).unwrap();
    assert!(close(g.page_frame(1).left, g.page_frame(2).left));
}
