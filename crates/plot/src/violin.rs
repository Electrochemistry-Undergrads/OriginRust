//! Violin plots (graphing spec 14.22): each plot's Y column drawn as a mirrored kernel density
//! silhouette standing at 1, 2, 3, ... like the box chart, with marks inside that depend on the
//! kind ([`ViolinInner`]).
//!
//! Origin documents the Gaussian kernel and the bandwidth rule (Scott, `sheaf_calc::kde`) and
//! that the curve extends past the data by a number of bandwidths; the look is not documented.
//! Every choice below that Origin does not give is Sheaf's own and unmeasured: they are named
//! `VIOLIN_*` and listed in the spec.

use crate::LayerGeometry;
use crate::data::{BoxStats, PlotData};
use crate::fill::fill_polygon;
use crate::plots::{symbol_item, symbol_path};
use crate::scene::{Item, Stroke, line_to, move_to};
use sheaf_calc::kde::{Bandwidth, kernel_density};
use sheaf_core::Color;
use sheaf_core::graph::{PlotKind, SymbolShape, SymbolStyle, ViolinInner};

/// Width of the widest part of a violin, in X axis units: the same 0.8 of the spacing as a box
/// (Sheaf's choice, unmeasured). Every violin is scaled to this width (Origin's "Width" scaling,
/// where all violins have the same width; its default is not documented).
pub(crate) const VIOLIN_WIDTH: f64 = crate::data::BOX_WIDTH;
/// How far the curve reaches past the smallest and largest value, in percent of the bandwidth
/// (Origin's "Extend (% bandwidth)"; its default is not documented, its example uses 300).
const VIOLIN_EXTEND_PCT: f64 = 200.0;
/// Width of the box inside a violin and of the half box of a half violin, in X axis units.
const VIOLIN_BOX_WIDTH: f64 = 0.12;
/// The tint of the silhouette's automatic fill: the plot colour this far towards white.
const VIOLIN_FILL_TINT: f64 = 0.6;
/// Diameter of the median marker and of the points of a violin with points, in pt.
const VIOLIN_MEDIAN_PT: f64 = 5.0;
const VIOLIN_POINT_PT: f64 = 3.0;
/// Width of the lines of the marks inside, in pt (sticks are thinner).
const VIOLIN_MARK_PT: f64 = 1.0;
const VIOLIN_STICK_PT: f64 = 0.5;
/// How far points spread across the local width of the silhouette (0.8 of it).
const VIOLIN_POINT_SPREAD: f64 = 0.8;
/// The half box of a half violin stands this far left of the middle line.
const VIOLIN_HALF_GAP: f64 = 0.04;

/// What a violin plot draws, from the finite, unmasked, visible values of its Y column.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ViolinData {
    /// The values, ascending.
    pub values: Vec<f64>,
    /// The Y values where the density was evaluated, ascending; empty when the column has no
    /// usable bandwidth (fewer than two values or no spread): then there is no silhouette.
    pub grid: Vec<f64>,
    /// Half the width of the silhouette at each grid point, in X axis units.
    pub half_width: Vec<f64>,
    /// The bandwidth of the kernel.
    pub bandwidth: Option<f64>,
}

impl ViolinData {
    pub fn new(values: &[f64]) -> ViolinData {
        let mut v: Vec<f64> = values.iter().copied().filter(|x| x.is_finite()).collect();
        v.sort_by(|a, b| a.total_cmp(b));
        let Some(d) = kernel_density(&v, Bandwidth::Scott, VIOLIN_EXTEND_PCT) else {
            return ViolinData { values: v, grid: Vec::new(), half_width: Vec::new(), bandwidth: None };
        };
        let peak = d.density.iter().copied().fold(0.0, f64::max);
        let half_width = d.density.iter().map(|f| f / peak * VIOLIN_WIDTH / 2.0).collect();
        ViolinData { values: v, grid: d.grid, half_width, bandwidth: Some(d.bandwidth) }
    }

    /// The lowest and highest Y the plot reaches: the ends of the curve, or of the data without one.
    pub fn y_extent(&self) -> Option<(f64, f64)> {
        match (self.grid.first(), self.grid.last()) {
            (Some(&lo), Some(&hi)) => Some((lo, hi)),
            _ => Some((*self.values.first()?, *self.values.last()?)),
        }
    }

    /// Half the width of the silhouette at `y`, interpolated between grid points; 0 outside.
    pub fn half_width_at(&self, y: f64) -> f64 {
        let (Some(&lo), Some(&hi)) = (self.grid.first(), self.grid.last()) else { return 0.0 };
        if !(lo..=hi).contains(&y) {
            return 0.0;
        }
        let i = self.grid.partition_point(|g| *g <= y).clamp(1, self.grid.len() - 1);
        let (a, b) = (self.grid[i - 1], self.grid[i]);
        let t = if b > a { (y - a) / (b - a) } else { 0.0 };
        self.half_width[i - 1] + t * (self.half_width[i] - self.half_width[i - 1])
    }
}

/// The fill of the silhouette: the plot colour toward white while the Fill settings are plain
/// (Origin's Auto fill is not documented), the plot colour itself once the plot has a
/// transparency, pattern or gradient, which then apply to it.
pub(crate) fn violin_fill_color(plot: &sheaf_core::graph::Plot) -> Color {
    if plot.decor.fill.is_plain() { plot.symbol.color.lerp(&Color::WHITE, VIOLIN_FILL_TINT) } else { plot.symbol.color }
}

/// One violin: the silhouette (filled, outlined in the plot colour), then the marks inside.
pub(crate) fn draw_violin(out: &mut Vec<Item>, pd: &PlotData, geom: &LayerGeometry) {
    let (Some(v), Some(b)) = (&pd.violin, &pd.boxed) else { return };
    let Some(inner) = pd.plot.kind.violin_inner() else { return };
    let half = pd.plot.kind == PlotKind::HalfViolin;
    let color = pd.plot.symbol.color;
    let width = pd.plot.line.width_pt.max(0.5);
    let at = |x: f64, y: f64| geom.checked_data_to_page(pd.display_x(x), pd.display_y(y));

    if v.grid.len() >= 2 {
        let right = v.grid.iter().zip(&v.half_width).map(|(&y, &w)| (b.pos + w, y));
        let left = v.grid.iter().zip(&v.half_width).rev().map(|(&y, &w)| (b.pos - if half { 0.0 } else { w }, y));
        let poly: Vec<(f64, f64)> = right.chain(left).filter_map(|(x, y)| at(x, y)).collect();
        if poly.len() >= 3 {
            fill_polygon(out, &poly, violin_fill_color(pd.plot), &pd.plot.decor.fill);
            let mut d = String::new();
            move_to(&mut d, poly[0].0, poly[0].1);
            for p in &poly[1..] {
                line_to(&mut d, p.0, p.1);
            }
            d.push('Z');
            out.push(Item::Path { d, fill: None, stroke: Some(Stroke::solid(color, width)) });
        }
    }

    // The side of the middle line the marks inside lie on: both, or the right one of a half violin.
    let lo_side = |w: f64| if half { 0.0 } else { -w };
    let line_at = |y: f64, scale: f64| {
        let w = v.half_width_at(y) * scale;
        (at(b.pos + lo_side(w), y), at(b.pos + w, y))
    };
    let seg = |d: &mut String, p: Option<(f64, f64)>, q: Option<(f64, f64)>| {
        if let (Some(p), Some(q)) = (p, q) {
            move_to(d, p.0, p.1);
            line_to(d, q.0, q.1);
        }
    };
    match inner {
        ViolinInner::Median => draw_median(out, b, &at, color, b.pos),
        ViolinInner::Box => {
            // A narrow box on the middle line; the half violin's half box stands left of it.
            let (l, r) = if half {
                (b.pos - VIOLIN_HALF_GAP - VIOLIN_BOX_WIDTH, b.pos - VIOLIN_HALF_GAP)
            } else {
                (b.pos - VIOLIN_BOX_WIDTH / 2.0, b.pos + VIOLIN_BOX_WIDTH / 2.0)
            };
            let mid = (l + r) / 2.0;
            let mut d = String::new();
            seg(&mut d, at(mid, b.whisker_low), at(mid, b.whisker_high));
            out.push(Item::Path { d, fill: None, stroke: Some(Stroke::solid(color, VIOLIN_MARK_PT)) });
            if let (Some(p), Some(q)) = (at(l, b.q1), at(r, b.q3)) {
                let rect = crate::Rect { x: p.0.min(q.0), y: p.1.min(q.1), w: (q.0 - p.0).abs(), h: (q.1 - p.1).abs() };
                out.push(Item::Rect { rect, fill: Some(color), stroke: Some(Stroke::solid(color, VIOLIN_MARK_PT)) });
            }
            if let (Some(p), Some(q)) = (at(l, b.median), at(r, b.median)) {
                let mut d = String::new();
                seg(&mut d, Some(p), Some(q));
                out.push(Item::Path { d, fill: None, stroke: Some(Stroke::solid(Color::WHITE, VIOLIN_MARK_PT)) });
            }
        }
        ViolinInner::Quartile => {
            let mut dashed = String::new();
            for y in [b.q1, b.q3] {
                let (p, q) = line_at(y, 1.0);
                seg(&mut dashed, p, q);
            }
            let mut stroke = Stroke::solid(color, VIOLIN_MARK_PT);
            stroke.dash = vec![3.0, 2.0];
            out.push(Item::Path { d: dashed, fill: None, stroke: Some(stroke) });
            let mut solid = String::new();
            let (p, q) = line_at(b.median, 1.0);
            seg(&mut solid, p, q);
            out.push(Item::Path { d: solid, fill: None, stroke: Some(Stroke::solid(color, 2.0 * VIOLIN_MARK_PT)) });
        }
        ViolinInner::Stick => {
            let mut d = String::new();
            for &y in &v.values {
                let (p, q) = line_at(y, 1.0);
                seg(&mut d, p, q);
            }
            if !d.is_empty() {
                out.push(Item::Path { d, fill: None, stroke: Some(Stroke::solid(color, VIOLIN_STICK_PT)) });
            }
        }
        ViolinInner::Point => {
            // Each value a dot at its height, spread across the local width by the golden ratio
            // sequence, so the picture is the same every time.
            let mut d = String::new();
            for (i, &y) in v.values.iter().enumerate() {
                let t = (i as f64 * 0.618_033_988_749_894_9).fract();
                let w = v.half_width_at(y) * VIOLIN_POINT_SPREAD;
                let x = if half { b.pos + t * w } else { b.pos + (2.0 * t - 1.0) * w };
                if let Some(p) = at(x, y) {
                    symbol_path(&mut d, SymbolShape::Circle, p.0, p.1, VIOLIN_POINT_PT);
                }
            }
            if !d.is_empty() {
                let style = SymbolStyle {
                    shape: SymbolShape::Circle,
                    size_pt: VIOLIN_POINT_PT,
                    color,
                    filled: true,
                    interior: None,
                    fill_color: None,
                    edge_width_pt: 0.5,
                    every: 1,
                };
                out.push(symbol_item(d, &style));
            }
            draw_median(out, b, &at, color, b.pos);
        }
    }
}

/// The median marker: a white dot with an outline in the plot colour.
fn draw_median(out: &mut Vec<Item>, b: &BoxStats, at: &dyn Fn(f64, f64) -> Option<(f64, f64)>, color: Color, x: f64) {
    if let Some(p) = at(x, b.median) {
        let mut d = String::new();
        symbol_path(&mut d, SymbolShape::Circle, p.0, p.1, VIOLIN_MEDIAN_PT);
        out.push(Item::Path { d, fill: Some(Color::WHITE), stroke: Some(Stroke::solid(color, VIOLIN_MARK_PT)) });
    }
}
