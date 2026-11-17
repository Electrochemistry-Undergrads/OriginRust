//! Wind roses (T113, graphing spec "Wind rose"): each speed band is a plot of wedges, one per direction
//! sector, piled on the bands plotted before it in the same layer. The layer is polar: X is the angle in
//! degrees, Y the radius (frequency, in percent for Raw Data).

use crate::data::PlotData;
use crate::scene::{self, Item};
use crate::{LayerGeometry, plots::line_stroke};
use sheaf_core::Project;
use sheaf_core::windrose::{WindRoseSpec, sector_of};

pub(crate) struct WindRoseData {
    /// The centre of each sector in degrees.
    pub angles: Vec<f64>,
    /// The band's value in each sector.
    pub values: Vec<f64>,
    /// The pile below the band in each sector (set by [`pile`]).
    pub base: Vec<f64>,
    /// The angular width of a sector in degrees.
    pub width: f64,
}

impl WindRoseData {
    pub(crate) fn new(project: &Project, pd: &PlotData, spec: WindRoseSpec) -> WindRoseData {
        let rows = pd.start..pd.end;
        let xcol = pd.plot.x.and_then(|id| project.numeric(id));
        let (angles, values) = match spec.speed {
            Some((lo, hi, closed)) => {
                // Raw Data: the percentage of all rows in each sector and this band.
                let n = spec.sectors.max(1);
                let mut counts = vec![0usize; n];
                let mut total = 0usize;
                for r in rows {
                    let (Some(d), Some(&s)) = (xcol.and_then(|x| x.get(r)).copied(), pd.y.get(r)) else { continue };
                    let Some(k) = sector_of(d, n).filter(|_| s.is_finite()) else { continue };
                    total += 1;
                    if s >= lo && (s < hi || (closed && s <= hi)) {
                        counts[k] += 1;
                    }
                }
                let pct = |c: usize| if total > 0 { 100.0 * c as f64 / total as f64 } else { 0.0 };
                ((0..n).map(|k| k as f64 * 360.0 / n as f64).collect(), counts.into_iter().map(pct).collect())
            }
            None => {
                // Binned Data: one sector per row; numeric directions as given, others spread evenly.
                let rows: Vec<usize> = rows.collect();
                let n = rows.len().max(1);
                let angles: Vec<f64> = rows
                    .iter()
                    .enumerate()
                    .map(|(k, &r)| match xcol.and_then(|x| x.get(r)).copied().filter(|d| d.is_finite()) {
                        Some(d) => d.rem_euclid(360.0),
                        None => k as f64 * 360.0 / n as f64,
                    })
                    .collect();
                let values: Vec<f64> =
                    rows.iter().map(|&r| pd.y.get(r).copied().filter(|v| v.is_finite()).unwrap_or(0.0)).collect();
                (angles, values)
            }
        };
        let width = 360.0 / angles.len().max(1) as f64;
        let base = vec![0.0; values.len()];
        WindRoseData { angles, values, base, width }
    }

    /// The span on the angle axis (`x`) or the radius axis.
    pub(crate) fn extent(&self, x: bool) -> (f64, f64) {
        if x {
            (0.0, 360.0)
        } else {
            let top = self.values.iter().zip(&self.base).map(|(v, b)| v + b).fold(0.0, f64::max);
            (0.0, top)
        }
    }
}

/// Piles the wind rose bands of a layer: each band starts, sector by sector, where the bands plotted
/// before it end.
pub(crate) fn pile(plots: &mut [PlotData]) {
    let mut acc: Vec<f64> = Vec::new();
    for pd in plots.iter_mut() {
        let Some(w) = pd.wind_rose.as_mut() else { continue };
        if acc.len() < w.values.len() {
            acc.resize(w.values.len(), 0.0);
        }
        for (k, v) in w.values.iter().enumerate() {
            w.base[k] = acc[k];
            acc[k] += v;
        }
    }
}

/// The wedges of one band.
pub(crate) fn draw(out: &mut Vec<Item>, pd: &PlotData, geom: &LayerGeometry) {
    let Some(w) = &pd.wind_rose else { return };
    let stroke = line_stroke(&pd.plot.line);
    for ((&c, &v), &b) in w.angles.iter().zip(&w.values).zip(&w.base) {
        if v.is_nan() || v <= 0.0 {
            continue;
        }
        let (a0, a1) = (c - w.width / 2.0, c + w.width / 2.0);
        // The arcs, sampled every degree or so.
        let steps = (w.width.ceil() as usize).max(4);
        let at = |k: usize| a0 + (a1 - a0) * k as f64 / steps as f64;
        let mut d = String::new();
        for k in 0..=steps {
            let (x, y) = geom.data_to_page(at(k), b + v);
            if k == 0 { scene::move_to(&mut d, x, y) } else { scene::line_to(&mut d, x, y) }
        }
        for k in (0..=steps).rev() {
            let (x, y) = geom.data_to_page(at(k), b);
            scene::line_to(&mut d, x, y);
        }
        d.push('Z');
        out.push(Item::Path { d, fill: Some(pd.plot.symbol.color), stroke: Some(stroke.clone()) });
    }
}
