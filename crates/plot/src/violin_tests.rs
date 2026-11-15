//! Violin plots (T37, graphing spec 14.22): the silhouette, the marks inside, the range and the data it leaves out.

use super::*;
use crate::data::Dim;
use crate::tests::{compare_golden, doc, svg_of, texts, with_class};
use sheaf_core::graph::PlotKind;
use sheaf_core::graph::ScaleKind;
use sheaf_core::template::Template;
use sheaf_core::{ColumnData, ColumnId, Designation};

/// A workbook with an X column and one Y column per entry of `cols`; the graph has one plot per Y column.
fn violins(template: Template, cols: &[Vec<f64>]) -> (Project, WindowId, Vec<ColumnId>) {
    let mut p = Project::new();
    let b = p.add_workbook("data");
    let n = cols.iter().map(Vec::len).max().unwrap();
    p.add_column(b, 0, Designation::X, ColumnData::numeric((1..=n).map(|i| i as f64).collect())).unwrap();
    let ys: Vec<ColumnId> = cols
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let id = p.add_column(b, 0, Designation::Y, ColumnData::numeric(c.clone())).unwrap();
            p.column_mut(id).unwrap().long_name = format!("Group {}", i + 1);
            id
        })
        .collect();
    let g = p.plot_columns(template, &ys).unwrap();
    (p, g, ys)
}

fn layer_data(p: &Project, g: WindowId) -> data::LayerData<'_> {
    data::collect(p, &p.graph(g).unwrap().layers[0])
}

/// A skewed sample: a tight cluster near 5 and a tail up to 20.
fn skewed() -> Vec<f64> {
    let mut v: Vec<f64> = (0..30).map(|i| 4.0 + (i % 7) as f64 * 0.3 + (i % 3) as f64 * 0.1).collect();
    v.extend([8.0, 9.5, 12.0, 15.0, 20.0]);
    v
}

fn numbers(d: &str) -> Vec<(f64, f64)> {
    let n: Vec<f64> = d.split(['M', 'L', 'Z', ' ']).filter(|t| !t.is_empty()).filter_map(|t| t.parse().ok()).collect();
    n.chunks_exact(2).map(|c| (c[0], c[1])).collect()
}

/// The page points of the silhouettes drawn (the filled paths without an outline), one list per plot.
fn silhouettes(p: &Project, g: WindowId) -> Vec<Vec<(f64, f64)>> {
    let s = layout::build_scene(p, g, &FixedMetrics).unwrap();
    s.layers[0]
        .plots
        .iter()
        .filter_map(|it| match it {
            scene::Item::Path { d, fill: Some(_), stroke: None } => Some(numbers(d)),
            _ => None,
        })
        // The dots of a violin with points are filled paths too, but short ones.
        .filter(|pts| pts.len() >= 2 * sheaf_calc::kde::GRID_POINTS)
        .collect()
}

#[test]
fn the_silhouette_is_symmetric_and_widest_at_the_mode() {
    let (p, g, _) = violins(Template::Violin, &[skewed()]);
    let sil = &silhouettes(&p, g)[0];
    assert_eq!(sil.len(), 2 * sheaf_calc::kde::GRID_POINTS);
    let (right, left) = sil.split_at(sil.len() / 2);
    let cx =
        (right.iter().map(|q| q.0).fold(f64::MIN, f64::max) + left.iter().map(|q| q.0).fold(f64::MAX, f64::min)) / 2.0;
    for (r, l) in right.iter().zip(left.iter().rev()) {
        assert!((r.1 - l.1).abs() < 0.01, "same height");
        assert!(((r.0 - cx) + (l.0 - cx)).abs() < 0.01, "mirrored: {r:?} {l:?}");
    }
    // The widest point is at the mode of the data (the cluster near 5), not at the middle of the range.
    let widest = right.iter().max_by(|a, b| a.0.total_cmp(&b.0)).unwrap();
    let v = layer_data(&p, g).plots[0].violin.clone().unwrap();
    let i = v.half_width.iter().position(|w| *w == v.half_width.iter().copied().fold(0.0, f64::max)).unwrap();
    assert!((4.0..6.5).contains(&v.grid[i]), "mode at {}", v.grid[i]);
    assert!(v.half_width[i] > 0.0 && (v.half_width[i] - 0.4).abs() < 1e-12, "peak half width is 0.4 X units");
    // Page: the right edge of the widest row is at that peak.
    assert!(right.iter().all(|q| q.0 <= widest.0 + 1e-9));
}

#[test]
fn the_curve_reaches_two_bandwidths_past_the_data_and_the_ranges_cover_it() {
    let data = skewed();
    let (p, g, _) = violins(Template::Violin, &[data.clone()]);
    let l = layer_data(&p, g);
    let v = l.plots[0].violin.as_ref().unwrap();
    let w = v.bandwidth.unwrap();
    let (min, max) = (4.0, 20.0);
    assert!((v.grid[0] - (min - 2.0 * w)).abs() < 1e-9 && (v.grid[v.grid.len() - 1] - (max + 2.0 * w)).abs() < 1e-9);
    let y = data::axis_range(l.plots.iter(), Dim::Y, ScaleKind::Linear).unwrap();
    assert!((y.0 - v.grid[0]).abs() < 1e-9 && (y.1 - v.grid[v.grid.len() - 1]).abs() < 1e-9, "{y:?}");
    let x = data::axis_range(l.plots.iter(), Dim::X, ScaleKind::Linear).unwrap();
    assert!((x.0 - 0.6).abs() < 1e-12 && (x.1 - 1.4).abs() < 1e-12, "{x:?}");
}

#[test]
fn violins_stand_at_1_2_3_with_the_column_names_as_tick_labels() {
    let (p, g, _) = violins(Template::Violin, &[skewed(), skewed(), skewed()]);
    let l = layer_data(&p, g);
    let pos: Vec<f64> = l.plots.iter().map(|pd| pd.boxed.as_ref().unwrap().pos).collect();
    assert_eq!(pos, [1.0, 2.0, 3.0]);
    let svg = svg_of(&p, g);
    let t = texts(&doc(&svg));
    for name in ["Group 1", "Group 2", "Group 3"] {
        assert!(t.iter().any(|s| s == name), "{name} in {t:?}");
    }
    assert_eq!(silhouettes(&p, g).len(), 3);
}

#[test]
fn the_box_inside_is_the_box_charts_box() {
    let cols = [skewed(), vec![1.0, 2.0, 2.5, 3.0, 9.0]];
    let (pb, gb, _) = violins(Template::BoxChart, &cols);
    let (pv, gv, _) = violins(Template::ViolinBox, &cols);
    let (lb, lv) = (layer_data(&pb, gb), layer_data(&pv, gv));
    for (b, v) in lb.plots.iter().zip(&lv.plots) {
        assert_eq!(b.boxed, v.boxed);
        assert!(b.violin.is_none() && v.violin.is_some());
    }
    // The drawn inner box runs from the quartile heights: a filled rect, its height the box's.
    let s = layout::build_scene(&pv, gv, &FixedMetrics).unwrap();
    let rects: Vec<_> = s.layers[0]
        .plots
        .iter()
        .filter_map(|it| match it {
            scene::Item::Rect { rect, fill: Some(_), .. } => Some(*rect),
            _ => None,
        })
        .collect();
    assert_eq!(rects.len(), 2, "one inner box per violin");
    assert!(rects.iter().all(|r| r.w > 0.0 && r.h > 0.0));
}

#[test]
fn each_kind_draws_its_own_marks_and_none_draws_data_labels_or_symbols() {
    let n = skewed().len();
    let count = |t: Template| {
        let (p, g, _) = violins(t, &[skewed()]);
        let s = layout::build_scene(&p, g, &FixedMetrics).unwrap();
        let items = &s.layers[0].plots;
        let paths = items.iter().filter(|i| matches!(i, scene::Item::Path { .. })).count();
        let rects = items.iter().filter(|i| matches!(i, scene::Item::Rect { .. })).count();
        let texts = items.iter().filter(|i| matches!(i, scene::Item::Text(_))).count();
        // stick lines: segments of the unfilled path with the thin stroke
        let sticks: usize = items
            .iter()
            .filter_map(|i| match i {
                scene::Item::Path { d, fill: None, stroke: Some(st) } if st.width == 0.5 && st.dash.is_empty() => {
                    Some(d.matches('M').count())
                }
                _ => None,
            })
            .sum();
        (paths, rects, texts, sticks)
    };
    let plain = count(Template::Violin);
    assert_eq!(plain.1, 0, "no box");
    assert_eq!(plain.2, 0);
    assert_eq!(count(Template::ViolinBox).1, 1);
    assert_eq!(count(Template::HalfViolin).1, 1);
    assert_eq!(count(Template::ViolinStick).3, n + 1, "a line per observation, and the outline");
    assert!(
        count(Template::ViolinQuartile).0 == plain.0 + 1,
        "a dashed and a solid path for the quartile and median lines, instead of the median dot"
    );
    assert_eq!(count(Template::ViolinPoint).0, plain.0 + 1, "one path of dots");
}

#[test]
fn a_half_violin_lies_right_of_its_middle_line_with_the_box_on_the_left() {
    let (p, g, _) = violins(Template::HalfViolin, &[skewed()]);
    let sil = &silhouettes(&p, g)[0];
    let (right, left) = sil.split_at(sil.len() / 2);
    let cx = left[0].0;
    assert!(left.iter().all(|q| (q.0 - cx).abs() < 0.01), "the left edge is the straight middle line");
    assert!(right.iter().all(|q| q.0 >= cx - 0.01));
    let s = layout::build_scene(&p, g, &FixedMetrics).unwrap();
    let rect = s.layers[0]
        .plots
        .iter()
        .find_map(|it| match it {
            scene::Item::Rect { rect, fill: Some(_), .. } => Some(*rect),
            _ => None,
        })
        .unwrap();
    assert!(rect.x + rect.w <= cx, "half box left of the middle line");
}

#[test]
fn missing_values_filtered_rows_and_masked_rows_are_left_out() {
    // Missing values are not data.
    let mut c = skewed();
    c.extend([f64::NAN, f64::NAN]);
    let (p, g, _) = violins(Template::Violin, &[c]);
    let a = layer_data(&p, g).plots[0].violin.clone().unwrap();
    let (p0, g0, _) = violins(Template::Violin, &[skewed()]);
    assert_eq!(a, layer_data(&p0, g0).plots[0].violin.clone().unwrap());

    // A masked row (the 20 at row 34) is not data.
    let (mut p, g, ys) = violins(Template::Violin, &[skewed()]);
    let row = skewed().iter().position(|v| *v == 20.0).unwrap();
    p.column_mut(ys[0]).unwrap().mask.insert(row);
    let l = layer_data(&p, g);
    let v = l.plots[0].violin.as_ref().unwrap();
    assert_eq!(*v.values.last().unwrap(), 15.0);
    assert_eq!(v.values.len(), skewed().len() - 1);
    assert_eq!(
        l.plots[0].boxed.as_ref().unwrap().whisker_high.max(15.0),
        15.0_f64.max(l.plots[0].boxed.as_ref().unwrap().whisker_high)
    );
    svg_of(&p, g);

    // The same with the box chart: masks do not change the box chart (it never skipped them).
    let (mut pb, gb, ys) = violins(Template::BoxChart, &[skewed()]);
    pb.column_mut(ys[0]).unwrap().mask.insert(row);
    let (pb0, gb0, _) = violins(Template::BoxChart, &[skewed()]);
    assert_eq!(layer_data(&pb, gb).plots[0].boxed, layer_data(&pb0, gb0).plots[0].boxed);

    // A worksheet filter hides rows: the 1000 never enters the density.
    let (mut p, g, _) = violins(Template::Violin, &[vec![1.0, 2.0, 1000.0, 3.0, 2.5]]);
    let book = p.workbooks().next().unwrap().id;
    p.set_row_filter(book, 0, "col(B) < 100").unwrap();
    let v = layer_data(&p, g).plots[0].violin.clone().unwrap();
    assert_eq!(v.values, [1.0, 2.0, 2.5, 3.0]);
}

#[test]
fn too_few_values_or_no_spread_draw_no_silhouette_but_still_render() {
    for col in [vec![5.0], vec![3.0, 3.0, 3.0], vec![f64::NAN, 2.0], vec![]] {
        for t in Template::ALL.into_iter().filter(Template::is_violin) {
            let (p, g, _) = violins(t, &[vec![1.0, 2.0, 3.0], col.clone()]);
            let l = layer_data(&p, g);
            if !col.iter().any(|v| v.is_finite()) {
                assert_eq!(l.plots.len(), 1, "an empty column has no violin: {t:?}");
            } else {
                assert!(l.plots[1].violin.as_ref().unwrap().grid.is_empty(), "{col:?}");
            }
            let svg = svg_of(&p, g);
            doc(&svg);
            assert_eq!(silhouettes(&p, g).len(), 1, "only the first column has a silhouette: {t:?} {col:?}");
        }
    }
}

#[test]
fn the_legend_shows_a_filled_sample_in_the_plot_colour() {
    let (p, g, _) = violins(Template::Violin, &[skewed()]);
    let svg = svg_of(&p, g);
    // The tinted fill of the silhouette (the plot's red toward white) is also the legend's sample, which comes
    // after the plots in the overlay.
    let overlay = &svg[svg.find("class=\"overlay\"").unwrap()..];
    assert!(overlay.contains("#FF9999"), "{overlay}");
    assert!(svg[..svg.find("class=\"overlay\"").unwrap()].contains("#FF9999"));
}

#[test]
fn a_user_set_fill_transparency_applies_to_the_silhouette() {
    let (mut p, g, _) = violins(Template::Violin, &[skewed()]);
    let auto = svg_of(&p, g);
    p.graph_mut(g).unwrap().layers[0].plots[0].decor.fill.transparency_pct = 50;
    let set = svg_of(&p, g);
    assert_ne!(auto, set);
    assert_eq!(layer_data(&p, g).plots[0].plot.kind, PlotKind::Violin);
}

#[test]
fn golden_violin_template() {
    let cols = [
        (0..24).map(|i| 8.0 + 3.0 * ((i as f64) * 0.7).sin() + (i % 5) as f64 * 0.2).collect::<Vec<_>>(),
        skewed(),
        (0..30).map(|i| if i % 2 == 0 { 3.0 + (i % 6) as f64 * 0.25 } else { 11.0 + (i % 7) as f64 * 0.3 }).collect(),
    ];
    let (p, g, _) = violins(Template::ViolinBox, &cols);
    compare_golden("violin", &svg_of(&p, g));
}
