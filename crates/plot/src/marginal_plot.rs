//! Marginal histograms and box charts (T110, graphing spec "Marginal plots"): the bins or the box of
//! the plot's Y column drawn in a margin layer. In the top margin the values run along X and the counts
//! (or the box's position, 1 in a 0.5 to 1.5 span) up Y; in the right margin the other way round.

use crate::data::{BoxStats, PlotData, box_stats, histogram_freq};
use crate::scene::{self, Item};
use crate::{LayerGeometry, plots::line_stroke};
use sheaf_core::marginal::MarginalSide;

/// The width of a marginal box in its layer's position units (measured about 0.4).
const BOX_WIDTH: f64 = 0.4;

pub(crate) enum MarginalShape {
    /// Each bin's lower edge, upper edge and count.
    Bins(Vec<(f64, f64, f64)>),
    Box(BoxStats),
}

pub(crate) struct MarginalData {
    pub side: MarginalSide,
    pub shape: MarginalShape,
}

impl MarginalData {
    pub(crate) fn new(
        values: &[f64],
        side: MarginalSide,
        hist: &sheaf_core::graph::HistogramStyle,
        bins: bool,
    ) -> Option<Self> {
        let shape = if bins {
            let f = histogram_freq(values, hist)?;
            let rows = f
                .rows
                .iter()
                .filter(|r| r.end.is_finite())
                .map(|r| (r.end - f.bin_size, r.end, r.count as f64))
                .collect();
            MarginalShape::Bins(rows)
        } else {
            let mut b = box_stats(values, 1.0, String::new())?;
            b.width = BOX_WIDTH;
            MarginalShape::Box(b)
        };
        Some(MarginalData { side, shape })
    }

    /// The span of the values and the span of the counts (or of the box's position).
    fn spans(&self) -> ((f64, f64), (f64, f64)) {
        match &self.shape {
            MarginalShape::Bins(rows) => {
                let lo = rows.iter().map(|r| r.0).fold(f64::INFINITY, f64::min);
                let hi = rows.iter().map(|r| r.1).fold(f64::NEG_INFINITY, f64::max);
                let top = rows.iter().map(|r| r.2).fold(0.0, f64::max);
                ((lo, hi), (0.0, top))
            }
            MarginalShape::Box(b) => {
                let lo = b.outliers.iter().copied().fold(b.whisker_low, f64::min);
                let hi = b.outliers.iter().copied().fold(b.whisker_high, f64::max);
                ((lo, hi), (0.5, 1.5))
            }
        }
    }

    /// The span the plot takes on the X axis (`x`) or the Y axis.
    pub(crate) fn extent(&self, x: bool) -> (f64, f64) {
        let (values, other) = self.spans();
        if x == (self.side == MarginalSide::Top) { values } else { other }
    }
}

/// The bars or the box of a marginal plot.
pub(crate) fn draw_marginal(out: &mut Vec<Item>, pd: &PlotData, geom: &LayerGeometry) {
    let Some(m) = &pd.marginal else { return };
    let top = m.side == MarginalSide::Top;
    // (value, across) to the page: values run along X on top, along Y on the right.
    let at = |v: f64, a: f64| if top { geom.data_to_page(v, a) } else { geom.data_to_page(a, v) };
    let stroke = line_stroke(&pd.plot.line);
    let rect = |out: &mut Vec<Item>, v0: f64, v1: f64, a0: f64, a1: f64, fill: Option<sheaf_core::Color>| {
        let (p0, p1, p2, p3) = (at(v0, a0), at(v1, a0), at(v1, a1), at(v0, a1));
        if ![p0.0, p0.1, p2.0, p2.1].iter().all(|c| c.is_finite()) {
            return;
        }
        let mut d = String::new();
        scene::move_to(&mut d, p0.0, p0.1);
        scene::line_to(&mut d, p1.0, p1.1);
        scene::line_to(&mut d, p2.0, p2.1);
        scene::line_to(&mut d, p3.0, p3.1);
        d.push('Z');
        out.push(Item::Path { d, fill, stroke: Some(stroke.clone()) });
    };
    let line = |out: &mut Vec<Item>, a: (f64, f64), b: (f64, f64)| {
        let (p, q) = (at(a.0, a.1), at(b.0, b.1));
        if ![p.0, p.1, q.0, q.1].iter().all(|c| c.is_finite()) {
            return;
        }
        let mut d = String::new();
        scene::move_to(&mut d, p.0, p.1);
        scene::line_to(&mut d, q.0, q.1);
        out.push(Item::Path { d, fill: None, stroke: Some(stroke.clone()) });
    };
    match &m.shape {
        MarginalShape::Bins(rows) => {
            for &(lo, hi, count) in rows {
                if count > 0.0 {
                    rect(out, lo, hi, 0.0, count, Some(pd.plot.symbol.color));
                }
            }
        }
        MarginalShape::Box(b) => {
            let (c, half) = (b.pos, b.width / 2.0);
            rect(out, b.q1, b.q3, c - half, c + half, None);
            line(out, (b.median, c - half), (b.median, c + half));
            line(out, (b.whisker_low, c), (b.q1, c));
            line(out, (b.q3, c), (b.whisker_high, c));
            for w in [b.whisker_low, b.whisker_high] {
                line(out, (w, c - half / 2.0), (w, c + half / 2.0));
            }
            // The mean as a small hollow square, the outliers as small crosses (4 pt).
            let r = 2.0;
            let (mx, my) = at(b.mean, c);
            if mx.is_finite() && my.is_finite() {
                let mut d = String::new();
                scene::move_to(&mut d, mx - r, my - r);
                scene::line_to(&mut d, mx + r, my - r);
                scene::line_to(&mut d, mx + r, my + r);
                scene::line_to(&mut d, mx - r, my + r);
                d.push('Z');
                out.push(Item::Path { d, fill: None, stroke: Some(stroke.clone()) });
            }
            for &o in &b.outliers {
                let (ox, oy) = at(o, c);
                if !(ox.is_finite() && oy.is_finite()) {
                    continue;
                }
                let mut d = String::new();
                scene::move_to(&mut d, ox - r, oy - r);
                scene::line_to(&mut d, ox + r, oy + r);
                scene::move_to(&mut d, ox - r, oy + r);
                scene::line_to(&mut d, ox + r, oy - r);
                out.push(Item::Path { d, fill: None, stroke: Some(stroke.clone()) });
            }
        }
    }
}
