//! The look settings that Plot Details adds beyond what graph.rs holds (brief T13): fills with a
//! hatch or a gradient, a border line, the page's border, layer icons and notes, and the
//! subgroup rules of a plot group.
//!
//! Every type has a default that means "as before", and every field is skipped when written while
//! it has that default, so a file written before these settings existed loads unchanged. Which of
//! the rules are Origin's own is in the graphing spec, section 18; what was not measured is said
//! there.

use crate::color::Color;
use crate::graph::Dash;
use crate::ids::ColumnId;
use serde::{Deserialize, Serialize};

fn is_default<T: Default + PartialEq>(v: &T) -> bool {
    *v == T::default()
}

fn yes() -> bool {
    true
}

fn is_true(v: &bool) -> bool {
    *v
}

/// The share of a colour that stays visible for a transparency in percent (0 to 100): 100 % hides
/// the colour, 0 % leaves it as it is.
pub fn opacity_for(transparency_pct: u8) -> f64 {
    1.0 - f64::from(transparency_pct.min(100)) / 100.0
}

// ----- fills ---------------------------------------------------------------------------------

/// A hatch drawn over the fill of a bar or an area. The list is Sheaf's own (Origin's list of
/// patterns was not measured).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum FillPattern {
    /// Only the flat colour.
    #[default]
    None,
    /// Lines from the lower left to the upper right.
    ForwardDiagonal,
    /// Lines from the upper left to the lower right.
    BackwardDiagonal,
    Horizontal,
    Vertical,
    /// Horizontal and vertical lines.
    Cross,
    /// Both diagonals.
    DiagonalCross,
}

impl FillPattern {
    pub const ALL: [FillPattern; 7] = [
        FillPattern::None,
        FillPattern::ForwardDiagonal,
        FillPattern::BackwardDiagonal,
        FillPattern::Horizontal,
        FillPattern::Vertical,
        FillPattern::Cross,
        FillPattern::DiagonalCross,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            FillPattern::None => "None",
            FillPattern::ForwardDiagonal => "Forward diagonal",
            FillPattern::BackwardDiagonal => "Backward diagonal",
            FillPattern::Horizontal => "Horizontal",
            FillPattern::Vertical => "Vertical",
            FillPattern::Cross => "Cross",
            FillPattern::DiagonalCross => "Diagonal cross",
        }
    }

    /// The directions of the hatch lines, in degrees counter-clockwise from the right.
    pub fn angles(&self) -> &'static [f64] {
        match self {
            FillPattern::None => &[],
            FillPattern::ForwardDiagonal => &[45.0],
            FillPattern::BackwardDiagonal => &[135.0],
            FillPattern::Horizontal => &[0.0],
            FillPattern::Vertical => &[90.0],
            FillPattern::Cross => &[0.0, 90.0],
            FillPattern::DiagonalCross => &[45.0, 135.0],
        }
    }
}

/// Which way a gradient runs, from the fill colour to the gradient colour.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum GradientDirection {
    #[default]
    LeftToRight,
    RightToLeft,
    TopToBottom,
    BottomToTop,
}

impl GradientDirection {
    pub const ALL: [GradientDirection; 4] = [
        GradientDirection::LeftToRight,
        GradientDirection::RightToLeft,
        GradientDirection::TopToBottom,
        GradientDirection::BottomToTop,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            GradientDirection::LeftToRight => "Left to right",
            GradientDirection::RightToLeft => "Right to left",
            GradientDirection::TopToBottom => "Top to bottom",
            GradientDirection::BottomToTop => "Bottom to top",
        }
    }

    /// The unit vector the gradient runs along on the page (y grows downwards).
    pub fn vector(&self) -> (f64, f64) {
        match self {
            GradientDirection::LeftToRight => (1.0, 0.0),
            GradientDirection::RightToLeft => (-1.0, 0.0),
            GradientDirection::TopToBottom => (0.0, 1.0),
            GradientDirection::BottomToTop => (0.0, -1.0),
        }
    }
}

/// A linear gradient: the fill colour at the start of the shape, `color` at its end.
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Gradient {
    pub color: Color,
    #[serde(default)]
    pub direction: GradientDirection,
}

impl Gradient {
    pub fn new(color: Color) -> Gradient {
        Gradient { color, direction: GradientDirection::LeftToRight }
    }
}

/// Origin's default pattern line width is not known; 0.5 pt is the width of Origin's thin lines.
pub const DEFAULT_PATTERN_WIDTH_PT: f64 = 0.5;
/// Distance between the lines of a hatch, in points (not measured against Origin).
pub const DEFAULT_PATTERN_SPACING_PT: f64 = 3.0;

fn default_pattern_color() -> Color {
    Color::BLACK
}

fn default_pattern_width() -> f64 {
    DEFAULT_PATTERN_WIDTH_PT
}

fn default_pattern_spacing() -> f64 {
    DEFAULT_PATTERN_SPACING_PT
}

/// How a bar, an area or a plot's area under its curve is filled beyond its flat colour: how see
/// through it is (on top of the alpha of the colour itself), a hatch over it, a gradient. The flat
/// colour is the plot's symbol colour (the colour of "Fill Area Under Curve" for a line plot).
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FillStyle {
    /// Transparency of the fill in percent (0 solid, 100 invisible).
    #[serde(default)]
    pub transparency_pct: u8,
    #[serde(default, skip_serializing_if = "is_default")]
    pub pattern: FillPattern,
    #[serde(default = "default_pattern_color")]
    pub pattern_color: Color,
    /// Line width of the hatch.
    #[serde(default = "default_pattern_width")]
    pub pattern_width_pt: f64,
    /// Distance between the lines of the hatch.
    #[serde(default = "default_pattern_spacing")]
    pub pattern_spacing_pt: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gradient: Option<Gradient>,
}

impl Default for FillStyle {
    fn default() -> Self {
        FillStyle {
            transparency_pct: 0,
            pattern: FillPattern::None,
            pattern_color: Color::BLACK,
            pattern_width_pt: DEFAULT_PATTERN_WIDTH_PT,
            pattern_spacing_pt: DEFAULT_PATTERN_SPACING_PT,
            gradient: None,
        }
    }
}

impl FillStyle {
    /// True when the fill is the plain colour: nothing over it, solid.
    pub fn is_plain(&self) -> bool {
        self.transparency_pct == 0 && self.pattern == FillPattern::None && self.gradient.is_none()
    }

    /// True for the style a plot starts with (the settings are not written then).
    pub fn is_default(&self) -> bool {
        *self == FillStyle::default()
    }
}

// ----- borders ------------------------------------------------------------------------------

/// A border line (colour, dash and width) around a rectangle.
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BorderStyle {
    pub color: Color,
    pub width_pt: f64,
    #[serde(default)]
    pub dash: Dash,
}

impl BorderStyle {
    pub fn solid(color: Color, width_pt: f64) -> BorderStyle {
        BorderStyle { color, width_pt, dash: Dash::Solid }
    }
}

/// What the Background tab of a layer adds to its colour (`Layer::background`): a gradient from
/// that colour and a border line around the frame. Nothing is drawn by default.
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct BackgroundStyle {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gradient: Option<Gradient>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border: Option<BorderStyle>,
}

impl BackgroundStyle {
    pub fn is_default(&self) -> bool {
        *self == BackgroundStyle::default()
    }
}

// ----- plot group ---------------------------------------------------------------------------

/// How the members of a group are split into subgroups.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum Subgrouping {
    #[default]
    None,
    /// Subgroups of this many plots, in plot order.
    BySize { size: u32 },
}

/// The property that steps from one subgroup to the next (Origin's "Subgroup on Increment"); the
/// other properties step inside a subgroup.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SubgroupOn {
    #[default]
    Color,
    LineStyle,
    SymbolShape,
}

impl SubgroupOn {
    pub const ALL: [SubgroupOn; 3] = [SubgroupOn::Color, SubgroupOn::LineStyle, SubgroupOn::SymbolShape];

    pub fn name(&self) -> &'static str {
        match self {
            SubgroupOn::Color => "Line Color",
            SubgroupOn::LineStyle => "Line Style",
            SubgroupOn::SymbolShape => "Symbol Type",
        }
    }
}

/// The steps of the members of a group: for member `k` of `n`, the step of the colour and the
/// number of steps the colour list runs over, the step of the line style and the step of the
/// symbol shape. Without subgroups every property steps with the member number; with subgroups of
/// `s` plots the property of `on` steps with the subgroup number and the others with the place
/// inside the subgroup.
pub fn group_steps(sub: Subgrouping, on: SubgroupOn, k: usize, n: usize) -> ((usize, usize), usize, usize) {
    match sub {
        Subgrouping::None => ((k, n), k, k),
        Subgrouping::BySize { size } => {
            let s = (size as usize).max(1);
            let (g, j) = (k / s, k % s);
            let groups = n.div_ceil(s);
            let within = n.min(s);
            match on {
                SubgroupOn::Color => ((g, groups), j, j),
                SubgroupOn::LineStyle => ((j, within), g, j),
                SubgroupOn::SymbolShape => ((j, within), j, g),
            }
        }
    }
}

// ----- page ---------------------------------------------------------------------------------

/// What the title of a graph window shows when it has both a short and a long name.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TitleDisplay {
    /// "Graph1 - long name", as before.
    #[default]
    Both,
    Name,
    LongName,
}

impl TitleDisplay {
    pub const ALL: [TitleDisplay; 3] = [TitleDisplay::Both, TitleDisplay::Name, TitleDisplay::LongName];

    pub fn name(&self) -> &'static str {
        match self {
            TitleDisplay::Both => "Short and long name",
            TitleDisplay::Name => "Short name",
            TitleDisplay::LongName => "Long name",
        }
    }
}

/// The settings of the page beyond its size and colour.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PageStyle {
    /// A line around the page's edge.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border: Option<BorderStyle>,
    /// The small layer number buttons at the top left of the graph window.
    #[serde(default = "yes", skip_serializing_if = "is_true")]
    pub layer_icons: bool,
    #[serde(default, skip_serializing_if = "is_default")]
    pub title_display: TitleDisplay,
    /// Free notes about the graph; the Object Manager shows them as the page's hover text.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub comments: String,
    /// Origin's page Display tab "Connect line across Axis Break" (off by default, graphing spec 3.6): off
    /// leaves a line out where it crosses the band of an axis break, on draws it through.
    #[serde(default, skip_serializing_if = "is_false")]
    pub connect_across_breaks: bool,
    /// Origin's page Display tab "Connect line across Missing Data" (off by default, graphing spec 18.7b): on
    /// joins the points before and after rows with a missing X or Y value instead of breaking the line.
    #[serde(default, skip_serializing_if = "is_false")]
    pub connect_across_missing: bool,
}

fn is_false(v: &bool) -> bool {
    !*v
}

impl Default for PageStyle {
    fn default() -> Self {
        PageStyle {
            border: None,
            layer_icons: true,
            title_display: TitleDisplay::Both,
            comments: String::new(),
            connect_across_breaks: false,
            connect_across_missing: false,
        }
    }
}

impl PageStyle {
    pub fn is_default(&self) -> bool {
        *self == PageStyle::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_write_nothing_new() {
        assert_eq!(serde_json::to_string(&BackgroundStyle::default()).unwrap(), "{}");
        assert_eq!(serde_json::to_string(&PageStyle::default()).unwrap(), "{}");
    }

    #[test]
    fn group_steps_follow_the_subgrouping() {
        let (none, color) = (Subgrouping::None, SubgroupOn::Color);
        assert_eq!(group_steps(none, color, 3, 5), ((3, 5), 3, 3));
        let sub = Subgrouping::BySize { size: 2 };
        // Plots 0..5: subgroups {0,1} {2,3} {4}; the line style steps per subgroup, colours inside.
        assert_eq!(group_steps(sub, SubgroupOn::LineStyle, 0, 5), ((0, 2), 0, 0));
        assert_eq!(group_steps(sub, SubgroupOn::LineStyle, 3, 5), ((1, 2), 1, 1));
        assert_eq!(group_steps(sub, SubgroupOn::LineStyle, 4, 5), ((0, 2), 2, 0));
        assert_eq!(group_steps(sub, SubgroupOn::SymbolShape, 3, 5), ((1, 2), 1, 1));
        assert_eq!(group_steps(sub, SubgroupOn::Color, 3, 5), ((1, 3), 1, 1));
    }

    #[test]
    fn the_new_page_and_fill_settings_round_trip() {
        let page = PageStyle {
            border: Some(BorderStyle::solid(Color::BLACK, 0.5)),
            layer_icons: false,
            title_display: TitleDisplay::LongName,
            comments: "notes".into(),
            connect_across_breaks: true,
            connect_across_missing: true,
        };
        let json = serde_json::to_string(&page).unwrap();
        assert_eq!(serde_json::from_str::<PageStyle>(&json).unwrap(), page);
        let mut fill = FillStyle { transparency_pct: 25, pattern: FillPattern::Cross, ..FillStyle::default() };
        fill.gradient = Some(Gradient { color: Color::RED, direction: GradientDirection::BottomToTop });
        let json = serde_json::to_string(&fill).unwrap();
        assert_eq!(serde_json::from_str::<FillStyle>(&json).unwrap(), fill);
        assert!(FillStyle::default().is_plain() && !fill.is_plain());
    }
}

/// What a ridgeline plot draws for its Y column (Origin's Plot Details Display tab, Type; graphing
/// spec 14.23).
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RidgeDisplay {
    /// The kernel density curve, filled down to its baseline.
    #[default]
    DistributionCurve,
    /// The curve with the observations as points on the baseline.
    WithDataPoints,
    /// The curve with a rug of short ticks below the baseline.
    WithRug,
    /// A histogram in place of the curve ("Single Block Bar").
    SingleBlockBar,
}

/// The kernel bandwidth rule of a ridgeline plot (Origin's Distribution tab, Kernel Smooth).
#[derive(Copy, Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RidgeBandwidth {
    /// `0.9 A n^(-1/5)`, `A = min(SD, IQR / 1.349)`.
    #[default]
    Scott,
    /// `1.059 A n^(-1/5)`.
    Silverman,
    /// `k` times the sample standard deviation ("k*SD").
    KSd(f64),
    /// A bandwidth typed in.
    Custom(f64),
}

impl RidgeBandwidth {
    /// The rule of the shared kernel density code.
    pub fn to_calc(self) -> sheaf_calc::kde::Bandwidth {
        use sheaf_calc::kde::Bandwidth;
        match self {
            RidgeBandwidth::Scott => Bandwidth::Scott,
            RidgeBandwidth::Silverman => Bandwidth::Silverman,
            RidgeBandwidth::KSd(k) => Bandwidth::Sd(k),
            RidgeBandwidth::Custom(h) => Bandwidth::Fixed(h),
        }
    }
}

/// The rug of a ridgeline plot (Origin: Color, Thickness, Size(%), Gap Between Curve). Sizes are in
/// percent of the spacing between baselines (Sheaf's reading of the percentages).
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RidgeRug {
    /// Tick colour; `None` is the plot's colour.
    pub color: Option<Color>,
    pub thickness_pt: f64,
    /// Tick length, percent of the baseline spacing.
    pub size_pct: f64,
    /// Space between the baseline and the ticks, percent of the baseline spacing.
    pub gap_pct: f64,
}

impl Default for RidgeRug {
    fn default() -> RidgeRug {
        RidgeRug { color: None, thickness_pt: 0.5, size_pct: 10.0, gap_pct: 3.0 }
    }
}

/// The settings of a ridgeline plot (graphing spec 14.23). Every field is skipped when written
/// while it has its default, so older files load unchanged.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RidgelineStyle {
    pub display: RidgeDisplay,
    pub bandwidth: RidgeBandwidth,
    /// Vertical lines from the baseline to the curve at the percentiles in `quantiles`.
    pub quantile_lines: bool,
    /// Percentiles, 0 to 100.
    pub quantiles: Vec<f64>,
    pub rug: RidgeRug,
    /// Height of the tallest ridge's peak, in baseline spacings (Sheaf's choice: 1.5, so ridges overlap).
    pub height: f64,
    /// Transparency of the fill, 0 to 100 percent (Sheaf's choice: 30).
    pub fill_transparency_pct: f64,
}

impl Default for RidgelineStyle {
    fn default() -> RidgelineStyle {
        RidgelineStyle {
            display: RidgeDisplay::default(),
            bandwidth: RidgeBandwidth::default(),
            quantile_lines: false,
            quantiles: vec![25.0, 50.0, 75.0],
            rug: RidgeRug::default(),
            height: 1.5,
            fill_transparency_pct: 30.0,
        }
    }
}

impl RidgelineStyle {
    pub fn is_default(&self) -> bool {
        *self == RidgelineStyle::default()
    }
}

/// How the centres of one axis of a hexbin plot are spaced (Origin's Bin Spacing or Number of Bins).
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HexbinBy {
    /// This many centres from Min Center to Max Center.
    Bins(u32),
    /// The distance between neighbouring centres, in axis units.
    Spacing(f64),
}

/// The bin settings of one axis of a hexbin plot (Origin's Min Center, Max Center, and Bin Spacing or
/// Number of Bins); `None` is the minimum or maximum of the data.
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct HexbinAxis {
    pub min_center: Option<f64>,
    pub max_center: Option<f64>,
    pub by: HexbinBy,
}

impl Default for HexbinAxis {
    fn default() -> HexbinAxis {
        HexbinAxis { min_center: None, max_center: None, by: HexbinBy::Bins(sheaf_calc::hexbin::DEFAULT_BINS as u32) }
    }
}

impl HexbinAxis {
    fn to_calc(self) -> sheaf_calc::hexbin::HexAxisSetup {
        use sheaf_calc::hexbin::{HexAxisSetup, HexStep};
        HexAxisSetup {
            min_center: self.min_center,
            max_center: self.max_center,
            step: match self.by {
                HexbinBy::Bins(n) => HexStep::Bins(n as usize),
                HexbinBy::Spacing(s) => HexStep::Spacing(s),
            },
        }
    }
}

/// The settings of a hexbin plot (graphing spec 14.24). Every field is skipped when written while it
/// has its default, so older files load unchanged.
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct HexbinStyle {
    pub x: HexbinAxis,
    pub y: HexbinAxis,
    /// Origin's "Prefer Regular Hexagons" (checked by default).
    pub prefer_regular: bool,
    /// Origin's "Derive Y from X" (the default); false is "Derive X from Y".
    pub derive_y_from_x: bool,
}

impl Default for HexbinStyle {
    fn default() -> HexbinStyle {
        HexbinStyle { x: HexbinAxis::default(), y: HexbinAxis::default(), prefer_regular: true, derive_y_from_x: true }
    }
}

impl HexbinStyle {
    pub fn is_default(&self) -> bool {
        *self == HexbinStyle::default()
    }

    /// The setting the binning takes, for a frame of `frame_aspect` (width / height on the page).
    pub fn to_calc(&self, frame_aspect: f64) -> sheaf_calc::hexbin::HexSetup {
        sheaf_calc::hexbin::HexSetup {
            x: self.x.to_calc(),
            y: self.y.to_calc(),
            derive_y_from_x: self.derive_y_from_x,
            prefer_regular: self.prefer_regular,
            frame_aspect,
        }
    }
}

/// The settings shared by the plots (one per axis) of a Parallel Plot or Parallel Index (graphing spec
/// 14.25). Every field is skipped when written while it has its default, so older files load unchanged.
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ParallelStyle {
    /// Origin's "Rescale Each Axis Independently": each axis has its own range (the default, Sheaf's choice);
    /// cleared, every axis takes one range, the union of the ranges of all the columns.
    pub rescale_each_axis: bool,
    /// Width of the lines, in pt (Origin's Plot Details Parallel tab, Width; the default is Sheaf's choice).
    pub width_pt: f64,
    /// Transparency of the lines, 0 (opaque) to 100 percent (Origin's Transparency).
    pub transparency_pct: f64,
    /// A Parallel Plot: the numeric column that colours the lines through the colour ramp, with a colour
    /// scale (`None` draws every line in the plot colour). A Parallel Index: the column whose text groups
    /// the rows, one colour and one legend entry per group.
    pub color: Option<ColumnId>,
}

impl Default for ParallelStyle {
    fn default() -> ParallelStyle {
        ParallelStyle { rescale_each_axis: true, width_pt: 0.5, transparency_pct: 0.0, color: None }
    }
}

impl ParallelStyle {
    pub fn is_default(&self) -> bool {
        *self == ParallelStyle::default()
    }
}

/// How the siblings of a treemap level are ordered (Plot Details Layout tab, Order; graphing spec 14.26).
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TreemapOrder {
    /// The order the categories first appear in the worksheet (Sheaf's default).
    #[default]
    Categorical,
    Alphabetical,
    /// Largest first.
    BySize,
    /// By the cells of a column, smallest first (the column is [`TreemapStyle::order_column`]).
    ByColumn,
}

/// The layout and order of one treemap level below the first.
#[derive(Copy, Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TreemapLevel {
    pub layout: TreemapLayout,
    pub order: TreemapOrder,
}

pub use sheaf_calc::treemap::Layout as TreemapLayout;

/// The error bars of a grouped column plot (T76).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IndexedError {
    /// No error bars (Sheaf's default; Origin's default is undocumented).
    #[default]
    None,
    /// The sample standard deviation (n - 1) of the rows of a bar.
    Sd,
    /// The standard error of the mean of the rows of a bar.
    Se,
}

/// Indexed data on a column or box plot (T76, Origin's plot_gindexed and plot_gboxindexed): the plot's Y column
/// holds all the values, and the rows are split by the levels of `group` (the categories on the X axis) and, in
/// one plot per level, by a second column.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Indexed {
    /// The main group column: one bar or box per level, in sorted order at 1, 2, ...
    pub group: ColumnId,
    /// The second group column and the level this plot shows (the plot's legend text is the level).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subset: Option<(ColumnId, String)>,
    /// Error bars of a grouped column plot.
    #[serde(default, skip_serializing_if = "is_default")]
    pub error: IndexedError,
}

/// The error bar of an interval plot point (T83).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntervalError {
    /// The sample standard deviation (n - 1).
    Sd,
    /// The standard error of the mean.
    Se,
    /// The Student-t confidence interval of the mean (Sheaf's default, as in the Graphical Summary).
    #[default]
    Ci,
}

/// A plot whose points are computed from its Y column's cells instead of read from them (T83).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatPlot {
    /// Interval Plot: the one point of this column is its mean at the next position 1, 2, ..., with an error bar
    /// (`level` is the confidence level in percent, used by [`IntervalError::Ci`]).
    Interval { error: IntervalError, level: f64 },
    /// Pareto chart, the bars: the cells of the Y column (text or numbers) counted per distinct value, in
    /// descending order of count.
    Pareto,
    /// Pareto chart, the curve on the right axis: the running percent of the counted cells.
    ParetoCumulative,
    /// Histogram + Probabilities, the curve on the right axis (T85): the cumulative percent of the
    /// histogram bins of the Y column (the bins of this plot's `hist`), flat across each bin at the
    /// percent reached at its right edge and rising at the bin edges.
    HistogramCumulative,
}

/// The settings of a treemap plot (Origin 2025b Treemap plot, graphing spec 14.26). The plot's own Y column is
/// the value column; the category columns are named here, the first the root level. Every field is skipped when
/// written while it has its default, so older files load unchanged.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TreemapStyle {
    /// The category columns, first = top level.
    pub categories: Vec<ColumnId>,
    pub layout: TreemapLayout,
    pub order: TreemapOrder,
    /// The column Order by column sorts by (a treemap without one falls back to the categorical order).
    pub order_column: Option<ColumnId>,
    /// The levels shown; `None` is Origin's Infinite (every level).
    pub depth: Option<u32>,
    /// The gap between rectangles, in pt (Sheaf's default 1.5).
    pub gap_pt: f64,
    /// Use `per_level` for the levels below the first (Origin's per-level check box).
    pub per_level_on: bool,
    /// Layout and order of level 2, 3, ...; a level without an entry takes the general layout and order.
    pub per_level: Vec<TreemapLevel>,
}

impl Default for TreemapStyle {
    fn default() -> TreemapStyle {
        TreemapStyle {
            categories: Vec::new(),
            layout: TreemapLayout::Squarified,
            order: TreemapOrder::Categorical,
            order_column: None,
            depth: None,
            gap_pt: 1.5,
            per_level_on: false,
            per_level: Vec::new(),
        }
    }
}

impl TreemapStyle {
    pub fn is_default(&self) -> bool {
        *self == TreemapStyle::default()
    }

    /// The layout and order of level `level` (0 = top).
    pub fn level_setting(&self, level: usize) -> TreemapLevel {
        if level > 0
            && self.per_level_on
            && let Some(l) = self.per_level.get(level - 1)
        {
            return *l;
        }
        TreemapLevel { layout: self.layout, order: self.order }
    }
}

/// How the wedges of a sunburst ring are ordered (Plot Details Order; graphing spec 14.27).
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SunburstOrder {
    /// The order the categories first appear in the worksheet (Sheaf's default).
    #[default]
    Worksheet,
    /// Largest first.
    BySize,
}

fn start_90() -> f64 {
    90.0
}

fn hole_30() -> f64 {
    0.3
}

fn border_half() -> f64 {
    0.5
}

/// The settings of a sunburst plot (Origin 2022 Sunburst plot, graphing spec 14.27). The plot's own Y column is the
/// value column; the category columns are named here, the first the innermost ring. Every field is skipped when
/// written while it has its default, so older files load unchanged.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SunburstStyle {
    /// The category columns, first = innermost ring.
    pub categories: Vec<ColumnId>,
    /// Where the first wedge starts, in degrees counter-clockwise from 3 o'clock like Origin's pie rotation (default
    /// 90: 12 o'clock); the wedges follow clockwise.
    #[serde(default = "start_90")]
    pub start_angle_deg: f64,
    /// The hole in the middle as a fraction of the outer radius (0 to 0.9; default 0.3).
    #[serde(default = "hole_30")]
    pub hole: f64,
    /// The gap between rings, in pt (default 0).
    pub ring_gap_pt: f64,
    /// The width of the border round each wedge, in pt (white; default 0.5).
    #[serde(default = "border_half")]
    pub border_pt: f64,
    /// Write the category names in the wedges that have room (default on).
    #[serde(default = "yes")]
    pub show_names: bool,
    /// Write the values in the wedges that have room (default off).
    pub show_values: bool,
    pub order: SunburstOrder,
}

impl Default for SunburstStyle {
    fn default() -> SunburstStyle {
        SunburstStyle {
            categories: Vec::new(),
            start_angle_deg: 90.0,
            hole: 0.3,
            ring_gap_pt: 0.0,
            border_pt: 0.5,
            show_names: true,
            show_values: false,
            order: SunburstOrder::Worksheet,
        }
    }
}

impl SunburstStyle {
    pub fn is_default(&self) -> bool {
        *self == SunburstStyle::default()
    }
}
