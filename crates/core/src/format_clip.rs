//! Copy Format and Paste Format on graphs (brief T72, graphing spec section 26).
//!
//! Copy Format takes one value per format group from a whole graph (every layer: page and layer
//! size, scales, colours, fonts, layer background, axis thickness, symbols and lines, text) or from
//! one axis or legend. Paste Format applies the groups that the paste asks for and that the copy holds.
//! The clipboard's format type identifies its matching target.
//!
//! The group of every property follows what was measured in Origin 2016 (spec 24): Colors takes
//! the axis line, tick-label, title and text colours, the layer background colour and the plots'
//! line and symbol colours; Fonts takes fonts without their colours; Dimensions the layer width,
//! left and top and the page width; Scales the axes' limits and steps; All Style Formats is
//! Colors, Fonts, Background, axis thickness and the plots' symbols and lines, but neither scales,
//! dimensions nor text; All adds those. Layers and plots are paired by position; a target with
//! more layers or plots than the source keeps the extra ones as they are, and extra source
//! layers and plots are dropped (Origin's rule for mismatched counts is only partly measured).

use crate::Color;
use crate::graph::{Axis, BackgroundStyle, Font, Frame, Graph, Layer, LineStyle, SymbolStyle, TextLabel};

/// One group of properties that a format holds.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Group {
    /// Layer width, left and top; page width.
    Dimensions,
    /// Limits, scale type, steps of both axes.
    Scales,
    /// Axis line, tick-label, title, legend and text colours; layer background colour; the plots'
    /// line and symbol colours.
    Colors,
    /// Fonts of tick labels, titles, legend and text, without their colours.
    Fonts,
    /// Layer background colour, gradient and border.
    Background,
    /// Width of the axis lines and ticks.
    Thickness,
    /// The plots' lines and symbols (sizes, shapes, widths).
    SymbolLineFill,
    /// The words of the text labels.
    Text,
}

/// What Copy Format of a graph copies (Edit menu and page menu of Origin).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum GraphPart {
    All,
    AllStyle,
    Colors,
    Fonts,
    Background,
    Dimensions,
    Scales,
}

impl GraphPart {
    /// The groups this choice copies.
    pub fn groups(self) -> &'static [Group] {
        use Group::*;
        match self {
            GraphPart::All => &[Dimensions, Scales, Colors, Fonts, Background, Thickness, SymbolLineFill, Text],
            GraphPart::AllStyle => &[Colors, Fonts, Background, Thickness, SymbolLineFill],
            GraphPart::Colors => &[Colors],
            GraphPart::Fonts => &[Fonts],
            GraphPart::Background => &[Background],
            GraphPart::Dimensions => &[Dimensions],
            GraphPart::Scales => &[Scales],
        }
    }
}

/// What Copy Format or Paste Format To of an axis chooses.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AxisPart {
    /// Styles and scale.
    All,
    /// Colour, thickness, tick directions and lengths, tick-label font, title font.
    Styles,
    /// Limits, scale type, steps.
    Scales,
    // The finer groups of the axis's Paste Format To (T171; Sheaf's field choices, not measured).
    // Each pastes a part of Styles and needs a clipboard that holds Styles.
    /// Major and minor tick directions.
    TickDirection,
    /// Tick directions, lengths, width and colour.
    Ticks,
    /// The axis line: colour (and following the plot) and thickness.
    Line,
    /// Tick label display: shown, font, angle, number format, prefix and suffix, minor and end labels.
    TickLabels,
    /// The title's font (family, size, bold, italic, colour) and following the plot's colour.
    TitleStyles,
    /// Every colour: line, ticks, tick labels and title, and following the plot.
    Colors,
    /// Line and tick widths.
    Thickness,
    /// The title's text and whether it is shown; needs a clipboard that holds All.
    TitleText,
}

/// The values of one axis format (a whole axis is kept; the groups say what pastes).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AxisFormat {
    pub styles: bool,
    pub scales: bool,
    axis: Box<Axis>,
}

/// The values of one layer of a graph format.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
struct LayerFormat {
    frame: Frame,
    /// The layer is validly linked: its frame is then relative to its parent, not to the page.
    linked: bool,
    x: Axis,
    y: Axis,
    background: Option<Color>,
    background_style: BackgroundStyle,
    legend_font: Font,
    labels: Vec<TextLabel>,
    plots: Vec<(LineStyle, SymbolStyle, bool)>,
}

/// The values of a whole graph format.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GraphFormat {
    groups: Vec<Group>,
    page_width_pt: f64,
    layers: Vec<LayerFormat>,
}

impl GraphFormat {
    /// The groups this format holds.
    pub fn groups(&self) -> &[Group] {
        &self.groups
    }
}

/// The appearance of the selected legend.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LegendStyles {
    font: Font,
    frame: bool,
    background: Option<Color>,
}

/// The format clipboard for a graph, an axis or a legend.
/// Serializable so that it can be kept as a graph theme (T102).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum FormatClip {
    Graph(GraphFormat),
    Axis(AxisFormat),
    LegendStyles(LegendStyles),
}

/// Copy Format on a graph.
pub fn copy_graph(graph: &Graph, part: GraphPart) -> FormatClip {
    FormatClip::Graph(GraphFormat {
        groups: part.groups().to_vec(),
        page_width_pt: graph.page.width_pt,
        layers: graph
            .layers
            .iter()
            .enumerate()
            .map(|(i, l)| LayerFormat {
                frame: l.frame,
                linked: graph.link_parent(i).is_some(),
                x: l.x_axis.clone(),
                y: l.y_axis.clone(),
                background: l.background,
                background_style: l.background_style,
                legend_font: l.legend.font.clone(),
                labels: l.labels.clone(),
                plots: l.plots.iter().map(|p| (p.line.clone(), p.symbol.clone(), p.auto_color)).collect(),
            })
            .collect(),
    })
}

/// Copy Format on one axis of a layer; `None` when the layer is not there.
pub fn copy_axis(graph: &Graph, layer: usize, y: bool, part: AxisPart) -> Option<FormatClip> {
    let l = graph.layers.get(layer)?;
    Some(FormatClip::Axis(AxisFormat {
        styles: matches!(part, AxisPart::All | AxisPart::Styles),
        scales: matches!(part, AxisPart::All | AxisPart::Scales),
        axis: Box::new(if y { l.y_axis.clone() } else { l.x_axis.clone() }),
    }))
}

/// Copies the appearance of the visible legend on the selected layer.
pub fn copy_legend_styles(graph: &Graph, layer: usize) -> Option<FormatClip> {
    let legend = &graph.layers.get(layer)?.legend;
    legend.visible.then(|| {
        FormatClip::LegendStyles(LegendStyles {
            font: legend.font.clone(),
            frame: legend.frame,
            background: legend.background,
        })
    })
}

fn paste_font(to: &mut Font, from: &Font) {
    let color = to.color;
    *to = Font { color, ..from.clone() };
}

fn paste_scale(to: &mut Axis, from: &Axis) {
    to.scale = from.scale;
    to.min = from.min;
    to.max = from.max;
    to.auto_range = from.auto_range;
    to.rescale_margin_pct = from.rescale_margin_pct;
    to.major_step = from.major_step;
    to.minor_ticks = from.minor_ticks;
    to.reverse = from.reverse;
    to.first_tick = from.first_tick;
}

fn paste_axis_styles(to: &mut Axis, from: &Axis) {
    to.color = from.color;
    to.color_follows_plot = from.color_follows_plot;
    to.title_color_follows_plot = from.title_color_follows_plot;
    to.tick_color = from.tick_color;
    to.line_width_pt = from.line_width_pt;
    to.tick_width_pt = from.tick_width_pt;
    to.tick_font = from.tick_font.clone();
    to.title_font = from.title_font.clone();
    to.major_tick = from.major_tick;
    to.minor_tick = from.minor_tick;
    to.major_tick_len_pt = from.major_tick_len_pt;
    to.minor_tick_len_pt = from.minor_tick_len_pt;
}

/// `to_linked`: the target layer is validly linked. A frame is pasted only between layers of the
/// same kind, since a linked frame is relative to its parent and a root frame to the page.
fn paste_layer(to: &mut Layer, to_linked: bool, from: &LayerFormat, has: &dyn Fn(Group) -> bool) {
    if has(Group::Dimensions) && to_linked == from.linked {
        to.frame.left = from.frame.left;
        to.frame.top = from.frame.top;
        to.frame.width = from.frame.width;
    }
    for (t, f) in [(&mut to.x_axis, &from.x), (&mut to.y_axis, &from.y)] {
        if has(Group::Scales) {
            paste_scale(t, f);
        }
        if has(Group::Colors) {
            t.color = f.color;
            t.color_follows_plot = f.color_follows_plot;
            t.title_color_follows_plot = f.title_color_follows_plot;
            t.tick_color = f.tick_color;
            t.tick_font.color = f.tick_font.color;
            t.title_font.color = f.title_font.color;
        }
        if has(Group::Fonts) {
            paste_font(&mut t.tick_font, &f.tick_font);
            paste_font(&mut t.title_font, &f.title_font);
        }
        if has(Group::Thickness) {
            t.line_width_pt = f.line_width_pt;
            t.tick_width_pt = f.tick_width_pt;
        }
    }
    if has(Group::Background) {
        to.background = from.background;
        to.background_style = from.background_style;
    } else if has(Group::Colors) {
        to.background = from.background;
    }
    if has(Group::Colors) {
        to.legend.font.color = from.legend_font.color;
    }
    if has(Group::Fonts) {
        paste_font(&mut to.legend.font, &from.legend_font);
    }
    for (t, f) in to.labels.iter_mut().zip(&from.labels) {
        if has(Group::Colors) {
            t.font.color = f.font.color;
        }
        if has(Group::Fonts) {
            paste_font(&mut t.font, &f.font);
        }
        if has(Group::Text) {
            t.text.clone_from(&f.text);
        }
    }
    for (t, (line, symbol, auto)) in to.plots.iter_mut().zip(&from.plots) {
        if has(Group::SymbolLineFill) {
            t.line = line.clone();
            t.symbol = symbol.clone();
            t.auto_color = *auto;
        } else if has(Group::Colors) {
            t.line.color = line.color;
            t.symbol.color = symbol.color;
            t.auto_color = *auto;
        }
    }
}

/// Paste Format of a graph format onto a graph: the groups of `parts` that the format holds.
/// Returns false for a different clipboard type.
pub fn paste_graph(graph: &mut Graph, clip: &FormatClip, parts: &[Group]) -> bool {
    let FormatClip::Graph(f) = clip else { return false };
    let has = |g: Group| parts.contains(&g) && f.groups.contains(&g);
    if has(Group::Dimensions) {
        graph.page.width_pt = f.page_width_pt;
    }
    let linked: Vec<bool> = (0..graph.layers.len()).map(|i| graph.link_parent(i).is_some()).collect();
    for ((layer, from), to_linked) in graph.layers.iter_mut().zip(&f.layers).zip(linked) {
        paste_layer(layer, to_linked, from, &has);
    }
    true
}

/// Paste Format To on one axis: the parts of `part` that the format holds. Returns false (and
/// changes nothing) for a different clipboard type or when the axis is absent.
pub fn paste_axis(graph: &mut Graph, layer: usize, y: bool, clip: &FormatClip, part: AxisPart) -> bool {
    let FormatClip::Axis(f) = clip else { return false };
    let Some(l) = graph.layers.get_mut(layer) else { return false };
    let axis = if y { &mut l.y_axis } else { &mut l.x_axis };
    if f.scales && matches!(part, AxisPart::All | AxisPart::Scales) {
        paste_scale(axis, &f.axis);
    }
    if f.styles && matches!(part, AxisPart::All | AxisPart::Styles) {
        paste_axis_styles(axis, &f.axis);
    }
    if f.styles {
        paste_axis_group(axis, &f.axis, part, f.scales);
    }
    true
}

/// The finer groups of Paste Format To (T171). `all`: the clipboard holds All (styles and scales).
fn paste_axis_group(to: &mut Axis, from: &Axis, part: AxisPart, all: bool) {
    match part {
        AxisPart::All | AxisPart::Styles | AxisPart::Scales => {}
        AxisPart::TickDirection => {
            to.major_tick = from.major_tick;
            to.minor_tick = from.minor_tick;
        }
        AxisPart::Ticks => {
            to.major_tick = from.major_tick;
            to.minor_tick = from.minor_tick;
            to.major_tick_len_pt = from.major_tick_len_pt;
            to.minor_tick_len_pt = from.minor_tick_len_pt;
            to.tick_width_pt = from.tick_width_pt;
            to.tick_color = from.tick_color;
        }
        AxisPart::Line => {
            to.color = from.color;
            to.color_follows_plot = from.color_follows_plot;
            to.line_width_pt = from.line_width_pt;
        }
        AxisPart::TickLabels => {
            to.show_tick_labels = from.show_tick_labels;
            to.tick_font = from.tick_font.clone();
            to.tick_label_angle_deg = from.tick_label_angle_deg;
            to.tick_format = from.tick_format;
            to.tick_prefix.clone_from(&from.tick_prefix);
            to.tick_suffix.clone_from(&from.tick_suffix);
            to.minor_tick_labels = from.minor_tick_labels;
            to.hide_end_label = from.hide_end_label;
        }
        AxisPart::TitleStyles => {
            to.title_font = from.title_font.clone();
            to.title_color_follows_plot = from.title_color_follows_plot;
        }
        AxisPart::Colors => {
            to.color = from.color;
            to.color_follows_plot = from.color_follows_plot;
            to.title_color_follows_plot = from.title_color_follows_plot;
            to.tick_color = from.tick_color;
            to.tick_font.color = from.tick_font.color;
            to.title_font.color = from.title_font.color;
        }
        AxisPart::Thickness => {
            to.line_width_pt = from.line_width_pt;
            to.tick_width_pt = from.tick_width_pt;
        }
        AxisPart::TitleText => {
            if all {
                to.title.clone_from(&from.title);
                to.show_title = from.show_title;
            }
        }
    }
}

/// Applies the stored style to the visible legend of one layer.
pub fn paste_legend_styles(graph: &mut Graph, layer: usize, clip: &FormatClip) -> bool {
    let FormatClip::LegendStyles(styles) = clip else { return false };
    let Some(layer) = graph.layers.get_mut(layer) else { return false };
    let legend = &mut layer.legend;
    if !legend.visible {
        return false;
    }
    legend.font.clone_from(&styles.font);
    legend.frame = styles.frame;
    legend.background = styles.background;
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::WindowId;
    use crate::template::Template;

    fn plot(color: Color, size: f64) -> crate::graph::Plot {
        let mut p: crate::graph::Plot = serde_json::from_value(serde_json::json!({
            "x": null, "y": 1, "kind": "line",
            "line": {"color": "#000000", "width_pt": 1.0},
            "symbol": {"shape": "circle", "size_pt": 5.0, "color": "#000000",
                       "filled": true, "edge_width_pt": 1.0},
        }))
        .unwrap();
        p.line.color = color;
        p.symbol.color = color;
        p.symbol.size_pt = size;
        p
    }

    /// A graph and a differently styled graph of the same shape.
    fn pair() -> (Graph, Graph) {
        let mut src = Template::Line.new_graph(WindowId(1), "S");
        let dst = Template::Line.new_graph(WindowId(2), "D");
        src.page.width_pt = 500.0;
        let l = &mut src.layers[0];
        l.frame = Frame { left: 0.2, top: 0.3, width: 0.5, height: 0.4 };
        l.x_axis.min = Some(1.0);
        l.x_axis.max = Some(9.0);
        l.x_axis.color = Color::RED;
        l.x_axis.tick_font.color = Color::BLUE;
        l.x_axis.tick_font.size_pt = 22.0;
        l.x_axis.line_width_pt = 3.0;
        l.background = Some(Color::rgb(1, 2, 3));
        l.legend.font.size_pt = 17.0;
        l.legend.font.color = Color::RED;
        l.labels.push(TextLabel::new(
            "source words",
            0.1,
            0.1,
            Font { size_pt: 30.0, color: Color::BLUE, ..l.legend.font.clone() },
        ));
        l.plots.push(plot(Color::RED, 12.0));
        let mut dst = dst;
        dst.layers[0].labels.push(TextLabel::new("target words", 0.5, 0.5, Font::new("Arial", 8.0)));
        dst.layers[0].plots.push(plot(Color::BLACK, 5.0));
        (src, dst)
    }

    fn pasted(part: GraphPart) -> (Graph, Graph) {
        let (src, mut dst) = pair();
        let before = dst.clone();
        assert!(paste_graph(&mut dst, &copy_graph(&src, part), part.groups()));
        (before, dst)
    }

    #[test]
    fn a_legend_style_clip_round_trips_and_only_changes_one_legend() {
        use crate::graph::{LegendPosition, LegendScope};

        let (mut src, mut dst) = pair();
        let legend = &mut src.layers[0].legend;
        legend.visible = true;
        legend.frame = true;
        legend.background = Some(Color::rgb(1, 2, 3));
        legend.font.bold = true;
        legend.font.italic = true;
        let styles = legend.clone();
        let clip = copy_legend_styles(&src, 0).unwrap();
        let clip: FormatClip = serde_json::from_str(&serde_json::to_string(&clip).unwrap()).unwrap();
        src.layers[0].legend.font.size_pt = 8.0;

        dst.add_layer(crate::layers::NewLayerKind::Inset, 0);
        let legend = &mut dst.layers[1].legend;
        legend.visible = true;
        legend.position = LegendPosition::Manual { x: 0.25, y: 0.5 };
        legend.text = Some("Target \\l(1) %(1)".into());
        legend.scope = LegendScope::Graph;
        legend.columns = 2;
        let mut expected = dst.clone();
        let legend = &mut expected.layers[1].legend;
        legend.font = styles.font;
        legend.frame = styles.frame;
        legend.background = styles.background;
        assert!(paste_legend_styles(&mut dst, 1, &clip));
        assert_eq!(dst, expected);
        assert!(!paste_graph(&mut dst, &clip, GraphPart::All.groups()));
        assert!(!paste_axis(&mut dst, 0, false, &clip, AxisPart::All));
        assert!(!paste_legend_styles(&mut dst, 2, &clip));
        dst.layers[1].legend.visible = false;
        assert!(copy_legend_styles(&dst, 1).is_none());
        assert!(!paste_legend_styles(&mut dst, 1, &clip));
    }

    #[test]
    fn dimensions_change_only_layer_width_left_top_and_page_width() {
        let (before, after) = pasted(GraphPart::Dimensions);
        assert_eq!(after.page.width_pt, 500.0);
        assert_eq!(after.page.height_pt, before.page.height_pt);
        let f = after.layers[0].frame;
        assert_eq!((f.left, f.top, f.width), (0.2, 0.3, 0.5));
        assert_eq!(f.height, before.layers[0].frame.height, "height is not copied");
        let mut expect = before.clone();
        expect.page.width_pt = 500.0;
        expect.layers[0].frame = f;
        assert_eq!(after, expect);
    }

    #[test]
    fn scales_change_only_the_axis_limits_and_steps() {
        let (before, after) = pasted(GraphPart::Scales);
        assert_eq!((after.layers[0].x_axis.min, after.layers[0].x_axis.max), (Some(1.0), Some(9.0)));
        let mut expect = before.clone();
        expect.layers[0].x_axis.min = Some(1.0);
        expect.layers[0].x_axis.max = Some(9.0);
        assert_eq!(after, expect);
    }

    #[test]
    fn colors_change_colours_and_nothing_else() {
        let (before, after) = pasted(GraphPart::Colors);
        let l = &after.layers[0];
        assert_eq!(l.x_axis.color, Color::RED);
        assert_eq!(l.x_axis.tick_font.color, Color::BLUE);
        assert_eq!(l.x_axis.tick_font.size_pt, before.layers[0].x_axis.tick_font.size_pt, "no sizes");
        assert_eq!(l.x_axis.line_width_pt, before.layers[0].x_axis.line_width_pt, "no thickness");
        assert_eq!(l.background, Some(Color::rgb(1, 2, 3)));
        assert_eq!(l.legend.font.color, Color::RED);
        assert_eq!(l.labels[0].font.color, Color::BLUE);
        assert_eq!(l.labels[0].font.size_pt, 8.0);
        assert_eq!(l.labels[0].text, "target words");
        assert_eq!((l.plots[0].line.color, l.plots[0].symbol.color), (Color::RED, Color::RED));
        assert_eq!(l.plots[0].symbol.size_pt, 5.0, "no symbol sizes");
        assert_eq!(l.x_axis.min, None, "no scales");
        assert_eq!(l.frame, before.layers[0].frame, "no dimensions");
    }

    #[test]
    fn fonts_change_sizes_and_keep_colours() {
        let (before, after) = pasted(GraphPart::Fonts);
        let l = &after.layers[0];
        assert_eq!(l.x_axis.tick_font.size_pt, 22.0);
        assert_eq!(l.x_axis.tick_font.color, before.layers[0].x_axis.tick_font.color, "no colours");
        assert_eq!(l.legend.font.size_pt, 17.0);
        assert_eq!(l.labels[0].font.size_pt, 30.0);
        assert_eq!(l.labels[0].font.color, before.layers[0].labels[0].font.color);
        assert_eq!(l.plots, before.layers[0].plots);
        assert_eq!(l.background, before.layers[0].background);
    }

    #[test]
    fn background_changes_the_layer_background_only() {
        let (before, after) = pasted(GraphPart::Background);
        let mut expect = before.clone();
        expect.layers[0].background = Some(Color::rgb(1, 2, 3));
        assert_eq!(after, expect);
    }

    #[test]
    fn all_style_formats_take_style_but_not_scales_dimensions_or_text() {
        let (before, after) = pasted(GraphPart::AllStyle);
        let (b, l) = (&before.layers[0], &after.layers[0]);
        assert_eq!(l.x_axis.line_width_pt, 3.0, "thickness");
        assert_eq!(l.plots[0].symbol.size_pt, 12.0, "symbol sizes");
        assert_eq!(l.x_axis.color, Color::RED);
        assert_eq!(l.x_axis.tick_font.size_pt, 22.0);
        assert_eq!((l.x_axis.min, l.frame, after.page.width_pt), (b.x_axis.min, b.frame, before.page.width_pt));
        assert_eq!(l.labels[0].text, "target words", "no text content");
    }

    #[test]
    fn all_adds_scales_dimensions_and_text() {
        let (_, after) = pasted(GraphPart::All);
        let l = &after.layers[0];
        assert_eq!(l.x_axis.min, Some(1.0));
        assert_eq!(after.page.width_pt, 500.0);
        assert_eq!(l.frame.width, 0.5);
        assert_eq!(l.labels[0].text, "source words");
        assert_eq!((l.labels[0].x, l.labels[0].y), (0.5, 0.5), "the label stays where it is");
    }

    #[test]
    fn a_paste_asks_for_parts_and_the_copy_must_hold_them() {
        let (src, mut dst) = pair();
        let before = dst.clone();
        let clip = copy_graph(&src, GraphPart::Fonts);
        assert!(paste_graph(&mut dst, &clip, &[Group::Colors, Group::Scales]));
        assert_eq!(dst, before, "a Fonts copy holds no colours or scales");
        assert!(paste_graph(&mut dst, &clip, &[Group::Fonts]));
        assert_ne!(dst, before);
    }

    #[test]
    fn pasting_twice_changes_nothing_more_and_the_copy_is_unaffected_by_later_edits() {
        let (mut src, mut dst) = pair();
        let clip = copy_graph(&src, GraphPart::All);
        src.layers[0].x_axis.min = Some(-5.0);
        paste_graph(&mut dst, &clip, GraphPart::All.groups());
        let once = dst.clone();
        paste_graph(&mut dst, &clip, GraphPart::All.groups());
        assert_eq!(dst, once);
        assert_eq!(dst.layers[0].x_axis.min, Some(1.0));
    }

    #[test]
    fn mismatched_plots_and_layers_pair_by_position() {
        let (mut src, mut dst) = pair();
        src.layers[0].plots.push(plot(Color::BLUE, 9.0));
        // One plot in the target takes the first plot of the source.
        paste_graph(&mut dst, &copy_graph(&src, GraphPart::AllStyle), GraphPart::AllStyle.groups());
        assert_eq!(dst.layers[0].plots.len(), 1);
        assert_eq!(dst.layers[0].plots[0].symbol.color, Color::RED);
        // Three plots in the target: the third is left alone.
        let (src, mut dst) = pair();
        dst.layers[0].plots.push(plot(Color::BLACK, 5.0));
        dst.layers[0].plots.push(plot(Color::BLACK, 7.0));
        paste_graph(&mut dst, &copy_graph(&src, GraphPart::AllStyle), GraphPart::AllStyle.groups());
        assert_eq!(dst.layers[0].plots[0].symbol.size_pt, 12.0);
        assert_eq!(dst.layers[0].plots[1].symbol.size_pt, 5.0);
        assert_eq!(dst.layers[0].plots[2].symbol.size_pt, 7.0);
        // A two-layer target: the second layer has no counterpart and stays.
        let (src, mut dst) = pair();
        dst.add_layer(crate::layers::NewLayerKind::Inset, 0);
        let second = dst.layers[1].clone();
        paste_graph(&mut dst, &copy_graph(&src, GraphPart::All), GraphPart::All.groups());
        assert_eq!(dst.layers[1], second);
        assert_eq!(dst.layers[0].x_axis.min, Some(1.0));
    }

    fn axis_pair() -> (Graph, Graph) {
        let (mut src, dst) = pair();
        src.layers[0].y_axis.max = Some(77.0);
        src.layers[0].y_axis.color = Color::BLUE;
        (src, dst)
    }

    #[test]
    fn axis_all_changes_only_the_chosen_axis() {
        let (src, mut dst) = axis_pair();
        let before = dst.clone();
        let clip = copy_axis(&src, 0, false, AxisPart::All).unwrap();
        assert!(paste_axis(&mut dst, 0, false, &clip, AxisPart::All));
        let (a, b) = (&dst.layers[0], &before.layers[0]);
        assert_eq!((a.x_axis.min, a.x_axis.max), (Some(1.0), Some(9.0)));
        assert_eq!(a.x_axis.color, Color::RED);
        assert_eq!(a.x_axis.tick_font.size_pt, 22.0);
        assert_eq!(a.x_axis.tick_font.color, Color::BLUE);
        assert_eq!(a.y_axis, b.y_axis, "the other axis is untouched");
        assert_eq!((a.frame, a.background, &a.plots), (b.frame, b.background, &b.plots));
        assert_eq!(dst.page, before.page);
    }

    #[test]
    fn axis_parts_scales_and_styles_are_separate() {
        let (src, mut dst) = axis_pair();
        let before = dst.clone();
        let all = copy_axis(&src, 0, false, AxisPart::All).unwrap();
        paste_axis(&mut dst, 0, false, &all, AxisPart::Scales);
        assert_eq!(dst.layers[0].x_axis.max, Some(9.0));
        assert_eq!(dst.layers[0].x_axis.color, before.layers[0].x_axis.color);
        let (_, mut dst) = axis_pair();
        paste_axis(&mut dst, 0, false, &all, AxisPart::Styles);
        assert_eq!(dst.layers[0].x_axis.max, None);
        assert_eq!(dst.layers[0].x_axis.color, Color::RED);
        // A copy of only the scales holds no styles.
        let (_, mut dst) = axis_pair();
        let scales = copy_axis(&src, 0, false, AxisPart::Scales).unwrap();
        paste_axis(&mut dst, 0, false, &scales, AxisPart::All);
        assert_eq!(dst.layers[0].x_axis.max, Some(9.0));
        assert_eq!(dst.layers[0].x_axis.color, before.layers[0].x_axis.color);
    }

    #[test]
    fn an_axis_format_can_go_to_the_other_axis_and_the_kinds_do_not_mix() {
        let (src, mut dst) = axis_pair();
        let before = dst.clone();
        let x = copy_axis(&src, 0, false, AxisPart::All).unwrap();
        paste_axis(&mut dst, 0, true, &x, AxisPart::All);
        assert_eq!(dst.layers[0].y_axis.max, Some(9.0));
        assert_eq!(dst.layers[0].x_axis, before.layers[0].x_axis);
        assert!(!paste_graph(&mut dst, &x, GraphPart::All.groups()));
        let g = copy_graph(&src, GraphPart::All);
        assert!(!paste_axis(&mut dst, 0, false, &g, AxisPart::All));
        assert!(copy_axis(&src, 5, false, AxisPart::All).is_none());
        assert!(!paste_axis(&mut dst, 5, false, &x, AxisPart::All));
    }

    #[test]
    fn colors_paste_carries_the_follow_plot_flags() {
        let (mut src, mut dst) = pair();
        dst.layers[0].y_axis.color_follows_plot = true;
        dst.layers[0].y_axis.title_color_follows_plot = true;
        src.layers[0].y_axis.color = Color::RED;
        let clip = copy_graph(&src, GraphPart::Colors);
        assert!(paste_graph(&mut dst, &clip, GraphPart::Colors.groups()));
        let y = &dst.layers[0].y_axis;
        assert_eq!(y.color, Color::RED);
        assert!(!y.color_follows_plot && !y.title_color_follows_plot);
        // An axis paste of styles carries them too.
        let mut dst2 = Template::Line.new_graph(WindowId(3), "D2");
        dst2.layers[0].y_axis.color_follows_plot = true;
        let axis = copy_axis(&src, 0, true, AxisPart::Styles).unwrap();
        assert!(paste_axis(&mut dst2, 0, true, &axis, AxisPart::Styles));
        assert!(!dst2.layers[0].y_axis.color_follows_plot);
    }

    #[test]
    fn dimensions_paste_skips_layers_of_another_link_kind() {
        let (src, mut dst) = pair();
        let mut second = dst.layers[0].clone();
        second.link = Some(crate::graph::LayerLink { parent: 0, x: Default::default(), y: Default::default() });
        dst.layers.push(second);
        let before_linked = dst.layers[1].frame;
        let clip = copy_graph(&src, GraphPart::Dimensions);
        paste_graph(&mut dst, &clip, GraphPart::Dimensions.groups());
        assert_eq!(dst.layers[0].frame.left, 0.2, "a root frame goes to a root layer");
        assert_eq!(dst.layers[1].frame, before_linked);
        // A linked source frame does not go onto a root target.
        let mut linked_src = src.clone();
        linked_src.layers.push(linked_src.layers[0].clone());
        linked_src.layers[1].link =
            Some(crate::graph::LayerLink { parent: 0, x: Default::default(), y: Default::default() });
        linked_src.layers[1].frame.left = 0.05;
        let (_, mut root_dst) = pair();
        root_dst.layers.push(root_dst.layers[0].clone());
        let root_before = root_dst.layers[1].frame;
        let clip = copy_graph(&linked_src, GraphPart::Dimensions);
        paste_graph(&mut root_dst, &clip, GraphPart::Dimensions.groups());
        assert_eq!(root_dst.layers[1].frame, root_before);
    }

    /// Two axes that differ in every group of the axis's Paste Format To.
    fn axis_group_source() -> (Graph, FormatClip, FormatClip) {
        let mut g = Template::Line.new_graph(WindowId(1), "G");
        let x = &mut g.layers[0].x_axis;
        x.color = Color::RED;
        x.line_width_pt = 3.0;
        x.major_tick = crate::graph::TickDirection::In;
        x.major_tick_len_pt = 9.0;
        x.tick_width_pt = Some(2.5);
        x.tick_font.size_pt = 21.0;
        x.tick_font.color = Color::BLUE;
        x.tick_prefix = "~".into();
        x.title_font.size_pt = 30.0;
        x.title = Some("Source title".into());
        x.min = Some(-5.0);
        let styles = copy_axis(&g, 0, false, AxisPart::Styles).unwrap();
        let all = copy_axis(&g, 0, false, AxisPart::All).unwrap();
        (g, styles, all)
    }

    #[test]
    fn each_paste_group_takes_only_its_part_of_the_axis() {
        let (g, styles, all) = axis_group_source();
        let fresh = || {
            let mut t = g.clone();
            t.layers[0].y_axis = Template::Line.new_graph(WindowId(2), "T").layers[0].y_axis.clone();
            t
        };
        let base = fresh().layers[0].y_axis.clone();
        let paste = |part: AxisPart, clip: &FormatClip| {
            let mut t = fresh();
            assert!(paste_axis(&mut t, 0, true, clip, part));
            t.layers[0].y_axis.clone()
        };
        let y = paste(AxisPart::TickDirection, &styles);
        assert_eq!(y.major_tick, crate::graph::TickDirection::In);
        assert_eq!((y.major_tick_len_pt, y.line_width_pt), (base.major_tick_len_pt, base.line_width_pt));
        let y = paste(AxisPart::Ticks, &styles);
        assert_eq!((y.major_tick_len_pt, y.tick_width_pt), (9.0, Some(2.5)));
        assert_eq!(y.color, base.color, "ticks leave the line colour");
        let y = paste(AxisPart::Line, &styles);
        assert_eq!((y.color, y.line_width_pt), (Color::RED, 3.0));
        assert_eq!(y.tick_font, base.tick_font);
        let y = paste(AxisPart::TickLabels, &styles);
        assert_eq!((y.tick_font.size_pt, y.tick_prefix.as_str()), (21.0, "~"));
        assert_eq!(y.title_font, base.title_font);
        let y = paste(AxisPart::TitleStyles, &styles);
        assert_eq!(y.title_font.size_pt, 30.0);
        assert_eq!(y.title, base.title, "the text is not a style");
        let y = paste(AxisPart::Colors, &styles);
        assert_eq!((y.color, y.tick_font.color), (Color::RED, Color::BLUE));
        assert_eq!((y.line_width_pt, y.tick_font.size_pt), (base.line_width_pt, base.tick_font.size_pt));
        let y = paste(AxisPart::Thickness, &styles);
        assert_eq!((y.line_width_pt, y.tick_width_pt), (3.0, Some(2.5)));
        assert_eq!(y.color, base.color);
        // Title Text needs an All copy; no group touches the scale.
        assert_eq!(paste(AxisPart::TitleText, &styles).title, base.title);
        let y = paste(AxisPart::TitleText, &all);
        assert_eq!(y.title.as_deref(), Some("Source title"));
        assert_eq!(y.min, base.min);
        // A Scales-only clipboard pastes no group.
        let scales = copy_axis(&g, 0, false, AxisPart::Scales).unwrap();
        assert_eq!(paste(AxisPart::Line, &scales), base);
    }
}
