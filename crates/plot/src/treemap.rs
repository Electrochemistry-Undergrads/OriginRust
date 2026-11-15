//! Treemap (Origin 2025b "Treemap plot", graphing spec 14.26): the hierarchy of the category columns of a plot's
//! [`TreemapStyle`] as nested rectangles, the area of each proportional to the sum of the plot's value column below it.
//!
//! Origin documents the plot type, the Plot Details Layout tab (Layout, Order, Depth, Gap, a layout and order per
//! level) and nothing of how it looks. Everything below is Sheaf's own and unmeasured, listed in the spec; the
//! layout and the hierarchy are in `sheaf_calc::treemap` and `sheaf_calc::treemap_tree`.
//!
//! - The rectangles fill the layer's frame; no axes, no frame line, no legend.
//! - One colour of the layer's palette per top-level category (by first appearance, whatever the order); a node
//!   `L` levels deep is that colour blended `1 - 0.7^L` of the way to white. Every rectangle has a white border of
//!   0.5 pt.
//! - A category with its children shown has a header band of 1.55 text heights holding its name, drawn in the
//!   band's colour; the band of each level below the first is 0.85 of the one above, with its text scaled the same,
//!   and a level-1+ rectangle too small for its band (twice the band high or wide) has none and its name is not
//!   drawn. Leaf names, and the values, are written in the rectangle they name.
//! - A leaf (the deepest level shown) shows its name at the top left and, below it, its value, in the layer's X
//!   axis tick font at 0.45 of its size; black or white, whichever reads better on the fill. The name is left out when it is wider than
//!   the rectangle or the rectangle is not as high as the text; the value line is left out when it does not fit
//!   under the name. Nothing is cut or clipped: a label is drawn whole or not at all.
//! - A value is written with up to four significant digits and no exponent.

use crate::LayerGeometry;
use crate::Rect;
use crate::data::PlotData;
use crate::fonts::TextMeasurer;
use crate::scene::{Anchor, Item, Stroke};
use crate::text::{self, ASCENT, DESCENT};
use sheaf_calc::treemap::Rect as CalcRect;
use sheaf_calc::treemap_tree::{self, Cell, Key, Level, Order, Params, Row, Tree};
use sheaf_core::graph::{Font, Layer, TreemapOrder, TreemapStyle};
use sheaf_core::{Color, Project};

/// Padding between a label and the edge of its rectangle, in font sizes.
pub(crate) const PAD_EM: f64 = 0.3;
/// Height of a header band, in text heights (ascent plus descent).
const HEADER_EM: f64 = 1.55;
/// Blend towards white per level: level `L` is `1 - TINT^L` of the way.
const TINT: f64 = 0.7;
/// The labels are this fraction of the X axis tick font's size (22 pt there, so 9.9 pt: a treemap has many labels).
const LABEL_SCALE: f64 = 0.45;
/// Width of the white border, in pt.
const BORDER_PT: f64 = 0.5;

/// What a treemap plot draws, held by its plot.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TreemapData {
    pub tree: Tree,
    pub style: TreemapStyle,
    /// The colour of each top-level category, by first appearance.
    pub colors: Vec<Color>,
    pub font: Font,
}

impl TreemapData {
    /// The hierarchy of the rows `pd` shows: category cells from the style's columns, values from the plot's Y.
    pub fn new(project: &Project, layer: &Layer, pd: &PlotData) -> Option<TreemapData> {
        let style = pd.plot.treemap.clone();
        let columns: Vec<_> = style.categories.iter().filter_map(|id| project.column(*id)).collect();
        if columns.is_empty() {
            return None;
        }
        let order_col = style.order_column.and_then(|id| project.column(id));
        let rows: Vec<Row> = pd
            .rows()
            .filter(|r| !pd.mask.is_some_and(|m| m.contains(r)))
            .map(|r| Row {
                path: columns.iter().map(|c| c.cell_text(r)).collect(),
                value: pd.y.get(r).copied().unwrap_or(f64::NAN),
                key: match order_col {
                    None => Key::Missing,
                    Some(c) => match c.data.text_cell(r) {
                        Some(t) => Key::Text(t.to_string()),
                        None => match c.numeric().and_then(|n| n.get(r)).copied().filter(|v| v.is_finite()) {
                            Some(v) => Key::Num(v),
                            None => Key::Missing,
                        },
                    },
                },
            })
            .collect();
        let tree = treemap_tree::build(&rows, columns.len());
        let n = tree.roots.len();
        let colors = (0..n).map(|i| layer.palette.color(i, n)).collect();
        let mut font = layer.x_axis.tick_font.clone();
        font.size_pt *= LABEL_SCALE;
        Some(TreemapData { tree, style, colors, font })
    }

    /// The nested layout in `area`, with a header band of `header` pt.
    pub fn cells(&self, area: Rect, header: f64) -> Vec<Cell> {
        let order = |o: TreemapOrder| match o {
            TreemapOrder::Categorical => Order::Categorical,
            TreemapOrder::Alphabetical => Order::Alphabetical,
            TreemapOrder::BySize => Order::BySize,
            TreemapOrder::ByColumn => Order::ByKey,
        };
        let levels = (0..self.tree.levels.max(1))
            .map(|i| {
                let l = self.style.level_setting(i);
                Level { layout: l.layout, order: order(l.order) }
            })
            .collect();
        let params =
            Params { depth: self.style.depth.map(|d| d as usize), gap: self.style.gap_pt.max(0.0), header, levels };
        treemap_tree::nested(&self.tree, CalcRect::new(area.x, area.y, area.w, area.h), &params)
    }

    /// The fill of a cell: its top-level colour blended towards white by its level.
    pub fn fill(&self, cell: &Cell) -> Color {
        let base = self.colors.get(cell.top).copied().unwrap_or(Color::BLACK);
        base.lerp(&Color::WHITE, 1.0 - TINT.powi(cell.level as i32))
    }
}

fn to_rect(r: CalcRect) -> Rect {
    Rect { x: r.x, y: r.y, w: r.w, h: r.h }
}

/// The height of the header band for text of size `size`.
pub(crate) fn header_height(size: f64) -> f64 {
    HEADER_EM * (ASCENT + DESCENT) * size + 2.0 * PAD_EM * size
}

/// A value as a label: up to four significant digits, no exponent.
pub(crate) fn value_text(v: f64) -> String {
    if !v.is_finite() {
        return String::new();
    }
    if v == 0.0 {
        return "0".to_string();
    }
    let digits = (3 - v.abs().log10().floor() as i32).clamp(0, 12) as usize;
    let s = format!("{v:.digits$}");
    if s.contains('.') { s.trim_end_matches('0').trim_end_matches('.').to_string() } else { s }
}

/// Black or white text, whichever contrasts better with `fill`.
pub(crate) fn text_color(fill: Color) -> Color {
    let lum = 0.299 * f64::from(fill.r) + 0.587 * f64::from(fill.g) + 0.114 * f64::from(fill.b);
    if lum > 140.0 { Color::BLACK } else { Color::WHITE }
}

/// Draws the treemap held by the plot.
pub(crate) fn draw_treemap(out: &mut Vec<Item>, pd: &PlotData, geom: &LayerGeometry, m: &dyn TextMeasurer) {
    let Some(tm) = pd.treemap.as_deref() else { return };
    let size = tm.font.size_pt;
    let band = header_height(size);
    let cells = tm.cells(geom.frame, band);
    let border = Stroke::solid(Color::WHITE, BORDER_PT);
    let pad = PAD_EM * size;
    let line = (ASCENT + DESCENT) * size;
    for c in &cells {
        let rect = to_rect(c.rect);
        if rect.w <= 0.0 || rect.h <= 0.0 {
            continue;
        }
        let fill = tm.fill(c);
        out.push(Item::Rect { rect, fill: Some(fill), stroke: Some(border.clone()) });
        let mut font = tm.font.clone();
        font.color = text_color(fill);
        if let Some(h) = c.header {
            let h = to_rect(h);
            let top = tm.colors.get(c.top).copied().unwrap_or(fill);
            font.color = text_color(top);
            let hs = size * treemap_tree::HEADER_SHRINK.powi(c.level as i32);
            let (hpad, hline) = (PAD_EM * hs, (ASCENT + DESCENT) * hs);
            if text::width(m, &c.name, &font, hs) + 2.0 * hpad <= h.w {
                text::place(
                    out,
                    m,
                    &c.name,
                    &font,
                    hs,
                    (h.x + hpad, h.y + (h.h - hline) / 2.0 + ASCENT * hs),
                    Anchor::Start,
                    0.0,
                );
            }
        } else if c.leaf {
            if text::width(m, &c.name, &font, size) + 2.0 * pad > rect.w || line + 2.0 * pad > rect.h {
                continue;
            }
            text::place(out, m, &c.name, &font, size, (rect.x + pad, rect.y + pad + ASCENT * size), Anchor::Start, 0.0);
            let value = value_text(c.value);
            if !value.is_empty()
                && text::width(m, &value, &font, size) + 2.0 * pad <= rect.w
                && 2.0 * line + 2.0 * pad <= rect.h
            {
                text::place(
                    out,
                    m,
                    &value,
                    &font,
                    size,
                    (rect.x + pad, rect.y + pad + line + ASCENT * size),
                    Anchor::Start,
                    0.0,
                );
            }
        }
    }
}
