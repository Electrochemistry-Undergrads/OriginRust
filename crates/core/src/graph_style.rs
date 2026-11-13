//! The look settings that Plot Details adds to a plot, a layer and a page (brief T13): fill
//! patterns and gradients, drop lines, point labels, the increment rules of a plot group, a
//! layer's background and display switches, and the page's units, border and titles.
//!
//! Every type has a default that means "as before", and every field is skipped when written while
//! it has that default, so a file written before these settings existed loads unchanged and a
//! file that does not use them looks as it did. Which of the rules are Origin's own is in the
//! graphing spec, section 18; what was not measured is said there.

use crate::color::Color;
use crate::graph::{Dash, Font, LineStyle, SymbolShape};
use crate::ids::ColumnId;
use serde::{Deserialize, Serialize};

fn is_default<T: Default + PartialEq>(v: &T) -> bool {
    *v == T::default()
}

fn is_zero_pct(v: &u8) -> bool {
    *v == 0
}

fn yes() -> bool {
    true
}

fn is_true(v: &bool) -> bool {
    *v
}

fn is_false(v: &bool) -> bool {
    !*v
}

/// The share of a colour that stays visible for a transparency in percent (0 to 100): 100 % hides
/// the colour, 0 % leaves it as it is.
pub fn opacity_for(transparency_pct: u8) -> f64 {
    1.0 - f64::from(transparency_pct.min(100)) / 100.0
}

// ----- fills ---------------------------------------------------------------------------------

/// A hatch drawn over the fill of a bar or an area.
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
/// through it is, a hatch over it, a gradient. The flat colour itself is the plot's symbol colour.
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FillStyle {
    /// Transparency of the fill in percent (0 solid, 100 invisible).
    #[serde(default, skip_serializing_if = "is_zero_pct")]
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
}

// ----- drop lines ---------------------------------------------------------------------------

/// Lines from every data point to the axes (Origin's Drop Lines tab): horizontal ones to the Y
/// axis, vertical ones to the X axis. `None` draws no such line.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct DropLines {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub horizontal: Option<LineStyle>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vertical: Option<LineStyle>,
}

impl DropLines {
    /// The style a drop line starts with when switched on: dashed, thin, in the plot's colour.
    pub fn starting_style(color: Color) -> LineStyle {
        LineStyle { dash: Dash::Dash, ..LineStyle::solid(color, 0.5) }
    }
}

// ----- point labels -------------------------------------------------------------------------

/// What a data label shows.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum LabelForm {
    /// The point's Y value.
    #[default]
    Y,
    /// The point's X value.
    X,
    /// The row number, counted from 1.
    Row,
    /// The cell of the same row in this column (text or numbers as the worksheet shows them).
    Column { column: ColumnId },
}

/// Where a data label stands against its point.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum LabelPosition {
    #[default]
    Right,
    Left,
    Above,
    Below,
    Center,
}

impl LabelPosition {
    pub const ALL: [LabelPosition; 5] =
        [LabelPosition::Right, LabelPosition::Left, LabelPosition::Above, LabelPosition::Below, LabelPosition::Center];

    pub fn name(&self) -> &'static str {
        match self {
            LabelPosition::Right => "Right",
            LabelPosition::Left => "Left",
            LabelPosition::Above => "Above",
            LabelPosition::Below => "Below",
            LabelPosition::Center => "Center",
        }
    }
}

/// Default size of a data label, in points.
pub const DEFAULT_POINT_LABEL_PT: f64 = 9.0;

fn default_label_font() -> Font {
    Font::new("Arial", DEFAULT_POINT_LABEL_PT)
}

/// Data labels of a plot (Origin's Label tab): a short text beside each point.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PointLabels {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub form: LabelForm,
    #[serde(default)]
    pub position: LabelPosition,
    #[serde(default = "default_label_font")]
    pub font: Font,
    /// Counter-clockwise rotation of the text in degrees.
    #[serde(default, skip_serializing_if = "is_default_f64")]
    pub rotation_deg: f64,
    /// Extra shift of the text to the right and down, in points (added to the position).
    #[serde(default, skip_serializing_if = "is_default_f64")]
    pub dx_pt: f64,
    #[serde(default, skip_serializing_if = "is_default_f64")]
    pub dy_pt: f64,
    /// Which rows are labelled, as a list of row numbers and ranges counted from 1 ("1,3,5-8");
    /// empty labels every row.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub only_rows: String,
    /// Decimal places of a number; `None` writes as few digits as the value needs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decimals: Option<u8>,
}

fn is_default_f64(v: &f64) -> bool {
    *v == 0.0
}

impl Default for PointLabels {
    fn default() -> Self {
        PointLabels {
            enabled: false,
            form: LabelForm::Y,
            position: LabelPosition::Right,
            font: default_label_font(),
            rotation_deg: 0.0,
            dx_pt: 0.0,
            dy_pt: 0.0,
            only_rows: String::new(),
            decimals: None,
        }
    }
}

impl PointLabels {
    /// True when row `row` (counted from 0) is labelled: every row without a list, otherwise the
    /// rows the list names. An entry that is not a number or a range is ignored.
    pub fn labels_row(&self, row: usize) -> bool {
        parse_row_list(&self.only_rows).is_none_or(|ranges| ranges.iter().any(|&(a, b)| (a..=b).contains(&(row + 1))))
    }
}

/// The ranges (counted from 1) a list like `1,3,5-8` names; `None` for an empty list or one with no
/// valid entry (all rows then count).
pub fn parse_row_list(text: &str) -> Option<Vec<(usize, usize)>> {
    let mut out = Vec::new();
    for part in text.split([',', ';', ' ']).map(str::trim).filter(|p| !p.is_empty()) {
        let range = match part.split_once('-') {
            Some((a, b)) => a.trim().parse::<usize>().ok().zip(b.trim().parse::<usize>().ok()),
            None => part.parse::<usize>().ok().map(|n| (n, n)),
        };
        if let Some((a, b)) = range
            && a >= 1
            && b >= a
        {
            out.push((a, b));
        }
    }
    (!out.is_empty()).then_some(out)
}

// ----- plot ---------------------------------------------------------------------------------

/// The settings of a plot that Plot Details adds to its line, symbol and error bars.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct PlotDetails {
    /// Transparency of the line in percent (0 solid, 100 invisible).
    #[serde(default, skip_serializing_if = "is_zero_pct")]
    pub line_transparency_pct: u8,
    /// A line plot fills the area between its curve and Y = 0 (Origin's Fill Area Under Curve), in
    /// the plot's symbol colour and `fill`.
    #[serde(default, skip_serializing_if = "is_false")]
    pub fill_under: bool,
    /// Transparency, pattern and gradient of the fill of a bar, an area, or the area under a curve.
    #[serde(default, skip_serializing_if = "FillStyle::is_plain_default")]
    pub fill: FillStyle,
    #[serde(default, skip_serializing_if = "is_default")]
    pub drop_lines: DropLines,
    #[serde(default, skip_serializing_if = "is_default")]
    pub labels: PointLabels,
}

impl FillStyle {
    fn is_plain_default(&self) -> bool {
        *self == FillStyle::default()
    }
}

impl PlotDetails {
    pub fn is_default(&self) -> bool {
        *self == PlotDetails::default()
    }
}

// ----- plot group ---------------------------------------------------------------------------

/// How a change to one member of a plot group reaches the others (Origin's Edit Mode).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum EditMode {
    /// The members follow the increment rules: changing the line style or symbol of one moves the
    /// others with it, and plots added or removed are styled again.
    #[default]
    Dependent,
    /// Nothing is styled again by the rules (only "Recolor plots" does it); every member keeps
    /// the style it has and changes one plot only.
    Independent,
}

/// How a property steps from one member of a group to the next.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Increment {
    /// No step: every plot keeps the value it has (Origin's "None" makes all members equal; here a
    /// plot that was given its own value keeps it).
    #[default]
    None,
    /// Member k takes the value k places after the first member's in the list.
    ByOne,
}

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

/// The increment rules of the plots of a layer that follow the palette (the plots whose
/// [`crate::graph::Plot::auto_color`] is set are the group's members). The colours step through
/// the layer's palette (`Palette::mode`: one colour per plot, a gradient, or no step at all).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct GroupRules {
    #[serde(default)]
    pub edit_mode: EditMode,
    #[serde(default)]
    pub line_style: Increment,
    #[serde(default)]
    pub symbol_shape: Increment,
    #[serde(default)]
    pub subgrouping: Subgrouping,
    #[serde(default)]
    pub subgroup_on: SubgroupOn,
}

impl GroupRules {
    pub fn is_default(&self) -> bool {
        *self == GroupRules::default()
    }

    /// The steps `(colour, line style, symbol shape)` of member `k` of `n`, and the number of steps
    /// the colour list runs over. Without subgroups every property steps with the member number; with
    /// subgroups of `s` plots the property of [`GroupRules::subgroup_on`] steps with the subgroup
    /// number and the others with the place inside the subgroup.
    pub fn steps(&self, k: usize, n: usize) -> ((usize, usize), usize, usize) {
        match self.subgrouping {
            Subgrouping::None => ((k, n), k, k),
            Subgrouping::BySize { size } => {
                let s = (size as usize).max(1);
                let (g, j) = (k / s, k % s);
                let groups = n.div_ceil(s);
                let within = n.min(s);
                match self.subgroup_on {
                    SubgroupOn::Color => ((g, groups), j, j),
                    SubgroupOn::LineStyle => ((j, within), g, j),
                    SubgroupOn::SymbolShape => ((j, within), j, g),
                }
            }
        }
    }

    /// The dash of a member that steps `step` places after `base` in the list of line styles.
    pub fn dash_at(&self, base: Dash, step: usize) -> Dash {
        let i = Dash::ALL.iter().position(|d| *d == base).unwrap_or(0);
        Dash::ALL[(i + step) % Dash::ALL.len()]
    }

    /// The symbol shape of a member that steps `step` places after `base` in the list of shapes.
    pub fn shape_at(&self, base: SymbolShape, step: usize) -> SymbolShape {
        let i = SymbolShape::ALL.iter().position(|s| *s == base).unwrap_or(0);
        SymbolShape::ALL[(i + step) % SymbolShape::ALL.len()]
    }
}

// ----- layer --------------------------------------------------------------------------------

/// What a layer shows (the Display tab of a layer in Plot Details). Everything is shown by default.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayerDisplay {
    /// False draws nothing of the layer: no plots, axes, legend, labels or drawn objects. The layer
    /// keeps its place, so layers linked to it do not move.
    #[serde(default = "yes", skip_serializing_if = "is_true")]
    pub show_layer: bool,
    #[serde(default = "yes", skip_serializing_if = "is_true")]
    pub show_plots: bool,
    /// The axis lines, ticks, tick labels and titles.
    #[serde(default = "yes", skip_serializing_if = "is_true")]
    pub show_axes: bool,
    /// The free text labels and the drawn objects.
    #[serde(default = "yes", skip_serializing_if = "is_true")]
    pub show_labels: bool,
}

impl Default for LayerDisplay {
    fn default() -> Self {
        LayerDisplay { show_layer: true, show_plots: true, show_axes: true, show_labels: true }
    }
}

impl LayerDisplay {
    pub fn is_default(&self) -> bool {
        *self == LayerDisplay::default()
    }
}

/// The area behind a layer's plots (the Background tab): a colour, optionally with a gradient, and
/// a border line around the layer's frame. Nothing is drawn by default.
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct LayerBackground {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<Color>,
    #[serde(default, skip_serializing_if = "is_zero_pct")]
    pub transparency_pct: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gradient: Option<Gradient>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border: Option<BorderStyle>,
}

impl LayerBackground {
    pub fn is_default(&self) -> bool {
        *self == LayerBackground::default()
    }
}

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

// ----- page ---------------------------------------------------------------------------------

/// The unit the page and layer sizes are shown in.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum LengthUnit {
    #[default]
    Inch,
    Centimetre,
    Millimetre,
    Point,
}

impl LengthUnit {
    pub const ALL: [LengthUnit; 4] =
        [LengthUnit::Inch, LengthUnit::Centimetre, LengthUnit::Millimetre, LengthUnit::Point];

    pub fn name(&self) -> &'static str {
        match self {
            LengthUnit::Inch => "inch",
            LengthUnit::Centimetre => "cm",
            LengthUnit::Millimetre => "mm",
            LengthUnit::Point => "pt",
        }
    }

    /// Points in one of the unit.
    pub fn points(&self) -> f64 {
        match self {
            LengthUnit::Inch => 72.0,
            LengthUnit::Centimetre => 72.0 / 2.54,
            LengthUnit::Millimetre => 72.0 / 25.4,
            LengthUnit::Point => 1.0,
        }
    }

    pub fn from_pt(&self, pt: f64) -> f64 {
        pt / self.points()
    }

    pub fn to_pt(&self, v: f64) -> f64 {
        v * self.points()
    }
}

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
    /// The unit of the page and layer sizes in Plot Details.
    #[serde(default, skip_serializing_if = "is_default")]
    pub units: LengthUnit,
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
        PageStyle {
            units: LengthUnit::Inch,
            border: None,
            layer_icons: true,
            title_display: TitleDisplay::Both,
            comments: String::new(),
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
    fn defaults_write_nothing() {
        assert_eq!(serde_json::to_string(&PlotDetails::default()).unwrap(), "{}");
        assert_eq!(serde_json::to_string(&LayerDisplay::default()).unwrap(), "{}");
        assert_eq!(serde_json::to_string(&LayerBackground::default()).unwrap(), "{}");
        assert_eq!(serde_json::to_string(&PageStyle::default()).unwrap(), "{}");
        assert_eq!(
            serde_json::to_string(&GroupRules::default()).unwrap(),
            r#"{"edit_mode":"dependent","line_style":"none","symbol_shape":"none","subgrouping":{"kind":"none"},"subgroup_on":"color"}"#
        );
    }

    #[test]
    fn plot_details_round_trip() {
        let mut d = PlotDetails::default();
        d.line_transparency_pct = 40;
        d.fill_under = true;
        d.fill.pattern = FillPattern::Cross;
        d.fill.transparency_pct = 25;
        d.fill.gradient = Some(Gradient { color: Color::RED, direction: GradientDirection::BottomToTop });
        d.drop_lines.vertical = Some(DropLines::starting_style(Color::BLUE));
        d.labels.enabled = true;
        d.labels.form = LabelForm::Column { column: ColumnId(4) };
        d.labels.only_rows = "1,3-5".into();
        let json = serde_json::to_string(&d).unwrap();
        assert_eq!(serde_json::from_str::<PlotDetails>(&json).unwrap(), d);
        assert!(json.contains(r#""form":{"kind":"column","column":4}"#), "{json}");
    }

    #[test]
    fn a_row_list_names_rows_and_ranges() {
        let l = PointLabels { only_rows: "1, 3, 5-7".into(), ..Default::default() };
        let hit: Vec<usize> = (0..10).filter(|&r| l.labels_row(r)).collect();
        assert_eq!(hit, vec![0, 2, 4, 5, 6]);
        // Nothing valid, or nothing: every row.
        for text in ["", "x", "0", "5-3"] {
            let l = PointLabels { only_rows: text.into(), ..Default::default() };
            assert!((0..4).all(|r| l.labels_row(r)), "{text:?}");
        }
    }

    #[test]
    fn units_convert_both_ways() {
        for unit in LengthUnit::ALL {
            assert!((unit.to_pt(unit.from_pt(100.0)) - 100.0).abs() < 1e-9);
        }
        assert!((LengthUnit::Inch.to_pt(1.0) - 72.0).abs() < 1e-12);
        assert!((LengthUnit::Centimetre.from_pt(72.0) - 2.54).abs() < 1e-12);
        assert!((LengthUnit::Millimetre.from_pt(72.0) - 25.4).abs() < 1e-12);
    }

    #[test]
    fn group_steps_follow_the_subgrouping() {
        let by_one = GroupRules::default();
        assert_eq!(by_one.steps(3, 5), ((3, 5), 3, 3));
        let sub = GroupRules {
            subgrouping: Subgrouping::BySize { size: 2 },
            subgroup_on: SubgroupOn::LineStyle,
            ..Default::default()
        };
        // Plots 0..5: subgroups {0,1} {2,3} {4}; the line style steps per subgroup, colours inside.
        assert_eq!(sub.steps(0, 5), ((0, 2), 0, 0));
        assert_eq!(sub.steps(3, 5), ((1, 2), 1, 1));
        assert_eq!(sub.steps(4, 5), ((0, 2), 2, 0));
    }
}
