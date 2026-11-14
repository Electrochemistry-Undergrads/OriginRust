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
}

impl Default for PageStyle {
    fn default() -> Self {
        PageStyle { border: None, layer_icons: true, title_display: TitleDisplay::Both, comments: String::new() }
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
