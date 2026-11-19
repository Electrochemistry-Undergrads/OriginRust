//! The look settings that Plot Details adds beyond the measured ones, drawn into the figure
//! (brief T13): hatches and gradients on fills, the data label extras, the layer background's
//! gradient and border, the X and Y axes switches, and the page border.

use super::*;
use sheaf_core::graph::{
    BorderStyle, FillPattern, Gradient, GradientDirection, LabelForm, LabelPosition, LineStyle, PlotKind,
};
use sheaf_core::template::Template;
use sheaf_core::{Color, ColumnData, Designation};

fn project(template: Template) -> (Project, WindowId) {
    let mut p = Project::new();
    let book = p.add_workbook("data");
    let x = p.add_column(book, 0, Designation::X, ColumnData::numeric(vec![1.0, 2.0, 3.0, 4.0])).unwrap();
    let y = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![2.0, 5.0, 3.0, 6.0])).unwrap();
    let g = p.add_graph(template);
    p.add_plot_xy(g, 0, Some(x), y, None).unwrap();
    (p, g)
}

fn plot_mut(p: &mut Project, g: WindowId) -> &mut sheaf_core::graph::Plot {
    &mut p.graph_mut(g).unwrap().layers[0].plots[0]
}

fn layer_mut(p: &mut Project, g: WindowId) -> &mut sheaf_core::graph::Layer {
    &mut p.graph_mut(g).unwrap().layers[0]
}

fn scene(p: &Project, g: WindowId) -> scene::Scene {
    layout::build_scene(p, g, &FixedMetrics).unwrap()
}

fn texts(items: &[scene::Item]) -> Vec<&scene::TextItem> {
    items
        .iter()
        .filter_map(|i| match i {
            scene::Item::Text(t) => Some(t),
            _ => None,
        })
        .collect()
}

/// The y values of the numbers in path data "M x y L x y ...".
fn xs(d: &str) -> Vec<f64> {
    d.trim_end_matches('Z')
        .split(['M', 'L'])
        .skip(1)
        .map(|p| p.split_whitespace().next().unwrap().parse().unwrap())
        .collect()
}

fn ys(d: &str) -> Vec<f64> {
    d.trim_end_matches('Z')
        .split(['M', 'L'])
        .skip(1)
        .map(|p| p.split_whitespace().nth(1).unwrap().parse().unwrap())
        .collect()
}

fn fills(items: &[scene::Item]) -> Vec<Color> {
    items
        .iter()
        .filter_map(|i| match i {
            scene::Item::Path { fill: Some(c), .. } => Some(*c),
            _ => None,
        })
        .collect()
}

#[test]
fn drop_lines_are_only_drawn_for_kinds_that_offer_them() {
    let (mut p, g) = project(Template::Line);
    let blue = Color::rgb(0, 0, 255);
    let drop = &mut plot_mut(&mut p, g).decor.drop.horizontal;
    (drop.on, drop.color) = (true, Some(blue));
    let blue_paths = |p: &Project| -> usize {
        scene(p, g).layers[0]
            .plots
            .iter()
            .filter(|i| matches!(i, scene::Item::Path { stroke: Some(s), .. } if s.color == blue))
            .count()
    };
    assert_eq!(blue_paths(&p), 1);
    // The plot becomes a column plot: its Drop Lines tab is gone, so no line may stay behind.
    plot_mut(&mut p, g).kind = PlotKind::Column;
    assert_eq!(blue_paths(&p), 0);
    // The settings are kept: back to a line, the drop lines are drawn again.
    plot_mut(&mut p, g).kind = PlotKind::Line;
    assert_eq!(blue_paths(&p), 1);
}

#[test]
fn a_hatched_column_gets_pattern_lines_and_the_outline_on_top() {
    let (mut p, g) = project(Template::Column);
    let plain = scene(&p, g).layers[0].plots.len();
    assert!(
        scene(&p, g).layers[0].plots.iter().all(|i| matches!(i, scene::Item::Rect { .. })),
        "one rectangle per bar"
    );
    let fill = &mut plot_mut(&mut p, g).decor.fill;
    fill.pattern = FillPattern::Cross;
    fill.pattern_color = Color::BLUE;
    let s = scene(&p, g);
    let items = &s.layers[0].plots;
    // Per bar: the fill, the hatch lines, the outline.
    assert_eq!(items.len(), 3 * plain);
    assert!(matches!(&items[0], scene::Item::Path { fill: Some(_), stroke: None, .. }));
    assert!(matches!(&items[1], scene::Item::Path { fill: None, stroke: Some(st), .. } if st.color == Color::BLUE));
    assert!(matches!(&items[2], scene::Item::Rect { fill: None, stroke: Some(_), .. }));
    // The hatch is cut to the bar: every line of the first bar lies inside the bar's own
    // rectangle (the outline item), in x as well as in y, and the bar is shorter than the frame.
    let frame = s.layers[0].clip;
    let scene::Item::Rect { rect: bar, .. } = &items[2] else { unreachable!() };
    assert!(bar.h < frame.h - 1.0, "the first bar is shorter than the frame: {bar:?} in {frame:?}");
    let scene::Item::Path { d, .. } = &items[1] else { unreachable!() };
    let eps = 1e-3;
    let (xs, ys) = (xs(d), ys(d));
    assert!(!xs.is_empty());
    assert!(ys.iter().all(|y| *y >= bar.y - eps && *y <= bar.y + bar.h + eps), "{d} in {bar:?}");
    assert!(xs.iter().all(|x| *x >= bar.x - eps && *x <= bar.x + bar.w + eps), "{d} in {bar:?}");
    // And it reaches both sides of the bar and its top and bottom, not a part of it.
    assert!(xs.iter().any(|x| (*x - bar.x).abs() < 1.0) && xs.iter().any(|x| (*x - bar.x - bar.w).abs() < 1.0));
}

#[test]
fn a_bar_far_outside_the_axis_range_hatches_in_bounded_time_and_size() {
    // A bar of 1e9 on a Y axis fixed to 0..10 is a rectangle about 3e9 points tall; its hatch must
    // not run a line for every spacing across all of it.
    let mut p = Project::new();
    let book = p.add_workbook("data");
    let x = p.add_column(book, 0, Designation::X, ColumnData::numeric(vec![1.0, 2.0, 3.0, 4.0])).unwrap();
    let y = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![1e9, 1.0, 1.0, 1.0])).unwrap();
    let g = p.add_graph(Template::Column);
    p.add_plot_xy(g, 0, Some(x), y, None).unwrap();
    layer_mut(&mut p, g).y_axis.min = Some(0.0);
    layer_mut(&mut p, g).y_axis.max = Some(10.0);
    plot_mut(&mut p, g).decor.fill.pattern = FillPattern::Cross;
    plot_mut(&mut p, g).decor.fill.pattern_spacing_pt = 0.5;
    let t = std::time::Instant::now();
    let s = scene(&p, g);
    assert!(t.elapsed().as_secs() < 10, "{:?}", t.elapsed());
    let size: usize =
        s.layers[0].plots.iter().map(|i| if let scene::Item::Path { d, .. } = i { d.len() } else { 0 }).sum();
    assert!(size < 20_000_000, "{size} bytes of paths");
}

#[test]
fn a_gradient_column_runs_from_the_fill_colour_to_the_end_colour() {
    let (mut p, g) = project(Template::Column);
    plot_mut(&mut p, g).symbol.color = Color::rgb(255, 0, 0);
    plot_mut(&mut p, g).decor.fill.gradient =
        Some(Gradient { color: Color::rgb(0, 0, 255), direction: GradientDirection::BottomToTop });
    let s = scene(&p, g);
    let bands = fills(&s.layers[0].plots);
    assert!(bands.len() >= 4 * 4, "several bands per bar: {}", bands.len());
    // One bar: the first band is the fill colour (at the bottom), the last the end colour.
    let last = bands.last().unwrap();
    assert!(bands[0].r > bands[0].b, "{:?}", bands[0]);
    assert!(last.b > last.r, "{last:?}");
}

#[test]
fn the_pattern_of_an_area_plot_and_of_the_fill_under_a_curve_is_drawn() {
    // An area plot: transparency on the fill.
    let (mut p, g) = project(Template::Line);
    plot_mut(&mut p, g).kind = PlotKind::Area;
    plot_mut(&mut p, g).decor.fill.transparency_pct = 50;
    let area = fills(&scene(&p, g).layers[0].plots);
    assert!(area.iter().any(|c| c.a == 128), "{area:?}");
    // The fill under a line takes the same settings: a hatch is a separate stroked path.
    let (mut p, g) = project(Template::Line);
    plot_mut(&mut p, g).decor.fill_under = Some(Color::rgb(10, 20, 30));
    let before = scene(&p, g).layers[0].plots.len();
    plot_mut(&mut p, g).decor.fill.pattern = FillPattern::Horizontal;
    plot_mut(&mut p, g).decor.fill.pattern_color = Color::RED;
    let s = scene(&p, g);
    assert_eq!(s.layers[0].plots.len(), before + 1);
    assert!(
        s.layers[0]
            .plots
            .iter()
            .any(|i| matches!(i, scene::Item::Path { fill: None, stroke: Some(st), .. } if st.color == Color::RED))
    );
}

#[test]
fn a_translucent_gradient_area_has_no_darker_seams() {
    let (mut p, g) = project(Template::Line);
    plot_mut(&mut p, g).kind = PlotKind::Area;
    let fill = &mut plot_mut(&mut p, g).decor.fill;
    fill.transparency_pct = 40;
    fill.gradient = Some(Gradient { color: Color::WHITE, direction: GradientDirection::TopToBottom });
    let s = scene(&p, g);
    let bands: Vec<(f64, f64)> = s.layers[0]
        .plots
        .iter()
        .filter_map(|i| match i {
            scene::Item::Path { d, fill: Some(c), .. } if c.a < 255 => {
                let y = ys(d);
                Some((
                    y.iter().copied().fold(f64::INFINITY, f64::min),
                    y.iter().copied().fold(f64::NEG_INFINITY, f64::max),
                ))
            }
            _ => None,
        })
        .collect();
    assert!(bands.len() >= 4);
    assert!(bands.windows(2).all(|w| (w[1].0 - w[0].1).abs() < 1e-6), "edge to edge: {bands:?}");
}

#[test]
fn the_legend_sample_of_a_hatched_bar_is_hatched_too() {
    let (mut p, g) = project(Template::Column);
    let plain = scene(&p, g).layers[0].overlay.len();
    plot_mut(&mut p, g).decor.fill.pattern = FillPattern::Vertical;
    assert!(scene(&p, g).layers[0].overlay.len() > plain);
}

fn label_texts(p: &Project, g: WindowId) -> Vec<String> {
    texts(&scene(p, g).layers[0].plots).iter().map(|t| t.text.clone()).collect()
}

#[test]
fn data_labels_can_read_a_column_use_decimals_and_name_the_rows() {
    let (mut p, g) = project(Template::LineSymbol);
    plot_mut(&mut p, g).decor.labels.enable = true;
    assert_eq!(label_texts(&p, g), ["2", "5", "3", "6"]);
    plot_mut(&mut p, g).decor.labels.decimals = Some(1);
    plot_mut(&mut p, g).decor.labels.only_rows = "2, 4".into();
    assert_eq!(label_texts(&p, g), ["5.0", "6.0"]);
    plot_mut(&mut p, g).decor.labels.only_rows.clear();
    // A text column of the sheet gives the words.
    let book = p.workbooks().next().unwrap().id;
    let names = p
        .add_column(
            book,
            0,
            Designation::Label,
            ColumnData::text(["a", "b", "c", "d"].iter().map(|s| s.to_string()).collect()),
        )
        .unwrap();
    plot_mut(&mut p, g).decor.labels.form = LabelForm::Column { column: names };
    assert_eq!(label_texts(&p, g), ["a", "b", "c", "d"]);
    assert!(p.graph(g).unwrap().referenced_columns().contains(&names));
    // A column that is gone leaves the labels empty rather than failing.
    p.column_mut(names).unwrap().data = ColumnData::numeric(vec![]);
    assert!(label_texts(&p, g).is_empty());
}

#[test]
fn a_label_moves_by_its_offset() {
    let (mut p, g) = project(Template::LineSymbol);
    plot_mut(&mut p, g).decor.labels.enable = true;
    plot_mut(&mut p, g).decor.labels.position = LabelPosition::Above;
    let at = |p: &Project| {
        let s = scene(p, g);
        let t = texts(&s.layers[0].plots)[0].clone();
        (t.x, t.y)
    };
    let (x, y) = at(&p);
    plot_mut(&mut p, g).decor.labels.dx_pt = 10.0;
    plot_mut(&mut p, g).decor.labels.dy_pt = -4.0;
    let (x2, y2) = at(&p);
    assert!((x2 - x - 10.0).abs() < 1e-9 && (y2 - y + 4.0).abs() < 1e-9);
}

#[test]
fn the_layer_background_gradient_and_border_are_drawn() {
    let (mut p, g) = project(Template::Line);
    let layer = layer_mut(&mut p, g);
    layer.background = Some(Color::rgb(200, 220, 255));
    layer.background_style.gradient = Some(Gradient::new(Color::WHITE));
    layer.background_style.border = Some(BorderStyle::solid(Color::rgb(1, 2, 3), 2.0));
    let s = scene(&p, g);
    let bands = fills(&s.layers[0].grid);
    assert!(bands.len() >= 4, "the gradient is a run of bands behind the grid");
    assert!(bands[0].b >= 250 && bands.last().unwrap().r > 240, "{bands:?}");
    let border = s.layers[0].overlay.iter().find(
        |i| matches!(i, scene::Item::Rect { fill: None, stroke: Some(st), .. } if st.color == Color::rgb(1, 2, 3)),
    );
    assert!(border.is_some(), "the border line is drawn along the frame");
    // Grid on Top of Data moves the grid lines only: the background bands stay behind the plots.
    let n = s.layers[0].grid.len();
    layer_mut(&mut p, g).y_axis.major_grid = Some(LineStyle::solid(Color::BLACK, 0.5));
    layer_mut(&mut p, g).display.grid_on_top = true;
    let on_top = scene(&p, g);
    assert_eq!(on_top.layers[0].grid.len(), n, "only the background items stay");
}

#[test]
fn the_x_and_y_axes_switches_hide_one_axis_each() {
    let (mut p, g) = project(Template::LineSymbol);
    let count = |p: &Project| texts(&scene(p, g).layers[0].overlay).len();
    let full = count(&p);
    layer_mut(&mut p, g).display.show_x_axes = false;
    let no_x = count(&p);
    assert!(no_x < full, "the X tick labels and title are gone: {no_x} < {full}");
    layer_mut(&mut p, g).display.show_y_axes = false;
    assert!(count(&p) < no_x, "and the Y ones");
    layer_mut(&mut p, g).display.show_x_axes = true;
    layer_mut(&mut p, g).display.show_y_axes = true;
    assert_eq!(count(&p), full);
}

#[test]
fn data_on_top_preserves_legends_annotations_and_the_independent_grid_order() {
    for template in [Template::LineSymbol, Template::Polar, Template::Ternary] {
        let (mut p, g) = project(template);
        let l = layer_mut(&mut p, g);
        l.display.data_on_top = true;
        l.x_axis.major_grid = Some(LineStyle::solid(Color::rgb(11, 22, 33), 0.5));
        l.y_axis.major_grid = Some(LineStyle::solid(Color::rgb(11, 22, 33), 0.5));
        let s = scene(&p, g);
        let l = &s.layers[0];
        let (axes, annotations) = l.split_overlay();
        assert!(!axes.is_empty(), "{template:?}: axes exist");
        assert!(!annotations.is_empty(), "{template:?}: legend exists");
        for object in &l.objects {
            if object.kind == scene::ObjectKind::Legend {
                assert!(object.items.start >= l.underlay_len, "legend remains above data");
            }
        }
        let old_clip = l.clip;
        let old_axes = axes.to_vec();
        let old_annotations = annotations.to_vec();
        layer_mut(&mut p, g).display.grid_on_top = true;
        let on_top = scene(&p, g);
        let l = &on_top.layers[0];
        assert_eq!(l.clip, old_clip);
        assert_eq!(l.split_overlay(), (old_axes.as_slice(), old_annotations.as_slice()));
        assert!(l.grid.is_empty(), "grid moves above the data independently");
    }
}

/// The overlay items (line, ticks, labels, title) that one axis has drawn, and its objects' items.
fn axis_items(s: &scene::Scene, kinds: &[scene::ObjectKind]) -> Vec<scene::Item> {
    let l = &s.layers[0];
    l.objects
        .iter()
        .filter(|o| kinds.contains(&o.kind) && o.list == scene::ItemList::Overlay)
        .flat_map(|o| l.overlay[o.items.clone()].to_vec())
        .collect()
}

#[test]
fn an_axes_switch_takes_the_line_ticks_labels_and_title_of_that_axis_and_leaves_the_grid_and_the_other_axis() {
    use scene::ObjectKind::{XAxis, XTitle, YAxis, YTitle};
    let (mut p, g) = project(Template::LineSymbol);
    layer_mut(&mut p, g).x_axis.major_grid = Some(LineStyle::solid(Color::BLACK, 0.5));
    layer_mut(&mut p, g).y_axis.major_grid = Some(LineStyle::solid(Color::BLACK, 0.5));
    let full = scene(&p, g);
    let x_kinds = [XAxis, XTitle];
    let y_kinds = [YAxis, YTitle];
    let (x_full, y_full) = (axis_items(&full, &x_kinds), axis_items(&full, &y_kinds));
    let paths = |items: &[scene::Item]| items.iter().filter(|i| matches!(i, scene::Item::Path { .. })).count();
    assert!(paths(&x_full) > 0 && paths(&y_full) > 0, "the axis line and the ticks are paths");
    assert!(!full.layers[0].grid.is_empty(), "the grid is drawn");

    layer_mut(&mut p, g).display.show_x_axes = false;
    let no_x = scene(&p, g);
    assert!(axis_items(&no_x, &x_kinds).is_empty(), "no X line, ticks, labels or title");
    assert_eq!(axis_items(&no_x, &y_kinds), y_full, "the Y axis is untouched");
    assert_eq!(no_x.layers[0].grid, full.layers[0].grid, "the grid stays");
    assert_eq!(no_x.layers[0].plots, full.layers[0].plots, "and the data");

    layer_mut(&mut p, g).display.show_x_axes = true;
    layer_mut(&mut p, g).display.show_y_axes = false;
    let no_y = scene(&p, g);
    assert!(axis_items(&no_y, &y_kinds).is_empty(), "no Y line, ticks, labels or title");
    assert_eq!(axis_items(&no_y, &x_kinds), x_full, "the X axis is untouched");
    assert_eq!(no_y.layers[0].grid, full.layers[0].grid, "the grid stays");
}

#[test]
fn a_page_border_is_drawn_inside_the_page_edge() {
    let (mut p, g) = project(Template::Line);
    p.graph_mut(g).unwrap().page.style.border = Some(BorderStyle::solid(Color::BLACK, 4.0));
    let s = scene(&p, g);
    let (w, h) = (s.width, s.height);
    let last = s.layers.last().unwrap();
    let hit = last.overlay.iter().any(|i| {
        matches!(i, scene::Item::Rect { rect, fill: None, stroke: Some(st) }
            if (rect.x - 2.0).abs() < 1e-9 && (rect.y - 2.0).abs() < 1e-9
                && (rect.w - (w - 4.0)).abs() < 1e-9 && (rect.h - (h - 4.0)).abs() < 1e-9 && st.width == 4.0)
    });
    assert!(hit, "{:?}", last.overlay.last());
}

#[test]
fn the_settings_reach_every_export_format() {
    let (mut p, g) = project(Template::Column);
    {
        let l = layer_mut(&mut p, g);
        l.background = Some(Color::rgb(240, 240, 240));
        l.background_style.gradient = Some(Gradient::new(Color::WHITE));
        let plot = &mut l.plots[0];
        plot.decor.fill.pattern = FillPattern::ForwardDiagonal;
        plot.decor.fill.transparency_pct = 30;
        plot.decor.fill.gradient = Some(Gradient::new(Color::WHITE));
        plot.decor.labels.enable = true;
        plot.decor.labels.white_out = true;
    }
    p.graph_mut(g).unwrap().page.style.border = Some(BorderStyle::solid(Color::BLACK, 1.0));
    assert!(render_svg_with(&p, g, &FixedMetrics).unwrap().contains("<path"));
    assert!(render_png(&p, g, 100.0).unwrap().len() > 100);
    assert!(render_pdf(&p, g).unwrap().len() > 100);
    assert!(render_eps(&p, g).unwrap().len() > 100);
    assert!(render_emf(&p, g).unwrap().len() > 100);
    assert!(render_tiff(&p, g, 100.0).unwrap().len() > 100);
}

#[test]
fn a_legend_goes_behind_the_data_or_over_the_labels() {
    // T152: the legend's Push to back of data and Bring to the front.
    let (mut p, g) = project(Template::LineSymbol);
    let font = layer_mut(&mut p, g).new_label_font();
    layer_mut(&mut p, g).labels.push(sheaf_core::graph::TextLabel::new("note", 0.5, 0.5, font));
    let legend_of = |s: &scene::Scene| {
        s.layers[0].objects.iter().find(|o| o.kind == scene::ObjectKind::Legend).cloned().expect("a legend")
    };
    let label_of = |s: &scene::Scene| {
        s.layers[0].objects.iter().find(|o| o.kind == scene::ObjectKind::Label(0)).cloned().expect("the label")
    };
    let before = scene(&p, g);
    let (legend, label) = (legend_of(&before), label_of(&before));
    assert_eq!(before.layers[0].underlay_len, 0);
    assert!(legend.items.end <= label.items.start, "under the labels by default");
    let drawn = before.layers[0].overlay[legend.items.clone()].to_vec();

    layer_mut(&mut p, g).legend.behind_data = true;
    let s = scene(&p, g);
    let (l2, lab2) = (legend_of(&s), label_of(&s));
    assert_eq!(s.layers[0].underlay_len, legend.items.len(), "only the legend is drawn before the data");
    assert!(!s.layers[0].data_on_top);
    assert_eq!(l2.items, 0..legend.items.len());
    assert_eq!(s.layers[0].overlay[l2.items.clone()], drawn[..], "the same items, moved");
    assert_eq!(
        s.layers[0].overlay[lab2.items.clone()],
        before.layers[0].overlay[label.items.clone()],
        "ranges stay right"
    );
    assert_eq!(s.layers[0].overlay.len(), before.layers[0].overlay.len());

    layer_mut(&mut p, g).legend.behind_data = false;
    layer_mut(&mut p, g).legend.on_top = true;
    let s = scene(&p, g);
    let (l3, lab3) = (legend_of(&s), label_of(&s));
    assert!(lab3.items.end <= l3.items.start, "over the labels after Bring to the front");
    assert_eq!(s.layers[0].overlay[l3.items.clone()], drawn[..]);
}

#[test]
fn an_axis_title_brought_to_the_front_is_drawn_last() {
    // T153: an axis title's Bring to the front.
    let (mut p, g) = project(Template::LineSymbol);
    let font = layer_mut(&mut p, g).new_label_font();
    layer_mut(&mut p, g).labels.push(sheaf_core::graph::TextLabel::new("note", 0.5, 0.5, font));
    let of = |s: &scene::Scene, k: scene::ObjectKind| {
        s.layers[0].objects.iter().find(|o| o.kind == k).cloned().expect("the object")
    };
    let before = scene(&p, g);
    let title = of(&before, scene::ObjectKind::YTitle);
    let label = of(&before, scene::ObjectKind::Label(0));
    assert!(title.items.end <= label.items.start, "under the labels by default");
    let drawn = before.layers[0].overlay[title.items.clone()].to_vec();
    layer_mut(&mut p, g).y_axis.title_on_top = true;
    let s = scene(&p, g);
    let (t2, l2) = (of(&s, scene::ObjectKind::YTitle), of(&s, scene::ObjectKind::Label(0)));
    assert_eq!(t2.items.end, s.layers[0].overlay.len(), "drawn last");
    assert_eq!(s.layers[0].overlay[t2.items.clone()], drawn[..]);
    assert_eq!(
        s.layers[0].overlay[l2.items.clone()],
        before.layers[0].overlay[label.items.clone()],
        "ranges stay right"
    );
    let x = of(&s, scene::ObjectKind::XTitle);
    assert_eq!(
        s.layers[0].overlay[x.items.clone()],
        before.layers[0].overlay[of(&before, scene::ObjectKind::XTitle).items][..]
    );
}
