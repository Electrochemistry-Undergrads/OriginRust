//! Violin plots (T37, graphing spec 14.22): the silhouette, the marks inside, the range and the data it leaves out.

use super::*;
use crate::data::Dim;
use crate::tests::{compare_golden, doc, svg_of, texts};
use sheaf_core::Color;
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
    n.as_chunks::<2>().0.iter().map(|c| (c[0], c[1])).collect()
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

/// The page x of the tick label `name` (its anchor is the tick position).
fn tick_x(p: &Project, g: WindowId, name: &str) -> f64 {
    let s = layout::build_scene(p, g, &FixedMetrics).unwrap();
    s.layers[0]
        .overlay
        .iter()
        .find_map(|it| match it {
            scene::Item::Text(t) if t.text == name => Some(t.x),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no tick label {name}"))
}

#[test]
fn every_violin_is_centred_on_its_tick() {
    let names = ["Group 1", "Group 2", "Group 3"];
    for (template, half) in [(Template::Violin, false), (Template::ViolinBox, false), (Template::HalfViolin, true)] {
        let (p, g, _) = violins(template, &[skewed(), skewed(), skewed()]);
        let sils = silhouettes(&p, g);
        assert_eq!(sils.len(), 3);
        let s = layout::build_scene(&p, g, &FixedMetrics).unwrap();
        let rects: Vec<crate::Rect> = s.layers[0]
            .plots
            .iter()
            .filter_map(|it| match it {
                scene::Item::Rect { rect, fill: Some(_), .. } => Some(*rect),
                _ => None,
            })
            .collect();
        for (i, name) in names.iter().enumerate() {
            let tick = tick_x(&p, g, name);
            let sil = &sils[i];
            let (right, left) = sil.split_at(sil.len() / 2);
            let max_x = right.iter().map(|q| q.0).fold(f64::MIN, f64::max);
            let min_x = left.iter().map(|q| q.0).fold(f64::MAX, f64::min);
            if half {
                // The flat edge is the tick; the curve lies to its right.
                assert!((min_x - tick).abs() < 0.01, "{template:?} {name}: flat edge {min_x} tick {tick}");
                assert!(max_x > tick + 1.0);
                assert!(rects[i].x + rects[i].w < tick, "the half box stands left of the tick");
            } else {
                assert!(
                    ((min_x + max_x) / 2.0 - tick).abs() < 0.01,
                    "{template:?} {name}: {min_x}..{max_x} tick {tick}"
                );
            }
            if template == Template::ViolinBox {
                let r = rects[i];
                assert!((r.x + r.w / 2.0 - tick).abs() < 0.01, "the inner box is centred on the tick");
            }
        }
    }
}

#[test]
fn the_silhouette_is_symmetric_and_widest_at_the_mode() {
    let (p, g, _) = violins(Template::Violin, &[skewed()]);
    let sil = &silhouettes(&p, g)[0];
    let v = layer_data(&p, g).plots[0].violin.clone().unwrap();
    assert_eq!(sil.len(), 2 * v.grid.len());
    assert!(v.grid.len() >= sheaf_calc::kde::GRID_POINTS);
    let (right, left) = sil.split_at(sil.len() / 2);
    let cx =
        (right.iter().map(|q| q.0).fold(f64::MIN, f64::max) + left.iter().map(|q| q.0).fold(f64::MAX, f64::min)) / 2.0;
    for (r, l) in right.iter().zip(left.iter().rev()) {
        assert!((r.1 - l.1).abs() < 0.01, "same height");
        assert!(((r.0 - cx) + (l.0 - cx)).abs() < 0.01, "mirrored: {r:?} {l:?}");
    }
    // The widest point is at the mode of the data (the cluster near 5), not at the middle of the range.
    let widest = right.iter().max_by(|a, b| a.0.total_cmp(&b.0)).unwrap();
    let i = v.half_width.iter().position(|w| *w == v.half_width.iter().copied().fold(0.0, f64::max)).unwrap();
    assert!((4.0..6.5).contains(&v.grid[i]), "mode at {}", v.grid[i]);
    assert!(v.half_width[i] > 0.0 && (v.half_width[i] - 0.4).abs() < 1e-12, "peak half width is 0.4 X units");
    // Page: the right edge of the widest row is at that peak.
    assert!(right.iter().all(|q| q.0 <= widest.0 + 1e-9));
}

#[test]
fn the_curve_reaches_two_bandwidths_past_the_data_and_the_ranges_cover_it() {
    let data = skewed();
    let (p, g, _) = violins(Template::Violin, std::slice::from_ref(&data));
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
    // Height: the box spans q1..q3 of the box chart's statistics. The violin's Y axis is its own, so map
    // data to page through the silhouette's own ends (the Y axis is linear).
    let sils = silhouettes(&pv, gv);
    for (i, (r, b)) in rects.iter().zip(&lv.plots).enumerate() {
        let v = b.violin.as_ref().unwrap();
        let sil = &sils[i];
        let right = &sil[..sil.len() / 2];
        let (y0, y1) = (right[0].1, right[right.len() - 1].1);
        let (g0, g1) = (v.grid[0], v.grid[v.grid.len() - 1]);
        let page = |y: f64| y0 + (y - g0) / (g1 - g0) * (y1 - y0);
        let bs = b.boxed.as_ref().unwrap();
        assert!((r.y - page(bs.q3)).abs() < 0.01, "top of the box at q3: {} vs {}", r.y, page(bs.q3));
        assert!((r.y + r.h - page(bs.q1)).abs() < 0.01, "bottom of the box at q1");
        assert!(r.w > 0.0);
    }
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
fn quartile_lines_and_points_are_told_apart_by_their_marks() {
    let n = skewed().len();
    let marks = |t: Template| {
        let (p, g, _) = violins(t, &[skewed()]);
        let s = layout::build_scene(&p, g, &FixedMetrics).unwrap();
        let plots = s.layers[0].plots.clone();
        // dashed segments, thick solid segments, dots (a filled path without outline other than the silhouette)
        let (mut dashed, mut thick, mut dots, mut median_dots) = (0, 0, 0, 0);
        for it in &plots {
            if let scene::Item::Path { d, fill, stroke } = it {
                match (fill, stroke) {
                    (None, Some(st)) if !st.dash.is_empty() => dashed += d.matches('M').count(),
                    (None, Some(st)) if st.width == 2.0 => thick += d.matches('M').count(),
                    (Some(_), None) if d.matches('M').count() > 1 => dots += d.matches('M').count(),
                    (Some(c), Some(_)) if *c == Color::WHITE => median_dots += 1,
                    _ => {}
                }
            }
        }
        (dashed, thick, dots, median_dots)
    };
    assert_eq!(marks(Template::ViolinQuartile), (2, 1, 0, 0), "quartiles dashed, median solid, no dots");
    assert_eq!(marks(Template::ViolinPoint), (0, 0, n, 1), "a dot per value and the median dot, no lines");
    assert_eq!(marks(Template::Violin), (0, 0, 0, 1), "the median dot only");
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
    // The inner box statistics leave the masked 20 out too: they equal those of the column without it.
    let mut kept = skewed();
    kept.retain(|v| *v != 20.0);
    let (pk, gk, _) = violins(Template::Violin, &[kept]);
    let boxed = l.plots[0].boxed.as_ref().unwrap();
    assert_eq!(Some(boxed), layer_data(&pk, gk).plots[0].boxed.as_ref());
    assert!(boxed.whisker_high < 20.0 && !boxed.outliers.contains(&20.0));
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
    let fill_of = |p: &Project, g: WindowId| {
        let s = layout::build_scene(p, g, &FixedMetrics).unwrap();
        s.layers[0]
            .plots
            .iter()
            .find_map(|it| match it {
                scene::Item::Path { d, fill: Some(c), stroke: None }
                    if d.matches('M').count() >= 1 && d.len() > 1000 =>
                {
                    Some(*c)
                }
                _ => None,
            })
            .unwrap()
    };
    let (mut p, g, _) = violins(Template::Violin, &[skewed()]);
    let auto = fill_of(&p, g);
    assert_eq!((auto.r, auto.g, auto.b, auto.a), (255, 153, 153, 255), "plain fill: the plot colour 60 % toward white");
    p.graph_mut(g).unwrap().layers[0].plots[0].decor.fill.transparency_pct = 50;
    let set = fill_of(&p, g);
    assert_eq!((set.r, set.g, set.b), (255, 0, 0), "the plot colour itself, which the transparency applies to");
    assert!((127..=128).contains(&set.a), "50 % transparent: alpha {}", set.a);
    assert_eq!(layer_data(&p, g).plots[0].plot.kind, PlotKind::Violin);
}

#[test]
fn far_outliers_leave_the_silhouette_with_a_bulk() {
    // 200 values near 0 and three far ones: the bulk must show a shape and carry the widest point.
    let mut c: Vec<f64> = (0..200).map(|i| ((i * 37 % 101) as f64 - 50.0) / 25.0).collect();
    c.extend([1e5, 1.1e5, 1.2e5]);
    let (p, g, _) = violins(Template::Violin, &[c]);
    let v = layer_data(&p, g).plots[0].violin.clone().unwrap();
    let w = v.bandwidth.unwrap();
    let i = v.half_width.iter().position(|h| *h == v.half_width.iter().copied().fold(0.0, f64::max)).unwrap();
    assert!(v.grid[i].abs() < 3.0, "widest at {}", v.grid[i]);
    let in_bulk = v.grid.iter().filter(|y| y.abs() < 2.0).count();
    assert!(in_bulk > 30, "{in_bulk} grid points in the bulk");
    assert!(v.grid.windows(2).all(|q| q[1] - q[0] <= 0.25 * w + 1e-9 || q[0] >= 2.0 + 3.0 * w - 1e-9));
    assert!((v.half_width[i] - 0.4).abs() < 1e-12);
    // The outlier bumps are drawn too.
    assert!(v.half_width_at(1.1e5) > 0.0);
    svg_of(&p, g);
}

#[test]
fn a_log_axis_keeps_a_range_covering_the_data_and_draws_the_curve_above_zero() {
    let data = vec![0.5, 0.6, 0.8, 1.0, 1.2, 1.5, 2.0, 3.0, 5.0, 9.0, 20.0, 50.0];
    let (mut p, g, _) = violins(Template::ViolinBox, std::slice::from_ref(&data));
    p.graph_mut(g).unwrap().layers[0].y_axis.scale = ScaleKind::Log10;
    let l = layer_data(&p, g);
    let v = l.plots[0].violin.as_ref().unwrap();
    assert!(v.grid[0] <= 0.0, "the curve reaches below zero: {}", v.grid[0]);
    let (lo, hi) = data::axis_range(l.plots.iter(), Dim::Y, ScaleKind::Log10).unwrap();
    assert_eq!(lo, 0.5, "the data minimum starts the range");
    assert!((hi - v.grid[v.grid.len() - 1]).abs() < 1e-9, "the upper end of the curve ends it");
    assert!(lo < hi);
    // Drawn: finite numbers only, a silhouette made of the points above zero, and the whole svg is valid.
    let svg = svg_of(&p, g);
    assert!(!svg.contains("NaN") && !svg.contains("inf"), "no non-finite coordinate");
    doc(&svg);
    let s = layout::build_scene(&p, g, &FixedMetrics).unwrap();
    let sil: Vec<_> = s.layers[0]
        .plots
        .iter()
        .filter_map(|it| match it {
            scene::Item::Path { d, fill: Some(_), stroke: None } => Some(numbers(d)),
            _ => None,
        })
        .max_by_key(Vec::len)
        .unwrap();
    let above_zero = v.grid.iter().filter(|y| **y > 0.0).count();
    assert_eq!(sil.len(), 2 * above_zero, "the points at or below zero are cut");
    assert!(sil.len() >= 3 && sil.iter().all(|q| q.0.is_finite() && q.1.is_finite()));
    // The same range with a data set that has no curve below zero is unchanged by the data additions.
    let (pl, gl, _) = violins(Template::ViolinBox, &[data]);
    let l = layer_data(&pl, gl);
    let v = l.plots[0].violin.as_ref().unwrap();
    let y = data::axis_range(l.plots.iter(), Dim::Y, ScaleKind::Linear).unwrap();
    assert_eq!(y, (v.grid[0], v.grid[v.grid.len() - 1]));
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
