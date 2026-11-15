//! On-graph trendlines (T43, graphing spec 18.11): the fitted or averaged curve of a plot and its
//! equation / R-squared label.
//!
//! The fit uses the plotted points as drawn (offsets, multipliers and stacking applied, before the
//! axis scale), left out when masked, filtered, or not finite. The curve is sampled at 200 places
//! evenly spaced along the X axis' own scale (so a log axis gets geometric spacing) between the
//! smallest and largest X of those points, and goes through the layer transform like the data.

use crate::LayerGeometry;
use crate::data::PlotData;
use crate::fonts::TextMeasurer;
use crate::plots::line_stroke;
use crate::scene::{Anchor, Item, line_to, move_to};
use sheaf_calc::trend::{self, TrendKind};
use sheaf_core::Color;
use sheaf_core::graph::{Font, Layer, LineStyle, Trendline};

/// Number of places a fitted curve is evaluated at.
const SAMPLES: usize = 200;
/// Page coordinates are limited to this magnitude so a fast-growing curve stays finite.
const LIMIT: f64 = 1.0e5;
/// Gap between the layer's top-left corner and the first label line, in points.
const LABEL_MARGIN: f64 = 6.0;

/// The plotted, unmasked, finite points of a plot, as drawn.
pub(crate) fn points(pd: &PlotData) -> (Vec<f64>, Vec<f64>) {
    let mut xs = Vec::new();
    let mut ys = Vec::new();
    for r in pd.rows() {
        if pd.mask.is_some_and(|m| m.contains(&r)) {
            continue;
        }
        let (x, y) = (pd.x_at(r), pd.y_at(r));
        if x.is_finite() && y.is_finite() {
            xs.push(x);
            ys.push(y);
        }
    }
    (xs, ys)
}

/// The curve of one trendline as data points, and its label text (empty when nothing is shown).
pub(crate) fn curve(t: &Trendline, xs: &[f64], ys: &[f64], geom: &LayerGeometry) -> Option<(Vec<(f64, f64)>, String)> {
    if let TrendKind::MovingAverage { window } = t.kind {
        let pts = trend::moving_average(xs, ys, usize::from(window));
        return (pts.len() >= 2).then_some((pts, String::new()));
    }
    let fit = trend::fit(t.kind, xs, ys)?;
    let lo = xs.iter().copied().fold(f64::INFINITY, f64::min);
    let hi = xs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let (u0, u1) = (geom.x.to_unit(lo), geom.x.to_unit(hi));
    let on_axis = u0.is_finite() && u1.is_finite() && geom.polar.is_none();
    let pts = (0..SAMPLES)
        .map(|i| {
            let f = i as f64 / (SAMPLES - 1) as f64;
            let x = if on_axis { geom.x.from_unit(u0 + f * (u1 - u0)) } else { lo + f * (hi - lo) };
            (x, fit.eval(x))
        })
        .collect();
    let mut parts = Vec::new();
    if t.show_equation {
        parts.push(fit.equation(4));
    }
    if t.show_r2
        && let Some(r2) = fit.r2
    {
        parts.push(format!("R\\+(2) = {}", trend::sig_str(r2, 4)));
    }
    Some((pts, parts.join(", ")))
}

fn clamp(p: (f64, f64)) -> (f64, f64) {
    (p.0.clamp(-LIMIT, LIMIT), p.1.clamp(-LIMIT, LIMIT))
}

/// Draws the trendlines of every plot of a layer after the data, and their labels one line per
/// trendline from the top left corner of the frame down.
pub(crate) fn draw(out: &mut Vec<Item>, layer: &Layer, pds: &[PlotData], geom: &LayerGeometry, m: &dyn TextMeasurer) {
    if geom.polar.is_some() || geom.ternary.is_some() {
        return;
    }
    let font = &layer.legend.font;
    let mut line = 0usize;
    for pd in pds {
        if pd.plot.decor.trendlines.is_empty()
            || pd.pie.is_some()
            || pd.boxed.is_some()
            || pd.violin.is_some()
            || pd.hist_raw.is_some()
        {
            continue;
        }
        let (xs, ys) = points(pd);
        for t in &pd.plot.decor.trendlines {
            let Some((pts, text)) = curve(t, &xs, &ys, geom) else { continue };
            let color: Color = t.color.unwrap_or_else(|| crate::rug::plot_color(pd));
            let mut d = String::new();
            let mut pen_down = false;
            for (x, y) in pts {
                match geom.checked_data_to_page(x, y) {
                    Some(p) => {
                        let (px, py) = clamp(p);
                        if pen_down {
                            line_to(&mut d, px, py);
                        } else {
                            move_to(&mut d, px, py);
                            pen_down = true;
                        }
                    }
                    None => pen_down = false,
                }
            }
            if !d.is_empty() && t.width_pt > 0.0 {
                let style = LineStyle { dash: t.dash, ..LineStyle::solid(color, t.width_pt) };
                out.push(Item::Path { d, fill: None, stroke: Some(line_stroke(&style)) });
            }
            if !text.is_empty() {
                let size = font.size_pt;
                let f = geom.frame;
                let font = Font { color, ..font.clone() };
                let at = (
                    f.x + LABEL_MARGIN + t.label_offset_pt.0,
                    f.y + LABEL_MARGIN + size + line as f64 * size * 1.3 + t.label_offset_pt.1,
                );
                crate::text::place(out, m, &text, &font, size, at, Anchor::Start, 0.0);
                line += 1;
            }
        }
    }
}
