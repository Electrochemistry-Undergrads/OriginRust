//! Worksheet > Conditional Formatting > Conditional Heatmap: every number of the range gets the
//! colour of the level its value falls in. The options follow Origin's documented dialog (Level:
//! Auto Rescale, From/To, scale Type, By Count or By Increment, First Level; Colors: Format Type,
//! Contrast Color for Text, Limited Mixing or 3-Color Limited Mixing with a Middle Position, Flip,
//! Below and Above). How a level's colour is mixed, the defaults and the level bounds are Sheaf's
//! own choices and unmeasured (Origin 2016 has no conditional formatting; docs/spec/gui-look.md).

use crate::color::Color;
use serde::{Deserialize, Serialize};

/// The most levels a heatmap has (each is one palette class of the rule).
pub const MAX_LEVELS: usize = 250;
/// Sheaf's defaults (Origin's are not documented).
pub const DEFAULT_LEVELS: u32 = 10;
pub const DEFAULT_FROM: Color = Color::rgb(255, 255, 255);
pub const DEFAULT_MIDDLE: Color = Color::rgb(255, 235, 132);
pub const DEFAULT_TO: Color = Color::rgb(248, 105, 107);

/// The scale type of the levels.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum LevelScale {
    #[default]
    Linear,
    /// Levels evenly spaced in log10 of the value; numbers at or below zero get no colour.
    Log10,
}

impl LevelScale {
    fn map(self, v: f64) -> Option<f64> {
        let t = match self {
            LevelScale::Linear => v,
            LevelScale::Log10 => (v > 0.0).then(|| v.log10())?,
        };
        t.is_finite().then_some(t)
    }
}

/// How the levels are spaced: a number of levels, or a step in the scale's units (one decade is 1
/// on a Log10 scale).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum LevelStep {
    Count(u32),
    Increment(f64),
}

/// Where the middle colour of a 3-colour mixing sits.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum MiddlePosition {
    /// The given percentile (0 to 100) of the range's numbers.
    Percentile(f64),
    /// A percent (0 to 100) of the way from the first level to the top.
    Percent(f64),
    /// A value.
    Value(f64),
}

/// Origin's Color Generation choices that Sheaf builds.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Mixing {
    /// Blend linearly from `from` to `to`.
    Limited { from: Color, to: Color },
    /// Blend from `from` to `middle` to `to`.
    ThreeColor { from: Color, middle: Color, to: Color, position: MiddlePosition },
}

/// What the level colour paints: Origin's Format Type.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Target {
    /// Fill Background.
    #[default]
    Fill,
    /// Color Text.
    Text,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Heatmap {
    /// The lowest and highest level; `None` is Auto Rescale (the range's minimum and maximum).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limits: Option<(f64, f64)>,
    #[serde(default)]
    pub scale: LevelScale,
    pub step: LevelStep,
    /// The first level's value; `None` is the lowest level. Values below it take the first colour.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_level: Option<f64>,
    #[serde(default)]
    pub target: Target,
    /// With Fill Background: black or white text, whichever contrasts with the fill.
    #[serde(default)]
    pub contrast_text: bool,
    pub mixing: Mixing,
    /// Reverses the colour order (From and To swap).
    #[serde(default)]
    pub flip: bool,
    /// Colours of values below From and above To (only with set limits).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub below: Option<Color>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub above: Option<Color>,
}

impl Default for Heatmap {
    fn default() -> Self {
        Heatmap {
            limits: None,
            scale: LevelScale::Linear,
            step: LevelStep::Count(DEFAULT_LEVELS),
            first_level: None,
            target: Target::Fill,
            contrast_text: true,
            mixing: Mixing::Limited { from: DEFAULT_FROM, to: DEFAULT_TO },
            flip: false,
            below: None,
            above: None,
        }
    }
}

/// Where a value falls.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    Level(usize),
    Below,
    Above,
}

/// The levels of a heatmap on a set of numbers, in scale units.
#[derive(Clone, Debug, PartialEq)]
pub struct Levels {
    pub first: f64,
    pub step: f64,
    pub count: usize,
    pub low: f64,
    pub high: f64,
    /// The middle colour's place, 0 to 1 along the levels (3-colour mixing only).
    pub middle: f64,
}

/// The black or white that reads best on `fill` (ITU-R BT.601 luma).
pub fn contrast(fill: Color) -> Color {
    let luma = 0.299 * f64::from(fill.r) + 0.587 * f64::from(fill.g) + 0.114 * f64::from(fill.b);
    if luma < 128.0 { Color::WHITE } else { Color::BLACK }
}

impl Heatmap {
    /// The levels for these numbers (the range's usable values), or `None` when nothing can be
    /// coloured: no number, or limits that are not finite and increasing in the scale.
    pub fn levels(&self, values: &[f64]) -> Option<Levels> {
        let mapped: Vec<f64> = values.iter().filter_map(|v| self.scale.map(*v)).collect();
        let (low, high) = match self.limits {
            Some((a, b)) => (self.scale.map(a)?, self.scale.map(b)?),
            None => {
                let lo = mapped.iter().copied().fold(f64::INFINITY, f64::min);
                let hi = mapped.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                (lo, hi)
            }
        };
        if !(low.is_finite() && high.is_finite()) || low > high || (self.limits.is_some() && low == high) {
            return None;
        }
        let first = self.first_level.and_then(|f| self.scale.map(f)).map_or(low, |f| f.clamp(low, high));
        let span = high - first;
        let (step, count) = match self.step {
            LevelStep::Count(n) => {
                let n = (n.max(1) as usize).min(MAX_LEVELS);
                (if span > 0.0 { span / n as f64 } else { 1.0 }, n)
            }
            LevelStep::Increment(d) if d.is_finite() && d > 0.0 && span > 0.0 => {
                let n = (span / d).ceil();
                (d, if n.is_finite() { (n as usize).clamp(1, MAX_LEVELS) } else { MAX_LEVELS })
            }
            LevelStep::Increment(_) => (1.0, 1),
        };
        let middle = match self.mixing {
            Mixing::Limited { .. } => 0.5,
            Mixing::ThreeColor { position, .. } => {
                let at = |t: f64| if span > 0.0 { (t - first) / span } else { 0.5 };
                let m = match position {
                    MiddlePosition::Percent(p) => p / 100.0,
                    MiddlePosition::Value(v) => self.scale.map(v).map_or(0.5, at),
                    MiddlePosition::Percentile(p) => percentile(&mapped, p).map_or(0.5, at),
                };
                if m.is_finite() { m.clamp(0.0, 1.0) } else { 0.5 }
            }
        };
        Some(Levels { first, step, count, low, high, middle })
    }

    /// Where `value` falls, or `None` when it cannot be placed (not a number of the scale).
    pub fn place(&self, levels: &Levels, value: f64) -> Option<Place> {
        let t = self.scale.map(value)?;
        if t < levels.low {
            return Some(Place::Below);
        }
        if t > levels.high {
            return Some(Place::Above);
        }
        let k = ((t - levels.first) / levels.step).floor();
        let k = if k.is_finite() && k > 0.0 { k as usize } else { 0 };
        Some(Place::Level(k.min(levels.count - 1)))
    }

    /// The colour of level `k`: the levels' colours run evenly from From to To (through Middle).
    pub fn level_color(&self, levels: &Levels, k: usize) -> Color {
        let s = if levels.count <= 1 { 0.0 } else { k as f64 / (levels.count - 1) as f64 };
        let swap = |a: Color, b: Color| if self.flip { (b, a) } else { (a, b) };
        match self.mixing {
            Mixing::Limited { from, to } => {
                let (a, b) = swap(from, to);
                a.lerp(&b, s)
            }
            Mixing::ThreeColor { from, middle, to, .. } => {
                let (a, b) = swap(from, to);
                let m = levels.middle;
                if s <= m {
                    if m <= 0.0 { middle } else { a.lerp(&middle, s / m) }
                } else if m >= 1.0 {
                    middle
                } else {
                    middle.lerp(&b, (s - m) / (1.0 - m))
                }
            }
        }
    }

    /// The cell style of a colour under this heatmap's Format Type.
    pub fn style(&self, color: Color) -> super::Colours {
        match self.target {
            Target::Fill => super::Colours { fill: Some(color), font: self.contrast_text.then(|| contrast(color)) },
            Target::Text => super::Colours { fill: None, font: Some(color) },
        }
    }
}

/// The `p`th percentile (0 to 100, linear between order statistics) of `values`.
fn percentile(values: &[f64], p: f64) -> Option<f64> {
    if values.is_empty() || !p.is_finite() {
        return None;
    }
    let mut v = values.to_vec();
    v.sort_by(f64::total_cmp);
    let pos = (p.clamp(0.0, 100.0) / 100.0) * (v.len() - 1) as f64;
    let (i, frac) = (pos.floor() as usize, pos.fract());
    Some(if i + 1 < v.len() { v[i] + (v[i + 1] - v[i]) * frac } else { v[i] })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn heat(step: LevelStep) -> Heatmap {
        Heatmap { step, ..Heatmap::default() }
    }

    #[test]
    fn count_levels_split_the_data_range_evenly() {
        let h = heat(LevelStep::Count(4));
        let l = h.levels(&[0.0, 10.0, 4.0]).unwrap();
        assert_eq!((l.first, l.step, l.count), (0.0, 2.5, 4));
        assert_eq!(h.place(&l, 0.0), Some(Place::Level(0)));
        assert_eq!(h.place(&l, 2.49), Some(Place::Level(0)));
        assert_eq!(h.place(&l, 2.5), Some(Place::Level(1)));
        // The top value belongs to the last level.
        assert_eq!(h.place(&l, 10.0), Some(Place::Level(3)));
    }

    #[test]
    fn increment_levels_and_first_level() {
        let h = Heatmap { first_level: Some(2.0), ..heat(LevelStep::Increment(3.0)) };
        let l = h.levels(&[0.0, 10.0]).unwrap();
        // 2..5, 5..8, 8..10: three levels; values under the first level take the first colour.
        assert_eq!((l.first, l.count), (2.0, 3));
        assert_eq!(h.place(&l, 1.0), Some(Place::Level(0)));
        assert_eq!(h.place(&l, 8.0), Some(Place::Level(2)));
        // A bad increment is one level.
        assert_eq!(heat(LevelStep::Increment(0.0)).levels(&[0.0, 1.0]).unwrap().count, 1);
        // Levels are capped.
        assert_eq!(heat(LevelStep::Increment(1e-9)).levels(&[0.0, 1.0]).unwrap().count, MAX_LEVELS);
    }

    #[test]
    fn set_limits_put_outside_values_below_and_above() {
        let h = Heatmap { limits: Some((2.0, 8.0)), ..heat(LevelStep::Count(3)) };
        let l = h.levels(&[0.0, 10.0]).unwrap();
        assert_eq!(h.place(&l, 1.0), Some(Place::Below));
        assert_eq!(h.place(&l, 9.0), Some(Place::Above));
        assert_eq!(h.place(&l, 8.0), Some(Place::Level(2)));
        // Limits that are equal, reversed or not numbers colour nothing.
        for limits in [(3.0, 3.0), (5.0, 1.0), (f64::NAN, 1.0)] {
            assert!(Heatmap { limits: Some(limits), ..h.clone() }.levels(&[1.0]).is_none());
        }
        assert!(heat(LevelStep::Count(3)).levels(&[]).is_none());
    }

    #[test]
    fn one_value_is_the_first_level() {
        let h = heat(LevelStep::Count(5));
        let l = h.levels(&[7.0, 7.0]).unwrap();
        assert_eq!(h.place(&l, 7.0), Some(Place::Level(0)));
        assert_eq!(h.level_color(&l, 0), DEFAULT_FROM);
    }

    #[test]
    fn log_scale_spaces_decades_and_skips_non_positive() {
        let h = Heatmap { scale: LevelScale::Log10, ..heat(LevelStep::Count(3)) };
        let l = h.levels(&[1.0, 10.0, 1000.0, -5.0, 0.0]).unwrap();
        assert_eq!((l.low, l.high, l.step), (0.0, 3.0, 1.0));
        assert_eq!(h.place(&l, 10.0), Some(Place::Level(1)));
        assert_eq!(h.place(&l, 0.0), None);
        assert_eq!(h.place(&l, -1.0), None);
    }

    #[test]
    fn limited_mixing_runs_from_from_to_to_and_flips() {
        let h = heat(LevelStep::Count(3));
        let l = h.levels(&[0.0, 3.0]).unwrap();
        assert_eq!(h.level_color(&l, 0), DEFAULT_FROM);
        assert_eq!(h.level_color(&l, 1), DEFAULT_FROM.lerp(&DEFAULT_TO, 0.5));
        assert_eq!(h.level_color(&l, 2), DEFAULT_TO);
        let f = Heatmap { flip: true, ..h };
        assert_eq!(f.level_color(&l, 0), DEFAULT_TO);
        assert_eq!(f.level_color(&l, 2), DEFAULT_FROM);
    }

    #[test]
    fn three_colour_middle_positions() {
        let three = |position| Heatmap {
            mixing: Mixing::ThreeColor { from: Color::BLACK, middle: DEFAULT_MIDDLE, to: Color::WHITE, position },
            ..heat(LevelStep::Count(5))
        };
        let values = [0.0, 1.0, 2.0, 3.0, 4.0, 100.0];
        let h = three(MiddlePosition::Percent(50.0));
        let l = h.levels(&values).unwrap();
        assert_eq!(l.middle, 0.5);
        assert_eq!(h.level_color(&l, 2), DEFAULT_MIDDLE);
        assert_eq!(h.level_color(&l, 0), Color::BLACK);
        assert_eq!(h.level_color(&l, 4), Color::WHITE);
        assert_eq!(three(MiddlePosition::Value(25.0)).levels(&values).unwrap().middle, 0.25);
        // The median of the six numbers is 2.5, 2.5 % of the way to 100.
        assert_eq!(three(MiddlePosition::Percentile(50.0)).levels(&values).unwrap().middle, 0.025);
        // Out-of-range positions are clamped; a middle at the ends still mixes.
        let h = three(MiddlePosition::Percent(0.0));
        let l = h.levels(&values).unwrap();
        assert_eq!(h.level_color(&l, 0), DEFAULT_MIDDLE);
        assert_eq!(three(MiddlePosition::Value(-50.0)).levels(&values).unwrap().middle, 0.0);
    }

    #[test]
    fn styles_follow_the_format_type() {
        let h = Heatmap::default();
        assert_eq!(h.style(Color::rgb(10, 10, 10)).font, Some(Color::WHITE));
        assert_eq!(h.style(Color::rgb(250, 250, 200)).font, Some(Color::BLACK));
        let t = Heatmap { target: Target::Text, ..h };
        assert_eq!(t.style(Color::rgb(1, 2, 3)), super::super::Colours { fill: None, font: Some(Color::rgb(1, 2, 3)) });
    }
}
