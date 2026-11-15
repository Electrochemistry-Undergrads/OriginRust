//! Treemap (T50, graphing spec 14.26): nesting with the gap, colours per top-level category, labels that are
//! hidden when they do not fit, depth, layouts and the other plot kinds left alone.

use super::*;
use crate::tests::{doc, svg_of, texts};
use sheaf_core::graph::{PlotKind, TreemapLayout, TreemapLevel, TreemapOrder};
use sheaf_core::template::Template;
use sheaf_core::{Color, ColumnData, ColumnId, Designation};

const TOL: f64 = 0.02;

/// Continent, country, population (a duplicate France row, an empty and a negative value to skip).
fn world() -> (Project, WindowId, Vec<ColumnId>, ColumnId) {
    let rows = [
        ("Asia", "China", 1412.0),
        ("Asia", "India", 1408.0),
        ("Asia", "Japan", 125.0),
        ("Europe", "France", 68.0),
        ("Europe", "Germany", 83.0),
        ("Europe", "France", 1.0),
        ("Europe", "Italy", f64::NAN),
        ("Europe", "Spain", -4.0),
        ("Africa", "Egypt", 109.0),
        ("Oceania", "Tuvalu", 0.011),
    ];
    let mut p = Project::new();
    let b = p.add_workbook("data");
    let text = |p: &mut Project, f: &dyn Fn(&(&str, &str, f64)) -> String| {
        p.add_column(b, 0, Designation::Y, ColumnData::text(rows.iter().map(f).collect())).unwrap()
    };
    let continent = text(&mut p, &|r| r.0.to_string());
    let country = text(&mut p, &|r| r.1.to_string());
    let pop = p.add_column(b, 0, Designation::Y, ColumnData::numeric(rows.iter().map(|r| r.2).collect())).unwrap();
    let g = p.plot_treemap(&[continent, country], pop).unwrap();
    (p, g, vec![continent, country], pop)
}

fn set(p: &mut Project, g: WindowId, f: impl Fn(&mut sheaf_core::graph::TreemapStyle)) {
    f(&mut p.graph_mut(g).unwrap().layers[0].plots[0].treemap);
}

fn frame(p: &Project, g: WindowId) -> Rect {
    let graph = p.graph(g).unwrap();
    let pds = data::collect_graph(p, graph);
    layout::graph_geometry(p, graph, &pds, &FixedMetrics)[0].frame
}

fn layer(p: &Project, g: WindowId) -> scene::LayerScene {
    layout::build_scene(p, g, &FixedMetrics).unwrap().layers.remove(0)
}

/// The filled rectangles of the plot, in drawing order, with their fills.
fn rects(l: &scene::LayerScene) -> Vec<(Rect, Color)> {
    l.plots
        .iter()
        .filter_map(|it| match it {
            scene::Item::Rect { rect, fill: Some(c), .. } => Some((*rect, *c)),
            _ => None,
        })
        .collect()
}

fn cells(p: &Project, g: WindowId) -> (Vec<sheaf_calc::treemap_tree::Cell>, treemap::TreemapData) {
    let graph = p.graph(g).unwrap();
    let ld = data::collect(p, &graph.layers[0]);
    let tm = ld.plots[0].treemap.as_deref().unwrap().clone();
    let f = frame(p, g);
    (tm.cells(f, treemap::header_height(tm.font.size_pt)), tm)
}

fn inside(a: &Rect, outer: &Rect, margin: f64) -> bool {
    a.x >= outer.x + margin - TOL
        && a.y >= outer.y + margin - TOL
        && a.x + a.w <= outer.x + outer.w - margin + TOL
        && a.y + a.h <= outer.y + outer.h - margin + TOL
}

#[test]
fn rectangles_are_nested_inside_their_parent_by_the_gap() {
    let (mut p, g, _, _) = world();
    set(&mut p, g, |t| t.gap_pt = 4.0);
    let l = layer(&p, g);
    let drawn = rects(&l);
    let (cells, _) = cells(&p, g);
    let f = frame(&p, g);
    // Every cell with room is drawn, in the order of the layout (parents first), where the layout puts it.
    let visible: Vec<_> = cells.iter().filter(|c| c.rect.w > 0.0 && c.rect.h > 0.0).collect();
    assert_eq!(drawn.len(), visible.len());
    for ((r, _), c) in drawn.iter().zip(&visible) {
        assert!((r.x - c.rect.x).abs() < TOL && (r.w - c.rect.w).abs() < TOL, "{}", c.name);
    }
    for c in &visible {
        assert!(inside(&to(c.rect), &f, 2.0), "{} is inside the frame", c.name);
        if c.level == 1 {
            let parent = visible.iter().find(|q| q.level == 0 && q.top == c.top).unwrap();
            let band = parent.header.map_or(0.0, |h| h.h);
            let (pr, cr) = (to(parent.rect), to(c.rect));
            assert!(cr.x >= pr.x + 4.0 - TOL && cr.x + cr.w <= pr.x + pr.w - 4.0 + TOL, "{}", c.name);
            assert!(cr.y >= pr.y + band + 4.0 - TOL && cr.y + cr.h <= pr.y + pr.h - 4.0 + TOL, "{}", c.name);
        }
    }
    // Asia has a header band holding its name.
    assert!(cells.iter().any(|c| c.name == "Asia" && c.header.is_some()));
}

fn to(r: sheaf_calc::treemap::Rect) -> Rect {
    Rect { x: r.x, y: r.y, w: r.w, h: r.h }
}

#[test]
fn one_palette_colour_per_top_level_category_and_lighter_tints_below() {
    let (p, g, _, _) = world();
    let (cells, tm) = cells(&p, g);
    let layer = &p.graph(g).unwrap().layers[0];
    let n = tm.tree.roots.len();
    assert_eq!(n, 4);
    for (i, c) in tm.colors.iter().enumerate() {
        assert_eq!(*c, layer.palette.color(i, n));
    }
    assert!(tm.colors.windows(2).all(|w| w[0] != w[1]), "four different colours");
    for c in &cells {
        let base = tm.colors[c.top];
        let want = match c.level {
            0 => base,
            l => base.lerp(&Color::WHITE, 1.0 - 0.7f64.powi(l as i32)),
        };
        assert_eq!(tm.fill(c), want, "{}", c.name);
    }
    // The scene carries those fills; the first rectangle is Asia's, in the first palette colour.
    let drawn = rects(&layer_scene(&p, g));
    assert_eq!(drawn[0].1, tm.colors[0]);
    assert!(drawn.iter().skip(1).any(|(_, c)| *c == tm.colors[0].lerp(&Color::WHITE, 0.3)));
}

fn layer_scene(p: &Project, g: WindowId) -> scene::LayerScene {
    layer(p, g)
}

#[test]
fn rectangles_have_thin_white_borders_and_no_axes_or_legend_are_drawn() {
    let (p, g, _, _) = world();
    let l = layer(&p, g);
    for it in &l.plots {
        if let scene::Item::Rect { stroke: Some(s), .. } = it {
            assert_eq!((s.color, s.width), (Color::WHITE, 0.5));
        }
    }
    let svg = svg_of(&p, g);
    let d = doc(&svg);
    let t = texts(&d);
    // Only labels of the treemap: no tick labels of an axis, no axis title, no legend entry.
    for bad in ["0", "500", "1000", "Layer 1"] {
        assert!(!t.iter().any(|x| x == bad), "{bad}");
    }
    assert!(t.iter().any(|x| x == "Asia") && t.iter().any(|x| x == "China"));
}

#[test]
fn labels_are_hidden_when_they_do_not_fit_and_values_follow_the_names() {
    let (mut p, g, _, _) = world();
    let svg = svg_of(&p, g);
    let t = texts(&doc(&svg));
    // China is large: name and value. Tuvalu is a sliver: neither.
    assert!(t.iter().any(|x| x == "China") && t.iter().any(|x| x == "1412"));
    assert!(!t.iter().any(|x| x == "Tuvalu") && !t.iter().any(|x| x == "0.011"));
    // The duplicate rows are summed (68 + 1), the empty and negative rows left out.
    assert!(t.iter().any(|x| x == "69"));
    assert!(!t.iter().any(|x| x == "Italy" || x == "Spain"));
    // A smaller frame hides more of them.
    p.graph_mut(g).unwrap().page.width_pt = 180.0;
    p.graph_mut(g).unwrap().page.height_pt = 140.0;
    let small = texts(&doc(&svg_of(&p, g)));
    assert!(small.len() < t.len(), "{} vs {}", small.len(), t.len());
    // Whatever is drawn is drawn whole: every name drawn is a real name.
    let names = ["Asia", "Europe", "Africa", "Oceania", "China", "India", "Japan", "France", "Germany", "Egypt"];
    for x in small.iter().filter(|x| x.chars().any(|c| c.is_alphabetic())) {
        assert!(names.contains(&x.as_str()), "{x}");
    }
}

#[test]
fn depth_one_draws_only_the_top_level_with_everything_below_summed() {
    let (mut p, g, _, _) = world();
    set(&mut p, g, |t| t.depth = Some(1));
    let (cells, _) = cells(&p, g);
    assert_eq!(cells.len(), 4);
    assert!(cells.iter().all(|c| c.leaf && c.header.is_none()));
    let asia = cells.iter().find(|c| c.name == "Asia").unwrap();
    assert_eq!(asia.value, 1412.0 + 1408.0 + 125.0);
    let t = texts(&doc(&svg_of(&p, g)));
    assert!(t.iter().any(|x| x == "Asia") && !t.iter().any(|x| x == "China"));
}

#[test]
fn every_layout_tiles_the_frame_and_keeps_the_areas_in_proportion() {
    for layout in TreemapLayout::ALL {
        for order in [TreemapOrder::Categorical, TreemapOrder::Alphabetical, TreemapOrder::BySize] {
            let (mut p, g, _, _) = world();
            set(&mut p, g, |t| {
                t.gap_pt = 0.0;
                t.layout = layout;
                t.order = order;
            });
            let (all, tm) = cells(&p, g);
            let f = frame(&p, g);
            let total: f64 = tm.tree.roots.iter().map(|n| n.value).sum();
            // The top level fills the frame in proportion.
            for c in all.iter().filter(|c| c.level == 0) {
                let (r, want) = (to(c.rect), c.value / total * f.w * f.h);
                assert!((r.w * r.h - want).abs() <= 1e-9 * f.w * f.h, "{layout:?} {order:?} {}", c.name);
            }
            // The children tile what is left of the parent below its header band, in proportion.
            for parent in all.iter().filter(|c| c.level == 0 && !c.leaf) {
                let band = parent.header.map_or(0.0, |h| h.h);
                let r = to(parent.rect);
                let inner = r.w * (r.h - band);
                let kids: Vec<_> = all.iter().filter(|c| c.level == 1 && c.top == parent.top).collect();
                let sum: f64 = kids.iter().map(|c| to(c.rect).w * to(c.rect).h).sum();
                assert!((sum - inner).abs() <= 1e-9 * inner, "{layout:?} {order:?} {}", parent.name);
                for k in &kids {
                    let want = k.value / parent.value * inner;
                    let a = to(k.rect).w * to(k.rect).h;
                    assert!((a - want).abs() <= 1e-9 * inner, "{layout:?} {order:?} {}", k.name);
                }
            }
        }
    }
}

#[test]
fn per_level_settings_and_the_order_column_change_the_arrangement() {
    let (mut p, g, cats, pop) = world();
    let names = |p: &Project| -> Vec<String> {
        cells(p, g).0.iter().filter(|c| c.level == 0).map(|c| c.name.clone()).collect()
    };
    set(&mut p, g, |t| {
        t.layout = TreemapLayout::Split;
        t.order = TreemapOrder::Alphabetical;
    });
    assert_eq!(names(&p), ["Africa", "Asia", "Europe", "Oceania"]);
    set(&mut p, g, |t| t.order = TreemapOrder::BySize);
    assert_eq!(names(&p), ["Asia", "Europe", "Africa", "Oceania"]);
    // By column: the population column orders the continents by their smallest population.
    set(&mut p, g, |t| {
        t.order = TreemapOrder::ByColumn;
        t.order_column = Some(pop);
    });
    assert_eq!(names(&p), ["Oceania", "Europe", "Africa", "Asia"]);
    // Per level: the countries alphabetically only when the box is checked.
    set(&mut p, g, |t| {
        t.order = TreemapOrder::Categorical;
        t.per_level = vec![TreemapLevel { layout: TreemapLayout::Split, order: TreemapOrder::Alphabetical }];
    });
    let europe = |p: &Project| -> Vec<String> {
        cells(p, g).0.iter().filter(|c| c.level == 1 && c.top == 1).map(|c| c.name.clone()).collect()
    };
    assert_eq!(europe(&p), ["France", "Germany"], "first appearance");
    set(&mut p, g, |t| {
        t.per_level_on = true;
        t.per_level[0].order = TreemapOrder::BySize;
    });
    assert_eq!(europe(&p), ["Germany", "France"]);
    let _ = cats;
}

#[test]
fn value_text_has_four_significant_digits_and_no_exponent() {
    for (v, s) in [
        (1412.0, "1412"),
        (0.011, "0.011"),
        (1234.567, "1235"),
        (0.123456, "0.1235"),
        (2.5, "2.5"),
        (0.0, "0"),
        (123456.0, "123456"),
    ] {
        assert_eq!(treemap::value_text(v), s);
    }
    assert_eq!(treemap::value_text(f64::NAN), "");
}

#[test]
fn a_treemap_without_category_columns_draws_nothing_and_other_kinds_are_unchanged() {
    let (mut p, g, _, _) = world();
    set(&mut p, g, |t| t.categories.clear());
    assert!(rects(&layer(&p, g)).is_empty());
    // A scatter graph renders the same with or without a treemap graph in the project.
    let mut q = Project::new();
    let b = q.add_workbook("d");
    let x = q.add_column(b, 0, Designation::X, ColumnData::numeric(vec![1.0, 2.0, 3.0])).unwrap();
    let y = q.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![3.0, 1.0, 2.0])).unwrap();
    let _ = x;
    let s = q.plot_columns(Template::Scatter, &[y]).unwrap();
    let alone = svg_of(&q, s);
    let label = q.add_column(b, 0, Designation::Y, ColumnData::text(vec!["a".into(), "b".into(), "c".into()])).unwrap();
    q.plot_treemap(&[label], y).unwrap();
    assert_eq!(svg_of(&q, s), alone);
    assert_eq!(q.graph(s).unwrap().layers[0].default_kind, PlotKind::Scatter);
}

#[test]
fn middle_levels_get_header_bands_and_names() {
    let mut p = Project::new();
    let b = p.add_workbook("data");
    let rows = [
        ("Asia", "China", "a", 400.0),
        ("Asia", "China", "b", 300.0),
        ("Asia", "India", "c", 500.0),
        ("Asia", "India", "d", 300.0),
        ("Europe", "France", "e", 450.0),
        ("Europe", "France", "f", 350.0),
        ("Europe", "Germany", "g", 700.0),
    ];
    let mut text = |f: &dyn Fn(&(&str, &str, &str, f64)) -> String| {
        p.add_column(b, 0, Designation::Y, ColumnData::text(rows.iter().map(f).collect())).unwrap()
    };
    let (c1, c2, c3) = (text(&|r| r.0.to_string()), text(&|r| r.1.to_string()), text(&|r| r.2.to_string()));
    let v = p.add_column(b, 0, Designation::Y, ColumnData::numeric(rows.iter().map(|r| r.3).collect())).unwrap();
    let g = p.plot_treemap(&[c1, c2, c3], v).unwrap();
    p.graph_mut(g).unwrap().page.width_pt = 800.0;
    p.graph_mut(g).unwrap().page.height_pt = 600.0;
    let (cs, _) = cells(&p, g);
    for name in ["China", "India", "France", "Germany"] {
        let c = cs.iter().find(|c| c.name == name).unwrap();
        assert_eq!(c.level, 1);
        assert!(c.header.is_some(), "{name} has a header band");
    }
    assert!(cs.iter().filter(|c| c.leaf).all(|c| c.header.is_none()));
    let t = texts(&doc(&svg_of(&p, g)));
    for name in ["Asia", "Europe", "China", "India", "France", "Germany", "a", "g"] {
        assert!(t.iter().any(|x| x == name), "{name} is drawn: {t:?}");
    }
    // A shallower depth drops the lowest level's names, and the middle level becomes the leaf level.
    set(&mut p, g, |t| t.depth = Some(2));
    let t = texts(&doc(&svg_of(&p, g)));
    assert!(t.iter().any(|x| x == "China") && !t.iter().any(|x| x == "a"));
}

#[test]
fn an_appended_treemap_draws_the_same() {
    let (mut p, g, _, _) = world();
    let pop = p.graph(g).unwrap().layers[0].plots[0].y;
    set(&mut p, g, |t| {
        t.order = TreemapOrder::ByColumn;
        t.order_column = Some(pop);
    });
    let before = svg_of(&p, g);
    let mut host = Project::new();
    let hb = host.add_workbook("host");
    for i in 0..3 {
        host.add_column(hb, 0, Designation::Y, ColumnData::numeric(vec![i as f64])).unwrap();
    }
    let (new, _) = host.append(p, "f");
    let after = svg_of(&host, new[1]);
    assert_eq!(before, after);
}
