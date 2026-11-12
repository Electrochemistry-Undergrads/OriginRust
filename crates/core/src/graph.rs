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
        }
        out
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
}

impl Layer {
    /// Recolors the plots whose color still comes from the palette, in plot order, and leaves
    /// colors chosen by hand alone. Used when plots are added or removed: with a palette that
    /// spreads its colors over the number of plots, every automatic color moves along.
    pub fn recolor_automatic(&mut self) {
        let n = self.plots.len();
        for (i, p) in self.plots.iter_mut().enumerate().filter(|(_, p)| p.auto_color) {
            let c = self.palette.color(i, n);
            p.line.color = c;
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
}

impl PlotKind {
    pub fn has_line(&self) -> bool {
        matches!(self, PlotKind::Line | PlotKind::LineSymbol)
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
}
