//! Graph > New Table (T136): a table label drawn as a grid of cells. Its anchor is the table's top-left
//! corner; every column is as wide as its widest cell (at least two and a half font sizes) plus a margin
//! of 0.4 font size on each side, every row 1.6 font sizes high; 0.5 pt black lines; the cell text in the
//! label's font, left aligned and centred in the row.

use crate::scene::{Anchor, Item, Stroke, line_to, move_to};
use crate::text::{self, ASCENT, DESCENT};
use crate::{Rect, TextMeasurer};
use sheaf_core::graph::TextLabel;

/// Draws the table and returns its rectangle (page points).
pub(crate) fn draw_table(out: &mut Vec<Item>, m: &dyn TextMeasurer, label: &TextLabel, frame: Rect) -> Rect {
    let size = label.font.size_pt;
    let cells = label.cells();
    let cols = cells.iter().map(Vec::len).max().unwrap_or(1).max(1);
    let pad = 0.4 * size;
    let row_h = 1.6 * size;
    let widths: Vec<f64> = (0..cols)
        .map(|c| {
            let widest =
                cells.iter().filter_map(|r| r.get(c)).map(|t| text::width(m, t, &label.font, size)).fold(0.0, f64::max);
            widest.max(2.5 * size) + 2.0 * pad
        })
        .collect();
    let (x0, y0) = (frame.x + label.x * frame.w, frame.y + label.y * frame.h);
    let w: f64 = widths.iter().sum();
    let h = row_h * cells.len() as f64;
    let mut d = String::new();
    for r in 0..=cells.len() {
        let y = y0 + row_h * r as f64;
        move_to(&mut d, x0, y);
        line_to(&mut d, x0 + w, y);
    }
    let mut x = x0;
    for c in 0..=cols {
        move_to(&mut d, x, y0);
        line_to(&mut d, x, y0 + h);
        if let Some(cw) = widths.get(c) {
            x += cw;
        }
    }
    out.push(Item::Path { d, fill: None, stroke: Some(Stroke::solid(sheaf_core::Color::BLACK, 0.5)) });
    for (r, row) in cells.iter().enumerate() {
        let baseline = y0 + row_h * r as f64 + (row_h + (ASCENT - DESCENT) * size) / 2.0;
        let mut x = x0;
        for (c, cell) in row.iter().enumerate() {
            if !cell.is_empty() {
                text::place(out, m, cell, &label.font, size, (x + pad, baseline), Anchor::Start, 0.0);
            }
            x += widths.get(c).copied().unwrap_or(0.0);
        }
    }
    Rect { x: x0, y: y0, w, h }
}
