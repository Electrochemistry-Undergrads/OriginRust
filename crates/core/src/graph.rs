//! The graph (figure) model. It describes *what* a figure shows and how it is
//! styled. It holds no pixels and no data: plots refer to worksheet columns
//! by [`ColumnId`]. The `sheaf-plot` crate turns a graph plus the project's
//! data into drawing commands.
//!
//! All sizes are in points (1/72 inch) so that a figure has the same
//! physical proportions on screen and in every export format.

use crate::color::Color;
use crate::ids::{ColumnId, WindowId};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Graph {
    pub id: WindowId,
    /// Short name, unique among all windows of the project (for example `Graph1`).
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub long_name: String,
    pub page: Page,
    pub layers: Vec<Layer>,
}

impl Graph {
    pub fn title(&self) -> String {
        if self.long_name.is_empty() { self.name.clone() } else { format!("{} - {}", self.name, self.long_name) }
    }

    /// Every column this graph reads.
    pub fn referenced_columns(&self) -> Vec<ColumnId> {
        let mut out = Vec::new();
        for layer in &self.layers {
            for p in &layer.plots {
                out.extend(p.x);
                out.push(p.y);
                out.extend(p.y_error);
            }
            for axis in [&layer.x_axis, &layer.y_axis] {
                if let TickLabelSource::Column { column } = axis.tick_label_source {
                    out.push(column);
                }
            }
        }
        out
    }

    /// The parent of layer `index` when the layer has a valid link: the parent comes earlier in
    /// the list and is not linked itself. A layer with an invalid link (to itself, to a later or
    /// missing layer, or to a linked layer) is drawn as an independent layer.
    pub fn link_parent(&self, index: usize) -> Option<usize> {
        let link = self.layers.get(index)?.link?;
        (link.parent < index && self.layers[link.parent].link.is_none()).then_some(link.parent)
    }

    /// The link of layer `index` when it is valid (see [`Graph::link_parent`]).
    pub fn valid_link(&self, index: usize) -> Option<LayerLink> {
        self.link_parent(index).and(self.layers[index].link)
    }

    /// The layer that layer `index` follows: its parent when validly linked, else itself.
    pub fn root_of(&self, index: usize) -> usize {
        self.link_parent(index).unwrap_or(index)
    }

    /// The layers drawn as one group with layer `index`: its root followed by every layer
    /// validly linked to that root, in layer order. Empty when there is no such layer.
    pub fn group(&self, index: usize) -> Vec<usize> {
        if index >= self.layers.len() {
            return Vec::new();
        }
        let root = self.root_of(index);
        (0..self.layers.len()).filter(|&i| i == root || self.link_parent(i) == Some(root)).collect()
    }

    /// The place of layer `index` inside the frame of its root, as fractions of that frame: the
    /// whole frame `{0, 0, 1, 1}` for a root, and the layer's own `frame` for a validly linked
    /// layer. A right Y layer has the place of the layer it belongs to; a panel linked to the first
    /// panel of a stack has a place of its own (graphing spec section 15).
    pub fn place(&self, index: usize) -> Frame {
        match self.link_parent(index) {
            Some(_) => self.layers[index].frame,
            None => Frame::WHOLE,
        }
    }

    /// The layers of layer `index`'s group that are drawn in the same frame as it (the same
    /// [`Graph::place`]), in layer order: a layer and its right Y layer, but not the other panels
    /// of a stack. Empty when there is no such layer.
    pub fn same_frame(&self, index: usize) -> Vec<usize> {
        if index >= self.layers.len() {
            return Vec::new();
        }
        let place = self.place(index);
        self.group(index).into_iter().filter(|&i| self.place(i) == place).collect()
    }

    /// The panels of the graph, in layer order: every layer except those drawn in the frame of an
    /// earlier layer of their group (a right Y layer is part of the panel it lies over). A graph
    /// with one layer, or a double-Y graph, has one panel.
    pub fn panels(&self) -> Vec<usize> {
        (0..self.layers.len()).filter(|&i| self.same_frame(i).first() == Some(&i)).collect()
    }

    /// The linked layer that gives the frame of layer `index` its right Y axis, if there is one: a
    /// member of the layer's group, drawn in the same frame, whose Y axis is on the right.
    pub fn right_y_member(&self, index: usize) -> Option<usize> {
        self.same_frame(index)
            .into_iter()
            .find(|&i| self.link_parent(i).is_some() && self.layers[i].y_axis.side == AxisSide::Opposite)
    }
}

/// The sheet of "paper" a graph is drawn on.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Page {
    pub width_pt: f64,
    pub height_pt: f64,
    pub background: Color,
}

impl Page {
    pub fn inches(width: f64, height: f64) -> Self {
        Page { width_pt: width * 72.0, height_pt: height * 72.0, background: Color::WHITE }
    }
}

/// The rectangle inside which data is drawn, as fractions of the page
/// (0..1, measured from the top-left corner). Axis titles and tick labels
/// are drawn outside this rectangle.
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Frame {
    pub left: f64,
    pub top: f64,
    pub width: f64,
    pub height: f64,
}

impl Frame {
    /// The whole rectangle it is measured in: the whole page, or the whole frame of the parent
    /// layer for a linked layer.
    pub const WHOLE: Frame = Frame { left: 0.0, top: 0.0, width: 1.0, height: 1.0 };
}

/// One set of axes with the plots drawn in it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Layer {
    pub frame: Frame,
    pub x_axis: Axis,
    pub y_axis: Axis,
    pub plots: Vec<Plot>,
    pub legend: Legend,
    /// Free text labels placed in the layer.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub labels: Vec<TextLabel>,
    /// The colors given to new plots, in order. See [`Layer::next_color`].
    pub palette: Palette,
    /// Style copied to each new plot added to this layer.
    pub default_line: LineStyle,
    pub default_symbol: SymbolStyle,
    pub default_kind: PlotKind,
    /// Set when this layer follows another layer of the graph: a double-Y graph is a layer with a
    /// right Y axis linked to the layer with the left one, and the panels of a stack are linked to
    /// the first panel to share its X axis. While the link is valid, `frame` is read as fractions
    /// of the parent's frame instead of the page.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link: Option<LayerLink>,
    /// How the plots are moved against each other so that they do not overlap (stacked lines,
    /// waterfall). Only the drawing moves; the data stay as they are.
    #[serde(default, skip_serializing_if = "Stack::is_none")]
    pub stack: Stack,
    /// Settings of single curves for [`Layer::stack`], found by the curve's Y column: the offsets
    /// of [`Stack::Individual`] mode and which curve a fitted curve moves with.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub plot_offsets: Vec<PlotOffset>,
    /// Width and spacing of the bars of the layer's column plots. All visible column plots of a
    /// layer stand side by side as one group (graphing spec section 14.4).
    #[serde(default, skip_serializing_if = "BarSpacing::is_default")]
    pub bars: BarSpacing,
}

/// How the bars of the column plots of a layer share the room at one X position (Origin's "Gap
/// Between Bars" and "Overlap"). The rules are in the graphing spec, section 14.4.
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BarSpacing {
    /// The gap between neighbouring groups of bars, in percent of the X spacing (0 to 100). The
    /// bars at one X position together take `100 - gap_pct` percent of the spacing.
    #[serde(default = "default_gap_pct")]
    pub gap_pct: f64,
    /// How much neighbouring bars of the group cover each other, in percent of a bar's width
    /// (-100 to 100). Negative values leave a gap between them; 100 puts them on top of each other.
    #[serde(default)]
    pub overlap_pct: f64,
}

/// Origin's default gap between bars, in percent.
pub const DEFAULT_BAR_GAP_PCT: f64 = 20.0;

fn default_gap_pct() -> f64 {
    DEFAULT_BAR_GAP_PCT
}

impl Default for BarSpacing {
    fn default() -> Self {
        BarSpacing { gap_pct: DEFAULT_BAR_GAP_PCT, overlap_pct: 0.0 }
    }
}

impl BarSpacing {
    pub fn is_default(&self) -> bool {
        *self == BarSpacing::default()
    }
}

/// Where the tick labels of an axis come from (Origin's tick label types "Numeric" and "Text from
/// dataset"). The rules are in the graphing spec, section 14.3.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum TickLabelSource {
    /// The X axis shows the cells of the first plot's X column when that column holds text (sample
    /// names), and numbers otherwise. A Y axis always shows numbers.
    #[default]
    Auto,
    /// Always numbers.
    Numbers,
    /// The major tick at value k shows the cell of row k (counted from 1) of this column; other
    /// ticks get no label.
    Column { column: ColumnId },
}

impl TickLabelSource {
    pub fn is_auto(&self) -> bool {
        *self == TickLabelSource::Auto
    }
}

/// How the plots of a layer are moved against each other (Origin's "Offset Grouped Data in
/// Layer" and its "Stacked Lines by Y Offsets" and "Waterfall" graphs). The rules are in the
/// graphing spec, section 13.
///
/// The members of the stack are the plots whose Y column holds numbers and that do not follow
/// another curve (see [`PlotOffset::follows`]), in plot order. Hidden plots are members too, so
/// that hiding a curve does not move the others.
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum Stack {
    /// No offsets.
    #[default]
    None,
    /// The first member stays where it is. Every later member is moved up until its lowest point
    /// lies one gap above the highest point of the member before it (after that one was moved).
    /// The gap is `gap_percent` percent of the height of the tallest member. Recomputed from the
    /// data at every drawing.
    Auto { gap_percent: f64 },
    /// Member k (counted from 0) is moved up by `k * step` axis units.
    Constant { step: f64 },
    /// Each member is moved by its own offset from [`Layer::plot_offsets`].
    Individual,
    /// A simple two-dimensional waterfall: member k is drawn moved right by `k * dx_percent` and
    /// up by `k * dy_percent` percent of the front frame, which shrinks to make room. Drawn only
    /// for a layer that is not linked to another and has no linked layers.
    Waterfall { dx_percent: f64, dy_percent: f64 },
}

impl Stack {
    pub fn is_none(&self) -> bool {
        *self == Stack::None
    }
}

/// Stack settings of the curves of one Y column in a layer (an entry of
/// [`Layer::plot_offsets`]).
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlotOffset {
    /// The Y column of the curve.
    pub column: ColumnId,
    /// Offset in X, in axis units; used in [`Stack::Individual`] mode.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub x: f64,
    /// Offset in Y, in axis units; used in [`Stack::Individual`] mode.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub y: f64,
    /// When set, the curve has no place of its own in the stack: it takes the offset (and the
    /// waterfall shift) of the first member whose Y column is this one. A fitted curve drawn over
    /// its data uses it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub follows: Option<ColumnId>,
}

fn is_zero(v: &f64) -> bool {
    *v == 0.0
}

/// How a layer follows its parent layer.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayerLink {
    /// Index of the parent in [`Graph::layers`]; it must be smaller than the layer's own index.
    pub parent: usize,
    #[serde(default)]
    pub x: ScaleLink,
    #[serde(default)]
    pub y: ScaleLink,
}

/// How one axis scale of a linked layer relates to the parent's.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ScaleLink {
    /// The layer's own scale.
    #[default]
    None,
    /// Exactly the parent's scale: limits, scale type, ticks and direction. The automatic range
    /// covers the data of the parent and of every layer linked this way.
    Straight,
}

/// Which edge of the frame an axis is drawn on.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AxisSide {
    /// Bottom for X, left for Y.
    #[default]
    Normal,
    /// Top for X, right for Y: line, ticks, tick labels and title move there.
    Opposite,
    /// Nothing of the axis is drawn; its scale still places the data.
    Hidden,
}

impl AxisSide {
    fn is_normal(&self) -> bool {
        *self == AxisSide::Normal
    }
}

/// Which plots a legend lists.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum LegendScope {
    /// The plots of its own layer.
    #[default]
    Layer,
    /// The plots of every layer drawn in its layer's frame ([`Graph::same_frame`]: the layer and
    /// its right Y layer), layer by layer. In a graph of one panel that is every layer.
    Graph,
}

impl LegendScope {
    fn is_layer(&self) -> bool {
        *self == LegendScope::Layer
    }
}

fn is_false(b: &bool) -> bool {
    !*b
}

impl Layer {
    /// Recolors the plots whose color still comes from the palette, in plot order, and leaves
    /// colors chosen by hand alone. Used when plots are added or removed: with a palette that
    /// spreads its colors over the number of plots, every automatic color moves along.
    ///
    /// A column plot gets the color as its fill only; its outline keeps its own color (black in
    /// the column template, as in Origin).
    pub fn recolor_automatic(&mut self) {
        let n = self.plots.len();
        for (i, p) in self.plots.iter_mut().enumerate().filter(|(_, p)| p.auto_color) {
            let c = self.palette.color(i, n);
            if p.kind != PlotKind::Column {
                p.line.color = c;
            }
            p.symbol.color = c;
        }
    }

    /// Recolors all plots from the palette, in plot order, including colors chosen by hand
    /// (the "Recolor plots" command); afterwards every color is automatic again.
    pub fn apply_palette(&mut self) {
        for p in &mut self.plots {
            p.auto_color = true;
        }
        self.recolor_automatic();
    }

    /// The stack settings of the curves of Y column `column`, if any were stored.
    pub fn plot_offset(&self, column: ColumnId) -> Option<&PlotOffset> {
        self.plot_offsets.iter().find(|o| o.column == column)
    }

    /// The stack settings of Y column `column`, added with no offset when missing.
    pub fn plot_offset_mut(&mut self, column: ColumnId) -> &mut PlotOffset {
        let i = match self.plot_offsets.iter().position(|o| o.column == column) {
            Some(i) => i,
            None => {
                self.plot_offsets.push(PlotOffset { column, x: 0.0, y: 0.0, follows: None });
                self.plot_offsets.len() - 1
            }
        };
        &mut self.plot_offsets[i]
    }

    /// The column whose curve the curves of `column` move with, when they follow one.
    pub fn follows(&self, column: ColumnId) -> Option<ColumnId> {
        self.plot_offset(column).and_then(|o| o.follows).filter(|leader| *leader != column)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ScaleKind {
    #[default]
    Linear,
    Log10,
}

/// How an axis picks its limits when `min`/`max` are not fixed.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AutoRange {
    /// Round outwards to whole tick steps, with a small margin around the data.
    #[default]
    Nice,
    /// Exactly the data range, no rounding.
    Tight,
    /// Symmetric about zero (`-L..+L`) so that zero sits in the middle.
    /// The limit is a whole number of tick steps, at most three per side.
    SymmetricZero,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TickDirection {
    #[default]
    In,
    Out,
    Both,
    None,
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case", tag = "kind", content = "digits")]
pub enum TickFormat {
    /// As few digits as the tick step needs.
    #[default]
    Auto,
    Decimal(u8),
    Scientific(u8),
    /// The axis values are date numbers (see `datetime`): ticks fall on round times and labels
    /// are dates (with the given pattern) or times of day, depending on the tick step.
    DateTime(crate::datetime::DatePattern),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Font {
    pub family: String,
    pub size_pt: f64,
    #[serde(default)]
    pub bold: bool,
    #[serde(default)]
    pub italic: bool,
    pub color: Color,
}

impl Font {
    pub fn new(family: &str, size_pt: f64) -> Self {
        Font { family: family.to_string(), size_pt, bold: false, italic: false, color: Color::BLACK }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Axis {
    #[serde(default)]
    pub scale: ScaleKind,
    /// Fixed lower limit; `None` means automatic.
    pub min: Option<f64>,
    /// Fixed upper limit; `None` means automatic.
    pub max: Option<f64>,
    #[serde(default)]
    pub auto_range: AutoRange,
    /// Distance between major ticks; `None` means automatic.
    pub major_step: Option<f64>,
    /// Number of minor ticks between two major ticks.
    pub minor_ticks: u32,
    /// Title text. `None` means "build it from the first plot's column":
    /// long name plus units in parentheses.
    pub title: Option<String>,
    pub show_title: bool,
    pub title_font: Font,
    pub show_tick_labels: bool,
    pub tick_font: Font,
    #[serde(default)]
    pub tick_format: TickFormat,
    /// Tick labels show the axis value divided by this factor (1 = unchanged).
    pub tick_label_divisor: f64,
    pub major_tick: TickDirection,
    pub minor_tick: TickDirection,
    pub major_tick_len_pt: f64,
    pub minor_tick_len_pt: f64,
    /// Width of the axis line and of the ticks.
    pub line_width_pt: f64,
    pub color: Color,
    /// Draw the axis line on the opposite side too (top for X, right for Y).
    /// With both axes set this closes the box around the plot.
    pub show_opposite: bool,
    /// Draw ticks (without labels) on the opposite side.
    pub opposite_ticks: bool,
    pub major_grid: Option<LineStyle>,
    pub minor_grid: Option<LineStyle>,
    /// The edge the axis is drawn on. With [`AxisSide::Opposite`], `show_opposite` and
    /// `opposite_ticks` refer to the bottom (X) or left (Y) edge.
    #[serde(default, skip_serializing_if = "AxisSide::is_normal")]
    pub side: AxisSide,
    /// Draw the axis line, its ticks and its tick labels in the color of the layer's first
    /// visible plot instead of `color` and the tick font's color.
    #[serde(default, skip_serializing_if = "is_false")]
    pub color_follows_plot: bool,
    /// Draw the title in that color too.
    #[serde(default, skip_serializing_if = "is_false")]
    pub title_color_follows_plot: bool,
    /// Where the tick labels come from: numbers, or the cells of a text column such as sample names.
    #[serde(default, skip_serializing_if = "TickLabelSource::is_auto")]
    pub tick_label_source: TickLabelSource,
    /// Counter-clockwise rotation of the tick labels in degrees. Used for X axes; a Y axis ignores it.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub tick_label_angle_deg: f64,
    /// Leave out the tick label at the far end of the axis: the top end of a Y axis, the right
    /// end of an X axis. Panels that touch use it, so that the labels at the joint do not run into
    /// the labels of the next panel (graphing spec section 15).
    #[serde(default, skip_serializing_if = "is_false")]
    pub hide_end_label: bool,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Dash {
    #[default]
    Solid,
    Dash,
    Dot,
    DashDot,
    DashDotDot,
    ShortDash,
    ShortDot,
    ShortDashDot,
}

impl Dash {
    /// Dash pattern as multiples of the line width (on, off, on, off, ...).
    /// Empty for a solid line.
    pub fn pattern(&self) -> &'static [f64] {
        match self {
            Dash::Solid => &[],
            Dash::Dash => &[6.0, 3.0],
            Dash::Dot => &[1.0, 3.0],
            Dash::DashDot => &[6.0, 3.0, 1.0, 3.0],
            Dash::DashDotDot => &[6.0, 3.0, 1.0, 3.0, 1.0, 3.0],
            Dash::ShortDash => &[3.0, 2.0],
            Dash::ShortDot => &[1.0, 1.5],
            Dash::ShortDashDot => &[3.0, 2.0, 1.0, 2.0],
        }
    }

    pub const ALL: [Dash; 8] = [
        Dash::Solid,
        Dash::Dash,
        Dash::Dot,
        Dash::DashDot,
        Dash::DashDotDot,
        Dash::ShortDash,
        Dash::ShortDot,
        Dash::ShortDashDot,
    ];
}

/// How consecutive data points are joined.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Connect {
    #[default]
    Straight,
    /// A smooth curve through every point (Catmull-Rom spline).
    Spline,
    /// Horizontal first, then vertical.
    StepHorizontal,
    /// Vertical first, then horizontal.
    StepVertical,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LineStyle {
    pub color: Color,
    pub width_pt: f64,
    #[serde(default)]
    pub dash: Dash,
    #[serde(default)]
    pub connect: Connect,
}

impl LineStyle {
    pub fn solid(color: Color, width_pt: f64) -> Self {
        LineStyle { color, width_pt, dash: Dash::Solid, connect: Connect::Straight }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SymbolShape {
    Square,
    #[default]
    Circle,
    TriangleUp,
    TriangleDown,
    Diamond,
    Plus,
    Cross,
    Star,
}

impl SymbolShape {
    pub const ALL: [SymbolShape; 8] = [
        SymbolShape::Square,
        SymbolShape::Circle,
        SymbolShape::TriangleUp,
        SymbolShape::TriangleDown,
        SymbolShape::Diamond,
        SymbolShape::Plus,
        SymbolShape::Cross,
        SymbolShape::Star,
    ];
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SymbolStyle {
    pub shape: SymbolShape,
    /// Width of the symbol's bounding box.
    pub size_pt: f64,
    pub color: Color,
    /// Filled with the color, or drawn as an outline.
    pub filled: bool,
    pub edge_width_pt: f64,
    /// Draw a symbol only on every n-th point (1 = every point).
    #[serde(default = "one")]
    pub every: u32,
}

fn one() -> u32 {
    1
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PlotKind {
    #[default]
    Line,
    Scatter,
    LineSymbol,
    /// Vertical bars from zero up to each value. The bar is filled with the
    /// symbol color and outlined with the line style.
    Column,
    /// A line with the area between it and Y = 0 filled with the symbol color (Origin's Area
    /// plot). Where 0 is outside the Y axis the fill reaches the edge of the frame.
    Area,
}

impl PlotKind {
    pub fn has_line(&self) -> bool {
        matches!(self, PlotKind::Line | PlotKind::LineSymbol | PlotKind::Area)
    }

    pub fn has_symbols(&self) -> bool {
        matches!(self, PlotKind::Scatter | PlotKind::LineSymbol)
    }
}

/// One curve: a Y column drawn against an X column.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Plot {
    /// The X column; `None` plots against the row number (1, 2, 3, ...).
    pub x: Option<ColumnId>,
    pub y: ColumnId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y_error: Option<ColumnId>,
    /// Half-open row range `[start, end)` to plot; `None` plots all rows.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rows: Option<(usize, usize)>,
    pub kind: PlotKind,
    pub line: LineStyle,
    pub symbol: SymbolStyle,
    /// Legend text. `None` means "use the Y column's comments, long name or short name".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default = "yes")]
    pub visible: bool,
    /// True while the plot's color comes from the layer's palette. Choosing a color by hand
    /// clears it, so that adding plots later re-spreads only the automatic colors.
    #[serde(default = "yes")]
    pub auto_color: bool,
}

fn yes() -> bool {
    true
}

/// Where the legend sits inside the layer frame.
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum LegendPosition {
    TopRight,
    TopLeft,
    BottomRight,
    BottomLeft,
    /// Legend centre at these fractions of the layer frame (0,0 = top-left).
    Manual {
        x: f64,
        y: f64,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Legend {
    pub visible: bool,
    pub position: LegendPosition,
    pub font: Font,
    /// Draw a box around the legend.
    pub frame: bool,
    pub background: Option<Color>,
    #[serde(default, skip_serializing_if = "LegendScope::is_layer")]
    pub scope: LegendScope,
    /// The number of columns the entries are arranged in, filled column by column; 1 is a single
    /// list.
    #[serde(default = "one_column", skip_serializing_if = "is_one_column")]
    pub columns: u32,
    /// The legend text in Origin's notation, one entry per line: `\l(n)` draws the symbol of
    /// plot n, `%(n)` is its name, any other text is shown as it is (with the label markup). `n`
    /// counts the plots the legend lists (`l.p`, layer and plot, when it lists several layers).
    /// `None` is the automatic text, `\l(1) %(1)` and so on for every plot, which follows the
    /// plots as they come and go.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

fn one_column() -> u32 {
    1
}

fn is_one_column(c: &u32) -> bool {
    *c == 1
}

impl Legend {
    /// The automatic legend text for plots with these ids (see [`Legend::text`]): a symbol and a
    /// name per line.
    pub fn automatic_text<S: AsRef<str>>(ids: &[S]) -> String {
        ids.iter().map(|id| format!("\\l({0}) %({0})", id.as_ref())).collect::<Vec<_>>().join("\n")
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TextLabel {
    pub text: String,
    /// Anchor point as fractions of the layer frame (0,0 = top-left).
    pub x: f64,
    pub y: f64,
    pub font: Font,
    /// Counter-clockwise rotation in degrees.
    #[serde(default)]
    pub rotation_deg: f64,
}

impl TextLabel {
    /// A label with no rotation.
    pub fn new(text: impl Into<String>, x: f64, y: f64, font: Font) -> Self {
        TextLabel { text: text.into(), x, y, font, rotation_deg: 0.0 }
    }
}

impl Layer {
    /// The font a new free text label of this layer starts with: the layer's legend font (the
    /// text size the figure already uses), black and upright.
    pub fn new_label_font(&self) -> Font {
        Font { bold: false, italic: false, color: Color::BLACK, ..self.legend.font.clone() }
    }
}

/// The colors a layer hands out to its plots.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Palette {
    pub colors: Vec<Color>,
    pub mode: PaletteMode,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaletteMode {
    /// Plot `i` takes color `i`, wrapping around at the end of the list.
    Cycle,
    /// Colors run as a gradient through the list: the first plot takes the
    /// first color and the last plot the last color, whatever the number of
    /// plots. Good for a sequence of scans that should read as one family.
    Spread,
}

impl Palette {
    /// The color of plot number `index` (0-based) out of `count` plots.
    pub fn color(&self, index: usize, count: usize) -> Color {
        if self.colors.is_empty() {
            return Color::BLACK;
        }
        match self.mode {
            PaletteMode::Cycle => self.colors[index % self.colors.len()],
            PaletteMode::Spread => {
                let k = self.colors.len();
                if k == 1 || count <= 1 {
                    return self.colors[0];
                }
                // Position along the list of colors, 0..k-1.
                let pos = index.min(count - 1) as f64 * (k - 1) as f64 / (count - 1) as f64;
                let lo = pos.floor() as usize;
                let hi = (lo + 1).min(k - 1);
                self.colors[lo].lerp(&self.colors[hi], pos - lo as f64)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pal(mode: PaletteMode) -> Palette {
        Palette { colors: vec![Color::rgb(0, 0, 0), Color::rgb(100, 100, 100), Color::rgb(200, 200, 200)], mode }
    }

    #[test]
    fn cycle_palette_wraps() {
        let p = pal(PaletteMode::Cycle);
        assert_eq!(p.color(0, 5), Color::rgb(0, 0, 0));
        assert_eq!(p.color(3, 5), Color::rgb(0, 0, 0));
        assert_eq!(p.color(4, 5), Color::rgb(100, 100, 100));
    }

    #[test]
    fn spread_palette_uses_the_whole_range() {
        let p = pal(PaletteMode::Spread);
        // Two plots: first and last color, not the first two.
        assert_eq!(p.color(0, 2), Color::rgb(0, 0, 0));
        assert_eq!(p.color(1, 2), Color::rgb(200, 200, 200));
        // Three plots: exactly the list.
        assert_eq!(p.color(1, 3), Color::rgb(100, 100, 100));
        // Five plots: interpolated in between.
        assert_eq!(p.color(1, 5), Color::rgb(50, 50, 50));
        assert_eq!(p.color(4, 5), Color::rgb(200, 200, 200));
        assert_eq!(p.color(0, 1), Color::rgb(0, 0, 0));
    }

    /// A layer as files written before linked layers existed store it: no `link`, `side`,
    /// `color_follows_plot`, `title_color_follows_plot` or `scope` keys.
    const OLD_LAYER: &str = r##"{
        "frame": {"left": 0.16, "top": 0.07, "width": 0.78, "height": 0.75},
        "x_axis": {"scale": "linear", "min": null, "max": 2.0, "auto_range": "nice", "major_step": null,
            "minor_ticks": 1, "title": null, "show_title": true,
            "title_font": {"family": "Arial", "size_pt": 14.0, "color": "#000000"},
            "show_tick_labels": true,
            "tick_font": {"family": "Arial", "size_pt": 12.0, "color": "#000000"},
            "tick_format": {"kind": "auto"}, "tick_label_divisor": 1.0, "major_tick": "out",
            "minor_tick": "out", "major_tick_len_pt": 5.0, "minor_tick_len_pt": 2.5, "line_width_pt": 1.0,
            "color": "#000000", "show_opposite": false, "opposite_ticks": false,
            "major_grid": null, "minor_grid": null},
        "y_axis": {"scale": "linear", "min": null, "max": null, "auto_range": "symmetric_zero",
            "major_step": null, "minor_ticks": 0, "title": "j", "show_title": true,
            "title_font": {"family": "Arial", "size_pt": 14.0, "color": "#000000"},
            "show_tick_labels": true,
            "tick_font": {"family": "Arial", "size_pt": 12.0, "color": "#000000"},
            "tick_label_divisor": 1.0, "major_tick": "in", "minor_tick": "none", "major_tick_len_pt": 5.0,
            "minor_tick_len_pt": 2.5, "line_width_pt": 1.0, "color": "#000000", "show_opposite": true,
            "opposite_ticks": false, "major_grid": null, "minor_grid": null},
        "plots": [],
        "legend": {"visible": true, "position": {"kind": "top_right"},
            "font": {"family": "Arial", "size_pt": 11.0, "color": "#000000"},
            "frame": false, "background": null},
        "palette": {"colors": ["#000000"], "mode": "cycle"},
        "default_line": {"color": "#000000", "width_pt": 1.5},
        "default_symbol": {"shape": "circle", "size_pt": 5.0, "color": "#000000", "filled": true,
            "edge_width_pt": 0.75},
        "default_kind": "line"
    }"##;

    #[test]
    fn a_layer_saved_before_linked_layers_loads_with_the_old_meaning() {
        let layer: Layer = serde_json::from_str(OLD_LAYER).expect("an old layer loads");
        assert_eq!(layer.link, None);
        for axis in [&layer.x_axis, &layer.y_axis] {
            assert_eq!(axis.side, AxisSide::Normal);
            assert!(!axis.color_follows_plot && !axis.title_color_follows_plot);
        }
        assert_eq!(layer.legend.scope, LegendScope::Layer);
        assert_eq!((layer.legend.columns, layer.legend.text.as_deref()), (1, None));
        assert_eq!(layer.x_axis.max, Some(2.0));
        // Saving it again writes none of the new keys, so the file looks as before.
        let json = serde_json::to_string(&layer).unwrap();
        for key in ["\"link\"", "\"side\"", "color_follows_plot", "\"scope\"", "\"columns\"", "\"text\""] {
            assert!(!json.contains(key), "{key} in {json}");
        }
        let back: Layer = serde_json::from_str(&json).unwrap();
        assert_eq!(back, layer);
    }

    #[test]
    fn new_fields_are_written_when_set_and_read_back() {
        let mut layer: Layer = serde_json::from_str(OLD_LAYER).unwrap();
        layer.link = Some(LayerLink { parent: 0, x: ScaleLink::Straight, y: ScaleLink::None });
        layer.y_axis.side = AxisSide::Opposite;
        layer.x_axis.side = AxisSide::Hidden;
        layer.y_axis.color_follows_plot = true;
        layer.y_axis.title_color_follows_plot = true;
        layer.legend.scope = LegendScope::Graph;
        layer.legend.columns = 2;
        layer.legend.text = Some("\\l(1) a\n%(2)".into());
        let json = serde_json::to_string(&layer).unwrap();
        assert!(json.contains(r#""link":{"parent":0,"x":"straight","y":"none"}"#), "{json}");
        assert!(json.contains(r#""side":"opposite""#) && json.contains(r#""side":"hidden""#), "{json}");
        assert!(json.contains(r#""scope":"graph""#), "{json}");
        let back: Layer = serde_json::from_str(&json).unwrap();
        assert_eq!(back, layer);
    }

    #[test]
    fn a_layer_saved_before_stacking_has_no_offsets_and_is_written_as_before() {
        let layer: Layer = serde_json::from_str(OLD_LAYER).expect("an old layer loads");
        assert_eq!(layer.stack, Stack::None);
        assert!(layer.plot_offsets.is_empty());
        let json = serde_json::to_string(&layer).unwrap();
        assert!(!json.contains("\"stack\"") && !json.contains("plot_offsets"), "{json}");
    }

    #[test]
    fn stack_modes_are_written_with_a_kind_and_read_back() {
        let cases = [
            (Stack::None, r#"{"kind":"none"}"#),
            (Stack::Auto { gap_percent: 10.0 }, r#"{"kind":"auto","gap_percent":10.0}"#),
            (Stack::Constant { step: -500.0 }, r#"{"kind":"constant","step":-500.0}"#),
            (Stack::Individual, r#"{"kind":"individual"}"#),
            (
                Stack::Waterfall { dx_percent: 5.0, dy_percent: 10.0 },
                r#"{"kind":"waterfall","dx_percent":5.0,"dy_percent":10.0}"#,
            ),
        ];
        for (stack, text) in cases {
            assert_eq!(serde_json::to_string(&stack).unwrap(), text);
            assert_eq!(serde_json::from_str::<Stack>(text).unwrap(), stack);
        }
        let mut layer: Layer = serde_json::from_str(OLD_LAYER).unwrap();
        layer.stack = Stack::Auto { gap_percent: 10.0 };
        layer.plot_offset_mut(ColumnId(7)).y = 100.0;
        let json = serde_json::to_string(&layer).unwrap();
        assert!(json.contains(r#""stack":{"kind":"auto","gap_percent":10.0}"#), "{json}");
        assert!(json.contains(r#""plot_offsets":[{"column":7,"y":100.0}]"#), "{json}");
        assert_eq!(serde_json::from_str::<Layer>(&json).unwrap(), layer);
    }

    #[test]
    fn plot_offsets_are_found_by_column_and_added_once() {
        let mut layer: Layer = serde_json::from_str(OLD_LAYER).unwrap();
        let entry: PlotOffset = serde_json::from_str(r#"{"column":3}"#).unwrap();
        assert_eq!(entry, PlotOffset { column: ColumnId(3), x: 0.0, y: 0.0, follows: None });
        assert_eq!(layer.plot_offset(ColumnId(3)), None);
        layer.plot_offset_mut(ColumnId(3)).x = 1.5;
        layer.plot_offset_mut(ColumnId(3)).follows = Some(ColumnId(2));
        assert_eq!(layer.plot_offsets.len(), 1);
        assert_eq!(layer.plot_offset(ColumnId(3)).map(|o| o.x), Some(1.5));
        assert_eq!(layer.follows(ColumnId(3)), Some(ColumnId(2)));
        // A curve cannot follow itself.
        layer.plot_offset_mut(ColumnId(4)).follows = Some(ColumnId(4));
        assert_eq!(layer.follows(ColumnId(4)), None);
        assert_eq!(layer.follows(ColumnId(9)), None);
    }

    #[test]
    fn a_layer_saved_before_text_tick_labels_and_bars_loads_with_the_defaults() {
        let layer: Layer = serde_json::from_str(OLD_LAYER).expect("an old layer loads");
        for axis in [&layer.x_axis, &layer.y_axis] {
            assert_eq!(axis.tick_label_source, TickLabelSource::Auto);
            assert_eq!(axis.tick_label_angle_deg, 0.0);
        }
        assert_eq!(layer.bars, BarSpacing { gap_pct: 20.0, overlap_pct: 0.0 });
        // Saved again, none of the new keys is written, and it reads back equal.
        let json = serde_json::to_string(&layer).unwrap();
        for key in ["tick_label_source", "tick_label_angle_deg", "\"bars\""] {
            assert!(!json.contains(key), "{key} in {json}");
        }
        assert_eq!(serde_json::from_str::<Layer>(&json).unwrap(), layer);
    }

    #[test]
    fn tick_label_sources_and_bar_spacing_are_written_when_set_and_read_back() {
        let mut layer: Layer = serde_json::from_str(OLD_LAYER).unwrap();
        layer.x_axis.tick_label_source = TickLabelSource::Column { column: ColumnId(5) };
        layer.x_axis.tick_label_angle_deg = 45.0;
        layer.y_axis.tick_label_source = TickLabelSource::Numbers;
        layer.bars.gap_pct = 50.0;
        let json = serde_json::to_string(&layer).unwrap();
        assert!(json.contains(r#""tick_label_source":{"kind":"column","column":5}"#), "{json}");
        assert!(json.contains(r#""tick_label_source":{"kind":"numbers"}"#), "{json}");
        assert!(json.contains(r#""tick_label_angle_deg":45.0"#), "{json}");
        assert!(json.contains(r#""bars":{"gap_pct":50.0,"overlap_pct":0.0}"#), "{json}");
        assert_eq!(serde_json::from_str::<Layer>(&json).unwrap(), layer);
        // A bar entry with only one of its numbers takes the default for the other.
        let bars: BarSpacing = serde_json::from_str(r#"{"overlap_pct":-50.0}"#).unwrap();
        assert_eq!(bars, BarSpacing { gap_pct: 20.0, overlap_pct: -50.0 });
        assert_eq!(serde_json::from_str::<TickLabelSource>(r#"{"kind":"auto"}"#).unwrap(), TickLabelSource::Auto);
    }

    #[test]
    fn a_layer_saved_before_panels_shows_every_end_label_and_is_written_as_before() {
        let layer: Layer = serde_json::from_str(OLD_LAYER).expect("an old layer loads");
        assert!(!layer.x_axis.hide_end_label && !layer.y_axis.hide_end_label);
        let json = serde_json::to_string(&layer).unwrap();
        assert!(!json.contains("hide_end_label"), "{json}");
        let mut layer = layer;
        layer.y_axis.hide_end_label = true;
        let json = serde_json::to_string(&layer).unwrap();
        assert_eq!(json.matches(r#""hide_end_label":true"#).count(), 1, "{json}");
        assert_eq!(serde_json::from_str::<Layer>(&json).unwrap(), layer);
    }

    #[test]
    fn layers_drawn_in_one_frame_form_one_panel() {
        // Layer 1 covers layer 0 (a right Y axis), layer 2 sits above it (a second panel), layer 3
        // covers layer 2, and layer 4 is a graph of its own.
        let mut g = graph_of(&[None, Some(0), Some(0), Some(0), None]);
        g.layers[1].frame = Frame::WHOLE;
        g.layers[2].frame = Frame { left: 0.0, top: -1.0, width: 1.0, height: 1.0 };
        g.layers[3].frame = g.layers[2].frame;
        g.layers[3].y_axis.side = AxisSide::Opposite;
        g.layers[1].y_axis.side = AxisSide::Opposite;
        assert_eq!(g.place(0), Frame::WHOLE);
        assert_eq!(g.place(2), g.layers[2].frame);
        assert_eq!(g.place(4), Frame::WHOLE, "a root's place is its whole frame");
        assert_eq!(g.group(0), vec![0, 1, 2, 3]);
        assert_eq!(g.same_frame(0), vec![0, 1]);
        assert_eq!(g.same_frame(3), vec![2, 3]);
        assert_eq!(g.same_frame(4), vec![4]);
        assert_eq!(g.same_frame(9), Vec::<usize>::new());
        assert_eq!(g.panels(), vec![0, 2, 4]);
        assert_eq!(g.right_y_member(0), Some(1));
        assert_eq!(g.right_y_member(2), Some(3));
        assert_eq!(g.right_y_member(4), None);
    }

    #[test]
    fn an_automatic_color_fills_a_column_plot_and_leaves_its_outline() {
        let mut layer: Layer = serde_json::from_str(OLD_LAYER).unwrap();
        layer.palette = Palette { colors: vec![Color::BLACK, Color::RED], mode: PaletteMode::Cycle };
        let plot = |kind: PlotKind| Plot {
            x: None,
            y: ColumnId(1),
            y_error: None,
            rows: None,
            kind,
            line: LineStyle::solid(Color::BLACK, 0.5),
            symbol: layer.default_symbol.clone(),
            label: None,
            visible: true,
            auto_color: true,
        };
        layer.plots = vec![plot(PlotKind::Column), plot(PlotKind::Column), plot(PlotKind::Line)];
        layer.recolor_automatic();
        assert_eq!(layer.plots[1].symbol.color, Color::RED);
        assert_eq!(layer.plots[1].line.color, Color::BLACK, "the outline keeps its color");
        // Other kinds take the color for line and symbol, as before.
        assert_eq!((layer.plots[2].line.color, layer.plots[2].symbol.color), (Color::BLACK, Color::BLACK));
        layer.plots.swap(0, 2);
        layer.recolor_automatic();
        assert_eq!((layer.plots[2].line.color, layer.plots[2].symbol.color), (Color::BLACK, Color::BLACK));
        assert_eq!((layer.plots[1].line.color, layer.plots[1].symbol.color), (Color::BLACK, Color::RED));
        layer.plots[0].kind = PlotKind::Line;
        layer.plots[0].line.color = Color::BLUE;
        layer.recolor_automatic();
        assert_eq!(layer.plots[0].line.color, Color::BLACK);
    }

    #[test]
    fn a_label_column_is_a_referenced_column() {
        let mut g = graph_of(&[None]);
        g.layers[0].x_axis.tick_label_source = TickLabelSource::Column { column: ColumnId(42) };
        assert_eq!(g.referenced_columns(), vec![ColumnId(42)]);
    }

    fn graph_of(links: &[Option<usize>]) -> Graph {
        let base: Layer = serde_json::from_str(OLD_LAYER).unwrap();
        let layers = links
            .iter()
            .map(|p| Layer {
                link: p.map(|parent| LayerLink { parent, x: ScaleLink::Straight, y: ScaleLink::None }),
                ..base.clone()
            })
            .collect();
        Graph { id: WindowId(1), name: "G".into(), long_name: String::new(), page: Page::inches(6.4, 4.8), layers }
    }

    #[test]
    fn only_links_to_an_earlier_unlinked_layer_are_valid() {
        let g = graph_of(&[None, Some(0), Some(1), Some(3), Some(9), Some(0)]);
        assert_eq!(g.link_parent(0), None);
        assert_eq!(g.link_parent(1), Some(0));
        assert_eq!(g.link_parent(2), None, "a chain 2 -> 1 -> 0 is not followed");
        assert_eq!(g.link_parent(3), None, "a link to itself");
        assert_eq!(g.link_parent(4), None, "a link to a missing layer");
        assert_eq!(g.link_parent(5), Some(0));
        assert_eq!(g.group(0), vec![0, 1, 5]);
        assert_eq!(g.group(5), vec![0, 1, 5]);
        assert_eq!(g.group(2), vec![2]);
        assert_eq!(g.group(17), Vec::<usize>::new());
        assert_eq!(g.root_of(4), 4);
        assert_eq!(g.right_y_member(0), None, "no member has its Y axis on the right");
        let mut g = g;
        g.layers[5].y_axis.side = AxisSide::Opposite;
        g.layers[2].y_axis.side = AxisSide::Opposite;
        assert_eq!(g.right_y_member(1), Some(5));
        assert_eq!(g.right_y_member(2), None, "a layer that is not linked is not a right-Y member");
    }
}
