use super::*;
use crate::column::{ColumnData, Designation};
use crate::ids::ColumnId;
use crate::template::Template;

fn close(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
}

/// A project with one workbook of an X and four Y columns and `n` one-plot Line graphs.
fn project(n: usize) -> (Project, Vec<WindowId>, Vec<ColumnId>) {
    let mut p = Project::new();
    let book = p.add_workbook("data");
    p.add_column(book, 0, Designation::X, ColumnData::numeric(vec![0.0, 1.0, 2.0])).unwrap();
    let ys: Vec<ColumnId> = (0..4)
        .map(|k| {
            let v = vec![k as f64, 2.0 * k as f64, 1.0];
            p.add_column(book, 0, Designation::Y, ColumnData::numeric(v)).unwrap()
        })
        .collect();
    let graphs = (0..n).map(|k| p.plot_columns(Template::Line, &[ys[k % ys.len()]]).unwrap()).collect();
    (p, graphs, ys)
}

fn opts(sources: &[WindowId]) -> MergeOptions {
    MergeOptions { sources: sources.to_vec(), ..MergeOptions::default() }
}

fn assert_pct(f: &Frame, left: f64, top: f64, width: f64, height: f64) {
    let ok = close(100.0 * f.left, left, 0.02)
        && close(100.0 * f.top, top, 0.02)
        && close(100.0 * f.width, width, 0.02)
        && close(100.0 * f.height, height, 0.02);
    assert!(ok, "{f:?} is not ({left}, {top}, {width}, {height}) percent");
}

#[test]
fn the_automatic_grid_follows_origin() {
    // Origin 2016: 2 graphs 2 x 1, 3 and 4 graphs 2 x 2, 5 graphs 3 x 2
    assert_eq!(auto_grid(2, None, None), (2, 1));
    assert_eq!(auto_grid(3, None, None), (2, 2));
    assert_eq!(auto_grid(4, None, None), (2, 2));
    assert_eq!(auto_grid(5, None, None), (3, 2));
    assert_eq!(auto_grid(9, None, None), (3, 3));
    assert_eq!(auto_grid(10, None, None), (4, 3));
    assert_eq!(auto_grid(3, Some(1), None), (1, 3));
    assert_eq!(auto_grid(5, None, Some(2)), (3, 2));
    assert_eq!(auto_grid(3, Some(3), Some(1)), (3, 1));
}

#[test]
fn the_text_factor_matches_origins_measured_merges() {
    let cases = [
        ((35.0, 34.99), 0.46),  // 2 x 2
        ((50.0, 50.0), 0.66),   // 2 x 2, no gaps or margins
        ((17.5, 17.5), 0.23),   // 2 x 2, margins 30
        ((21.66, 21.68), 0.29), // 3 x 3
        ((21.66, 34.99), 0.36), // 2 x 3
        ((35.0, 21.68), 0.36),  // 3 x 2
        ((15.0, 15.01), 0.20),  // 4 x 4
        ((20.57, 21.68), 0.28), // 3 x 1 keeping the aspect ratio
        ((39.99, 21.68), 0.39), // 3 x 1, side margins 30
        ((35.0, 20.0), 0.35),   // 1 x 2, top and bottom margins 40
        ((50.0, 34.99), 0.55),  // 2 x 2, no horizontal gap or side margins
        ((37.5, 37.5), 0.50),   // 2 x 2, no gaps
        ((31.64, 33.34), 0.43), // 3 x 1 keeping the aspect ratio, no vertical gap or margins
        ((75.01, 21.68), 1.0),  // 3 x 1
        ((21.66, 75.0), 1.0),   // 1 x 3
        ((75.01, 34.99), 1.0),  // 2 x 1
        ((35.0, 75.0), 1.0),    // 1 x 2
        ((75.01, 15.01), 1.0),  // 4 x 1
        ((70.0, 21.68), 1.0),   // 3 x 1, right margin 15
        ((65.0, 21.68), 0.50),  // 3 x 1, right margin 20
        ((21.66, 74.0), 1.0),   // 1 x 3, bottom margin 16
        ((21.66, 70.0), 0.52),  // 1 x 3, bottom margin 20
    ];
    for (cell, want) in cases {
        assert!(close(auto_text_factor(cell), want, 1e-9), "{cell:?}: {} not {want}", auto_text_factor(cell));
    }
}

#[test]
fn three_graphs_in_three_rows_take_origins_rectangles_and_keep_their_text() {
    let (mut p, gs, _) = project(3);
    let before: Vec<Graph> = gs.iter().map(|&g| p.graph(g).unwrap().clone()).collect();
    let id = p.merge_graphs(&MergeOptions { rows: Some(3), cols: Some(1), ..opts(&gs) }).unwrap();
    let m = p.graph(id).unwrap();
    assert_eq!(m.name, "Graph4");
    assert!(close(m.page.width_pt, 11.41 * 72.0, 1e-9) && close(m.page.height_pt, 7.973 * 72.0, 1e-9));
    assert_eq!(m.layers.len(), 3);
    // Origin: (15, 9.992, 75.01, 21.68), tops 36.66 and 63.34
    assert_pct(&m.layers[0].frame, 15.0, 10.0, 75.0, 21.667);
    assert_pct(&m.layers[1].frame, 15.0, 36.667, 75.0, 21.667);
    assert_pct(&m.layers[2].frame, 15.0, 63.333, 75.0, 21.667);
    for (k, layer) in m.layers.iter().enumerate() {
        let src = &before[k].layers[0];
        assert_eq!(layer.plots, src.plots, "the copy plots the same columns");
        assert_eq!(layer.x_axis.tick_font.size_pt, src.x_axis.tick_font.size_pt, "factor 1 in one column");
        assert!(layer.link.is_none(), "merged layers are not linked");
    }
    for (g, b) in gs.iter().zip(&before) {
        assert_eq!(p.graph(*g).unwrap(), b, "the sources are kept unchanged");
    }
}

#[test]
fn panel_labels_are_scaled_and_placed_once_per_source_graph() {
    use crate::layers::NewLayerKind;
    let (mut p, sources, _) = project(3);
    p.graph_mut(sources[1]).unwrap().add_layer(NewLayerKind::RightY, 0).unwrap();
    let original = p.clone();
    use MergeLabelPosition::*;
    for position in [TopLeftOutside, TopLeftInside, TopRightOutside, TopRightInside] {
        let graphs = sources.iter().map(|&id| p.graph(id).unwrap()).collect::<Vec<_>>();
        let labels = ["A", "B", "C"].into_iter().map(|text| MergePanelLabel { text: text.into(), width_pt: 20.0 });
        let result = merge_graphs(
            &graphs,
            &MergeOptions { panel_labels: labels.collect(), label_position: position, ..opts(&sources) },
        )
        .unwrap();
        assert!(result.layers[2].labels.is_empty(), "the secondary Y layer gets no separate label");
        for (index, want) in [(0, "A"), (1, "B"), (3, "C")] {
            let layer = &result.layers[index];
            let [label] = layer.labels.as_slice() else { panic!("expected one panel label") };
            assert_eq!(label.text, want);
            let width = 20.0 * 0.46 / (layer.frame.width * result.page.width_pt);
            let half_height = 0.5 * label.font.size_pt / (layer.frame.height * result.page.height_pt);
            let ((edge_x, edge_y), (want_x, want_y)) = match position {
                TopLeftOutside => ((label.x + width, label.y + half_height), (-0.02, -0.02)),
                TopLeftInside => ((label.x, label.y - half_height), (0.02, 0.02)),
                TopRightOutside => ((label.x, label.y + half_height), (1.02, -0.02)),
                TopRightInside => ((label.x + width, label.y - half_height), (0.98, 0.02)),
            };
            assert!(close(edge_x, want_x, 1e-9) && close(edge_y, want_y, 1e-9));
            assert!(close(label.font.size_pt, 13.8, 1e-9));
        }
    }
    assert_eq!(p, original);
}

#[test]
fn three_graphs_fill_a_two_by_two_grid_and_their_text_shrinks() {
    let (mut p, gs, _) = project(3);
    let tick = p.graph(gs[0]).unwrap().layers[0].x_axis.tick_font.size_pt;
    let id = p.merge_graphs(&opts(&gs)).unwrap();
    let m = p.graph(id).unwrap();
    assert_eq!(m.layers.len(), 3, "the empty cell gets no layer");
    // Origin: (15, 9.992, 35, 34.99), (55, 9.992), (15, 50)
    assert_pct(&m.layers[0].frame, 15.0, 10.0, 35.0, 35.0);
    assert_pct(&m.layers[1].frame, 55.0, 10.0, 35.0, 35.0);
    assert_pct(&m.layers[2].frame, 15.0, 50.0, 35.0, 35.0);
    // Origin's factor for these cells, whatever the source's own layer size
    assert!(close(m.layers[0].x_axis.tick_font.size_pt, tick * 0.46, 1e-9));
    let keep = p.merge_graphs(&MergeOptions { text_scale: MergeTextScale::Keep, ..opts(&gs) }).unwrap();
    assert_eq!(p.graph(keep).unwrap().layers[0].x_axis.tick_font.size_pt, tick);
    let fixed = p.merge_graphs(&MergeOptions { text_scale: MergeTextScale::Fixed(0.5), ..opts(&gs) }).unwrap();
    assert_eq!(
        p.graph(fixed).unwrap().layers[2].legend.font.size_pt,
        0.5 * p.graph(gs[2]).unwrap().layers[0].legend.font.size_pt
    );
}

#[test]
fn one_row_and_a_portrait_page() {
    let (mut p, gs, _) = project(3);
    let id = p.merge_graphs(&MergeOptions { rows: Some(1), cols: Some(3), portrait: true, ..opts(&gs) }).unwrap();
    let m = p.graph(id).unwrap();
    assert!(close(m.page.width_pt, 7.973 * 72.0, 1e-9) && close(m.page.height_pt, 11.41 * 72.0, 1e-9));
    // Origin 1 x 3: lefts 15, 41.67, 68.33, width 21.66, height 75
    assert_pct(&m.layers[0].frame, 15.0, 10.0, 21.667, 75.0);
    assert_pct(&m.layers[1].frame, 41.667, 10.0, 21.667, 75.0);
    assert_pct(&m.layers[2].frame, 68.333, 10.0, 21.667, 75.0);
}

#[test]
fn a_double_y_source_stays_together_in_its_cell_with_its_link() {
    let (mut p, gs, ys) = project(1);
    let dy = p.plot_columns_double_y(Template::Line, &[ys[1]], &[ys[2]]).unwrap();
    let src = p.graph(dy).unwrap().clone();
    assert_eq!(src.layers.len(), 2);
    let id = p.merge_graphs(&MergeOptions { rows: Some(2), cols: Some(1), ..opts(&[gs[0], dy]) }).unwrap();
    let m = p.graph(id).unwrap();
    assert_eq!(m.layers.len(), 3);
    // Origin: the source's left layer is the second cell, its right Y layer linked to it (layer 2)
    assert_pct(&m.layers[1].frame, 15.0, 50.0, 75.0, 35.0);
    assert_eq!(m.link_parent(2), Some(1));
    assert_eq!(m.layers[2].frame, src.layers[1].frame, "a linked layer keeps its place in its parent");
    assert_eq!(m.layers[2].link.unwrap().x, src.layers[1].link.unwrap().x);
    assert_eq!(m.panels(), vec![0, 1]);
}

#[test]
fn a_multi_panel_source_is_scaled_into_one_cell() {
    let (mut p, gs, ys) = project(1);
    let groups = vec![vec![ys[0]], vec![ys[1]]];
    let panels = p.plot_panels(Template::Line, &groups, &crate::panels::PanelLayout::stack(2)).unwrap();
    let src = p.graph(panels).unwrap().clone();
    let id = p.merge_graphs(&MergeOptions { rows: Some(1), cols: Some(2), ..opts(&[gs[0], panels]) }).unwrap();
    let m = p.graph(id).unwrap();
    assert_eq!(m.layers.len(), 3);
    // the stack's panel 0 is its only root: it fills the second cell; panel 1 follows it as before
    assert_pct(&m.layers[1].frame, 55.0, 10.0, 35.0, 75.0);
    assert_eq!(m.link_parent(2), Some(1));
    assert_eq!(m.layers[2].frame, src.layers[1].frame);
}

#[test]
fn two_roots_of_one_source_keep_their_relative_places() {
    let (mut p, gs, _) = project(2);
    // put a second, unlinked layer in the right half of the second graph
    let g = p.graph_mut(gs[1]).unwrap();
    g.layers[0].frame = Frame { left: 0.1, top: 0.1, width: 0.3, height: 0.8 };
    let mut right = g.layers[0].clone();
    right.frame = Frame { left: 0.6, top: 0.1, width: 0.3, height: 0.8 };
    g.layers.push(right);
    let id = p.merge_graphs(&MergeOptions { rows: Some(2), cols: Some(1), ..opts(&gs) }).unwrap();
    let m = p.graph(id).unwrap();
    // the box (0.1..0.9, 0.1..0.9) maps onto the cell (15, 50, 75, 35)
    assert_pct(&m.layers[1].frame, 15.0, 50.0, 28.125, 35.0);
    assert_pct(&m.layers[2].frame, 61.875, 50.0, 28.125, 35.0);
}

#[test]
fn keeping_the_aspect_ratio_puts_the_layer_at_the_top_left_of_its_cell() {
    let (mut p, gs, _) = project(3);
    let src = p.graph(gs[0]).unwrap().layers[0].frame;
    let id = p.merge_graphs(&MergeOptions { rows: Some(3), cols: Some(1), keep_aspect: true, ..opts(&gs) }).unwrap();
    let f = p.graph(id).unwrap().layers[1].frame;
    assert!(close(f.width / f.height, src.width / src.height, 1e-9));
    assert!(close(f.height, (100.0 - 25.0 - 10.0) / 300.0, 1e-9), "the height fills the cell");
    assert!(close(f.left, 0.15, 1e-12) && close(f.top, (10.0 + 21.666_666_666_666_668 + 5.0) / 100.0, 1e-9));
}

#[test]
fn linking_follows_the_first_layer_in_position_and_share_x_adds_the_scale() {
    let (mut p, gs, _) = project(3);
    let id = p.merge_graphs(&MergeOptions { rows: Some(3), cols: Some(1), link: true, ..opts(&gs) }).unwrap();
    let m = p.graph(id).unwrap().clone();
    for layer in &m.layers[1..3] {
        let link = layer.link.unwrap();
        assert_eq!((link.parent, link.x, link.y), (0, ScaleLink::None, ScaleLink::None));
    }
    // Origin: layer 2 at (0, 123, 100, 100) percent of layer 1
    let f = m.layers[1].frame;
    assert!(close(f.left, 0.0, 1e-9) && close(f.width, 1.0, 1e-9) && close(f.height, 1.0, 1e-9));
    assert!(close(f.top, (21.666_666_666_666_668 + 5.0) / 21.666_666_666_666_668, 1e-9));
    assert_pct(&m.page_frame(2), 15.0, 63.333, 75.0, 21.667);
    let shared = p.merge_graphs(&MergeOptions { rows: Some(3), cols: Some(1), share_x: true, ..opts(&gs) }).unwrap();
    let s = p.graph(shared).unwrap();
    assert_eq!(s.layers[2].link.unwrap().x, ScaleLink::Straight);
}

#[test]
fn show_axes_frame_hides_the_x_axis_of_panels_above_another() {
    let (mut p, gs, _) = project(3);
    let o = MergeOptions { rows: Some(3), cols: Some(1), v_gap: 0.0, hide_inner: true, ..opts(&gs) };
    let id = p.merge_graphs(&o).unwrap();
    let m = p.graph(id).unwrap();
    // Origin: heights 25, tops 9.992, 34.99, 59.99; the upper two show no X labels or ticks
    assert_pct(&m.layers[1].frame, 15.0, 35.0, 75.0, 25.0);
    for (i, shown) in [(0, false), (1, false), (2, true)] {
        let x = &m.layers[i].x_axis;
        assert_eq!(x.show_tick_labels, shown, "layer {i}");
        assert_eq!(x.major_tick == TickDirection::None, !shown, "layer {i}");
    }
    // with a gap nothing is hidden
    let gap = p.merge_graphs(&MergeOptions { v_gap: 5.0, ..o }).unwrap();
    assert!(p.graph(gap).unwrap().layers[0].x_axis.show_tick_labels);
}

#[test]
fn not_keeping_the_sources_deletes_them() {
    let (mut p, gs, _) = project(2);
    let id = p.merge_graphs(&MergeOptions { keep_sources: false, ..opts(&gs) }).unwrap();
    assert!(gs.iter().all(|g| p.graph(*g).is_none()));
    assert_eq!(p.graphs().map(|g| g.id).collect::<Vec<_>>(), vec![id]);
    assert!(!p.root.windows.contains(&gs[0]));
}

#[test]
fn bad_selections_are_refused_and_change_nothing() {
    let (mut p, gs, _) = project(3);
    let count = p.windows.len();
    let err = |p: &mut Project, o: MergeOptions| p.merge_graphs(&o).unwrap_err();
    assert_eq!(err(&mut p, opts(&gs[..1])), MergeError::TooFew);
    assert_eq!(err(&mut p, opts(&[gs[0], gs[0]])), MergeError::Duplicate("Graph1".into()));
    let book = p.workbooks().next().unwrap().id;
    assert_eq!(err(&mut p, opts(&[gs[0], book])), MergeError::NotAGraph(book));
    assert_eq!(err(&mut p, opts(&[gs[0], WindowId(9999)])), MergeError::NotAGraph(WindowId(9999)));
    let small = MergeOptions { rows: Some(1), cols: Some(2), ..opts(&gs) };
    assert_eq!(err(&mut p, small), MergeError::TooManyGraphs { rows: 1, cols: 2, graphs: 3 });
    assert_eq!(err(&mut p, MergeOptions { text_scale: MergeTextScale::Fixed(0.0), ..opts(&gs) }), MergeError::Factor);
    assert_eq!(err(&mut p, MergeOptions { page_in: Some((0.1, 5.0)), ..opts(&gs) }), MergeError::Page);
    assert!(matches!(err(&mut p, MergeOptions { margin_left: 95.0, ..opts(&gs) }), MergeError::Layout(_)));
    p.graph_mut(gs[1]).unwrap().layers.clear();
    assert_eq!(err(&mut p, opts(&gs)), MergeError::Empty("Graph2".into()));
    assert_eq!(p.windows.len(), count);
}

#[test]
fn a_merged_graph_saves_and_loads() {
    let (mut p, gs, _) = project(2);
    let id = p.merge_graphs(&MergeOptions { link: true, ..opts(&gs) }).unwrap();
    let g = p.graph(id).unwrap();
    let back: Graph = serde_json::from_str(&serde_json::to_string(g).unwrap()).unwrap();
    assert_eq!(&back, g);
}

#[test]
fn without_rearranging_the_layers_keep_their_source_frames() {
    let (mut p, gs, _) = project(3);
    let src = p.graph(gs[0]).unwrap().layers[0].clone();
    let o = MergeOptions { rearrange: false, rows: Some(1), cols: Some(1), ..opts(&gs) };
    let id = p.merge_graphs(&o).unwrap();
    let m = p.graph(id).unwrap();
    assert_eq!(m.layers.len(), 3, "a 1 x 1 grid is not used when the layout is not rearranged");
    for l in &m.layers {
        assert_eq!(l.frame, src.frame);
        assert_eq!(l.x_axis.tick_font.size_pt, src.x_axis.tick_font.size_pt);
    }
}

#[test]
fn a_typed_page_is_used_as_given_and_links_need_the_rearranged_layout() {
    let (mut p, gs, _) = project(2);
    let id = p.merge_graphs(&MergeOptions { page_in: Some((5.0, 10.0)), ..opts(&gs) }).unwrap();
    let g = p.graph(id).unwrap();
    assert!(close(g.page.width_pt, 360.0, 1e-9) && close(g.page.height_pt, 720.0, 1e-9));
    let o = MergeOptions { rearrange: false, link: true, share_x: true, ..opts(&gs) };
    let id = p.merge_graphs(&o).unwrap();
    assert!(p.graph(id).unwrap().layers.iter().all(|l| l.link.is_none()));
    // a fixed factor still applies without rearranging; the automatic one is 1
    let o = MergeOptions { rearrange: false, text_scale: MergeTextScale::Fixed(0.5), ..opts(&gs) };
    let id = p.merge_graphs(&o).unwrap();
    let tick = p.graph(gs[0]).unwrap().layers[0].x_axis.tick_font.size_pt;
    assert!(close(p.graph(id).unwrap().layers[0].x_axis.tick_font.size_pt, 0.5 * tick, 1e-9));
}

#[test]
fn data_labels_shrink_with_the_rest_of_the_text() {
    let (mut p, gs, _) = project(3);
    p.graph_mut(gs[0]).unwrap().layers[0].plots[0].decor.labels.font.size_pt = 20.0;
    let id = p.merge_graphs(&opts(&gs)).unwrap();
    assert!(close(p.graph(id).unwrap().layers[0].plots[0].decor.labels.font.size_pt, 20.0 * 0.46, 1e-9));
}
