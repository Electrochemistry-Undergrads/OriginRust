//! Straight lines at an axis value (T67, graphing spec "Straight lines"): Origin's Graph > Add
//! Straight Line. A vertical line at an X value or a horizontal one at a Y value, attached to the
//! layer's scales, with a label that shows the value.

use crate::Color;
use crate::graph::{Dash, Font};
use serde::{Deserialize, Serialize};

/// Where the value label stands relative to the line (Origin's Label Location).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum StraightLineLabel {
    #[default]
    Top,
    Bottom,
    Left,
    Right,
}

impl StraightLineLabel {
    pub const ALL: [StraightLineLabel; 4] =
        [StraightLineLabel::Top, StraightLineLabel::Bottom, StraightLineLabel::Left, StraightLineLabel::Right];

    pub fn name(self) -> &'static str {
        match self {
            StraightLineLabel::Top => "Top",
            StraightLineLabel::Bottom => "Bottom",
            StraightLineLabel::Left => "Left",
            StraightLineLabel::Right => "Right",
        }
    }

    /// Reads "top", "bottom", "left" or "right" (any case).
    pub fn parse(s: &str) -> Option<Self> {
        StraightLineLabel::ALL.into_iter().find(|l| l.name().eq_ignore_ascii_case(s.trim()))
    }
}

/// A straight line of a layer, vertical at an X value or horizontal at a Y value.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StraightLine {
    /// True for a vertical line (`value` is an X value), false for a horizontal one (a Y value).
    pub vertical: bool,
    /// Where the line is, in data units of the axis it crosses.
    pub value: f64,
    /// The extent along the line, in data units of the other axis. `None` runs across the whole frame.
    /// Origin fixes the ends when the line is made, so the dialog stores the axis range of that moment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub span: Option<(f64, f64)>,
    #[serde(default = "black")]
    pub color: Color,
    #[serde(default = "default_width")]
    pub width_pt: f64,
    /// The eight line styles of Origin all exist as [`Dash`].
    #[serde(default)]
    pub dash: Dash,
    #[serde(default = "yes")]
    pub show_label: bool,
    /// Label Display Format: `None` is the axis' tick label format, `Some(".2")` two fixed decimals.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(default)]
    pub location: StraightLineLabel,
    /// `None` is the layer's new-label font ([`crate::graph::Layer::new_label_font`]) in the line's colour.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font: Option<Font>,
    /// Origin's object name: `vline`, `vline1`, ... or `hline`, `hline1`, ...
    #[serde(default)]
    pub name: String,
    #[serde(default = "yes")]
    pub selectable: bool,
    #[serde(default = "yes")]
    pub movable: bool,
}

fn black() -> Color {
    Color::BLACK
}
fn default_width() -> f64 {
    0.5
}
fn yes() -> bool {
    true
}

impl StraightLine {
    /// A line with Origin's defaults: black, 0.5 pt, solid, label on top showing the value.
    pub fn new(vertical: bool, value: f64) -> Self {
        StraightLine {
            vertical,
            value,
            span: None,
            color: Color::BLACK,
            width_pt: 0.5,
            dash: Dash::Solid,
            show_label: true,
            format: None,
            location: StraightLineLabel::Top,
            font: None,
            name: String::new(),
            selectable: true,
            movable: true,
        }
    }

    /// The first free Origin name among `lines`: `vline`, `vline1`, `vline2`, ... or `hline`, ...
    pub fn next_name(lines: &[StraightLine], vertical: bool) -> String {
        let base = if vertical { "vline" } else { "hline" };
        let taken = |n: &str| lines.iter().any(|l| l.name == n);
        if !taken(base) {
            return base.to_string();
        }
        (1..).map(|k| format!("{base}{k}")).find(|n| !taken(n)).unwrap_or_default()
    }

    /// The number of fixed decimals of a format like ".2" (a bare "2" is accepted too); `None` for
    /// the axis' own format or text that is not a decimal count.
    pub fn fixed_decimals(&self) -> Option<usize> {
        let f = self.format.as_deref()?.trim();
        f.strip_prefix('.').unwrap_or(f).parse::<usize>().ok().map(|d| d.min(15))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::Layer;

    #[test]
    fn defaults_are_origins() {
        let l = StraightLine::new(true, 3.0);
        assert!(l.vertical && l.value == 3.0 && l.span.is_none());
        assert_eq!((l.color, l.width_pt, l.dash), (Color::BLACK, 0.5, Dash::Solid));
        assert!(l.show_label && l.selectable && l.movable);
        assert_eq!(l.location, StraightLineLabel::Top);
    }

    #[test]
    fn names_count_up_per_orientation() {
        let mut v = Vec::new();
        assert_eq!(StraightLine::next_name(&v, true), "vline");
        v.push(StraightLine { name: "vline".into(), ..StraightLine::new(true, 1.0) });
        assert_eq!(StraightLine::next_name(&v, true), "vline1");
        assert_eq!(StraightLine::next_name(&v, false), "hline");
        v.push(StraightLine { name: "vline1".into(), ..StraightLine::new(true, 1.0) });
        v.push(StraightLine { name: "hline".into(), ..StraightLine::new(false, 1.0) });
        assert_eq!(StraightLine::next_name(&v, true), "vline2");
        assert_eq!(StraightLine::next_name(&v, false), "hline1");
        v.remove(0);
        assert_eq!(StraightLine::next_name(&v, true), "vline");
    }

    #[test]
    fn format_accepts_dot_n() {
        let mut l = StraightLine::new(false, 1.0);
        assert_eq!(l.fixed_decimals(), None);
        l.format = Some(".2".into());
        assert_eq!(l.fixed_decimals(), Some(2));
        l.format = Some("3".into());
        assert_eq!(l.fixed_decimals(), Some(3));
        l.format = Some("abc".into());
        assert_eq!(l.fixed_decimals(), None);
    }

    #[test]
    fn label_location_parses() {
        assert_eq!(StraightLineLabel::parse("Bottom"), Some(StraightLineLabel::Bottom));
        assert_eq!(StraightLineLabel::parse(" left "), Some(StraightLineLabel::Left));
        assert_eq!(StraightLineLabel::parse("middle"), None);
    }

    #[test]
    fn round_trips_and_old_files_load_without_the_field() {
        let mut s = StraightLine::new(false, -2.5);
        s.span = Some((1.0, 9.0));
        s.color = Color::rgb(1, 2, 3);
        s.dash = Dash::DashDot;
        s.format = Some(".2".into());
        s.location = StraightLineLabel::Right;
        s.font = Some(Font::new("Arial", 12.0));
        s.name = "hline".into();
        s.movable = false;
        let back: StraightLine = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        assert_eq!(back, s);
        let min: StraightLine = serde_json::from_str(r#"{"vertical":true,"value":1.5}"#).unwrap();
        assert_eq!(min, StraightLine::new(true, 1.5));

        let mut p = crate::Project::new();
        let g = p.add_graph(crate::template::Template::Scatter);
        let layer = &mut p.graph_mut(g).unwrap().layers[0];
        let json = serde_json::to_string(layer).unwrap();
        assert!(!json.contains("straight_lines"));
        assert!(serde_json::from_str::<Layer>(&json).unwrap().straight_lines.is_empty());
        layer.straight_lines.push(s);
        let back: Layer = serde_json::from_str(&serde_json::to_string(layer).unwrap()).unwrap();
        assert_eq!(back.straight_lines, layer.straight_lines);
    }
}
