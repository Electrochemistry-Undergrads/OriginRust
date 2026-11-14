//! The look settings of Plot Details (brief T13) drawn into the figure: line transparency, the fill
//! under a curve, drop lines, data labels, hatches and gradients, a layer's background and display
//! switches, and the page border.

use super::*;
use sheaf_core::graph::{
    BorderStyle, DropLines, FillPattern, Gradient, GradientDirection, LabelForm, LabelPosition, LayerBackground,
    PlotKind,
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

fn svg(p: &Project, g: WindowId) -> String {
    render_svg_with(p, g, &FixedMetrics).unwrap()
}

fn plot_mut(p: &mut Project, g: WindowId) -> &mut sheaf_core::graph::Plot {
    &mut p.graph_mut(g).unwrap().layers[0].plots[0]
}

/// The elements of the plots group of layer 0, as text lines.
fn plot_lines(svg: &str) -> Vec<String> {
    let start = svg.find("<g class=\"plots\"").expect("plots group");
    let end = svg[start..].find("</g>").unwrap() + start;
    svg[start..end].lines().skip(1).map(str::to_string).collect()
}

#[test]
fn line_transparency_makes_the_stroke_translucent() {
    let (mut p, g) = project(Template::Line);
    let solid = plot_lines(&svg(&p, g));
    assert!(!solid[0].contains("stroke-opacity"));
    plot_mut(&mut p, g).details.line_transparency_pct = 60;
    let lines = plot_lines(&svg(&p, g));
    assert!(lines[0].contains("stroke-opacity=\"0.4\""), "{}", lines[0]);
}

#[test]
fn fill_under_a_line_adds_a_filled_area_below_the_line_and_follows_the_fill_colour() {
    let (mut p, g) = project(Template::Line);
    assert_eq!(plot_lines(&svg(&p, g)).len(), 1);
    {
        let plot = plot_mut(&mut p, g);
        plot.details.fill_under = true;
        plot.symbol.color = Color::rgb(10, 20, 30);
        plot.details.fill.transparency_pct = 50;
    }
    let lines = plot_lines(&svg(&p, g));
    assert_eq!(lines.len(), 2, "the fill and the line: {lines:?}");
    assert!(lines[0].contains("fill=\"#0A141E\"") && lines[0].contains("fill-opacity=\"0.502\""), "{}", lines[0]);
    assert!(lines[1].contains("fill=\"none\""), "the line is drawn over the fill");
}

#[test]
fn a_scatter_plot_has_no_area_to_fill() {
    let (mut p, g) = project(Template::Scatter);
    let before = plot_lines(&svg(&p, g));
    plot_mut(&mut p, g).details.fill_under = true;
    assert_eq!(plot_lines(&svg(&p, g)), before);
}

#[test]
fn drop_lines_run_from_each_point_to_the_axes() {
    let (mut p, g) = project(Template::Line);
    let n = plot_lines(&svg(&p, g)).len();
    plot_mut(&mut p, g).details.drop_lines =
        DropLines { horizontal: None, vertical: Some(DropLines::starting_style(Color::RED)) };
    let lines = plot_lines(&svg(&p, g));
    assert_eq!(lines.len(), n + 1);
    let drop = lines.iter().find(|l| l.contains("stroke=\"#FF0000\"")).expect("a red drop line path");
    // Four points: four segments, each ending on the same y (the bottom of the frame).
    assert_eq!(drop.matches('M').count(), 4, "{drop}");
    let ends: Vec<&str> =
        drop.split('L').skip(1).map(|s| s.split(['M', '"']).next().unwrap().split(' ').nth(1).unwrap()).collect();
    assert!(ends.windows(2).all(|w| w[0] == w[1]), "{ends:?}");
    assert!(drop.contains("stroke-dasharray"), "dashed as started");
    plot_mut(&mut p, g).details.drop_lines.horizontal = Some(DropLines::starting_style(Color::BLUE));
    assert_eq!(plot_lines(&svg(&p, g)).len(), n + 2);
}

#[test]
fn drop_lines_are_only_drawn_for_kinds_that_offer_them() {
    let (mut p, g) = project(Template::Line);
    let drops = DropLines { horizontal: Some(DropLines::starting_style(Color::BLUE)), vertical: None };
    plot_mut(&mut p, g).details.drop_lines = drops;
    let with_line = plot_lines(&svg(&p, g)).len();
    // The plot becomes a column plot: its Drop Lines tab is gone, so no line may stay behind.
    plot_mut(&mut p, g).kind = PlotKind::Column;
    let column = plot_lines(&svg(&p, g));
    assert!(!column.iter().any(|l| l.contains("stroke=\"#0000FF\"")), "{column:?}");
    // The settings are kept: back to a line, the drop lines are drawn again.
    plot_mut(&mut p, g).kind = PlotKind::Line;
    assert_eq!(plot_lines(&svg(&p, g)).len(), with_line);
}

fn label_texts(svg: &str) -> Vec<String> {
    plot_lines(svg)
        .iter()
        .filter(|l| l.starts_with("<text"))
        .map(|l| l.split('>').nth(1).unwrap().trim_end_matches("</text").to_string())
        .collect()
}

#[test]
fn data_labels_show_the_y_value_the_row_or_a_column_on_the_chosen_rows() {
    let (mut p, g) = project(Template::LineSymbol);
    assert!(label_texts(&svg(&p, g)).is_empty());
    plot_mut(&mut p, g).details.labels.enabled = true;
    assert_eq!(label_texts(&svg(&p, g)), ["2", "5", "3", "6"]);
    plot_mut(&mut p, g).details.labels.decimals = Some(1);
    plot_mut(&mut p, g).details.labels.only_rows = "2, 4".into();
    assert_eq!(label_texts(&svg(&p, g)), ["5.0", "6.0"]);
    plot_mut(&mut p, g).details.labels.only_rows.clear();
    plot_mut(&mut p, g).details.labels.form = LabelForm::X;
    assert_eq!(label_texts(&svg(&p, g)), ["1.0", "2.0", "3.0", "4.0"]);
    plot_mut(&mut p, g).details.labels.form = LabelForm::Row;
    assert_eq!(label_texts(&svg(&p, g)), ["1", "2", "3", "4"]);
    // A text column of the sheet gives the words.
    let book = p.workbooks().next().unwrap().id;
    let names = p
        .add_column(
            book,
            0,
            Designation::Label,
            ColumnData::Text(vec!["a".to_string(), "b".into(), "c".into(), "d".into()].into()),
        )
        .unwrap();
    plot_mut(&mut p, g).details.labels.form = LabelForm::Column { column: names };
    assert_eq!(label_texts(&svg(&p, g)), ["a", "b", "c", "d"]);
}

#[test]
fn a_label_stands_on_the_side_its_position_names_and_moves_by_its_offset() {
    let (mut p, g) = project(Template::LineSymbol);
    plot_mut(&mut p, g).details.labels.enabled = true;
    let anchor_of = |p: &Project| {
        let s = svg(p, g);
        let line = plot_lines(&s).into_iter().find(|l| l.starts_with("<text")).unwrap();
        let attr = |name: &str| -> f64 {
            let at = line.find(&format!(" {name}=\"")).unwrap() + name.len() + 3;
            line[at..].split('"').next().unwrap().parse().unwrap()
        };
        (attr("x"), attr("y"), line.contains("text-anchor=\"end\""), line.contains("text-anchor=\"middle\""))
    };
    let right = anchor_of(&p);
    plot_mut(&mut p, g).details.labels.position = LabelPosition::Left;
    let left = anchor_of(&p);
    assert!(left.0 < right.0 && left.2 && !right.2, "left of the point: {left:?} vs {right:?}");
    plot_mut(&mut p, g).details.labels.position = LabelPosition::Above;
    let above = anchor_of(&p);
    assert!(above.1 < right.1 && above.3, "above is higher, centred: {above:?}");
    plot_mut(&mut p, g).details.labels.dx_pt = 10.0;
    assert!((anchor_of(&p).0 - above.0 - 10.0).abs() < 1e-6);
}

#[test]
fn a_hatched_column_gets_pattern_lines_and_the_outline_on_top() {
    let (mut p, g) = project(Template::Column);
    let plain = plot_lines(&svg(&p, g));
    assert!(plain.iter().all(|l| l.starts_with("<rect")), "one rectangle per bar");
    {
        let plot = plot_mut(&mut p, g);
        plot.details.fill.pattern = FillPattern::Cross;
        plot.details.fill.pattern_color = Color::BLUE;
    }
    let lines = plot_lines(&svg(&p, g));
    // Per bar: the fill, the hatch lines, the outline.
    assert_eq!(lines.len(), 3 * plain.len());
    assert!(lines[0].starts_with("<path") && lines[0].contains("fill=\""));
    assert!(lines[1].contains("stroke=\"#0000FF\"") && lines[1].contains("fill=\"none\""));
    assert!(lines[2].starts_with("<rect") && lines[2].contains("fill=\"none\""));
}

#[test]
fn a_gradient_column_is_drawn_as_bands_between_the_two_colours() {
    let (mut p, g) = project(Template::Column);
    {
        let plot = plot_mut(&mut p, g);
        plot.symbol.color = Color::rgb(255, 0, 0);
        plot.details.fill.gradient =
            Some(Gradient { color: Color::rgb(0, 0, 255), direction: GradientDirection::BottomToTop });
    }
    let lines = plot_lines(&svg(&p, g));
    let fills: Vec<&String> = lines.iter().filter(|l| l.starts_with("<path")).collect();
    assert!(fills.len() > 8, "bands: {}", fills.len());
    // The first band of the first bar is nearly the fill colour (it starts at the bottom).
    assert!(fills[0].contains("fill=\"#F"), "{}", fills[0]);
}

#[test]
fn the_layer_background_fills_the_frame_and_its_border_is_drawn_over_the_plots() {
    let (mut p, g) = project(Template::Line);
    assert!(!svg(&p, g).contains("class=\"grid\""), "no background, no grid: nothing in the grid group");
    p.graph_mut(g).unwrap().layers[0].background = LayerBackground {
        color: Some(Color::rgb(200, 220, 255)),
        transparency_pct: 0,
        gradient: None,
        border: Some(BorderStyle::solid(Color::rgb(1, 2, 3), 2.0)),
    };
    let s = svg(&p, g);
    let grid = &s[s.find("class=\"grid\"").expect("a grid group")..];
    assert!(grid.lines().nth(1).unwrap().contains("fill=\"#C8DCFF\""), "{grid}");
    let overlay = &s[s.find("class=\"overlay\"").unwrap()..];
    assert!(overlay.lines().nth(1).unwrap().contains("stroke=\"#010203\""), "the border comes first in the overlay");
}

#[test]
fn display_switches_leave_out_plots_axes_labels_or_the_whole_layer() {
    let (mut p, g) = project(Template::LineSymbol);
    {
        let l = &mut p.graph_mut(g).unwrap().layers[0];
        l.labels.push(sheaf_core::graph::TextLabel::new("note", 0.5, 0.5, l.new_label_font()));
    }
    let full = svg(&p, g);
    assert!(full.contains(">note<") && full.contains("class=\"plots\""));
    let count_text = |s: &str| s.matches("<text").count();
    let n_full = count_text(&full);
    p.graph_mut(g).unwrap().layers[0].display.show_labels = false;
    assert!(!svg(&p, g).contains(">note<"));
    p.graph_mut(g).unwrap().layers[0].display.show_labels = true;
    p.graph_mut(g).unwrap().layers[0].display.show_axes = false;
    let no_axes = svg(&p, g);
    assert!(count_text(&no_axes) < n_full, "tick labels and titles are gone");
    assert!(no_axes.contains(">note<"));
    p.graph_mut(g).unwrap().layers[0].display.show_axes = true;
    p.graph_mut(g).unwrap().layers[0].display.show_plots = false;
    assert!(plot_lines(&svg(&p, g)).is_empty());
    p.graph_mut(g).unwrap().layers[0].display.show_plots = true;
    p.graph_mut(g).unwrap().layers[0].display.show_layer = false;
    let hidden = svg(&p, g);
    assert!(!hidden.contains("<text") && plot_lines(&hidden).is_empty(), "nothing of a hidden layer");
}

#[test]
fn a_page_border_is_drawn_inside_the_page_edge() {
    let (mut p, g) = project(Template::Line);
    p.graph_mut(g).unwrap().page.style.border = Some(BorderStyle::solid(Color::BLACK, 4.0));
    let s = svg(&p, g);
    assert!(s.contains("<rect x=\"2\" y=\"2\" width=\"767.84\" height=\"586.76\" fill=\"none\" stroke=\"#000000\" stroke-width=\"4\"/>"), "{s}");
}

#[test]
fn the_settings_reach_every_export_format() {
    let (mut p, g) = project(Template::Column);
    {
        let l = &mut p.graph_mut(g).unwrap().layers[0];
        l.background.color = Some(Color::rgb(240, 240, 240));
        let plot = &mut l.plots[0];
        plot.kind = PlotKind::Column;
        plot.details.fill.pattern = FillPattern::ForwardDiagonal;
        plot.details.fill.transparency_pct = 30;
        plot.details.fill.gradient = Some(Gradient::new(Color::WHITE));
        plot.details.labels.enabled = true;
        plot.details.line_transparency_pct = 20;
    }
    for ext in ["png", "pdf", "svg", "eps", "emf"] {
        let path = std::env::temp_dir().join(format!("sheaf-details-{}.{ext}", std::process::id()));
        export_file(&p, g, &path, 150.0).unwrap_or_else(|e| panic!("{ext}: {e}"));
        assert!(std::fs::metadata(&path).unwrap().len() > 100, "{ext}");
        let _ = std::fs::remove_file(&path);
    }
}
