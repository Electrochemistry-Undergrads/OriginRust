//! Grouped columns and grouped box charts of indexed data (T76): the categories, the mean and error of each bar,
//! unbalanced subgroups, the boxes and where they stand, and the axis range.

use super::*;
use crate::data::{BOX_WIDTH, Dim, box_stats};
use crate::tests::{compare_golden, doc, svg_of, texts};
use sheaf_core::graph::{IndexedError, PlotKind};
use sheaf_core::template::Template;
use sheaf_core::{ColumnData, ColumnId, Designation};

/// Group G (c, a, b ... in sheet order), subgroup S, and the data: a/u {1, 3}, a/v {10}, b/u {2, 4, 6},
/// c/v {20, 30, 25, 100}; one row without a group, one without a value (both left out). Group c has no u,
/// group b has no v.
fn indexed(template: Template, two_groups: bool) -> (Project, WindowId, ColumnId, ColumnId, ColumnId) {
    let rows: [(&str, &str, f64); 12] = [
        ("c", "v", 20.0),
        ("a", "u", 1.0),
        ("b", "u", 2.0),
        ("a", "u", 3.0),
        ("c", "v", 30.0),
        ("b", "u", 4.0),
        ("a", "v", 10.0),
        ("c", "v", 25.0),
        ("b", "u", 6.0),
        ("c", "v", 100.0),
        ("", "u", 50.0),
        ("a", "u", f64::NAN),
    ];
    let mut p = Project::new();
    let b = p.add_workbook("data");
    let text = |p: &mut Project, f: &dyn Fn(&(&str, &str, f64)) -> String| {
        p.add_column(b, 0, Designation::X, ColumnData::text(rows.iter().map(f).collect())).unwrap()
    };
    let g = text(&mut p, &|r| r.0.to_string());
    let s = text(&mut p, &|r| r.1.to_string());
    let y = p.add_column(b, 0, Designation::Y, ColumnData::numeric(rows.iter().map(|r| r.2).collect())).unwrap();
    let groups: Vec<ColumnId> = if two_groups { vec![g, s] } else { vec![g] };
    let graph = p.plot_indexed(template, &groups, y).unwrap();
    (p, graph, g, s, y)
}

fn data_of(p: &Project, g: WindowId) -> data::LayerData<'_> {
    data::collect(p, &p.graph(g).unwrap().layers[0])
}

fn set_error(p: &mut Project, g: WindowId, e: IndexedError) {
    for plot in &mut p.graph_mut(g).unwrap().layers[0].plots {
        plot.indexed.as_mut().unwrap().error = e;
    }
}

fn close(a: f64, b: f64) -> bool {
    (a.is_nan() && b.is_nan()) || (a - b).abs() < 1e-9
}

fn assert_vec(got: &[f64], want: &[f64]) {
    assert_eq!(got.len(), want.len(), "{got:?} vs {want:?}");
    assert!(got.iter().zip(want).all(|(a, b)| close(*a, *b)), "{got:?} vs {want:?}");
}

#[test]
fn one_group_column_gives_one_bar_per_level_at_the_mean() {
    let (p, g, ..) = indexed(Template::GroupedColumns, false);
    let l = data_of(&p, g);
    assert_eq!(l.plots.len(), 1);
    let pd = &l.plots[0];
    assert_eq!(pd.categories.as_deref(), Some(&["a".to_string(), "b".to_string(), "c".to_string()][..]));
    assert_vec(pd.x.as_deref().unwrap(), &[1.0, 2.0, 3.0]);
    // a: 1, 3, 10 (the row without a value is out); b: 2, 4, 6; c: 20, 30, 25, 100; the row without a group is out.
    assert_vec(&pd.y, &[14.0 / 3.0, 4.0, 43.75]);
    assert!(pd.err.is_none(), "no error bars by default");
}

#[test]
fn the_error_is_the_sample_sd_or_the_standard_error() {
    let (mut p, g, ..) = indexed(Template::GroupedColumns, true);
    set_error(&mut p, g, IndexedError::Sd);
    let l = data_of(&p, g);
    // Subgroup u: a {1, 3} -> sd sqrt 2; b {2, 4, 6} -> 2; c has none.
    assert_vec(l.plots[0].err.as_deref().unwrap(), &[2f64.sqrt(), 2.0, f64::NAN]);
    // Subgroup v: a {10} has one value (no spread); b none; c {20, 30, 25, 100}.
    let sd_c = {
        let m = 43.75;
        ((23.75f64.powi(2) + 13.75f64.powi(2) + 18.75f64.powi(2) + 56.25f64.powi(2)) / 3.0).sqrt() + 0.0 * m
    };
    assert_vec(l.plots[1].err.as_deref().unwrap(), &[f64::NAN, f64::NAN, sd_c]);
    set_error(&mut p, g, IndexedError::Se);
    let l = data_of(&p, g);
    assert_vec(l.plots[0].err.as_deref().unwrap(), &[1.0, 2.0 / 3f64.sqrt(), f64::NAN]);
    assert_vec(&l.plots[1].err.as_deref().unwrap()[2..], &[sd_c / 2.0]);
}

#[test]
fn a_missing_combination_has_no_bar_and_the_others_keep_their_slots() {
    let (p, g, ..) = indexed(Template::GroupedColumns, true);
    let l = data_of(&p, g);
    assert_eq!(l.plots.len(), 2);
    assert_vec(&l.plots[0].y, &[2.0, 4.0, f64::NAN]);
    assert_vec(&l.plots[1].y, &[10.0, f64::NAN, 43.75]);
    // Side by side in plot order, whichever categories a subgroup lacks: spacing 1, group of 0.8 shared by two.
    for (pd, left) in l.plots.iter().zip([-0.4, 0.0]) {
        let slot = pd.bar.unwrap();
        assert!((slot.spacing - 1.0).abs() < 1e-12 && (slot.width - 0.4).abs() < 1e-12, "{slot:?}");
        assert!((slot.left - left).abs() < 1e-12, "{slot:?}");
    }
    // Four bars are drawn (2 per subgroup), not six.
    let scene = layout::build_scene(&p, g, &FixedMetrics).unwrap();
    let bars = scene.layers[0].plots.iter().filter(|it| matches!(it, scene::Item::Rect { fill: Some(_), .. })).count();
    assert_eq!(bars, 4);
    // The legend names the subgroups.
    let svg = svg_of(&p, g);
    let t = texts(&doc(&svg));
    assert!(t.contains(&"u".to_string()) && t.contains(&"v".to_string()), "{t:?}");
}

#[test]
fn the_x_axis_names_the_levels_in_sorted_order() {
    let (p, g, ..) = indexed(Template::GroupedColumns, true);
    let svg = svg_of(&p, g);
    let t = texts(&doc(&svg));
    let pos = |s: &str| t.iter().position(|x| x == s).unwrap_or_else(|| panic!("{s} in {t:?}"));
    assert!(pos("a") < pos("b") && pos("b") < pos("c"));
    // The title of the X axis is the group column's name.
    assert!(t.contains(&"A".to_string()), "{t:?}");
}

#[test]
fn numeric_levels_sort_numerically_on_the_axis() {
    let mut p = Project::new();
    let b = p.add_workbook("data");
    let g = p.add_column(b, 0, Designation::X, ColumnData::numeric(vec![10.0, 2.0, 10.0, 2.0, 1.0])).unwrap();
    let y = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![1.0, 2.0, 3.0, 4.0, 5.0])).unwrap();
    let gr = p.plot_columns(Template::GroupedColumns, &[g, y]).unwrap();
    let l = data_of(&p, gr);
    assert_eq!(l.plots[0].categories.as_deref(), Some(&["1".to_string(), "2".to_string(), "10".to_string()][..]));
    assert_vec(&l.plots[0].y, &[5.0, 3.0, 2.0]);
}

#[test]
fn box_stats_per_category_equal_box_stats_of_the_filtered_values() {
    let (p, g, ..) = indexed(Template::GroupedBox, false);
    let l = data_of(&p, g);
    assert_eq!(l.plots.len(), 1);
    let pd = &l.plots[0];
    assert!(pd.boxed.is_none());
    let want = [
        box_stats(&[1.0, 3.0, 10.0], 1.0, "a".into()).unwrap(),
        box_stats(&[2.0, 4.0, 6.0], 2.0, "b".into()).unwrap(),
        box_stats(&[20.0, 30.0, 25.0, 100.0], 3.0, "c".into()).unwrap(),
    ];
    assert_eq!(pd.gboxes, want);
    // One subgroup: the existing box width.
    assert!(pd.gboxes.iter().all(|b| b.width == BOX_WIDTH));
    assert_vec(pd.x.as_deref().unwrap(), &[1.0, 2.0, 3.0]);
}

#[test]
fn subgroup_boxes_are_dodged_across_the_box_band() {
    let (p, g, ..) = indexed(Template::GroupedBox, true);
    let l = data_of(&p, g);
    assert_eq!(l.plots.len(), 2);
    // u has a and b; v has a and c: two boxes share the band of 0.8, so each is 0.4 wide.
    let u: Vec<(f64, &str)> = l.plots[0].gboxes.iter().map(|b| (b.pos, b.label.as_str())).collect();
    let v: Vec<(f64, &str)> = l.plots[1].gboxes.iter().map(|b| (b.pos, b.label.as_str())).collect();
    assert_eq!(u.len(), 2);
    assert_eq!(v.len(), 2);
    let eq = |a: f64, b: f64| (a - b).abs() < 1e-12;
    assert!(eq(u[0].0, 0.8) && eq(u[1].0, 1.8), "{u:?}");
    assert!(eq(v[0].0, 1.2) && eq(v[1].0, 3.2), "{v:?}");
    assert!(l.plots.iter().flat_map(|pd| &pd.gboxes).all(|b| eq(b.width, 0.4)));
    // The stats of a box are those of its own rows.
    let c = &l.plots[1].gboxes[1];
    assert_eq!(c.median, box_stats(&[20.0, 30.0, 25.0, 100.0], 0.0, String::new()).unwrap().median);
}

#[test]
fn the_y_range_covers_whiskers_and_outliers() {
    let (p, g, ..) = indexed(Template::GroupedBox, false);
    let l = data_of(&p, g);
    let b = &l.plots[0].gboxes[2];
    assert_eq!(b.whisker_high, 100.0);
    let (lo, hi) = data::axis_range(l.plots.iter(), Dim::Y, sheaf_core::graph::ScaleKind::Linear).unwrap();
    assert!(lo <= 1.0 && hi >= 100.0, "{lo} {hi}");
    // An outlier beyond the whisker is in the range too.
    let mut q = Project::new();
    let bk = q.add_workbook("data");
    let gc = q.add_column(bk, 0, Designation::X, ColumnData::text(vec!["g".to_string(); 6])).unwrap();
    let yc = q.add_column(bk, 0, Designation::Y, ColumnData::numeric(vec![1.0, 2.0, 3.0, 4.0, 5.0, 100.0])).unwrap();
    let gq = q.plot_indexed(Template::GroupedBox, &[gc], yc).unwrap();
    let lq = data_of(&q, gq);
    assert_eq!(lq.plots[0].gboxes[0].outliers, vec![100.0]);
    let (_, hi) = data::axis_range(lq.plots.iter(), Dim::Y, sheaf_core::graph::ScaleKind::Linear).unwrap();
    assert!(hi >= 100.0, "{hi}");
    // A grouped column's range holds the error bars.
    let (mut p, g, ..) = indexed(Template::GroupedColumns, false);
    set_error(&mut p, g, IndexedError::Sd);
    let l = data_of(&p, g);
    let (_, hi) = data::axis_range(l.plots.iter(), Dim::Y, sheaf_core::graph::ScaleKind::Linear).unwrap();
    assert!(hi >= 81.47, "{hi}");
}

#[test]
fn the_plots_draw_and_other_box_and_column_plots_are_untouched() {
    let (p, g, ..) = indexed(Template::GroupedBox, true);
    let svg = svg_of(&p, g);
    assert!(roxmltree::Document::parse(&svg).is_ok());
    assert_eq!(p.graph(g).unwrap().layers[0].plots[0].kind, PlotKind::Box);
    compare_golden("grouped-box", &svg_of(&p, g));
    let (mut p, g, ..) = indexed(Template::GroupedColumns, true);
    set_error(&mut p, g, IndexedError::Se);
    compare_golden("grouped-columns", &svg_of(&p, g));
    // A plain box plot of the same column still has its single `boxed` and no category names.
    let (p2, g2) = crate::tests::project(Template::BoxChart, vec![1.0, 2.0, 3.0], vec![vec![1.0, 2.0, 3.0]]);
    let l = data_of(&p2, g2);
    assert!(l.plots[0].boxed.is_some() && l.plots[0].gboxes.is_empty() && l.plots[0].categories.is_none());
}

// ---- Interval plot and Pareto chart (T82) ----

fn stat_project(template: Template) -> (Project, WindowId) {
    let mut p = Project::new();
    let b = p.add_workbook("data");
    // A: 2, 4, 6, 8 (mean 5, SD sqrt(20/3), SE SD/2); B: 1, 3 and a gap; defects as text.
    let a = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![2.0, 4.0, 6.0, 8.0])).unwrap();
    let c = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![1.0, 3.0, f64::NAN, f64::NAN])).unwrap();
    let t = p
        .add_column(
            b,
            0,
            Designation::X,
            ColumnData::text(["scratch", "dent", "scratch", "crack", "scratch", "dent", ""].map(String::from).to_vec()),
        )
        .unwrap();
    let g = if template == Template::Pareto {
        p.plot_columns(template, &[t]).unwrap()
    } else {
        p.plot_columns(template, &[a, c]).unwrap()
    };
    (p, g)
}

#[test]
fn interval_points_are_the_column_means_with_a_t_confidence_interval() {
    use sheaf_core::graph::{IntervalError, StatPlot};
    let (mut p, g) = stat_project(Template::IntervalPlot);
    let l = data_of(&p, g);
    assert_eq!(l.plots.len(), 2);
    assert_vec(&l.plots[0].y, &[5.0]);
    assert_vec(l.plots[0].x.as_deref().unwrap(), &[1.0]);
    assert_vec(l.plots[1].x.as_deref().unwrap(), &[2.0]);
    assert_vec(&l.plots[1].y, &[2.0]);
    // t(0.975, 3) = 3.182446305; SE = sqrt(20/3) / 2.
    let se = (20.0f64 / 3.0).sqrt() / 2.0;
    assert_vec(l.plots[0].err.as_deref().unwrap(), &[3.182446305284263 * se]);
    // Two numbers: t(0.975, 1) = 12.7062047, SE = sqrt(2) / sqrt(2) = 1.
    assert_vec(l.plots[1].err.as_deref().unwrap(), &[12.706204736174694]);
    assert_eq!(l.plots[0].slot_label.as_deref(), Some("A"));
    for (error, level, want) in [
        (IntervalError::Sd, 95.0, (20.0f64 / 3.0).sqrt()),
        (IntervalError::Se, 95.0, se),
        (IntervalError::Ci, 90.0, 2.353363435555 * se),
    ] {
        for plot in &mut p.graph_mut(g).unwrap().layers[0].plots {
            plot.stat = Some(StatPlot::Interval { error, level });
        }
        assert_vec(data_of(&p, g).plots[0].err.as_deref().unwrap(), &[want]);
    }
}

#[test]
fn an_empty_column_does_not_use_up_an_interval_slot() {
    let mut p = Project::new();
    let b = p.add_workbook("data");
    let a = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![1.0, 3.0])).unwrap();
    let e = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![f64::NAN, f64::NAN])).unwrap();
    let c = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![2.0, 6.0])).unwrap();
    let g = p.plot_columns(Template::IntervalPlot, &[a, e, c]).unwrap();
    let l = data_of(&p, g);
    assert_eq!(l.plots.len(), 2);
    assert_vec(l.plots[0].x.as_deref().unwrap(), &[1.0]);
    assert_vec(l.plots[1].x.as_deref().unwrap(), &[2.0]);
}

#[test]
fn interval_plot_draws_symbols_bars_and_column_names() {
    let (p, g) = stat_project(Template::IntervalPlot);
    let svg = svg_of(&p, g);
    let t = texts(&doc(&svg));
    assert!(t.iter().any(|s| s == "A") && t.iter().any(|s| s == "B" || s == "Y"), "{t:?}");
    // The mean is a solid circle (the documented BoxInterval look).
    let sym = p.graph(g).unwrap().layers[0].plots[0].symbol.clone();
    assert!(sym.shape == sheaf_core::graph::SymbolShape::Circle && sym.filled);
    compare_golden("interval-plot", &svg);
}

#[test]
fn pareto_counts_sorts_and_accumulates() {
    let (mut p, g) = stat_project(Template::Pareto);
    let layers = data::collect_graph(&p, p.graph(g).unwrap());
    let (bars, line) = (&layers[0], &layers[1]);
    assert_eq!(bars.plots[0].categories.as_deref(), Some(&["scratch".to_string(), "dent".into(), "crack".into()][..]));
    assert_vec(&bars.plots[0].y, &[3.0, 2.0, 1.0]);
    // Origin's symr = 1: the line starts at (0, 0) at the left edge of the first bar and its symbols sit at the
    // right edge of each bar; the default gap is 20 %, so a bar is 0.8 wide.
    assert_vec(line.plots[0].x.as_deref().unwrap(), &[0.6, 1.4, 2.4, 3.4]);
    assert_vec(&line.plots[0].y, &[0.0, 50.0, 50.0 * 5.0 / 3.0, 100.0]);
    // The edges follow the drawn bars: a gap of 50 % makes them 0.5 wide.
    p.graph_mut(g).unwrap().layers[0].bars.gap_pct = 50.0;
    let layers = data::collect_graph(&p, p.graph(g).unwrap());
    assert_vec(layers[1].plots[0].x.as_deref().unwrap(), &[0.75, 1.25, 2.25, 3.25]);
    p.graph_mut(g).unwrap().layers[0].bars.gap_pct = 20.0;
    let svg = svg_of(&p, g);
    let t = texts(&doc(&svg));
    for want in ["scratch", "dent", "crack", "Count", "Cumulative %", "100"] {
        assert!(t.iter().any(|s| s == want), "{want} in {t:?}");
    }
    compare_golden("pareto", &svg);
}
