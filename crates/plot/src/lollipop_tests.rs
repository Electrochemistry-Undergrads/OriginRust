//! Drop lines to the next data plot and the Plus-Minus colour (T33, lollipop plot).

use super::*;
use sheaf_core::graph::{DropColorMode, DropTarget, PLUS_MINUS_NEGATIVE, PLUS_MINUS_POSITIVE};
use sheaf_core::template::Template;
use sheaf_core::{Color, ColumnData, ColumnId, Designation};

const NAN: f64 = f64::NAN;

/// A workbook with an X column (1, 2, 3, ...) and one Y column per entry of `cols`.
fn book(cols: &[Vec<f64>]) -> (Project, ColumnId, Vec<ColumnId>) {
    let mut p = Project::new();
    let b = p.add_workbook("data");
    let n = cols[0].len();
    let x = p.add_column(b, 0, Designation::X, ColumnData::numeric((1..=n).map(|i| i as f64).collect())).unwrap();
    let ys = cols.iter().map(|c| p.add_column(b, 0, Designation::Y, ColumnData::numeric(c.clone())).unwrap()).collect();
    (p, x, ys)
}

/// An upright scatter graph (not exchanged) whose plots have vertical drop lines to the next plot.
fn upright(cols: &[Vec<f64>], mode: DropColorMode) -> (Project, WindowId) {
    let (mut p, x, ys) = book(cols);
    let g = p.add_graph(Template::Scatter);
    for y in &ys {
        p.add_plot_xy(g, 0, Some(x), *y, None).unwrap();
    }
    for pl in &mut p.graph_mut(g).unwrap().layers[0].plots {
        let d = &mut pl.decor.drop.vertical;
        (d.on, d.target, d.color_mode) = (true, DropTarget::NextDataPlot, mode);
    }
    (p, g)
}

/// The unfilled paths drawn (drop lines; symbols are filled) as (x0, y0, x1, y1, colour) segments.
fn segments(p: &Project, g: WindowId) -> Vec<(f64, f64, f64, f64, Color)> {
    let s = layout::build_scene(p, g, &FixedMetrics).unwrap();
    let mut out = Vec::new();
    for item in &s.layers[0].plots {
        if let scene::Item::Path { d, fill: None, stroke: Some(st) } = item {
            let nums: Vec<f64> =
                d.split(['M', 'L', ' ']).filter(|t| !t.is_empty()).filter_map(|t| t.parse().ok()).collect();
            for c in nums.as_chunks::<4>().0 {
                out.push((c[0], c[1], c[2], c[3], st.color));
            }
        }
    }
    out
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 0.05
}

#[test]
fn three_plots_join_in_turn_and_the_last_returns_to_the_first() {
    let cols = [vec![1.0, 2.0], vec![2.0, 4.0], vec![4.0, 1.0]];
    let (p, g) = upright(&cols, DropColorMode::Auto);
    let segs = segments(&p, g);
    assert_eq!(segs.len(), 6, "{segs:?}");
    // Row 1 has the values 1, 2, 4 on the plots 1, 2, 3; each plot's path lists its segments by row.
    let x0 = segs.iter().map(|s| s.0).fold(f64::MAX, f64::min);
    let row1: Vec<_> = segs.iter().filter(|s| near(s.0, x0)).collect();
    assert_eq!(row1.len(), 3);
    let (a_b, b_c, c_a) = (row1[0], row1[1], row1[2]);
    assert!(near(a_b.3, b_c.1) && near(b_c.3, c_a.1) && near(c_a.3, a_b.1), "{row1:?}");
    // Page y runs down: 4 is above 2, which is above 1, and 2 to 4 is twice 1 to 2.
    let (ya, yb, yc) = (a_b.1, a_b.3, b_c.3);
    assert!(yc < yb && yb < ya);
    assert!(near(yb - yc, (ya - yb) * 2.0));
    // Vertical: both ends of a segment stand over the same X.
    assert!(segs.iter().all(|s| near(s.0, s.2)));
}

#[test]
fn two_plots_draw_each_segment_once() {
    let cols = [vec![1.0, 2.0, 3.0], vec![3.0, 1.0, 2.0]];
    let (p, g) = upright(&cols, DropColorMode::Auto);
    assert_eq!(segments(&p, g).len(), 3, "one segment per row, not two");
}

#[test]
fn a_single_plot_has_no_segments() {
    let (p, g) = upright(&[vec![1.0, 2.0, 3.0]], DropColorMode::Auto);
    assert!(segments(&p, g).is_empty());
}

#[test]
fn a_missing_value_gives_no_segment_to_or_from_it() {
    let cols = [vec![1.0, NAN, 3.0], vec![2.0, 2.0, 1.0], vec![3.0, 3.0, NAN]];
    let (p, g) = upright(&cols, DropColorMode::Auto);
    // Row 1 has all three segments; row 2 only 2->3 (plot 1 is missing); row 3 only 1->2 (plot 3 is missing).
    assert_eq!(segments(&p, g).len(), 3 + 1 + 1);
}

#[test]
fn the_exchanged_lollipop_runs_along_the_value_axis() {
    let (mut p, _x, ys) = book(&[vec![1.0, 2.0, 3.0], vec![2.0, 4.0, 1.0], vec![4.0, 3.0, 5.0]]);
    let g = p.plot_columns(Template::Lollipop, &ys).unwrap();
    let layer = &p.graph(g).unwrap().layers[0];
    // X and Y are traded: the Y columns are plotted along X.
    assert!(layer.plots.iter().all(|pl| ys.contains(&pl.x.unwrap())));
    assert_eq!(layer.default_kind, sheaf_core::graph::PlotKind::Scatter);
    let segs = segments(&p, g);
    assert_eq!(segs.len(), 9, "{segs:?}");
    // Sideways: both ends of a segment are at the same height (the row's X value).
    assert!(segs.iter().all(|s| near(s.1, s.3)), "{segs:?}");
    // The first row holds 1, 2, 4 along the horizontal axis: 1->2 goes right, 2->4 right, 4->1 left.
    let y0 = segs[0].1;
    let row1: Vec<_> = segs.iter().filter(|s| near(s.1, y0)).collect();
    assert_eq!(row1.len(), 3);
    assert!(row1[0].2 > row1[0].0 && row1[1].2 > row1[1].0 && row1[2].2 < row1[2].0, "{row1:?}");
}

#[test]
fn plus_minus_colours_each_segment_by_the_sign_of_its_value() {
    let cols = [vec![1.0, -1.0], vec![2.0, -2.0], vec![-3.0, 3.0]];
    let (p, g) = upright(&cols, DropColorMode::PlusMinus);
    let segs = segments(&p, g);
    assert_eq!(segs.len(), 6);
    let (pos, neg) = (PLUS_MINUS_POSITIVE, PLUS_MINUS_NEGATIVE);
    assert_ne!(pos, neg);
    let x0 = segs.iter().map(|s| s.0).fold(f64::MAX, f64::min);
    let count = |first_row: bool, c: Color| segs.iter().filter(|s| near(s.0, x0) == first_row && s.4 == c).count();
    // Segments start at their own plot's point. Row 1: 1, 2, -3; row 2: -1, -2, 3.
    assert_eq!((count(true, pos), count(true, neg)), (2, 1));
    assert_eq!((count(false, pos), count(false, neg)), (1, 2));
}

#[test]
fn plus_minus_applies_to_drops_to_the_axis_too_and_a_fixed_colour_wins() {
    let (mut p, g) = upright(&[vec![1.0, -1.0], vec![2.0, 2.0]], DropColorMode::PlusMinus);
    for pl in &mut p.graph_mut(g).unwrap().layers[0].plots {
        pl.decor.drop.vertical.target = DropTarget::Axis;
    }
    let segs = segments(&p, g);
    assert_eq!(segs.len(), 4);
    assert!(segs.iter().any(|s| s.4 == PLUS_MINUS_NEGATIVE));
    let red = Color::rgb(255, 0, 0);
    for pl in &mut p.graph_mut(g).unwrap().layers[0].plots {
        pl.decor.drop.vertical.color = Some(red);
    }
    assert!(segments(&p, g).iter().all(|s| s.4 == red));
}

#[test]
fn the_default_drop_to_axis_is_unchanged() {
    let (mut p, g) = upright(&[vec![1.0, 2.0], vec![2.0, 1.0]], DropColorMode::Auto);
    for pl in &mut p.graph_mut(g).unwrap().layers[0].plots {
        pl.decor.drop.vertical.target = DropTarget::Axis;
    }
    let segs = segments(&p, g);
    assert_eq!(segs.len(), 4);
    // Every drop ends on the bottom edge of the frame: one common end height.
    assert!(segs.iter().all(|s| near(s.3, segs[0].3)));
}

/// Three plots with one worksheet each (X 1..=3); the next plot of a lollipop is on its own sheet.
fn two_books(a: Vec<f64>, b: Vec<f64>) -> (Project, WindowId, WindowId) {
    let mut p = Project::new();
    let mut plots = Vec::new();
    let mut books = Vec::new();
    for ys in [a, b] {
        let bk = p.add_workbook("d");
        let n = ys.len();
        let x = p.add_column(bk, 0, Designation::X, ColumnData::numeric((1..=n).map(|i| i as f64).collect())).unwrap();
        let y = p.add_column(bk, 0, Designation::Y, ColumnData::numeric(ys)).unwrap();
        plots.push((x, y));
        books.push(bk);
    }
    let g = p.add_graph(Template::Scatter);
    for (x, y) in plots {
        p.add_plot_xy(g, 0, Some(x), y, None).unwrap();
    }
    p.graph_mut(g).unwrap().layers[0].plots[0].decor.drop.vertical.on = true;
    p.graph_mut(g).unwrap().layers[0].plots[0].decor.drop.vertical.target = DropTarget::NextDataPlot;
    (p, g, books[1])
}

#[test]
fn no_segment_ends_at_a_row_the_next_plot_does_not_draw() {
    let (mut p, g, b2) = two_books(vec![1.0, 2.0, 3.0], vec![3.0, 1.0, 2.0]);
    assert_eq!(segments(&p, g).len(), 3);
    // A row filter on the next plot's worksheet hides row 2 (X = 2).
    p.set_row_filter(b2, 0, "col(A) != 2").unwrap();
    let segs = segments(&p, g);
    assert_eq!(segs.len(), 2, "{segs:?}");
    // A row range on the next plot that starts after row 1 drops that row's segment too.
    p.remove_row_filter(b2, 0);
    p.graph_mut(g).unwrap().layers[0].plots[1].rows = Some((1, 3));
    assert_eq!(segments(&p, g).len(), 2);
}
