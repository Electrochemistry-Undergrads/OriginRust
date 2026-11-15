//! Ridgeline charts (graphing spec 14.23): each plot's Y column drawn as a density curve (or a
//! histogram) on a baseline at its column index, the first column at the bottom, nearer ridges
//! drawn over the farther ones.
//!
//! Origin documents the display types, the bandwidth rules (shared with the violin plot in
//! `sheaf_calc::kde`), quantile lines and the rug settings; the look (height, overlap, fill,
//! evaluation grid) is not documented. Every choice below that Origin does not give is Sheaf's own
//! and unmeasured: they are named `RIDGE_*` and listed in the spec.

use crate::LayerGeometry;
use crate::data::PlotData;
use crate::fill::fill_polygon;
use crate::plots::{symbol_item, symbol_path};
use crate::scene::{Item, Stroke, line_to, move_to};
use sheaf_calc::kde::{bandwidth, density_at};
use sheaf_calc::stats::quantile_origin;
use sheaf_core::graph::{FillStyle, RidgeDisplay, RidgelineStyle};

/// Points where the density curve is evaluated, evenly from `min - 3 h` to `max + 3 h`.
pub(crate) const RIDGE_GRID_POINTS: usize = 200;
/// How far the curve reaches past the smallest and largest value, in bandwidths.
pub(crate) const RIDGE_REACH: f64 = 3.0;
/// Width of the quantile lines, in pt.
const RIDGE_QUANTILE_PT: f64 = 1.0;

/// One bar of a "Single Block Bar" ridge: its edges and its density (count / (n * bin size)).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct RidgeBar {
    pub lo: f64,
    pub hi: f64,
    pub density: f64,
}

/// What a ridgeline plot draws, from the finite, unmasked, visible values of its Y column.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct RidgeData {
    /// The baseline, in Y axis units (the column index, 1 for the first plot).
    pub pos: f64,
    /// The tick label at the baseline.
    pub label: String,
    /// The values, ascending.
    pub values: Vec<f64>,
    /// The kernel bandwidth; `None` when the column has no usable one (fewer than two values or no spread).
    pub bandwidth: Option<f64>,
    /// Where the density was evaluated and its value there (empty without a bandwidth).
    pub grid: Vec<f64>,
    pub density: Vec<f64>,
    /// The bars of the block bar display (empty for the other displays).
    pub bars: Vec<RidgeBar>,
    /// Page height per unit of density, in Y axis units: set once all ridges of the layer are known
    /// so that the tallest peak is as high as `RidgelineStyle::height`.
    pub scale: f64,
}

impl RidgeData {
    pub fn new(
        values: &[f64],
        pos: f64,
        label: String,
        style: &RidgelineStyle,
        hist: &sheaf_core::graph::HistogramStyle,
    ) -> RidgeData {
        let mut v: Vec<f64> = values.iter().copied().filter(|x| x.is_finite()).collect();
        v.sort_by(|a, b| a.total_cmp(b));
        let h = bandwidth(&v, style.bandwidth.to_calc());
        let mut out = RidgeData {
            pos,
            label,
            values: v,
            bandwidth: h,
            grid: Vec::new(),
            density: Vec::new(),
            bars: Vec::new(),
            scale: 0.0,
        };
        if style.display == RidgeDisplay::SingleBlockBar {
            if let Some(f) = crate::data::histogram_freq(&out.values, hist) {
                let n = f.total.max(1) as f64;
                out.bars = f
                    .rows
                    .iter()
                    .map(|r| RidgeBar {
                        lo: r.center - f.bin_size / 2.0,
                        hi: r.center + f.bin_size / 2.0,
                        density: r.count as f64 / (n * f.bin_size),
                    })
                    .collect();
            }
        } else if let (Some(h), Some(&lo), Some(&hi)) = (h, out.values.first(), out.values.last()) {
            let (a, b) = (lo - RIDGE_REACH * h, hi + RIDGE_REACH * h);
            out.grid =
                (0..RIDGE_GRID_POINTS).map(|i| a + (b - a) * i as f64 / (RIDGE_GRID_POINTS - 1) as f64).collect();
            out.density = out.grid.iter().map(|&x| density_at(&out.values, h, x)).collect();
        }
        out
    }

    /// The highest density drawn.
    pub fn peak(&self) -> f64 {
        let c = self.density.iter().copied().fold(0.0, f64::max);
        self.bars.iter().map(|b| b.density).fold(c, f64::max)
    }

    /// The left and right ends of what is drawn along X: the ends of the curve or the bars, or of the data.
    pub fn x_extent(&self) -> Option<(f64, f64)> {
        if let (Some(a), Some(b)) = (self.bars.first(), self.bars.last()) {
            return Some((a.lo, b.hi));
        }
        if let (Some(&a), Some(&b)) = (self.grid.first(), self.grid.last()) {
            return Some((a, b));
        }
        Some((*self.values.first()?, *self.values.last()?))
    }

    /// The density drawn at `x`: the curve's own value, or the height of the bar that holds `x`.
    pub fn density_at_x(&self, style: &RidgelineStyle, x: f64) -> f64 {
        if style.display == RidgeDisplay::SingleBlockBar {
            let last = self.bars.len().saturating_sub(1);
            return self
                .bars
                .iter()
                .enumerate()
                .find(|(i, b)| x >= b.lo && (x < b.hi || *i == last && x <= b.hi))
                .map_or(0.0, |(_, b)| b.density);
        }
        match self.bandwidth {
            Some(h) => density_at(&self.values, h, x),
            None => 0.0,
        }
    }

    /// How far the rug reaches below the baseline, in baseline spacings (0 without a rug).
    pub fn rug_reach(&self, style: &RidgelineStyle) -> f64 {
        if style.display == RidgeDisplay::WithRug { (style.rug.gap_pct + style.rug.size_pct) / 100.0 } else { 0.0 }
    }
}

/// Sets the scale of every ridge of a layer: the tallest peak reaches `height` spacings (the height of
/// the first ridge plot's settings).
pub(crate) fn set_scales(plots: &mut [PlotData]) {
    let height = plots.iter().find(|pd| pd.ridge.is_some()).map_or(0.0, |pd| pd.plot.ridge.height);
    let top = plots.iter().filter_map(|pd| pd.ridge.as_ref()).map(RidgeData::peak).fold(0.0, f64::max);
    let scale = if top > 0.0 && height.is_finite() && height > 0.0 { height / top } else { 0.0 };
    for r in plots.iter_mut().filter_map(|pd| pd.ridge.as_mut()) {
        r.scale = scale;
    }
}

/// One ridge: the fill and the curve (or the bars), the quantile lines, then the points or the rug.
pub(crate) fn draw_ridge(out: &mut Vec<Item>, pd: &PlotData, geom: &LayerGeometry) {
    let Some(r) = &pd.ridge else { return };
    let style = &pd.plot.ridge;
    let color = pd.plot.symbol.color;
    let width = pd.plot.line.width_pt.max(0.5);
    let at = |x: f64, y: f64| geom.checked_data_to_page(pd.display_x(x), pd.display_y(y));
    let fill = FillStyle {
        transparency_pct: style.fill_transparency_pct.round().clamp(0.0, 100.0) as u8,
        ..pd.plot.decor.fill
    };

    // The outline of what is drawn, from the left end on the baseline to the right end on it.
    let mut outline: Vec<(f64, f64)> = Vec::new();
    if style.display == RidgeDisplay::SingleBlockBar {
        for b in &r.bars {
            let top = r.pos + b.density * r.scale;
            outline.extend([(b.lo, top), (b.hi, top)]);
        }
    } else {
        outline.extend(r.grid.iter().zip(&r.density).map(|(&x, &d)| (x, r.pos + d * r.scale)));
    }
    if let (Some(first), Some(last)) = (outline.first().copied(), outline.last().copied()) {
        let mut poly = vec![(first.0, r.pos)];
        poly.extend(&outline);
        poly.push((last.0, r.pos));
        let page: Vec<(f64, f64)> = poly.iter().filter_map(|&(x, y)| at(x, y)).collect();
        fill_polygon(out, &page, color, &fill);
        let line: Vec<(f64, f64)> = outline.iter().filter_map(|&(x, y)| at(x, y)).collect();
        if line.len() >= 2 {
            let mut d = String::new();
            move_to(&mut d, line[0].0, line[0].1);
            for p in &line[1..] {
                line_to(&mut d, p.0, p.1);
            }
            out.push(Item::Path { d, fill: None, stroke: Some(Stroke::solid(color, width)) });
        }
    }

    if style.quantile_lines && !r.values.is_empty() {
        let mut d = String::new();
        for &p in style.quantiles.iter().filter(|p| p.is_finite() && (0.0..=100.0).contains(*p)) {
            let q = quantile_origin(&r.values, p / 100.0);
            if let (Some(a), Some(b)) = (at(q, r.pos), at(q, r.pos + r.density_at_x(style, q) * r.scale)) {
                move_to(&mut d, a.0, a.1);
                line_to(&mut d, b.0, b.1);
            }
        }
        if !d.is_empty() {
            out.push(Item::Path { d, fill: None, stroke: Some(Stroke::solid(color, RIDGE_QUANTILE_PT)) });
        }
    }

    match style.display {
        RidgeDisplay::WithDataPoints => {
            let mut d = String::new();
            for &x in &r.values {
                if let Some(p) = at(x, r.pos) {
                    symbol_path(&mut d, pd.plot.symbol.shape, p.0, p.1, pd.plot.symbol.size_pt);
                }
            }
            if !d.is_empty() {
                out.push(symbol_item(d, &pd.plot.symbol));
            }
        }
        RidgeDisplay::WithRug => {
            let rug = &style.rug;
            let (top, bottom) = (r.pos - rug.gap_pct / 100.0, r.pos - (rug.gap_pct + rug.size_pct) / 100.0);
            let mut d = String::new();
            for &x in &r.values {
                if let (Some(a), Some(b)) = (at(x, top), at(x, bottom)) {
                    move_to(&mut d, a.0, a.1);
                    line_to(&mut d, b.0, b.1);
                }
            }
            if !d.is_empty() {
                out.push(Item::Path {
                    d,
                    fill: None,
                    stroke: Some(Stroke::solid(rug.color.unwrap_or(color), rug.thickness_pt.max(0.1))),
                });
            }
        }
        _ => {}
    }
}
