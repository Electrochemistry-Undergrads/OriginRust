//! High-Low-Close, Japanese Candlestick and OHLC Bar Chart (T108, graphing spec "Stock plots"). Every row
//! is drawn at its X: a high-low line, and a close tick (HLC), open and close ticks (OHLC) or an
//! open-close body (candlestick). A falling day (close below open) takes the spec's `down` colour, the
//! candle's body filled; a rising day the plot's line colour, the body hollow.

use crate::data::PlotData;
use crate::scene::{self, Item};
use crate::{LayerGeometry, plots::line_stroke};
use sheaf_core::Project;
use sheaf_core::graph::PlotKind;
use sheaf_core::stock::StockSpec;

pub(crate) struct StockData {
    pub spec: StockSpec,
    /// Open (NaN for HLC), high, low and close of each row, as displayed.
    pub rows: Vec<[f64; 4]>,
}

impl StockData {
    pub(crate) fn new(project: &Project, pd: &PlotData, spec: StockSpec, rows: usize) -> StockData {
        let col = |id| project.numeric(id).unwrap_or_default();
        let at = |v: &[f64], i: usize| pd.display_y(v.get(i).copied().unwrap_or(f64::NAN));
        let open = spec.open.map(col).unwrap_or_default();
        let (high, low, close) = (col(spec.high), col(spec.low), col(spec.close));
        let rows = (0..rows)
            .map(|i| {
                let o = if spec.open.is_some() { at(open, i) } else { f64::NAN };
                [o, at(high, i), at(low, i), at(close, i)]
            })
            .collect();
        StockData { spec, rows }
    }

    /// The prices of rows `start..end`, for the axis ranges.
    pub(crate) fn values(&self, start: usize, end: usize) -> impl Iterator<Item = f64> + '_ {
        let end = end.min(self.rows.len());
        self.rows[start.min(end)..end].iter().flatten().copied()
    }
}

/// The smallest positive gap between neighbouring rows' X positions on the page (1 point if none).
fn spacing(xs: &[f64]) -> f64 {
    let gap =
        xs.windows(2).map(|w| (w[1] - w[0]).abs()).filter(|d| *d > 0.0 && d.is_finite()).fold(f64::INFINITY, f64::min);
    if gap.is_finite() { gap } else { 1.0 }
}

/// The bars, ticks or candles of a stock plot.
pub(crate) fn draw_stock(out: &mut Vec<Item>, pd: &PlotData, geom: &LayerGeometry) {
    let Some(s) = &pd.stock else { return };
    let kind = pd.plot.kind;
    let up = line_stroke(&pd.plot.line);
    let mut down = up.clone();
    down.color = s.spec.down;
    let page_x: Vec<f64> = (pd.start..pd.end).map(|i| geom.data_to_page(pd.x_at(i), 0.0).0).collect();
    let size = spacing(&page_x) * s.spec.width;
    let line = |out: &mut Vec<Item>, st: &scene::Stroke, pts: [(f64, f64); 2]| {
        let mut d = String::new();
        scene::move_to(&mut d, pts[0].0, pts[0].1);
        scene::line_to(&mut d, pts[1].0, pts[1].1);
        out.push(Item::Path { d, fill: None, stroke: Some(st.clone()) });
    };
    for (k, i) in (pd.start..pd.end).enumerate() {
        let Some(&[o, h, l, c]) = s.rows.get(i) else { continue };
        let x = page_x[k];
        if !x.is_finite() || !h.is_finite() || !l.is_finite() {
            continue;
        }
        let py = |v: f64| geom.data_to_page(pd.x_at(i), v).1;
        let falling = c < o;
        let st = if falling { &down } else { &up };
        match kind {
            PlotKind::Candlestick => {
                if !o.is_finite() || !c.is_finite() {
                    continue;
                }
                let (top, bottom) = (py(o.max(c)), py(o.min(c)));
                line(out, st, [(x, py(h)), (x, top)]);
                line(out, st, [(x, bottom), (x, py(l))]);
                let half = size / 2.0;
                let mut d = String::new();
                scene::move_to(&mut d, x - half, top);
                scene::line_to(&mut d, x + half, top);
                scene::line_to(&mut d, x + half, bottom);
                scene::line_to(&mut d, x - half, bottom);
                d.push('Z');
                out.push(Item::Path { d, fill: falling.then_some(s.spec.down), stroke: Some(st.clone()) });
            }
            _ => {
                line(out, st, [(x, py(h)), (x, py(l))]);
                if kind == PlotKind::OhlcBar && o.is_finite() {
                    line(out, st, [(x - size, py(o)), (x, py(o))]);
                }
                if c.is_finite() {
                    line(out, st, [(x, py(c)), (x + size, py(c))]);
                }
            }
        }
    }
}
