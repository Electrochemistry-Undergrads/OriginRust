//! Plot > Stock > High-Low-Close, Japanese Candlestick and OHLC Bar Chart (T108), as Origin 2016's,
//! measured on 2026-11-16 (`docs/spec/graphing.md`, "Stock plots"):
//!
//! - **High-Low-Close** takes X and exactly three Y columns (High, Low, Close); Origin refuses other
//!   selections ("you need to select columns designated as 'XYYY' with exact number of columns"). Each
//!   row is a vertical line from high to low with a tick to the right at the close.
//! - **Japanese Candlestick** and **OHLC Bar Chart** take X and exactly four Y columns (Open, High, Low,
//!   Close). A candle is a body from open to close, 0.4 of the X spacing wide, with wicks to the high and
//!   the low; an OHLC bar is a vertical high-low line with a tick to the left at the open and one to the
//!   right at the close, each 0.3 of the X spacing long.
//! - A day whose close is below its open (Origin's "Declining Price") is drawn all red, the candle's body
//!   filled; the other days (Advancing Price) are black, the candle's body hollow. Lines are 0.5 pt.
//! - The axes take in every row's high and low.

use crate::Color;
use crate::graph::PlotKind;
use crate::ids::{ColumnId, WindowId};
use crate::project::{Project, ProjectError};
use crate::template::Template;
use serde::{Deserialize, Serialize};

/// The price columns and colours of a stock plot (in [`crate::graph::PlotDecor::stock`]). The plot's own
/// Y column is the first of the selection (Open, or High for High-Low-Close).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct StockSpec {
    /// `None` for High-Low-Close.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub open: Option<ColumnId>,
    pub high: ColumnId,
    pub low: ColumnId,
    pub close: ColumnId,
    /// The colour of a falling day (close below open); rising days take the plot's line colour.
    pub down: Color,
    /// Candle body width, or OHLC/HLC tick length, as a fraction of the X spacing.
    pub width: f64,
}

/// The number of Y columns a stock template takes.
pub fn y_columns(template: Template) -> usize {
    if template == Template::HighLowClose { 3 } else { 4 }
}

impl Project {
    /// Plots a stock chart from the Y columns `ys` (High, Low, Close for High-Low-Close; Open, High,
    /// Low, Close otherwise), with the X column of the first.
    pub fn plot_stock(&mut self, template: Template, ys: &[ColumnId]) -> Result<WindowId, ProjectError> {
        let n = y_columns(template);
        if ys.len() != n {
            return Err(ProjectError::StockColumns(if n == 3 {
                "a High-Low-Close plot needs an X column and exactly three Y columns (High, Low, Close)"
            } else {
                "this stock plot needs an X column and exactly four Y columns (Open, High, Low, Close)"
            }));
        }
        let (open, rest) = if n == 4 { (Some(ys[0]), &ys[1..]) } else { (None, ys) };
        let (kind, width) = match template {
            Template::HighLowClose => (PlotKind::HighLowClose, 0.3),
            Template::Candlestick => (PlotKind::Candlestick, 0.4),
            _ => (PlotKind::OhlcBar, 0.3),
        };
        let spec = StockSpec { open, high: rest[0], low: rest[1], close: rest[2], down: Color::RED, width };
        let y = ys[0];
        let (book, sheet, _) = self.locate(y).ok_or(ProjectError::NoSuchColumn(y))?;
        let x = sheet.column_index(y).and_then(|i| sheet.x_for(i)).map(|c| c.id);
        let name = book.title_for_graph();
        let id = self.add_graph(template);
        self.add_plot_xy(id, 0, x, y, None)?;
        let plot = self.graph_mut(id).and_then(|g| g.layers[0].plots.last_mut()).expect("a plot was just added");
        plot.kind = kind;
        plot.line.color = Color::BLACK;
        plot.line.width_pt = 0.5;
        plot.auto_color = false;
        plot.decor.stock = Some(spec);
        if let Some(g) = self.graph_mut(id) {
            g.long_name = name;
        }
        Ok(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::column::{ColumnData, Designation};

    #[test]
    fn hlc_takes_three_y_columns_and_candlestick_four() {
        let mut p = Project::new();
        let b = p.add_workbook("");
        let x = p.add_column(b, 0, Designation::X, ColumnData::numeric(vec![1.0, 2.0])).unwrap();
        let mut ys = Vec::new();
        for v in [[10.0, 12.0], [13.0, 14.0], [9.0, 11.0], [12.0, 11.0]] {
            ys.push(p.add_column(b, 0, Designation::Y, ColumnData::numeric(v.to_vec())).unwrap());
        }
        assert!(p.plot_stock(Template::HighLowClose, &ys).is_err(), "Origin wants exactly XYYY");
        assert!(p.plot_stock(Template::Candlestick, &ys[..3]).is_err());
        let g = p.plot_stock(Template::HighLowClose, &ys[1..]).unwrap();
        let plot = &p.graph(g).unwrap().layers[0].plots[0];
        let s = plot.decor.stock.unwrap();
        assert_eq!((plot.kind, plot.x, plot.y), (PlotKind::HighLowClose, Some(x), ys[1]));
        assert_eq!((s.open, s.high, s.low, s.close), (None, ys[1], ys[2], ys[3]));
        let g = p.plot_stock(Template::Candlestick, &ys).unwrap();
        let plot = &p.graph(g).unwrap().layers[0].plots[0];
        let s = plot.decor.stock.unwrap();
        assert_eq!(
            (plot.kind, s.open, s.close, s.width, s.down),
            (PlotKind::Candlestick, Some(ys[0]), ys[3], 0.4, Color::RED)
        );
        assert_eq!((plot.line.color, plot.line.width_pt), (Color::BLACK, 0.5));
        let g = p.plot_stock(Template::OhlcBar, &ys).unwrap();
        assert_eq!(p.graph(g).unwrap().layers[0].plots[0].kind, PlotKind::OhlcBar);
    }
}
