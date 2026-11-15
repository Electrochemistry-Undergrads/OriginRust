//! Ridgeline charts (T44, graphing spec 14.23): bandwidths, the shared density, baselines, scaling, drawing order,
//! quantile lines, rug, points and bars.

use super::*;
use crate::data::Dim;
use crate::tests::{doc, svg_of, texts};
use sheaf_calc::kde::density_at;
use sheaf_core::graph::{PlotKind, RidgeBandwidth, RidgeDisplay, ScaleKind};
use sheaf_core::template::Template;
use sheaf_core::{ColumnData, ColumnId, Designation};

fn ridges(cols: &[Vec<f64>]) -> (Project, WindowId, Vec<ColumnId>) {
    let mut p = Project::new();
    let b = p.add_workbook("data");
    let ys: Vec<ColumnId> = cols
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let id = p.add_column(b, 0, Designation::Y, ColumnData::numeric(c.clone())).unwrap();
            p.column_mut(id).unwrap().long_name = format!("Col {}", i + 1);
            id
        })
        .collect();
    let g = p.plot_columns(Template::Ridgeline, &ys).unwrap();
    (p, g, ys)
}

fn set(p: &mut Project, g: WindowId, f: impl Fn(&mut sheaf_core::graph::RidgelineStyle)) {
    for pl in &mut p.graph_mut(g).unwrap().layers[0].plots {
        f(&mut pl.ridge);
    }
}

fn layer_data(p: &Project, g: WindowId) -> data::LayerData<'_> {
    data::collect(p, &p.graph(g).unwrap().layers[0])
}

fn sample(shift: f64, n: usize) -> Vec<f64> {
    (0..n).map(|i| shift + ((i * 7 % 13) as f64) * 0.3 + (i % 3) as f64 * 0.1).collect()
}

fn numbers(d: &str) -> Vec<(f64, f64)> {
    let n: Vec<f64> = d.split(['M', 'L', 'Z', ' ']).filter(|t| !t.is_empty()).filter_map(|t| t.parse().ok()).collect();
    n.as_chunks::<2>().0.iter().map(|c| (c[0], c[1])).collect()
}

fn plot_items(p: &Project, g: WindowId) -> Vec<scene::Item> {
    layout::build_scene(p, g, &FixedMetrics).unwrap().layers[0].plots.clone()
}

/// The filled areas in drawing order: page points of each.
fn areas(items: &[scene::Item]) -> Vec<Vec<(f64, f64)>> {
    items
        .iter()
        .filter_map(|it| match it {
            scene::Item::Path { d, fill: Some(_), stroke: None } => Some(numbers(d)),
            _ => None,
        })
        .filter(|pts| pts.len() >= 100)
        .collect()
}

#[test]
fn the_bandwidth_rules_follow_origins_formulas() {
    let v: Vec<f64> = (1..=10).map(f64::from).collect();
    let sd = (82.5f64 / 9.0).sqrt();
    // Quartiles by Origin's rule: 3 and 8, IQR 5; A = min(SD, IQR / 1.349) = SD.
    assert!(5.0 / 1.349 > sd);
    let n15 = 10f64.powf(-0.2);
    let style = |b| sheaf_core::graph::RidgelineStyle { bandwidth: b, ..Default::default() };
    let h = |b| data_ridge(&v, &style(b)).bandwidth.unwrap();
    assert!((h(RidgeBandwidth::Scott) - 0.9 * sd * n15).abs() < 1e-12);
    assert!((h(RidgeBandwidth::Silverman) - 1.059 * sd * n15).abs() < 1e-12);
    assert!((h(RidgeBandwidth::KSd(0.5)) - 0.5 * sd).abs() < 1e-12);
    assert_eq!(h(RidgeBandwidth::Custom(0.7)), 0.7);
    // A skewed sample whose IQR / 1.349 is the smaller spread (numpy: sd 7.0711, iqr/1.349 = 2.0015... by Origin's quartiles).
    let w = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 30.0];
    let n = 10f64;
    let mean = w.iter().sum::<f64>() / n;
    let sd = (w.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0)).sqrt();
    let a = sd.min(5.0 / 1.349);
    assert!(a < sd);
    assert!((h_of(&w, RidgeBandwidth::Scott) - 0.9 * a * n15).abs() < 1e-12);
    assert!((h_of(&w, RidgeBandwidth::Silverman) - 1.059 * a * n15).abs() < 1e-12);
}

fn data_ridge(v: &[f64], s: &sheaf_core::graph::RidgelineStyle) -> crate::ridgeline::RidgeData {
    crate::ridgeline::RidgeData::new(v, 1.0, "x".into(), s, &Default::default(), ScaleKind::Linear)
}

fn h_of(v: &[f64], b: RidgeBandwidth) -> f64 {
    data_ridge(v, &sheaf_core::graph::RidgelineStyle { bandwidth: b, ..Default::default() }).bandwidth.unwrap()
}

#[test]
fn the_curve_is_the_shared_kde_on_200_points_three_bandwidths_past_the_data() {
    let col = sample(1.0, 40);
    let (p, g, _) = ridges(std::slice::from_ref(&col));
    let l = layer_data(&p, g);
    let r = l.plots[0].ridge.as_ref().unwrap();
    let h = r.bandwidth.unwrap();
    assert_eq!(r.grid.len(), 200);
    let (min, max) =
        (col.iter().copied().fold(f64::INFINITY, f64::min), col.iter().copied().fold(f64::NEG_INFINITY, f64::max));
    assert!((r.grid[0] - (min - 3.0 * h)).abs() < 1e-12 && (r.grid[199] - (max + 3.0 * h)).abs() < 1e-12);
    for (x, d) in r.grid.iter().zip(&r.density) {
        assert!((d - density_at(&col, h, *x)).abs() < 1e-15);
    }
    let h2 = sheaf_calc::kde::bandwidth(&col, sheaf_calc::kde::Bandwidth::Scott).unwrap();
    assert_eq!(h, h2);
}

#[test]
fn baselines_sit_at_the_column_index_and_the_tallest_peak_is_the_ridge_height() {
    let (p, g, _) = ridges(&[sample(0.0, 30), sample(3.0, 30), (0..30).map(|i| 5.0 + (i % 5) as f64 * 0.05).collect()]);
    let l = layer_data(&p, g);
    let pos: Vec<f64> = l.plots.iter().map(|pd| pd.ridge.as_ref().unwrap().pos).collect();
    assert_eq!(pos, [1.0, 2.0, 3.0]);
    let tops: Vec<f64> = l.plots.iter().map(|pd| pd.ridge.as_ref().unwrap()).map(|r| r.peak() * r.scale).collect();
    let tallest = tops.iter().copied().fold(0.0, f64::max);
    assert!((tallest - 1.5).abs() < 1e-12, "{tops:?}");
    assert!(tops.iter().all(|t| *t <= 1.5 + 1e-12));
    let y = data::axis_range(l.plots.iter(), Dim::Y, ScaleKind::Linear).unwrap();
    assert!(y.0 <= 1.0 && (y.1 - (3.0 + tops[2])).abs() < 1e-9 && y.1 > 3.0, "{y:?}");
    // Column names label the baselines.
    let t = texts(&doc(&svg_of(&p, g)));
    for name in ["Col 1", "Col 2", "Col 3"] {
        assert!(t.iter().any(|s| s == name), "{name} in {t:?}");
    }
}

#[test]
fn ridges_are_drawn_back_to_front() {
    let (p, g, _) = ridges(&[sample(0.0, 30), sample(1.0, 30), sample(2.0, 30)]);
    let a = areas(&plot_items(&p, g));
    assert_eq!(a.len(), 3);
    // The baseline of an area is its first and last point; the highest ridge (smallest page y) first.
    let base: Vec<f64> = a.iter().map(|pts| pts[0].1).collect();
    assert!(base[0] < base[1] && base[1] < base[2], "{base:?}");
}

#[test]
fn quantile_lines_rise_from_the_baseline_to_the_curve() {
    let (mut p, g, _) = ridges(&[sample(0.0, 30), sample(1.0, 30)]);
    set(&mut p, g, |r| {
        r.quantile_lines = true;
        r.quantiles = vec![50.0];
    });
    let l = layer_data(&p, g);
    let items = plot_items(&p, g);
    let a = areas(&items);
    let unit = a[0][0].1 - a[1][0].1; // page height of one baseline spacing (ridge 2 drawn first)
    let r1 = l.plots[0].ridge.as_ref().unwrap();
    let q = sheaf_calc::stats::quantile_origin(&r1.values, 0.5);
    let want = density_at(&r1.values, r1.bandwidth.unwrap(), q) * r1.scale * unit.abs();
    let lines: Vec<Vec<(f64, f64)>> = items
        .iter()
        .filter_map(|it| match it {
            scene::Item::Path { d, fill: None, stroke: Some(s) } if s.width == 1.0 => Some(numbers(d)),
            _ => None,
        })
        .filter(|pts| pts.len() == 2)
        .collect();
    assert_eq!(lines.len(), 2);
    // Ridge 2 is drawn first; ridge 1's line is the second.
    let l1 = &lines[1];
    assert!((l1[0].0 - l1[1].0).abs() < 1e-9, "vertical");
    assert!(((l1[0].1 - l1[1].1).abs() - want).abs() < 0.01, "{} vs {want}", (l1[0].1 - l1[1].1).abs());
    assert!((l1[0].1 - a[1][0].1).abs() < 0.01, "starts on the baseline");
}

#[test]
fn rug_ticks_stand_below_the_baseline_and_points_on_it() {
    let col = sample(0.0, 20);
    let (mut p, g, _) = ridges(&[col.clone(), sample(1.0, 20)]);
    set(&mut p, g, |r| r.display = RidgeDisplay::WithRug);
    let items = plot_items(&p, g);
    let a = areas(&items);
    let unit = (a[0][0].1 - a[1][0].1).abs();
    let ticks: Vec<Vec<(f64, f64)>> = items
        .iter()
        .filter_map(|it| match it {
            scene::Item::Path { d, fill: None, stroke: Some(s) } if s.width == 0.5 => Some(numbers(d)),
            _ => None,
        })
        .filter(|pts| pts.len() == 2 * col.len())
        .collect();
    assert_eq!(ticks.len(), 2);
    let t1 = &ticks[1];
    let base = a[1][0].1;
    assert!((t1[0].1 - (base + 0.03 * unit)).abs() < 0.02 && (t1[1].1 - (base + 0.13 * unit)).abs() < 0.02);

    set(&mut p, g, |r| r.display = RidgeDisplay::WithDataPoints);
    let items = plot_items(&p, g);
    assert!(items.iter().any(|it| matches!(it, scene::Item::Path { d, .. } if d.matches('M').count() == 20)));
}

#[test]
fn single_block_bars_are_the_histogram_bins_scaled_like_the_curve() {
    let col = sample(0.0, 50);
    let (mut p, g, _) = ridges(std::slice::from_ref(&col));
    set(&mut p, g, |r| r.display = RidgeDisplay::SingleBlockBar);
    let l = layer_data(&p, g);
    let r = l.plots[0].ridge.as_ref().unwrap();
    let f = data::histogram_freq(&col, &Default::default()).unwrap();
    assert_eq!(r.bars.len(), f.rows.len());
    assert!(r.grid.is_empty());
    let total: f64 = r.bars.iter().map(|b| b.density * (b.hi - b.lo)).sum();
    assert!((total - 1.0).abs() < 1e-9);
    assert!((r.peak() * r.scale - 1.5).abs() < 1e-12);
    let a = areas_any(&plot_items(&p, g));
    // Baseline corner, two points per bar, baseline corner.
    assert_eq!(a[0].len(), 2 * f.rows.len() + 2);
}

fn areas_any(items: &[scene::Item]) -> Vec<Vec<(f64, f64)>> {
    items
        .iter()
        .filter_map(|it| match it {
            scene::Item::Path { d, fill: Some(_), stroke: None } => Some(numbers(d)),
            _ => None,
        })
        .collect()
}

#[test]
fn degenerate_columns_draw_no_curve_and_the_kind_has_no_right_axis() {
    let (p, g, _) = ridges(&[vec![3.0; 10], sample(0.0, 20)]);
    let l = layer_data(&p, g);
    assert!(l.plots[0].ridge.as_ref().unwrap().bandwidth.is_none());
    assert_eq!(areas(&plot_items(&p, g)).len(), 1);
    let mut q = Project::new();
    let b = q.add_workbook("d");
    let y = q.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![1.0, 2.0])).unwrap();
    assert!(q.plot_columns_double_y(Template::Ridgeline, &[y], &[y]).is_err());
    assert_eq!(p.graph(g).unwrap().layers[0].plots[0].kind, PlotKind::Ridgeline);
}

/// 99 values of a bell shape plus one far outlier: a narrow kernel next to a wide range.
fn outlier_column() -> Vec<f64> {
    let mut v: Vec<f64> = (1..100)
        .map(|i| {
            // Normal quantiles: bisection on the CDF integrated by Simpson's rule.
            let p = i as f64 / 100.0;
            let (mut lo, mut hi) = (-6.0f64, 6.0f64);
            for _ in 0..60 {
                let m = (lo + hi) / 2.0;
                // CDF of the normal by Simpson integration of the density.
                let n = 400;
                let h = (m + 8.0) / n as f64;
                let f = |x: f64| (-0.5 * x * x).exp() / (2.0 * std::f64::consts::PI).sqrt();
                let c: f64 = (0..n)
                    .map(|k| {
                        h / 6.0
                            * (f(-8.0 + k as f64 * h)
                                + 4.0 * f(-8.0 + (k as f64 + 0.5) * h)
                                + f(-8.0 + (k + 1) as f64 * h))
                    })
                    .sum();
                if c < p { lo = m } else { hi = m }
            }
            (lo + hi) / 2.0
        })
        .collect();
    v.push(1000.0);
    v
}

#[test]
fn a_narrow_kernel_and_a_far_outlier_still_give_a_smooth_curve_with_the_true_peak() {
    let col = outlier_column();
    let style = sheaf_core::graph::RidgelineStyle::default();
    let r = data_ridge(&col, &style);
    let h = r.bandwidth.unwrap();
    assert!(h < 0.5 && 1000.0 / 199.0 > 10.0 * h, "the even grid would be far too coarse: {h}");
    assert!(r.grid.windows(2).all(|w| w[1] > w[0]));
    // Where the kernel matters the step is at most a quarter bandwidth.
    for w in r.grid.windows(2).filter(|w| w[0] > -3.0 * h && w[1] < 3.0) {
        assert!(w[1] - w[0] <= 0.25 * h + 1e-9, "{w:?}");
    }
    // The highest drawn point is within 1 % of the true highest density (found on a fine scan).
    let truth = (0..=40000).map(|i| density_at(&col, h, -4.0 + 8.0 * i as f64 / 40000.0)).fold(0.0, f64::max);
    assert!((r.peak() - truth).abs() / truth < 0.01, "{} vs {truth}", r.peak());
    assert!(r.grid.len() < 2000);
    // Quantile lines never exceed the drawn curve.
    let mut s = style.clone();
    s.quantile_lines = true;
    for q in 0..=100 {
        let x = sheaf_calc::stats::quantile_origin(&col, q as f64 / 100.0);
        let drawn = r.density_at_x(&s, x);
        let i = r.grid.partition_point(|&g| g <= x).clamp(1, r.grid.len() - 1);
        let (x0, x1) = (r.grid[i - 1], r.grid[i]);
        let line = r.density[i - 1] + (x - x0) / (x1 - x0) * (r.density[i] - r.density[i - 1]);
        assert!(drawn <= line + 1e-12, "{q}: {drawn} > {line}");
    }
}

#[test]
fn a_log_x_axis_keeps_the_range_and_closes_the_fill_on_the_baseline() {
    let col: Vec<f64> = (1..=100).map(f64::from).collect();
    let (mut p, g, _) = ridges(std::slice::from_ref(&col));
    p.graph_mut(g).unwrap().layers[0].x_axis.scale = ScaleKind::Log10;
    let l = layer_data(&p, g);
    let r = l.plots[0].ridge.as_ref().unwrap();
    assert!(r.grid.iter().all(|&x| x > 0.0), "the grid stays inside the log domain");
    let (lo, hi) = data::axis_range(l.plots.iter(), Dim::X, ScaleKind::Log10).unwrap();
    assert!(lo <= 1.0 && hi >= 100.0 && lo > 0.0, "{lo} {hi}");
    let items = plot_items(&p, g);
    let area = areas(&items);
    assert_eq!(area.len(), 1);
    let a = &area[0];
    let base = a[0].1;
    assert!((a[a.len() - 1].1 - base).abs() < 1e-6, "ends on the baseline");
    // The polygon has every grid point: the first curve point sits above the first corner.
    assert_eq!(a.len(), r.grid.len() + 2);
    assert!((a[0].0 - a[1].0).abs() < 1e-6 && (a[a.len() - 1].0 - a[a.len() - 2].0).abs() < 1e-6);
}

#[test]
fn a_plot_added_to_a_ridgeline_layer_takes_its_ridge_settings() {
    let (mut p, g, _) = ridges(&[sample(0.0, 30)]);
    set(&mut p, g, |r| {
        r.bandwidth = RidgeBandwidth::Silverman;
        r.display = RidgeDisplay::WithRug;
    });
    let layer = &p.graph(g).unwrap().layers[0];
    let got = layer.inherited_ridge();
    assert_eq!(got.bandwidth, RidgeBandwidth::Silverman);
    assert_eq!(got.display, RidgeDisplay::WithRug);
    let mut empty = layer.clone();
    empty.plots.clear();
    assert_eq!(empty.inherited_ridge(), Default::default());
}
