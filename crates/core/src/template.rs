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
    /// Stacked column (Origin's "Stacked Column"): the column template with the plots piled up.
    StackedColumn,
    /// 100 % stacked column: every pile scaled to 100, Y axis from 0 to 100.
    Stacked100Column,
    /// Horizontal bars (Origin's "Bar").
    Bar,
    /// Floating column (Origin's "Floating Column"): each bar runs from one Y column to the next.
    FloatingColumn,
    /// Stacked bar: the horizontal form of the stacked column.
    StackedBar,
    /// 100 % stacked bar: every pile scaled to 100, value axis from 0 to 100.
    Stacked100Bar,
    /// Floating bar: the horizontal form of the floating column.
    FloatingBar,
    /// Stacked area (Origin's "Stacked Area"): the area template with the areas piled up.
    StackedArea,
    /// 100 % stacked area: every pile scaled to 100, Y axis from 0 to 100.
    Stacked100Area,
    /// Fill area (Origin's "Fill Area"): the area between two curves, a pair of Y columns each.
    FillArea,
    /// Histogram (Origin's "Histogram"): the bin counts of each Y column as touching bars.
    Histogram,
    /// Box chart (Origin's "Box Chart"): one box per Y column.
    BoxChart,
    /// Bubble plot (Origin's "Bubble"): open circles whose size follows the Z column.
    Bubble,
    /// Colour mapped scatter plot: the colour of each symbol follows the Z column.
    ColorMapped,
    /// Both: bubbles whose size and colour follow the Z column.
    BubbleColorMapped,
    /// Pie chart (Origin's pie): the Y column's rows as slices, in a layer without axes.
    Pie,
    /// 3D pie seen from above at a slant (Origin's "3D Color Pie").
    Pie3D,
    /// Flat black and white pie with hatch patterns (Origin's "2D B&W Pie").
    PieBw,
    /// Polar plot, angle in degrees from the X column and radius from Y (Origin's "Polar theta(X) r(Y)").
    Polar,
    /// Polar plot with the radius from the X column and the angle from Y ("Polar r(X) theta(Y)").
    PolarRTheta,
    /// Radar chart: every Y column a closed polygon over spokes named by the X column (Origin's "Radar").
    Radar,
    /// Ternary plot of the X, Y and Z columns (Origin's "Ternary").
    Ternary,
}

impl Template {
    pub const ALL: [Template; 31] = [
        Template::Line,
        Template::Scatter,
        Template::LineSymbol,
        Template::Column,
        Template::StackedColumn,
        Template::Stacked100Column,
        Template::Bar,
        Template::FloatingColumn,
        Template::StackedBar,
        Template::Stacked100Bar,
        Template::FloatingBar,
        Template::Area,
        Template::StackedArea,
        Template::Stacked100Area,
        Template::FillArea,
        Template::Histogram,
        Template::BoxChart,
        Template::Bubble,
        Template::ColorMapped,
        Template::BubbleColorMapped,
        Template::Pie,
        Template::Pie3D,
        Template::PieBw,
        Template::Polar,
        Template::PolarRTheta,
        Template::Radar,
        Template::Ternary,
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
            Template::StackedColumn => "stacked-column",
            Template::Stacked100Column => "stacked-column-100",
            Template::Bar => "bar",
            Template::FloatingColumn => "floating-column",
            Template::StackedBar => "stacked-bar",
            Template::Stacked100Bar => "stacked-bar-100",
            Template::FloatingBar => "floating-bar",
            Template::StackedArea => "stacked-area",
            Template::Stacked100Area => "stacked-area-100",
            Template::FillArea => "fill-area",
            Template::Histogram => "histogram",
            Template::BoxChart => "box",
            Template::Bubble => "bubble",
            Template::ColorMapped => "color-mapped",
            Template::BubbleColorMapped => "bubble-color-mapped",
            Template::Pie => "pie",
            Template::Pie3D => "pie-3d",
            Template::PieBw => "pie-bw",
            Template::Polar => "polar",
            Template::PolarRTheta => "polar-r-theta",
            Template::Radar => "radar",
            Template::Ternary => "ternary",
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
            Template::StackedColumn => "Stacked Column",
            Template::Stacked100Column => "100% Stacked Column",
            Template::Bar => "Bar",
            Template::FloatingColumn => "Floating Column",
            Template::StackedBar => "Stacked Bar",
            Template::Stacked100Bar => "100% Stacked Bar",
            Template::FloatingBar => "Floating Bar",
            Template::StackedArea => "Stacked Area",
            Template::Stacked100Area => "100% Stacked Area",
            Template::FillArea => "Fill Area",
            Template::Histogram => "Histogram",
            Template::BoxChart => "Box Chart",
            Template::Bubble => "Bubble",
            Template::ColorMapped => "Color Mapped",
            Template::BubbleColorMapped => "Bubble + Color Mapped",
            Template::Pie => "Pie",
            Template::Pie3D => "3D Color Pie",
            Template::PieBw => "2D B&W Pie",
            Template::Polar => "Polar theta(X) r(Y)",
            Template::PolarRTheta => "Polar r(X) theta(Y)",
            Template::Radar => "Radar",
            Template::Ternary => "Ternary",
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
            Template::Column => from_red(column(id, name, PlotKind::Column)),
            Template::StackedColumn => themed(column(id, name, PlotKind::StackedColumn)),
            Template::Stacked100Column => themed(column(id, name, PlotKind::Stacked100Column)),
            Template::Bar => column(id, name, PlotKind::Bar),
            Template::FloatingColumn => from_blue(column(id, name, PlotKind::FloatingColumn)),
            Template::StackedBar => themed(column(id, name, PlotKind::StackedBar)),
            Template::Stacked100Bar => themed(column(id, name, PlotKind::Stacked100Bar)),
            Template::FloatingBar => from_blue(column(id, name, PlotKind::FloatingBar)),
            Template::Area => themed(general(id, name, PlotKind::Area)),
            Template::StackedArea => themed(general(id, name, PlotKind::StackedArea)),
            Template::Stacked100Area => themed(general(id, name, PlotKind::Stacked100Area)),
            Template::FillArea => themed(general(id, name, PlotKind::Area)),
            Template::Histogram => from_red(general(id, name, PlotKind::Histogram)),
            Template::BoxChart => from_red(general(id, name, PlotKind::Box)),
            Template::Bubble => bubble(id, name, false, true),
            Template::ColorMapped => bubble(id, name, true, false),
            Template::BubbleColorMapped => bubble(id, name, true, true),
            Template::Pie => pie(id, name, PlotKind::Pie),
            Template::Pie3D => pie(id, name, PlotKind::Pie3D),
            Template::PieBw => pie(id, name, PlotKind::PieBw),
            Template::Polar => polar(id, name, RadiusColumn::Y),
            Template::PolarRTheta => polar(id, name, RadiusColumn::X),
            Template::Radar => radar(id, name),
            Template::Ternary => ternary(id, name),
        }
    }
}

/// Default font family. Arial ships with Windows; the renderer falls back
/// to a metric-compatible family where it is missing.
pub const DEFAULT_FONT: &str = "Arial";

/// Origin's colour list for a new group of plots: its base palette in order (black, red, green,
/// blue, cyan, magenta, yellow, dark yellow, navy, purple, wine, olive, dark cyan, royal, orange,
/// violet, pink, white, light gray, gray, light yellow, light cyan, light magenta, dark gray), then
/// round again (graphing spec section 1.5).
pub const ORIGIN_COLOR_LIST: [(u8, u8, u8); 24] = [
    (0, 0, 0),
    (255, 0, 0),
    (0, 255, 0),
    (0, 0, 255),
    (0, 255, 255),
    (255, 0, 255),
    (255, 255, 0),
    (128, 128, 0),
    (0, 0, 128),
    (128, 0, 128),
    (128, 0, 0),
    (0, 128, 0),
    (0, 128, 128),
    (0, 0, 160),
    (255, 128, 0),
    (128, 0, 255),
    (255, 0, 128),
    (255, 255, 255),
    (192, 192, 192),
    (128, 128, 128),
    (255, 255, 128),
    (128, 255, 255),
    (255, 128, 255),
    (64, 64, 64),
];

/// The colour list of the owner's graph theme, which Origin's area, stacked area and stacked column
/// templates use (black, red, blue, magenta, green, navy, violet, purple, wine, dark yellow, then
/// custom colours; graphing spec section 1.5), measured on the legends of plots made from those
/// templates.
pub const THEME_COLOR_LIST: [(u8, u8, u8); 23] = [
    (0, 0, 0),
    (255, 0, 0),
    (0, 0, 255),
    (255, 0, 255),
    (0, 128, 0),
    (0, 0, 128),
    (128, 0, 255),
    (128, 0, 128),
    (128, 0, 0),
    (128, 128, 0),
    (43, 99, 162),
    (30, 150, 150),
    (155, 100, 26),
    (16, 199, 62),
    (185, 36, 122),
    (45, 197, 204),
    (63, 65, 152),
    (147, 172, 43),
    (128, 128, 128),
    (150, 100, 100),
    (100, 150, 100),
    (43, 163, 202),
    (50, 100, 150),
];

/// The graph with its colour list replaced by the theme list ([`THEME_COLOR_LIST`]).
fn themed(mut g: Graph) -> Graph {
    for l in &mut g.layers {
        l.palette.colors = THEME_COLOR_LIST.iter().map(|&(r, g, b)| Color::rgb(r, g, b)).collect();
    }
    g
}

/// The graph with its colour list started at the second entry of the base palette (red, green, blue,
/// cyan, ...), as Origin's Column and Box templates do (measured: the first column of a plain
/// Column graph is red, not black).
fn from_red(g: Graph) -> Graph {
    from_entry(g, 1)
}

/// The graph with its colour list started at blue, the fourth entry of the base palette, as Origin's
/// Floating Column template does (measured: its first plot is filled blue, graphing.md 14.12; the
/// colours of further plots were not measured and follow the base palette from there).
fn from_blue(g: Graph) -> Graph {
    from_entry(g, 3)
}

fn from_entry(mut g: Graph, first: usize) -> Graph {
    for l in &mut g.layers {
        l.palette.colors = (first..first + ORIGIN_COLOR_LIST.len())
            .map(|i| ORIGIN_COLOR_LIST[i % ORIGIN_COLOR_LIST.len()])
            .map(|(r, g, b)| Color::rgb(r, g, b))
            .collect();
    }
    g
}

/// Origin's default page, in inches (6432 by 4923 units of 1/600 inch). Its font sizes (18 pt tick
/// labels, 22 pt titles) are meant for this size.
pub const ORIGIN_PAGE_IN: (f64, f64) = (10.72, 8.205);

fn base_axis(title_pt: f64, tick_pt: f64) -> Axis {
    Axis {
        scale: ScaleKind::Linear,
        min: None,
        max: None,
        auto_range: AutoRange::Nice,
        rescale_margin_pct: crate::autorange::DEFAULT_RESCALE_MARGIN_PCT,
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
        major_tick_len_pt: 9.0,
        minor_tick_len_pt: 4.5,
        line_width_pt: 1.5,
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
        reverse: false,
        first_tick: None,
        axis_break: None,
        special_ticks: Vec::new(),
        tick_label_formula: String::new(),
        minor_tick_labels: false,
        tick_color: None,
        tick_width_pt: None,
        tick_prefix: String::new(),
        tick_suffix: String::new(),
    }
}

fn general(id: WindowId, name: &str, kind: PlotKind) -> Graph {
    let axis = base_axis(22.0, 18.0);
    let palette = Palette {
        colors: ORIGIN_COLOR_LIST.iter().map(|&(r, g, b)| Color::rgb(r, g, b)).collect(),
        mode: PaletteMode::Cycle,
    };
    Graph {
        id,
        name: name.to_string(),
        long_name: String::new(),
        page: Page::inches(ORIGIN_PAGE_IN.0, ORIGIN_PAGE_IN.1),
        layers: vec![Layer {
            frame: Frame { left: 0.1791, top: 0.1158, width: 0.6816, height: 0.7183 },
            x_axis: axis.clone(),
            y_axis: axis,
            plots: Vec::new(),
            legend: Legend {
                visible: true,
                position: LegendPosition::TopRight,
                font: Font::new(DEFAULT_FONT, 22.0),
                frame: true,
                background: None,
                scope: LegendScope::Layer,
                columns: 1,
                text: None,
            },
            labels: Vec::new(),
            shapes: Vec::new(),
            palette,
            default_line: LineStyle::solid(Color::BLACK, 0.5),
            default_symbol: SymbolStyle {
                shape: SymbolShape::Square,
                size_pt: 9.0,
                color: Color::BLACK,
                filled: true,
                edge_width_pt: 0.5,
                every: 1,
            },
            default_kind: kind,
            link: None,
            stack: Stack::None,
            plot_offsets: Vec::new(),
            bars: BarSpacing::default(),
            coords: Coords::Cartesian,
            background: None,
            group_style: GroupStyle::default(),
            display: LayerDisplay::default(),
        }],
    }
}

/// The scatter template for bubble and colour mapped plots. Bubbles are open circles in the first
/// colour of the list (measured: black rings), colour mapped symbols the usual filled squares, and
/// bubbles that are also colour mapped are filled circles.
fn bubble(id: WindowId, name: &str, mapped: bool, sized: bool) -> Graph {
    let mut g = general(id, name, PlotKind::Scatter);
    let s = &mut g.layers[0].default_symbol;
    if sized {
        s.shape = SymbolShape::Circle;
        s.filled = mapped;
    }
    g
}

/// The pie template: no axes (the plot draws its own frame, slices, labels and legend), the colour
/// list of the line template (measured: black, red, green, blue, cyan, ...).
fn pie(id: WindowId, name: &str, kind: PlotKind) -> Graph {
    let mut g = general(id, name, kind);
    let l = &mut g.layers[0];
    // Origin's pie templates have a larger layer than the line template (measured on the 3D and
    // the B&W pie: 7 % from the left, 86.5 % wide, 8 % from the top, 83 % high).
    l.frame = Frame { left: 0.0695, top: 0.079, width: 0.865, height: 0.83 };
    l.x_axis.side = AxisSide::Hidden;
    l.y_axis.side = AxisSide::Hidden;
    l.legend.visible = false;
    g
}

/// The polar template: the line template in a layer of polar coordinates. The frame is a square
/// on the page (the circle is as wide as the frame is high), the grid rings and spokes are grey
/// and solid (Sheaf's usual grid style, as in Origin's polar template), and the axis titles are
/// not drawn.
fn polar(id: WindowId, name: &str, radius: RadiusColumn) -> Graph {
    polar_of(id, name, radius, PlotKind::Line)
}

fn polar_of(id: WindowId, name: &str, radius: RadiusColumn, kind: PlotKind) -> Graph {
    let mut g = general(id, name, kind);
    // Origin's polar page (measured, spec 14.18): 8.04 by 6.15 in with a square layer of 4.55 in, its
    // left edge at 23.3 % of the page, and the legend above the layer's top right corner.
    g.page = Page::inches(8.04, 6.15);
    let l = &mut g.layers[0];
    l.frame = Frame { left: 0.2326, top: 0.1174, width: 0.5665, height: 0.7402 };
    l.legend.position = LegendPosition::Manual { x: 0.84, y: -0.108 };
    l.coords = Coords::Polar(PolarCoords { radius, ..PolarCoords::default() });
    let grid = LineStyle::solid(Color::rgb(204, 204, 204), 0.5);
    for a in [&mut l.x_axis, &mut l.y_axis] {
        a.major_grid = Some(grid.clone());
        a.show_title = false;
        a.major_tick = TickDirection::Out;
        a.major_tick_len_pt = 6.0;
        a.minor_ticks = 0;
    }
    g
}

/// The radar template: the line + symbol template in a layer of polar coordinates whose spokes are
/// the rows (graphing spec section 14.19). The grid is made of polygons at the radius ticks and
/// straight spokes; the spoke names stand outside, the outer polygon has no ticks.
fn radar(id: WindowId, name: &str) -> Graph {
    let mut g = polar_of(id, name, RadiusColumn::Y, PlotKind::LineSymbol);
    let l = &mut g.layers[0];
    l.coords = Coords::Polar(PolarCoords { zero_deg: 90.0, clockwise: true, radar: true, ..PolarCoords::default() });
    // Origin's radar page (measured, spec 14.19): 8 by 6.125 in, the layer 61 % wide and 75 % high from
    // 11.7 % and 10.9 %, the legend at the top right corner of the layer.
    g.page = Page::inches(8.0, 6.125);
    let l = &mut g.layers[0];
    l.frame = Frame { left: 0.1173, top: 0.1092, width: 0.612, height: 0.7535 };
    l.legend.position = LegendPosition::Manual { x: 1.0116, y: -0.0208 };
    l.x_axis.major_tick = TickDirection::None;
    l.y_axis.major_tick_len_pt = 5.0;
    g
}

/// The ternary template: a scatter plot in a layer of triangle coordinates. The frame is a square
/// on the page (the triangle is as wide as it fits), the grid lines parallel to the sides are on
/// at the major ticks, minor ticks are off. The three axis titles are the column names.
fn ternary(id: WindowId, name: &str) -> Graph {
    // Origin's ternary graph keeps the usual page and layer frame (its picture is 10.72 by 8.205 in,
    // measured); the triangle is the largest that fits in the frame.
    let mut g = general(id, name, PlotKind::Scatter);
    let l = &mut g.layers[0];
    l.coords = Coords::Ternary(TernaryCoords::default());
    // Origin's default ternary graph (measured, graphing spec 14.20): a legend (the plot is named
    // after its Z column), major grid lines in Gray, minor ones dashed in LT Gray, one minor tick.
    l.legend.visible = true;
    let grid = LineStyle::solid(Color::rgb(128, 128, 128), 0.5);
    let minor = LineStyle { dash: Dash::Dash, ..LineStyle::solid(Color::rgb(192, 192, 192), 0.5) };
    for a in [&mut l.x_axis, &mut l.y_axis] {
        a.major_grid = Some(grid.clone());
        a.minor_grid = Some(minor.clone());
        a.major_tick = TickDirection::Out;
        a.minor_tick = TickDirection::Out;
        a.major_tick_len_pt = 6.0;
        a.minor_tick_len_pt = 3.0;
        a.minor_ticks = 1;
    }
    g
}

fn cyclic_voltammetry(id: WindowId, name: &str) -> Graph {
    let mut x = base_axis(12.0, 10.0);
    // The owner's figure style keeps its own line and tick sizes.
    x.line_width_pt = 1.0;
    x.major_tick_len_pt = 5.0;
    x.minor_tick_len_pt = 2.5;
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
    layer.default_symbol = SymbolStyle {
        shape: SymbolShape::Circle,
        size_pt: 5.0,
        color: Color::BLACK,
        filled: true,
        edge_width_pt: 0.75,
        every: 1,
    };
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
    // Origin's frame is 68 % wide, which leaves room for the right tick labels and title as well
    // as the left ones.
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

/// Offsets that stack the curves with a gap of 10 % of the tallest curve, as Origin's
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
fn column(id: WindowId, name: &str, kind: PlotKind) -> Graph {
    let mut g = general(id, name, kind);
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
    if kind.is_horizontal() {
        // Lying bars: the value axis is the horizontal one.
        std::mem::swap(&mut layer.x_axis.major_tick, &mut layer.y_axis.major_tick);
        std::mem::swap(&mut layer.x_axis.minor_tick, &mut layer.y_axis.minor_tick);
        std::mem::swap(&mut layer.x_axis.minor_ticks, &mut layer.y_axis.minor_ticks);
    }
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
    l.shapes.clear();
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
        PaletteMode::Same => {}
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
        assert_eq!(Template::from_name("stacked-column-100"), Some(Template::Stacked100Column));
        assert_eq!(Template::from_name("bar"), Some(Template::Bar));
        assert_eq!(Template::from_name("floating-column"), Some(Template::FloatingColumn));
        assert_eq!(Template::from_name("pie-3d"), Some(Template::Pie3D));
        assert_eq!(Template::from_name("pie-bw"), Some(Template::PieBw));
        assert_eq!(Template::ALL.len(), 31);
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
        // Page, fonts and legend are those of the line template; the colours start at red (measured).
        let line = Template::Line.new_graph(WindowId(1), "Graph1");
        assert_eq!(g.page, line.page);
        assert_eq!(l.legend, line.layers[0].legend);
        assert_eq!(l.palette.colors[0], Color::rgb(255, 0, 0));
        assert_eq!(l.palette.colors.len(), line.layers[0].palette.colors.len());
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
        // The first left curve is black and the first right curve takes the third colour of the list
        // (green), so that the two differ.
        assert_eq!(left.palette.colors[0], Color::rgb(0, 0, 0));
        assert_eq!(right.palette.colors[0], Color::rgb(0, 255, 0));
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

    #[test]
    fn floating_column_starts_its_colours_at_blue_as_origin() {
        let g = Template::FloatingColumn.new_graph(WindowId(0), "");
        assert_eq!(g.layers[0].palette.colors[0], Color::rgb(0, 0, 255));
        let c = Template::Column.new_graph(WindowId(0), "");
        assert_eq!(c.layers[0].palette.colors[0], Color::rgb(255, 0, 0), "plain columns still start at red");
    }

    /// Measured on the legends of 12 and 13 plots in Origin (spec 14.12).
    #[test]
    fn horizontal_stacked_and_floating_bars_use_the_colour_lists_of_their_vertical_forms() {
        let colors = |t: Template| t.new_graph(WindowId(0), "").layers[0].palette.colors.clone();
        let floating: Vec<Color> = [
            (0, 0, 255),
            (0, 255, 255),
            (255, 0, 255),
            (255, 255, 0),
            (128, 128, 0),
            (0, 0, 128),
            (128, 0, 128),
            (128, 0, 0),
            (0, 128, 0),
            (0, 128, 128),
            (0, 0, 160),
            (255, 128, 0),
        ]
        .map(|(r, g, b)| Color::rgb(r, g, b))
        .to_vec();
        for t in [Template::FloatingColumn, Template::FloatingBar] {
            assert_eq!(colors(t)[..12], floating[..], "{t:?}");
        }
        for t in [Template::StackedBar, Template::Stacked100Bar] {
            assert_eq!(colors(t), colors(Template::StackedColumn), "{t:?}");
            assert_eq!(colors(t)[1], Color::rgb(255, 0, 0));
            assert_eq!(colors(t)[2], Color::rgb(0, 0, 255), "black, red, blue, magenta, green, ...");
        }
    }
}
