//! Colour scales and bubble scales of a layer (T80, graphing spec 26): the objects behind Graph > New
//! Color Scale and the legend's Add Color Scale / Add Bubble Scale.
//!
//! A colour scale is the bar the hexbin, sunburst and waterfall plots draw (`hexbin::draw_color_bar`) over
//! the smallest to the largest Z of the layer's colour mapped plot. A bubble scale is Origin's measured
//! size legend: four circles, the smallest Z and three equal steps up to the largest, labelled with two
//! decimals, titled with the Z column. Where Origin puts them by default, and their other options, were
//! not measured: Sheaf stacks them right of the frame in the order they were added.

use crate::LayerGeometry;
use crate::data::PlotData;
use crate::fonts::TextMeasurer;
use crate::hexbin::{BAR_GAP_PT, LABEL_GAP_PT, draw_color_bar, scale_reserve};
use crate::hit;
use crate::plots::{BUBBLE_MAX_PT, BUBBLE_MIN_PT, symbol_item, symbol_path, z_fraction, z_range};
use crate::scene::{Anchor, Item, ItemList, ObjectKind, SceneObject};
use crate::text::{self, ASCENT, DESCENT};
use sheaf_core::Project;
use sheaf_core::graph::Layer;
use sheaf_core::layer_scale::{LayerScale, LayerScaleKind};

/// Circles of a bubble scale.
const BUBBLES: usize = 4;
/// Gap between two circles of a bubble scale.
const BUBBLE_GAP_PT: f64 = 4.0;

/// The room one scale takes right of the frame, for a tick font of `size` pt.
fn slot_width(kind: LayerScaleKind, size: f64) -> f64 {
    match kind {
        LayerScaleKind::Color => scale_reserve(size),
        LayerScaleKind::Bubble => BAR_GAP_PT + BUBBLE_MAX_PT + LABEL_GAP_PT + 0.6 * size * 5.0 + 6.0,
    }
}

/// The room all scales of `layer` that describe a plot take right of the frame.
pub(crate) fn reserve(layer: &Layer) -> f64 {
    let size = layer.y_axis.tick_font.size_pt;
    layer.scales.iter().filter(|s| LayerScale::applies(layer, s.kind)).map(|s| slot_width(s.kind, size)).sum()
}

/// The title: the scale's own, or the name of the Z column of the plot.
fn title_of(project: &Project, layer: &Layer, s: &LayerScale) -> Option<String> {
    if let Some(t) = &s.title {
        return Some(t.clone());
    }
    let plot = LayerScale::plot_of(layer, s.kind)?;
    let id = match s.kind {
        LayerScaleKind::Color => plot.color_from?,
        LayerScaleKind::Bubble => plot.size_from?,
    };
    Some(project.column(id).map(|c| c.display_name().to_string()).unwrap_or_default())
}

/// Draws the scales of a layer into its overlay and records each as a pickable object.
#[allow(clippy::too_many_arguments)]
pub(crate) fn draw(
    out: &mut Vec<Item>,
    objects: &mut Vec<SceneObject>,
    project: &Project,
    layer: &Layer,
    index: usize,
    pds: &[PlotData],
    geom: &LayerGeometry,
    m: &dyn TextMeasurer,
) {
    if geom.polar.is_some() || geom.ternary.is_some() {
        return;
    }
    let size = layer.y_axis.tick_font.size_pt;
    let mut offset = 0.0;
    for (k, s) in layer.scales.iter().enumerate() {
        let Some(plot) = LayerScale::plot_of(layer, s.kind) else { continue };
        let Some(pd) = pds.iter().find(|pd| std::ptr::eq(pd.plot, plot)) else { continue };
        let z = match s.kind {
            LayerScaleKind::Color => pd.color_z,
            LayerScaleKind::Bubble => pd.size_z,
        };
        let Some(z) = z else { continue };
        let (lo, hi) = z_range(z, pd);
        if !(lo.is_finite() && hi.is_finite()) {
            continue;
        }
        let title = title_of(project, layer, s).unwrap_or_default();
        let start = out.len();
        match s.kind {
            LayerScaleKind::Color => {
                let ticks = crate::sunburst::scale_ticks(lo, hi);
                draw_color_bar(out, layer, geom, m, &title, &ticks, (offset, 0.0));
            }
            LayerScaleKind::Bubble => draw_bubbles(out, layer, pd, geom, m, &title, (lo, hi), offset),
        }
        offset += slot_width(s.kind, size);
        hit::record(objects, out, start, ObjectKind::Scale(k), index, ItemList::Overlay, &[], m);
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_bubbles(
    out: &mut Vec<Item>,
    layer: &Layer,
    pd: &PlotData,
    geom: &LayerGeometry,
    m: &dyn TextMeasurer,
    title: &str,
    (lo, hi): (f64, f64),
    offset: f64,
) {
    let f = geom.frame;
    let ya = &layer.y_axis;
    let (size, title_size) = (ya.tick_font.size_pt, ya.title_font.size_pt);
    let x0 = f.x + f.w + offset + BAR_GAP_PT;
    text::place(out, m, title, &ya.title_font, title_size, (x0, f.y + ASCENT * title_size), Anchor::Start, 0.0);
    let mut y = f.y + (ASCENT + DESCENT) * title_size + 0.4 * title_size;
    // All values equal: one circle at the middle size.
    let n = if hi > lo { BUBBLES } else { 1 };
    for k in 0..n {
        let v = if n == 1 { lo } else { lo + (hi - lo) * k as f64 / (BUBBLES - 1) as f64 };
        let mut style = pd.plot.symbol.clone();
        style.size_pt = BUBBLE_MIN_PT + (BUBBLE_MAX_PT - BUBBLE_MIN_PT) * z_fraction((lo, hi), v);
        let cy = y + style.size_pt / 2.0;
        let cx = x0 + BUBBLE_MAX_PT / 2.0;
        let mut d = String::new();
        symbol_path(&mut d, style.shape, cx, cy, style.size_pt);
        out.push(symbol_item(d, &style));
        text::place(
            out,
            m,
            &format!("{v:.2}"),
            &ya.tick_font,
            size,
            (x0 + BUBBLE_MAX_PT + LABEL_GAP_PT, cy + (ASCENT - DESCENT) / 2.0 * size),
            Anchor::Start,
            0.0,
        );
        y += style.size_pt + BUBBLE_GAP_PT;
    }
}
