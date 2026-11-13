//! Built-in figure templates: starting styles for new graphs.
//!
//! A template only sets style. Plots are added afterwards with
//! [`crate::project::Project::add_plot`].

use crate::color::Color;
use crate::graph::*;
use crate::ids::WindowId;
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Template {
    /// General line plot: axes on the bottom and left, ticks pointing out.
    #[default]
    Line,
    /// Scatter plot with the same axes as [`Template::Line`].
    Scatter,
    /// Line plot with a symbol on every point.
    LineSymbol,
    /// Cyclic voltammetry overlay: smooth lines without markers, no grid,
    /// ticks pointing inwards, a closed black box around the plot, and a
    /// current axis that is symmetric about zero.
    CyclicVoltammetry,
    /// Two Y axes over one X axis: the line template with a second, linked layer whose Y axis is
    /// on the right (Origin's "Double-Y"). Selected columns alternate between the left and the
    /// right axis, and each Y axis is drawn in the color of its first curve.
    DoubleY,
    /// Curves stacked above each other with a gap (Origin's "Stacked Lines by Y Offsets"): the
    /// line template with automatic offsets and a 10 % gap, no Y tick labels and no Y ticks, and
    /// a closed box, as in the owner's XRD figures.
    StackedLines,
    /// A simple two-dimensional waterfall: each later curve is drawn moved right and up.
    Waterfall,
    /// Column chart (Origin's "Column"): one bar per row from zero, several Y columns side by side,
    /// filled from the palette with a thin black outline, in a closed box with ticks pointing in.
    /// With a text X column (sample names) the names are written under the bars.
    Column,
    /// Area plot (Origin's "Area"): the line template with each curve filled down to Y = 0.
    Area,
}

impl Template {
    pub const ALL: [Template; 9] = [
        Template::Line,
        Template::Scatter,
        Template::LineSymbol,
        Template::Column,
        Template::Area,
        Template::CyclicVoltammetry,
        Template::DoubleY,
        Template::StackedLines,
        Template::Waterfall,
    ];

    /// The name used on the command line and in menus.
    pub fn name(&self) -> &'static str {
        match self {
            Template::Line => "line",
            Template::Scatter => "scatter",
            Template::LineSymbol => "line-symbol",
            Template::CyclicVoltammetry => "cv",
            Template::DoubleY => "double-y",
            Template::StackedLines => "stacked",
            Template::Waterfall => "waterfall",
            Template::Column => "column",
            Template::Area => "area",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Template::Line => "Line",
            Template::Scatter => "Scatter",
            Template::LineSymbol => "Line + Symbol",
            Template::CyclicVoltammetry => "Cyclic voltammetry overlay",
            Template::DoubleY => "Double Y",
            Template::StackedLines => "Stacked lines (Y offsets)",
            Template::Waterfall => "Waterfall",
            Template::Column => "Column",
            Template::Area => "Area",
        }
    }

    pub fn from_name(name: &str) -> Option<Template> {
        Template::ALL.into_iter().find(|t| t.name().eq_ignore_ascii_case(name))
    }

    /// Builds an empty graph (no plots) in this template's style: one layer, or two for
    /// [`Template::DoubleY`].
    pub fn new_graph(&self, id: WindowId, name: &str) -> Graph {
        match self {
            Template::Line => general(id, name, PlotKind::Line),
            Template::Scatter => general(id, name, PlotKind::Scatter),
            Template::LineSymbol => general(id, name, PlotKind::LineSymbol),
            Template::CyclicVoltammetry => cyclic_voltammetry(id, name),
            Template::DoubleY => double_y(id, name),
            Template::StackedLines => stacked_lines(id, name),
            Template::Waterfall => waterfall(id, name),
            Template::Column => column(id, name),
            Template::Area => general(id, name, PlotKind::Area),
        }
    }
}

/// Default font family. Arial ships with Windows; the renderer falls back
/// to a metric-compatible family where it is missing.
pub const DEFAULT_FONT: &str = "Arial";

fn base_axis(title_pt: f64, tick_pt: f64) -> Axis {
    Axis {
        scale: ScaleKind::Linear,
        min: None,
        max: None,
        auto_range: AutoRange::Nice,
        major_step: None,
        minor_ticks: 1,
        title: None,
        show_title: true,
        title_font: Font::new(DEFAULT_FONT, title_pt),
        show_tick_labels: true,
        tick_font: Font::new(DEFAULT_FONT, tick_pt),
        tick_format: TickFormat::Auto,
        tick_label_divisor: 1.0,
        major_tick: TickDirection::Out,
        minor_tick: TickDirection::Out,
        major_tick_len_pt: 5.0,
        minor_tick_len_pt: 2.5,
        line_width_pt: 1.0,
        color: Color::BLACK,
        show_opposite: false,
        opposite_ticks: false,
        major_grid: None,
        minor_grid: None,
        side: AxisSide::Normal,
        color_follows_plot: false,
        title_color_follows_plot: false,
        tick_label_source: TickLabelSource::Auto,
        tick_label_angle_deg: 0.0,
        hide_end_label: false,
    }
}

fn general(id: WindowId, name: &str, kind: PlotKind) -> Graph {
    let axis = base_axis(14.0, 12.0);
    let palette = Palette {
        colors: vec![
            Color::rgb(0, 0, 0),
            Color::rgb(220, 30, 30),
            Color::rgb(30, 80, 220),
            Color::rgb(0, 150, 70),
            Color::rgb(190, 50, 190),
            Color::rgb(235, 140, 0),
            Color::rgb(0, 160, 190),
            Color::rgb(130, 80, 40),
            Color::rgb(110, 110, 110),
        ],
        mode: PaletteMode::Cycle,
    };
    Graph {
        id,
        name: name.to_string(),
        long_name: String::new(),
        page: Page::inches(6.4, 4.8),
        layers: vec![Layer {
            frame: Frame { left: 0.16, top: 0.07, width: 0.78, height: 0.75 },
            x_axis: axis.clone(),
            y_axis: axis,
            plots: Vec::new(),
            legend: Legend {
                visible: true,
                position: LegendPosition::TopRight,
                font: Font::new(DEFAULT_FONT, 11.0),
                frame: false,
                background: None,
                scope: LegendScope::Layer,
                columns: 1,
                text: None,
            },
            labels: Vec::new(),
            palette,
            default_line: LineStyle::solid(Color::BLACK, 1.5),
            default_symbol: SymbolStyle {
                shape: SymbolShape::Circle,
                size_pt: 5.0,
                color: Color::BLACK,
                filled: true,
                edge_width_pt: 0.75,
                every: 1,
            },
            default_kind: kind,
            link: None,
            stack: Stack::None,
            plot_offsets: Vec::new(),
            bars: BarSpacing::default(),
        }],
    }
}

fn cyclic_voltammetry(id: WindowId, name: &str) -> Graph {
    let mut x = base_axis(12.0, 10.0);
    x.auto_range = AutoRange::Tight;
    x.minor_ticks = 0;
    x.major_tick = TickDirection::In;
    x.minor_tick = TickDirection::None;
    x.show_opposite = true;
    let mut y = x.clone();
    y.auto_range = AutoRange::SymmetricZero;

    let mut g = general(id, name, PlotKind::Line);
    g.page = Page::inches(5.0, 3.6);
    let layer = &mut g.layers[0];
    layer.frame = Frame { left: 0.17, top: 0.06, width: 0.78, height: 0.74 };
    layer.x_axis = x;
    layer.y_axis = y;
    layer.palette = Palette {
        colors: vec![Color::rgb(0x46, 0xB1, 0xE1), Color::rgb(0x4E, 0x95, 0xD9), Color::rgb(0x10, 0x48, 0x62)],
        mode: PaletteMode::Spread,
    };
    layer.default_line = LineStyle { color: Color::BLACK, width_pt: 1.75, dash: Dash::Solid, connect: Connect::Spline };
    layer.legend = Legend {
        visible: true,
        position: LegendPosition::BottomRight,
        font: Font::new(DEFAULT_FONT, 10.0),
        frame: false,
        background: None,
        scope: LegendScope::Layer,
        columns: 1,
        text: None,
    };
    g
}

fn double_y(id: WindowId, name: &str) -> Graph {
    let mut g = general(id, name, PlotKind::Line);
    let layer = &mut g.layers[0];
    // Narrower than the line template, so that the right tick labels and title fit as well as the
    // left ones.
    layer.frame = Frame { left: 0.16, top: 0.07, width: 0.68, height: 0.75 };
    // The top line closes the box; the right edge belongs to the right Y axis.
    layer.x_axis.show_opposite = true;
    layer.x_axis.opposite_ticks = true;
    layer.y_axis.color_follows_plot = true;
    layer.legend.scope = LegendScope::Graph;
    layer.legend.position = LegendPosition::TopRight;
    let right = right_y_layer(layer, 0);
    g.layers.push(right);
    g
}

/// Offsets that stack the curves with a gap of 10 % of the tallest curve, as Origin 2016's
/// "Stacked Lines by Y Offsets" does.
pub const STACK_AUTO_DEFAULT: Stack = Stack::Auto { gap_percent: 10.0 };

/// The shift of the waterfall template: 5 % of the front frame to the right and 10 % up per
/// curve. These are Sheaf's own values.
pub const WATERFALL_DEFAULT: Stack = Stack::Waterfall { dx_percent: 5.0, dy_percent: 10.0 };

fn stacked_lines(id: WindowId, name: &str) -> Graph {
    let mut g = general(id, name, PlotKind::Line);
    let layer = &mut g.layers[0];
    layer.stack = STACK_AUTO_DEFAULT;
    // The Y values on the axis are true only for the first curve, so the Y axis shows neither
    // tick labels nor ticks; a closed box frames the curves.
    let y = &mut layer.y_axis;
    y.show_tick_labels = false;
    y.major_tick = TickDirection::None;
    y.minor_tick = TickDirection::None;
    for axis in [&mut layer.x_axis, &mut layer.y_axis] {
        axis.show_opposite = true;
        axis.opposite_ticks = false;
    }
    g
}

fn waterfall(id: WindowId, name: &str) -> Graph {
    let mut g = general(id, name, PlotKind::Line);
    g.layers[0].stack = WATERFALL_DEFAULT;
    g
}

/// The column template: the line template's page, fonts, legend and colors, with bars outlined in
/// black 0.5 pt (Origin's default outline) and the owner's house style for the axes: Y ticks
/// pointing in and a closed box. The X axis has no ticks, as in the owner's column figures: a tick
/// pointing in would cross the middle bar of each group.
fn column(id: WindowId, name: &str) -> Graph {
    let mut g = general(id, name, PlotKind::Column);
    let layer = &mut g.layers[0];
    layer.default_line = LineStyle::solid(Color::BLACK, 0.5);
    layer.default_symbol.color = Color::BLACK;
    layer.default_symbol.filled = true;
    for axis in [&mut layer.x_axis, &mut layer.y_axis] {
        axis.show_opposite = true;
        axis.opposite_ticks = false;
    }
    let x = &mut layer.x_axis;
    x.major_tick = TickDirection::None;
    x.minor_tick = TickDirection::None;
    x.minor_ticks = 0;
    let y = &mut layer.y_axis;
    y.major_tick = TickDirection::In;
    y.minor_tick = TickDirection::In;
    y.minor_ticks = 1;
    g
}

/// Builds the layer that gives layer `parent_index` (whose content is `parent`) a right Y axis:
/// a layer linked to it with the same X scale, its own Y scale, and the parent's style.
///
/// The new layer is a copy of the parent without plots and free labels. Its frame covers the
/// parent's frame, its X axis is not drawn, and its Y axis is drawn on the right with automatic
/// limits and step and a title built from its own first plot. Its legend is hidden: the parent's
/// legend lists the plots of both layers (see [`crate::project::Project::add_right_y_layer`]).
/// Its palette starts at the parent's third color (for a gradient: runs the other way), so the
/// first right curve differs from the first left one.
pub fn right_y_layer(parent: &Layer, parent_index: usize) -> Layer {
    let mut l = parent.clone();
    l.plots.clear();
    l.labels.clear();
    l.plot_offsets.clear();
    // The right axis has curves of its own: they are not stacked, and its values are shown even
    // when the stacked parent hides its own (offset) Y values.
    if l.stack != Stack::None {
        l.stack = Stack::None;
        l.y_axis.show_tick_labels = true;
    }
    l.link = Some(LayerLink { parent: parent_index, x: ScaleLink::Straight, y: ScaleLink::None });
    l.frame = Frame { left: 0.0, top: 0.0, width: 1.0, height: 1.0 };
    l.x_axis.side = AxisSide::Hidden;
    l.x_axis.major_grid = None;
    l.x_axis.minor_grid = None;
    let y = &mut l.y_axis;
    y.side = AxisSide::Opposite;
    y.min = None;
    y.max = None;
    y.major_step = None;
    y.title = None;
    y.tick_label_divisor = 1.0;
    y.show_opposite = false;
    y.opposite_ticks = false;
    y.major_grid = None;
    y.minor_grid = None;
    l.legend.visible = false;
    l.legend.scope = LegendScope::Layer;
    let colors = &mut l.palette.colors;
    match l.palette.mode {
        PaletteMode::Cycle => {
            if !colors.is_empty() {
                let start = if colors.len() >= 3 { 2 } else { colors.len() - 1 };
                colors.rotate_left(start);
            }
        }
        PaletteMode::Spread => colors.reverse(),
    }
    l
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cv_template_matches_the_requested_style() {
        let g = Template::CyclicVoltammetry.new_graph(WindowId(1), "Graph1");
        let l = &g.layers[0];
        assert_eq!(l.default_kind, PlotKind::Line, "lines without markers");
        assert_eq!(l.default_line.connect, Connect::Spline, "smooth lines");
        assert!(l.x_axis.major_grid.is_none() && l.y_axis.major_grid.is_none(), "no gridlines");
        assert_eq!(l.x_axis.major_tick, TickDirection::In);
        assert_eq!(l.y_axis.major_tick, TickDirection::In);
        assert!(l.x_axis.show_opposite && l.y_axis.show_opposite, "closed box");
        assert_eq!(l.x_axis.color, Color::BLACK);
        assert_eq!(l.y_axis.auto_range, AutoRange::SymmetricZero);
    }

    #[test]
    fn template_names_round_trip() {
        for t in Template::ALL {
            assert_eq!(Template::from_name(t.name()), Some(t));
        }
        assert_eq!(Template::from_name("CV"), Some(Template::CyclicVoltammetry));
        assert_eq!(Template::from_name("double-y"), Some(Template::DoubleY));
        assert_eq!(Template::from_name("stacked"), Some(Template::StackedLines));
        assert_eq!(Template::from_name("Waterfall"), Some(Template::Waterfall));
        assert_eq!(Template::from_name("column"), Some(Template::Column));
        assert_eq!(Template::from_name("nope"), None);
        assert_eq!(Template::from_name("area"), Some(Template::Area));
        assert_eq!(Template::ALL.len(), 9);
    }

    #[test]
    fn column_template_draws_outlined_bars_in_a_closed_box_with_y_ticks_in() {
        let g = Template::Column.new_graph(WindowId(1), "Graph1");
        assert_eq!(g.layers.len(), 1);
        let l = &g.layers[0];
        assert_eq!(l.default_kind, PlotKind::Column);
        assert_eq!(l.default_line, LineStyle::solid(Color::BLACK, 0.5), "a thin black outline");
        assert!(l.default_symbol.filled);
        assert_eq!(l.default_symbol.color, Color::BLACK);
        for axis in [&l.x_axis, &l.y_axis] {
            assert!(axis.show_opposite && !axis.opposite_ticks, "closed box");
            assert!(axis.major_grid.is_none() && axis.minor_grid.is_none(), "no grid");
            assert_eq!(axis.tick_label_source, TickLabelSource::Auto);
            assert!(axis.show_tick_labels);
        }
        assert_eq!((l.y_axis.major_tick, l.y_axis.minor_tick), (TickDirection::In, TickDirection::In));
        assert_eq!((l.x_axis.major_tick, l.x_axis.minor_tick), (TickDirection::None, TickDirection::None));
        assert_eq!((l.x_axis.minor_ticks, l.y_axis.minor_ticks), (0, 1));
        assert_eq!(l.bars, BarSpacing { gap_pct: 20.0, overlap_pct: 0.0 });
        // Page, fonts, legend and colors are those of the line template.
        let line = Template::Line.new_graph(WindowId(1), "Graph1");
        assert_eq!(g.page, line.page);
        assert_eq!(l.legend, line.layers[0].legend);
        assert_eq!(l.palette, line.layers[0].palette);
        assert_eq!(l.x_axis.tick_font, line.layers[0].x_axis.tick_font);
        // No template writes the new keys, so their files look as before.
        for t in Template::ALL {
            let json = serde_json::to_string(&t.new_graph(WindowId(1), "G")).unwrap();
            for key in ["tick_label_source", "tick_label_angle_deg", "\"bars\""] {
                assert!(!json.contains(key), "{t:?}: {key}");
            }
        }
    }

    #[test]
    fn stacked_template_stacks_with_a_10_percent_gap_in_a_closed_box() {
        let g = Template::StackedLines.new_graph(WindowId(1), "Graph1");
        assert_eq!(g.layers.len(), 1);
        let l = &g.layers[0];
        assert_eq!(l.stack, Stack::Auto { gap_percent: 10.0 });
        assert!(!l.y_axis.show_tick_labels);
        assert_eq!((l.y_axis.major_tick, l.y_axis.minor_tick), (TickDirection::None, TickDirection::None));
        assert!(l.x_axis.show_opposite && l.y_axis.show_opposite, "closed box");
        assert!(!l.x_axis.opposite_ticks && !l.y_axis.opposite_ticks);
        // The X axis and the legend are those of the line template.
        let line = Template::Line.new_graph(WindowId(1), "Graph1");
        assert_eq!(l.x_axis.major_tick, line.layers[0].x_axis.major_tick);
        assert!(l.x_axis.show_tick_labels);
        assert_eq!(l.legend, line.layers[0].legend);
        assert_eq!(l.default_kind, PlotKind::Line);
    }

    #[test]
    fn waterfall_template_is_the_line_template_with_a_waterfall() {
        let g = Template::Waterfall.new_graph(WindowId(1), "Graph1");
        let mut line = Template::Line.new_graph(WindowId(1), "Graph1");
        assert_eq!(g.layers[0].stack, Stack::Waterfall { dx_percent: 5.0, dy_percent: 10.0 });
        line.layers[0].stack = g.layers[0].stack;
        assert_eq!(g, line);
        // Every other template has no offsets.
        for t in
            [Template::Line, Template::Scatter, Template::LineSymbol, Template::CyclicVoltammetry, Template::DoubleY]
        {
            let g = t.new_graph(WindowId(1), "G");
            assert!(g.layers.iter().all(|l| l.stack.is_none() && l.plot_offsets.is_empty()));
            // ... and their files are written as before.
            let json = serde_json::to_string(&g).unwrap();
            assert!(!json.contains("\"stack\"") && !json.contains("plot_offsets"), "{json}");
        }
    }

    #[test]
    fn double_y_template_has_a_right_layer_linked_to_the_first() {
        let g = Template::DoubleY.new_graph(WindowId(1), "Graph1");
        assert_eq!(g.layers.len(), 2);
        let (left, right) = (&g.layers[0], &g.layers[1]);
        assert_eq!(right.link, Some(LayerLink { parent: 0, x: ScaleLink::Straight, y: ScaleLink::None }));
        assert_eq!(g.link_parent(1), Some(0));
        assert_eq!(right.frame, Frame { left: 0.0, top: 0.0, width: 1.0, height: 1.0 });
        assert_eq!(right.x_axis.side, AxisSide::Hidden);
        assert_eq!(right.y_axis.side, AxisSide::Opposite);
        assert!(!right.legend.visible);
        assert_eq!(left.legend.scope, LegendScope::Graph);
        assert!(!left.y_axis.show_opposite && left.x_axis.show_opposite && left.x_axis.opposite_ticks);
        for y in [&left.y_axis, &right.y_axis] {
            assert!(y.color_follows_plot && !y.title_color_follows_plot);
        }
        assert!(!left.x_axis.color_follows_plot);
        // The first right curve is blue, the first left curve black, as in Origin.
        assert_eq!(left.palette.colors[0], Color::rgb(0, 0, 0));
        assert_eq!(right.palette.colors[0], Color::rgb(30, 80, 220));
        assert_eq!(left.x_axis.major_tick, TickDirection::Out);
    }

    #[test]
    fn right_y_layer_copies_the_parent_style_but_not_its_limits() {
        let mut g = Template::CyclicVoltammetry.new_graph(WindowId(1), "Graph1");
        let parent = &mut g.layers[0];
        parent.y_axis.min = Some(-3.0);
        parent.y_axis.title = Some("j".into());
        parent.labels.push(TextLabel {
            text: "note".into(),
            x: 0.1,
            y: 0.1,
            font: Font::new(DEFAULT_FONT, 9.0),
            rotation_deg: 0.0,
        });
        g.layers[0].plot_offset_mut(crate::ids::ColumnId(4)).y = 3.0;
        let r = right_y_layer(&g.layers[0], 0);
        assert!(r.plot_offsets.is_empty(), "offsets belong to the parent's curves");
        assert_eq!(r.y_axis.major_tick, TickDirection::In);
        assert_eq!(r.y_axis.auto_range, AutoRange::SymmetricZero);
        assert_eq!(r.default_line, g.layers[0].default_line);
        assert_eq!((r.y_axis.min, r.y_axis.title.clone()), (None, None));
        assert!(r.labels.is_empty() && r.plots.is_empty());
        assert!(!r.y_axis.show_opposite && !r.y_axis.opposite_ticks);
        // A gradient runs the other way on the right.
        let mut reversed = g.layers[0].palette.colors.clone();
        reversed.reverse();
        assert_eq!(r.palette.colors, reversed);
        // Short color lists start at their last color.
        let mut two = g.layers[0].clone();
        two.palette = Palette { colors: vec![Color::BLACK, Color::RED], mode: PaletteMode::Cycle };
        assert_eq!(right_y_layer(&two, 0).palette.colors, vec![Color::RED, Color::BLACK]);
        two.palette.colors.clear();
        assert!(right_y_layer(&two, 0).palette.colors.is_empty());
        // The right layer of a stacked graph is not stacked and shows its values.
        let mut stacked = g.layers[0].clone();
        stacked.stack = Stack::Constant { step: 2.0 };
        stacked.y_axis.show_tick_labels = false;
        let r = right_y_layer(&stacked, 0);
        assert_eq!(r.stack, Stack::None);
        assert!(r.y_axis.show_tick_labels);
    }
}
