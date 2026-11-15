//! Hexbin plots (graphing spec 14.24): the X and Y columns binned into pointy-top hexagons
//! (`sheaf_calc::hexbin`), each non-empty hexagon filled with a colour of the count, and a colour scale
//! beside the layer.
//!
//! Origin documents the bin controls and that the colour stands for the number of points; the rest is
//! Sheaf's own and unmeasured, and is named below and listed in the spec:
//!
//! - Colour: `ramp(count / max count)`, linear from 0 to the largest count, with the colour ramp of the
//!   colour mapped scatter (`plots::map_color`: blue, cyan, green, yellow, red). Empty bins are not drawn.
//! - Outline: none, but a hairline stroke ([`HEX_SEAM_PT`]) in the fill's own colour keeps the anti-aliasing
//!   from leaving seams between neighbours.
//! - Regular hexagons: the binning is redone when the layer is drawn, with the real frame and axis spans, so
//!   that "Prefer Regular Hexagons" gives regular hexagons on the page for any frame shape, not only a
//!   square one (the lattice in data units follows `frame aspect x axis span ratio`). The first pass (for
//!   the axis ranges) uses the nominal frame; where the drawn hexagons then reach beyond the axis range
//!   (the colour scale narrows the frame), `layout::graph_geometry` widens the range to them and resolves
//!   the axes again until they fit.
//! - Colour scale: a vertical bar [`BAR_GAP_PT`] right of the frame, [`BAR_W_PT`] wide, with the title
//!   "Count" above it, ticks at nice counts from 0 and their labels in the Y axis tick font. The frame makes
//!   room for it. Several hexbin plots in a
//!   layer overlap (the user's choice); their colours share one scale, `count / (largest count of all of
//!   them)`, and the colour scale shows that maximum.
//! - Non-linear axes: the lattice is built in the scale's transformed coordinates (log10 on a log axis), so
//!   hexagons are regular on the page and map back inside the scale's domain. Points the scale cannot place
//!   (<= 0 on a log axis) are skipped and counted in `HexData::excluded`. Min/Max Center are data values;
//!   a spacing is in scale units (decades on log10).

use crate::data::PlotData;
use crate::scene::{Anchor, Item, Stroke, line_to, move_to};
use crate::text::{self, ASCENT, DESCENT};
use crate::{LayerGeometry, Rect, TextMeasurer};
use sheaf_calc::hexbin::{Hexbin, hexbin};
use sheaf_core::Color;
use sheaf_core::graph::{HexbinStyle, Layer, ScaleKind};
use std::cell::Cell;

/// Width of the stroke in the fill's colour that hides seams, in pt.
pub(crate) const HEX_SEAM_PT: f64 = 0.25;
/// Distance from the frame to the colour scale bar, in pt.
pub(crate) const BAR_GAP_PT: f64 = 14.0;
/// Width of the colour scale bar, in pt.
pub(crate) const BAR_W_PT: f64 = 14.0;
/// Distance from the bar to its tick labels, in pt (the tick is 3 pt long).
pub(crate) const LABEL_GAP_PT: f64 = 6.0;
/// Number of strips the gradient of the bar is drawn with.
const BAR_STRIPS: usize = 64;
/// The title of the colour scale.
pub(crate) const SCALE_TITLE: &str = "Count";

/// What a hexbin plot bins: the points as drawn, and the first-pass binning for the axis ranges.
///
/// The points are held in the axis scale's transformed coordinates (`ScaleKind::forward`: log10 on a log
/// axis), where the lattice is built, so that the hexagons are regular on the page and never cross the
/// domain of the scale. Points the scale cannot place (<= 0 on a log axis) are not binned; they are
/// counted in `excluded`.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct HexData {
    /// X and Y in the transformed coordinates of the axes.
    pub xs: Vec<f64>,
    pub ys: Vec<f64>,
    pub x_scale: ScaleKind,
    pub y_scale: ScaleKind,
    /// Finite points the scale of an axis cannot place, which are left out of the binning.
    pub excluded: usize,
    /// The binning for the nominal frame: it sets the axis ranges, the drawing redoes it.
    pub first: Option<Hexbin>,
    /// The extent (data units) of the hexagons as drawn, once the real frame is known and when they
    /// reached beyond the axis range of the first pass; kept in the axis range (see `layout::graph_geometry`).
    grown: Cell<Option<(f64, f64, f64, f64)>>,
    /// The largest count over every hexagon of every hexbin plot of the layer, set before drawing.
    shared_max: Cell<usize>,
}

impl HexData {
    pub fn new(pd: &PlotData, style: &HexbinStyle, aspect: f64, x_scale: ScaleKind, y_scale: ScaleKind) -> HexData {
        let (mut xs, mut ys) = (Vec::new(), Vec::new());
        let mut excluded = 0;
        for r in pd.rows().filter(|r| !pd.mask.is_some_and(|m| m.contains(r))) {
            let (x, y) = (pd.display_x(pd.source_x_at(r)), pd.display_y(pd.y[r]));
            if !(x.is_finite() && y.is_finite()) {
                continue;
            }
            if x_scale.accepts(x) && y_scale.accepts(y) {
                xs.push(x_scale.forward(x));
                ys.push(y_scale.forward(y));
            } else {
                excluded += 1;
            }
        }
        let mut h = HexData {
            xs,
            ys,
            x_scale,
            y_scale,
            excluded,
            first: None,
            grown: Cell::new(None),
            shared_max: Cell::new(0),
        };
        h.refit(style, aspect);
        h
    }

    /// The binning setting in transformed coordinates: Min and Max Center are given in data units and move
    /// with the scale; a spacing is in the units of the scale (decades on a log10 axis).
    fn setup(&self, style: &HexbinStyle, aspect: f64) -> sheaf_calc::hexbin::HexSetup {
        let mut s = style.to_calc(aspect);
        let map = |v: Option<f64>, k: ScaleKind| v.map(|v| k.forward(v)).filter(|v| v.is_finite());
        s.x.min_center = map(s.x.min_center, self.x_scale);
        s.x.max_center = map(s.x.max_center, self.x_scale);
        s.y.min_center = map(s.y.min_center, self.y_scale);
        s.y.max_center = map(s.y.max_center, self.y_scale);
        s
    }

    /// Bins again for a nominal frame of `aspect` (width / height).
    pub fn refit(&mut self, style: &HexbinStyle, aspect: f64) {
        self.first = hexbin(&self.xs, &self.ys, &self.setup(style, aspect));
    }

    /// The binning for the drawn frame `w` x `h` pt and the axis spans (in axis scale units).
    pub fn bin_for(&self, style: &HexbinStyle, w: f64, h: f64, x_span: f64, y_span: f64) -> Option<Hexbin> {
        let setup = self.setup(style, 1.0);
        let (xr, yr) = sheaf_calc::hexbin::centre_ranges(&self.xs, &self.ys, &setup)?;
        let (xr, yr) = (xr.1 - xr.0, yr.1 - yr.0);
        // The lattice derives `dy = sx * a * (yr / xr) * sqrt(3) / 2`; regular on the page needs
        // `a (yr / xr) = (w / h) (y_span / x_span)`.
        let want = (w / h) * (y_span / x_span);
        let aspect = if want.is_finite() && want > 0.0 && xr > 0.0 && yr > 0.0 { want * xr / yr } else { 1.0 };
        hexbin(&self.xs, &self.ys, &self.setup(style, aspect))
    }

    /// A corner of the lattice in data units.
    pub fn corners(&self, h: &Hexbin, b: &sheaf_calc::hexbin::HexBin) -> [(f64, f64); 6] {
        h.lattice.corners(b.x, b.y).map(|(x, y)| (self.x_scale.inverse(x), self.y_scale.inverse(y)))
    }

    fn extent_of(&self, h: &Hexbin) -> Option<(f64, f64, f64, f64)> {
        let mut e = (f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY);
        for b in &h.bins {
            for (x, y) in self.corners(h, b) {
                if x.is_finite() && y.is_finite() {
                    e = (e.0.min(x), e.1.max(x), e.2.min(y), e.3.max(y));
                }
            }
        }
        e.0.is_finite().then_some(e)
    }

    /// The extent of the hexagons (in data units): `(x lo, x hi, y lo, y hi)`; the first pass, widened by what
    /// the drawn binning needed.
    pub fn extent(&self) -> Option<(f64, f64, f64, f64)> {
        let first = self.first.as_ref().and_then(|h| self.extent_of(h));
        match (first, self.grown.get()) {
            (Some(a), Some(b)) => Some((a.0.min(b.0), a.1.max(b.1), a.2.min(b.2), a.3.max(b.3))),
            (a, b) => a.or(b),
        }
    }

    /// Widens the extent to the hexagons drawn in `geom` when they leave its axis ranges. True when the
    /// extent changed (the axes have to be resolved again).
    pub fn grow_to_drawn(&self, style: &HexbinStyle, geom: &LayerGeometry) -> bool {
        let f = geom.frame;
        let Some(h) = self.bin_for(style, f.w, f.h, geom.x.scale_span(), geom.y.scale_span()) else { return false };
        let Some(e) = self.extent_of(&h) else { return false };
        let eps = |a: f64, b: f64| 1e-9 * (a.abs() + b.abs() + 1.0);
        let inside = e.0 >= geom.x.min - eps(e.0, geom.x.min)
            && e.1 <= geom.x.max + eps(e.1, geom.x.max)
            && e.2 >= geom.y.min - eps(e.2, geom.y.min)
            && e.3 <= geom.y.max + eps(e.3, geom.y.max);
        if inside {
            return false;
        }
        let new = match self.grown.get() {
            Some(g) => (g.0.min(e.0), g.1.max(e.1), g.2.min(e.2), g.3.max(e.3)),
            None => e,
        };
        let changed = self.grown.get() != Some(new);
        self.grown.set(Some(new));
        changed
    }
}

/// The nominal frame aspect (width / height) of a layer on a page of `page` pt.
pub(crate) fn nominal_aspect(layer: &Layer, page_w: f64, page_h: f64) -> f64 {
    let f = &layer.frame;
    let a = (f.width * page_w) / (f.height * page_h);
    if a.is_finite() && a > 0.0 { a } else { 1.0 }
}

/// The binning at the drawn size, for the layer's hexbin plot.
fn drawn_bins(pd: &PlotData, geom: &LayerGeometry) -> Option<Hexbin> {
    let hex = pd.hex.as_ref()?;
    let f = geom.frame;
    hex.bin_for(&pd.plot.hexbin, f.w, f.h, geom.x.scale_span(), geom.y.scale_span())
}

/// The colour of a bin with `count` of at most `max`.
pub(crate) fn bin_color(count: usize, max: usize) -> Color {
    crate::plots::map_color(count as f64 / max.max(1) as f64)
}

/// The largest count over every hexbin plot of the layer (the colours and the colour scale share it).
fn layer_max<'a>(pds: impl Iterator<Item = &'a PlotData<'a>>, geom: &LayerGeometry) -> usize {
    pds.filter(|pd| pd.hex.is_some()).filter_map(|pd| drawn_bins(pd, geom)).map(|h| h.max_count).max().unwrap_or(0)
}

/// Fixes the colour normalisation of the hexbin plots of a layer before they are drawn.
pub(crate) fn share_max(pds: &[PlotData], geom: &LayerGeometry) {
    let max = layer_max(pds.iter(), geom);
    for hex in pds.iter().filter_map(|pd| pd.hex.as_ref()) {
        hex.shared_max.set(max);
    }
}

/// Draws the hexagons.
pub(crate) fn draw_hexbin(out: &mut Vec<Item>, pd: &PlotData, geom: &LayerGeometry) {
    let Some(h) = drawn_bins(pd, geom) else { return };
    let Some(hex) = pd.hex.as_ref() else { return };
    let max = match hex.shared_max.get() {
        0 => h.max_count,
        m => m,
    };
    for b in &h.bins {
        let color = bin_color(b.count, max);
        let mut d = String::new();
        let mut closed = true;
        for (i, (x, y)) in hex.corners(&h, b).into_iter().enumerate() {
            let Some(p) = geom.checked_data_to_page(x, y) else {
                closed = false;
                break;
            };
            if i == 0 {
                move_to(&mut d, p.0, p.1);
            } else {
                line_to(&mut d, p.0, p.1);
            }
        }
        if closed {
            d.push('Z');
            out.push(Item::Path { d, fill: Some(color), stroke: Some(Stroke::solid(color, HEX_SEAM_PT)) });
        }
    }
}

/// Nice count values for the ticks of the colour scale: 0, then multiples of a 1, 2 or 5 times a power of
/// ten step that gives about four intervals, up to `max`.
pub(crate) fn scale_ticks(max: usize) -> Vec<usize> {
    if max == 0 {
        return vec![0];
    }
    let step = (crate::ticks::nice_step(max as f64 / 4.0).round() as usize).max(1);
    (0..=max / step).map(|k| k * step).collect()
}

/// The room the colour scale takes right of the frame (bar, ticks and labels), for a tick font of `size` pt.
pub(crate) fn scale_reserve(size: f64) -> f64 {
    BAR_GAP_PT + BAR_W_PT + LABEL_GAP_PT + 0.6 * size * 5.0 + 6.0
}

/// Draws the colour scale of the layer's hexbin plot beside the frame.
pub(crate) fn draw_scale(
    out: &mut Vec<Item>,
    layer: &Layer,
    pds: &[PlotData],
    geom: &LayerGeometry,
    m: &dyn TextMeasurer,
) {
    if !pds.iter().any(|pd| pd.hex.is_some()) {
        return;
    }
    let max_count = layer_max(pds.iter(), geom);
    if max_count == 0 {
        return;
    }
    let ticks: Vec<(f64, String)> =
        scale_ticks(max_count).into_iter().map(|v| (v as f64 / max_count as f64, v.to_string())).collect();
    draw_color_bar(out, layer, geom, m, SCALE_TITLE, &ticks, (0.0, 0.0));
}

/// Draws a colour scale beside the frame: the title above a vertical bar of the colour ramp (0 at the bottom, 1 at
/// the top), with a tick and a label at each `(position 0..1 along the bar, label)` of `ticks`. Shared by the hexbin
/// plot and the Parallel Plot.
pub(crate) fn draw_color_bar(
    out: &mut Vec<Item>,
    layer: &Layer,
    geom: &LayerGeometry,
    m: &dyn TextMeasurer,
    title: &str,
    ticks: &[(f64, String)],
    shift: (f64, f64),
) {
    let f = Rect { x: geom.frame.x + shift.0, y: geom.frame.y + shift.1, ..geom.frame };
    let ya = &layer.y_axis;
    let size = ya.tick_font.size_pt;
    let title_size = ya.title_font.size_pt;
    let x0 = f.x + f.w + BAR_GAP_PT;
    let top = f.y + (ASCENT + DESCENT) * title_size + 0.4 * title_size;
    let bar = Rect { x: x0, y: top, w: BAR_W_PT, h: (f.y + f.h - top).max(1.0) };
    text::place(out, m, title, &ya.title_font, title_size, (x0, f.y + ASCENT * title_size), Anchor::Start, 0.0);
    // The gradient: strips from the top (the largest value) down to the bottom.
    let strip = bar.h / BAR_STRIPS as f64;
    for i in 0..BAR_STRIPS {
        let t = 1.0 - (i as f64 + 0.5) / BAR_STRIPS as f64;
        // Slightly taller strips so that no seam shows.
        let rect = Rect { x: bar.x, y: bar.y + i as f64 * strip, w: bar.w, h: strip + 0.3 };
        out.push(Item::Rect { rect, fill: Some(crate::plots::map_color(t)), stroke: None });
    }
    let line = ya.tick_font.color;
    out.push(Item::Rect { rect: bar, fill: None, stroke: Some(Stroke::solid(line, 0.5)) });
    let mut d = String::new();
    for (frac, label) in ticks {
        let y = bar.y + (1.0 - frac) * bar.h;
        move_to(&mut d, bar.x + bar.w, y);
        line_to(&mut d, bar.x + bar.w + 3.0, y);
        text::place(
            out,
            m,
            label,
            &ya.tick_font,
            size,
            (bar.x + bar.w + LABEL_GAP_PT, y + (ASCENT - DESCENT) / 2.0 * size),
            Anchor::Start,
            0.0,
        );
    }
    out.push(Item::Path { d, fill: None, stroke: Some(Stroke::solid(line, 0.5)) });
}
