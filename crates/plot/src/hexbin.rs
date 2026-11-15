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
//!   the axis ranges) uses the nominal frame.
//! - Colour scale: a vertical bar [`BAR_GAP_PT`] right of the frame, [`BAR_W_PT`] wide, with the title
//!   "Count" above it, ticks at nice counts from 0 and their labels in the Y axis tick font. The frame makes
//!   room for it. Non-linear axes bin in data units (the hexagons are then not regular on the page).

use crate::data::PlotData;
use crate::scene::{Anchor, Item, Stroke, line_to, move_to};
use crate::text::{self, ASCENT, DESCENT};
use crate::{LayerGeometry, Rect, TextMeasurer};
use sheaf_calc::hexbin::{Hexbin, hexbin};
use sheaf_core::Color;
use sheaf_core::graph::{HexbinStyle, Layer};

/// Width of the stroke in the fill's colour that hides seams, in pt.
pub(crate) const HEX_SEAM_PT: f64 = 0.25;
/// Distance from the frame to the colour scale bar, in pt.
pub(crate) const BAR_GAP_PT: f64 = 14.0;
/// Width of the colour scale bar, in pt.
pub(crate) const BAR_W_PT: f64 = 14.0;
/// Distance from the bar to its tick labels, in pt (the tick is 3 pt long).
const LABEL_GAP_PT: f64 = 6.0;
/// Number of strips the gradient of the bar is drawn with.
const BAR_STRIPS: usize = 64;
/// The title of the colour scale.
pub(crate) const SCALE_TITLE: &str = "Count";

/// What a hexbin plot bins: the points as drawn, and the first-pass binning for the axis ranges.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct HexData {
    pub xs: Vec<f64>,
    pub ys: Vec<f64>,
    /// The binning for the nominal frame: it sets the axis ranges, the drawing redoes it.
    pub first: Option<Hexbin>,
}

impl HexData {
    pub fn new(pd: &PlotData, style: &HexbinStyle, aspect: f64) -> HexData {
        let (mut xs, mut ys) = (Vec::new(), Vec::new());
        for r in pd.rows().filter(|r| !pd.mask.is_some_and(|m| m.contains(r))) {
            let (x, y) = (pd.display_x(pd.source_x_at(r)), pd.display_y(pd.y[r]));
            if x.is_finite() && y.is_finite() {
                xs.push(x);
                ys.push(y);
            }
        }
        let first = hexbin(&xs, &ys, &style.to_calc(aspect));
        HexData { xs, ys, first }
    }

    /// Bins again for a nominal frame of `aspect` (width / height).
    pub fn refit(&mut self, style: &HexbinStyle, aspect: f64) {
        self.first = hexbin(&self.xs, &self.ys, &style.to_calc(aspect));
    }

    /// The binning for the drawn frame `w` x `h` pt and the axis spans (in axis scale units).
    pub fn bin_for(&self, style: &HexbinStyle, w: f64, h: f64, x_span: f64, y_span: f64) -> Option<Hexbin> {
        let setup = style.to_calc(1.0);
        let (xr, yr) = sheaf_calc::hexbin::centre_ranges(&self.xs, &self.ys, &setup)?;
        let (xr, yr) = (xr.1 - xr.0, yr.1 - yr.0);
        // The lattice derives `dy = sx * a * (yr / xr) * sqrt(3) / 2`; regular on the page needs
        // `a (yr / xr) = (w / h) (y_span / x_span)`.
        let want = (w / h) * (y_span / x_span);
        let aspect = if want.is_finite() && want > 0.0 && xr > 0.0 && yr > 0.0 { want * xr / yr } else { 1.0 };
        hexbin(&self.xs, &self.ys, &style.to_calc(aspect))
    }

    /// The extent of the hexagons (and of the points, for a narrow centre range): `(x lo, x hi, y lo, y hi)`.
    pub fn extent(&self) -> Option<(f64, f64, f64, f64)> {
        let h = self.first.as_ref()?;
        let mut e = (f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY);
        for b in &h.bins {
            for (x, y) in h.lattice.corners(b.x, b.y) {
                e = (e.0.min(x), e.1.max(x), e.2.min(y), e.3.max(y));
            }
        }
        e.0.is_finite().then_some(e)
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

/// Draws the hexagons.
pub(crate) fn draw_hexbin(out: &mut Vec<Item>, pd: &PlotData, geom: &LayerGeometry) {
    let Some(h) = drawn_bins(pd, geom) else { return };
    for b in &h.bins {
        let color = bin_color(b.count, h.max_count);
        let mut d = String::new();
        let mut closed = true;
        for (i, (x, y)) in h.lattice.corners(b.x, b.y).into_iter().enumerate() {
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
    let Some(pd) = pds.iter().find(|pd| pd.hex.is_some()) else { return };
    let Some(h) = drawn_bins(pd, geom) else { return };
    let f = geom.frame;
    let ya = &layer.y_axis;
    let size = ya.tick_font.size_pt;
    let title_size = ya.title_font.size_pt;
    let x0 = f.x + f.w + BAR_GAP_PT;
    let top = f.y + (ASCENT + DESCENT) * title_size + 0.4 * title_size;
    let bar = Rect { x: x0, y: top, w: BAR_W_PT, h: (f.y + f.h - top).max(1.0) };
    text::place(out, m, SCALE_TITLE, &ya.title_font, title_size, (x0, f.y + ASCENT * title_size), Anchor::Start, 0.0);
    // The gradient: strips from the top (the largest count) down to 0.
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
    for v in scale_ticks(h.max_count) {
        let y = bar.y + (1.0 - v as f64 / h.max_count.max(1) as f64) * bar.h;
        move_to(&mut d, bar.x + bar.w, y);
        line_to(&mut d, bar.x + bar.w + 3.0, y);
        let label = v.to_string();
        text::place(
            out,
            m,
            &label,
            &ya.tick_font,
            size,
            (bar.x + bar.w + LABEL_GAP_PT, y + (ASCENT - DESCENT) / 2.0 * size),
            Anchor::Start,
            0.0,
        );
    }
    out.push(Item::Path { d, fill: None, stroke: Some(Stroke::solid(line, 0.5)) });
}
