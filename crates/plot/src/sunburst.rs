//! Sunburst plot (Origin 2022 "Sunburst Plot", graphing spec 14.27): the hierarchy of the category columns of a
//! plot's [`SunburstStyle`] as concentric rings, the angle of each wedge proportional to the sum of the plot's value
//! column below it.
//!
//! Origin documents the plot type (a variant of the Doughnut plot), that a wedge's colour follows its value, that a
//! category's colour follows the mean value of its children and that a missing text category is unfilled. The rest is
//! Sheaf's own and unmeasured, listed in the spec; the hierarchy and the angles are in `sheaf_calc`.
//!
//! - The rings fill a circle centred in the layer's frame, its diameter the shorter side of the frame (so it stays
//!   round); no axes, no frame line, no legend. The first level is the innermost ring; every shown level has the
//!   same width from the hole radius (a fraction of the outer radius) to the outer radius, less the ring gap.
//! - The first wedge starts at the start angle (degrees counter-clockwise from 3 o'clock, so 90 is 12 o'clock) and
//!   the wedges follow clockwise. Arcs are polylines of steps of at most 2 degrees.
//! - A leaf is coloured by its value with the shared colour ramp over the range of the leaf values, a category by
//!   the mean of its leaves' values over the same range; a category whose cell was empty is outline only. Every
//!   wedge has a white border (0.5 pt by default).
//! - A label sits at the centre of its wedge, along the radial direction and turned to stay upright. Names and, when
//!   asked, values are drawn whole or not at all: a line is left out unless the text fits the ring's width and the
//!   lines together fit the wedge's arc length. The labels are 0.45 of the X axis tick font, as the treemap's.
//! - The colour scale beside the frame is titled with the value column's name.

use crate::LayerGeometry;
use crate::Rect;
use crate::data::PlotData;
use crate::fonts::TextMeasurer;
use crate::scene::{Item, Stroke, line_to, move_to};
use crate::text::{self, ASCENT, DESCENT};
use crate::treemap::{PAD_EM, text_color, value_text};
use sheaf_calc::sunburst::{Wedge, layout as wedge_layout, leaf_range};
use sheaf_calc::treemap_tree::{self, Key, Order, Row};
use sheaf_core::graph::{Font, Layer, SunburstOrder, SunburstStyle};
use sheaf_core::{Color, Project};
use std::f64::consts::PI;

/// The labels are this fraction of the X axis tick font's size.
const LABEL_SCALE: f64 = 0.45;
/// The largest angle of one segment of an arc, in degrees.
pub(crate) const MAX_STEP_DEG: f64 = 2.0;

/// What a sunburst plot draws, held by its plot.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SunburstData {
    pub wedges: Vec<Wedge>,
    /// The number of rings.
    pub levels: usize,
    pub style: SunburstStyle,
    /// The smallest and largest leaf value (the colour scale's ends).
    pub range: Option<(f64, f64)>,
    pub font: Font,
    /// The colour of the outline of an unfilled wedge.
    pub line: Color,
    /// The title of the colour scale: the value column's name.
    pub title: String,
}

impl SunburstData {
    /// The hierarchy of the rows `pd` shows: category cells from the style's columns, values from the plot's Y.
    pub fn new(project: &Project, layer: &Layer, pd: &PlotData) -> Option<SunburstData> {
        let style = pd.plot.sunburst.clone();
        let columns: Vec<_> = style.categories.iter().filter_map(|id| project.column(*id)).collect();
        if columns.is_empty() {
            return None;
        }
        // Nested frequencies (T392): without a numeric value column every row of the categories counts 1.
        let counted = project.numeric(pd.plot.y).is_none();
        let all: Vec<usize> =
            if counted { (0..columns.iter().map(|c| c.len()).max().unwrap_or(0)).collect() } else { Vec::new() };
        let shown: Vec<usize> =
            if counted { all } else { pd.rows().filter(|r| !pd.mask.is_some_and(|m| m.contains(r))).collect() };
        let rows: Vec<Row> = shown
            .into_iter()
            .map(|r| Row {
                path: columns.iter().map(|c| c.cell_text(r)).collect(),
                value: if counted { 1.0 } else { pd.y.get(r).copied().unwrap_or(f64::NAN) },
                key: Key::Missing,
            })
            .collect();
        let tree = treemap_tree::build(&rows, columns.len());
        let order = match style.order {
            SunburstOrder::Worksheet => Order::Categorical,
            SunburstOrder::BySize => Order::BySize,
        };
        let wedges = wedge_layout(&tree, order);
        let range = leaf_range(&wedges);
        let mut font = layer.x_axis.tick_font.clone();
        font.size_pt *= LABEL_SCALE;
        Some(SunburstData {
            wedges,
            levels: columns.len(),
            style,
            range,
            font,
            line: layer.x_axis.tick_font.color,
            title: if project.numeric(pd.plot.y).is_none() { "Count".to_string() } else { project.plot_label(pd.plot) },
        })
    }

    /// The position 0 to 1 of `value` on the colour scale.
    pub fn position(&self, value: f64) -> f64 {
        match self.range {
            Some((lo, hi)) if hi > lo => ((value - lo) / (hi - lo)).clamp(0.0, 1.0),
            _ => 0.5,
        }
    }

    /// The fill of a wedge: its colour value on the ramp; `None` for a category whose cell was empty.
    pub fn fill(&self, w: &Wedge) -> Option<Color> {
        if w.blank { None } else { Some(crate::plots::map_color(self.position(w.color_value))) }
    }
}

/// The circle and rings of a sunburst in a frame.
#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) struct Rings {
    pub cx: f64,
    pub cy: f64,
    /// The outer radius.
    pub outer: f64,
    /// The radius of the hole.
    pub hole: f64,
    /// The width of a ring, gap included.
    pub width: f64,
    gap: f64,
    levels: usize,
}

impl Rings {
    pub fn new(frame: Rect, style: &SunburstStyle, levels: usize) -> Rings {
        let outer = (0.5 * frame.w.min(frame.h)).max(1.0);
        let hole = outer * style.hole.clamp(0.0, 0.9);
        let width = (outer - hole) / levels.max(1) as f64;
        Rings {
            cx: frame.x + 0.5 * frame.w,
            cy: frame.y + 0.5 * frame.h,
            outer,
            hole,
            width,
            gap: style.ring_gap_pt.clamp(0.0, 0.5 * width),
            levels: levels.max(1),
        }
    }

    /// The inner and outer radius of ring `level` (the gap is taken half from each side, not at the hole or the rim).
    pub fn radii(&self, level: usize) -> (f64, f64) {
        let inner = self.hole + level as f64 * self.width + if level > 0 { self.gap / 2.0 } else { 0.0 };
        let outer =
            self.hole + (level + 1) as f64 * self.width - if level + 1 < self.levels { self.gap / 2.0 } else { 0.0 };
        (inner, outer)
    }

    /// The point at radius `r` and angle `deg` (degrees counter-clockwise from 3 o'clock).
    pub fn point(&self, r: f64, deg: f64) -> (f64, f64) {
        let a = deg.to_radians();
        (self.cx + r * a.cos(), self.cy - r * a.sin())
    }
}

/// The angle in degrees of the turn fraction `t` for a plot starting at `start`: clockwise.
pub(crate) fn angle(start: f64, t: f64) -> f64 {
    start - 360.0 * t
}

fn arc(d: &mut String, rings: &Rings, r: f64, from: f64, to: f64, first: bool) {
    let steps = (((to - from).abs() / MAX_STEP_DEG).ceil() as usize).max(1);
    for k in 0..=steps {
        let (x, y) = rings.point(r, from + (to - from) * k as f64 / steps as f64);
        if k == 0 && first {
            move_to(d, x, y);
        } else {
            line_to(d, x, y);
        }
    }
}

/// The outline of the wedge from angle `a0` to `a1` (degrees, `a1 < a0`: clockwise) between the radii `ri` and `ro`.
/// A wedge round the whole turn is two circles, the hole cut out.
pub(crate) fn wedge_path(rings: &Rings, ri: f64, ro: f64, a0: f64, a1: f64) -> String {
    let mut d = String::new();
    if (a0 - a1).abs() >= 360.0 - 1e-6 {
        arc(&mut d, rings, ro, a0, a1, true);
        d.push('Z');
        if ri > 1e-9 {
            arc(&mut d, rings, ri, a1, a0, true);
            d.push('Z');
        }
        return d;
    }
    arc(&mut d, rings, ro, a0, a1, true);
    if ri > 1e-9 {
        arc(&mut d, rings, ri, a1, a0, false);
    } else {
        line_to(&mut d, rings.cx, rings.cy);
    }
    d.push('Z');
    d
}

/// Draws the sunburst held by the plot.
pub(crate) fn draw_sunburst(out: &mut Vec<Item>, pd: &PlotData, geom: &LayerGeometry, m: &dyn TextMeasurer) {
    let Some(sb) = pd.sunburst.as_deref() else { return };
    let style = &sb.style;
    let rings = Rings::new(geom.frame, style, sb.levels);
    let border = style.border_pt.max(0.0);
    for w in &sb.wedges {
        let (ri, ro) = rings.radii(w.level);
        let (a0, a1) = (angle(style.start_angle_deg, w.from), angle(style.start_angle_deg, w.from + w.span));
        let d = wedge_path(&rings, ri, ro, a0, a1);
        let fill = sb.fill(w);
        let stroke = match fill {
            Some(_) => (border > 0.0).then(|| Stroke::solid(Color::WHITE, border)),
            None => Some(Stroke::solid(sb.line, border.max(0.5))),
        };
        out.push(Item::Path { d, fill, stroke });
    }
    for w in &sb.wedges {
        label(out, sb, &rings, w, m);
    }
}

/// Writes the name and value of a wedge at its centre when they fit.
fn label(out: &mut Vec<Item>, sb: &SunburstData, rings: &Rings, w: &Wedge, m: &dyn TextMeasurer) {
    let style = &sb.style;
    let size = sb.font.size_pt;
    let (ri, ro) = rings.radii(w.level);
    let (rm, ring) = ((ri + ro) / 2.0, ro - ri);
    let mid = angle(style.start_angle_deg, w.from + w.span / 2.0);
    let span_rad = w.span * 2.0 * PI;
    // The room across the wedge at radius `r`: the chord, or the diameter of a half turn or more.
    let across = |r: f64| if span_rad >= PI { 2.0 * r } else { 2.0 * r * (span_rad / 2.0).sin() };
    let pad = PAD_EM * size;
    let line = (ASCENT + DESCENT) * size;
    let mut font = sb.font.clone();
    font.color = match sb.fill(w) {
        Some(c) => text_color(c),
        None => sb.line,
    };
    let fits = |s: &str| text::width(m, s, &font, size) + 2.0 * pad <= ring;
    let mut lines: Vec<String> = Vec::new();
    if style.show_names {
        // A value alone would not say what it belongs to: when the name does not fit, the whole label goes.
        if !fits(&w.name) {
            return;
        }
        lines.push(w.name.clone());
    }
    let value = value_text(w.value);
    if style.show_values && !value.is_empty() && fits(&value) {
        lines.push(value);
    }
    // A radial line spans rm +- width / 2, so the wedge is narrowest at the line's inner end: the lines together must
    // fit the room across the wedge there. Both lines need twice the height; when they do not fit, the first alone may.
    while !lines.is_empty() {
        let widest = lines.iter().map(|s| text::width(m, s, &font, size)).fold(0.0, f64::max);
        let inner = (rm - widest / 2.0).max(0.0);
        if lines.len() as f64 * line + 2.0 * pad <= across(inner) {
            break;
        }
        lines.pop();
    }
    if lines.is_empty() {
        return;
    }
    // Along the radial direction; a label that would read upside down is turned half a turn.
    let mut rot = mid.rem_euclid(360.0);
    if rot > 90.0 && rot < 270.0 {
        rot = (rot + 180.0).rem_euclid(360.0);
    }
    let (cx, cy) = rings.point(rm, mid);
    let t = rot.to_radians();
    let down = (t.sin(), t.cos());
    let n = lines.len() as f64;
    for (i, s) in lines.iter().enumerate() {
        let off = (i as f64 - (n - 1.0) / 2.0) * line + (ASCENT - DESCENT) / 2.0 * size;
        text::place(out, m, s, &font, size, (cx + down.0 * off, cy + down.1 * off), crate::scene::Anchor::Middle, rot);
    }
}

/// The ticks of the colour scale over `(lo, hi)`: (position 0..1, label).
pub(crate) fn scale_ticks(lo: f64, hi: f64) -> Vec<(f64, String)> {
    if hi.partial_cmp(&lo) != Some(std::cmp::Ordering::Greater) {
        return vec![(0.5, value_text(lo))];
    }
    let step = crate::ticks::nice_step((hi - lo) / 4.0);
    if !(step.is_finite() && step > 0.0) {
        return vec![(0.0, value_text(lo)), (1.0, value_text(hi))];
    }
    let first = (lo / step).ceil() as i64;
    let last = (hi / step).floor() as i64;
    (first..=last)
        .map(|k| {
            let v = k as f64 * step;
            ((v - lo) / (hi - lo), value_text(v))
        })
        .collect()
}

/// Draws the colour scale of the layer's sunburst plot beside the frame.
pub(crate) fn draw_scale(
    out: &mut Vec<Item>,
    layer: &Layer,
    pds: &[PlotData],
    geom: &LayerGeometry,
    m: &dyn TextMeasurer,
) {
    let Some(sb) = pds.iter().find_map(|pd| pd.sunburst.as_deref()) else { return };
    let Some((lo, hi)) = sb.range else { return };
    crate::hexbin::draw_color_bar(out, layer, geom, m, &sb.title, &scale_ticks(lo, hi), (0.0, 0.0));
}
