//! Compact derived row geometry. Its storage depends on formatting intervals, never data length.

use crate::Worksheet;
use crate::cellformat::{FormatIndex, FormatTarget, ResolvedStyle};

#[derive(Clone, Debug)]
struct Band {
    start: usize,
    height: f64,
    top: f64,
}

#[derive(Clone, Debug)]
pub struct RowHeights {
    bands: Vec<Band>,
}

impl RowHeights {
    /// Resolve at formatting boundaries, considering only displayed columns.
    pub fn new(
        sheet: &Worksheet,
        index: &FormatIndex,
        default: f64,
        measure: impl Fn(ResolvedStyle<'_>) -> f64,
    ) -> Self {
        let mut starts = vec![0];
        for range in &sheet.cell_formats {
            if let FormatTarget::Cells { rows: Some((first, last)), .. } = &range.target {
                starts.push(*first);
                if let Some(next) = last.and_then(|r| r.checked_add(1)) {
                    starts.push(next);
                }
            }
        }
        starts.sort_unstable();
        starts.dedup();
        let mut result = Self { bands: Vec::new() };
        for row in starts {
            let height = sheet
                .columns
                .iter()
                .filter(|c| !c.hidden)
                .map(|c| measure(index.resolve(sheet, c.id, row)))
                .fold(default, f64::max);
            result.push(row, height);
        }
        result
    }

    fn push(&mut self, start: usize, height: f64) {
        if self.bands.last().is_some_and(|b| b.height == height) {
            return;
        }
        let top = self.bands.last().map_or(0.0, |b| b.top + (start - b.start) as f64 * b.height);
        self.bands.push(Band { start, height, top });
    }

    fn band(&self, row: usize) -> &Band {
        &self.bands[self.bands.partition_point(|b| b.start <= row) - 1]
    }

    pub fn uniform_height(&self) -> Option<f64> {
        (self.bands.len() == 1).then_some(self.bands[0].height)
    }

    pub fn height(&self, row: usize) -> f64 {
        self.band(row).height
    }

    /// The top of `row`, including a row beyond the current data or an open formatted tail.
    pub fn offset(&self, row: usize) -> f64 {
        let b = self.band(row);
        b.top + (row - b.start) as f64 * b.height
    }

    /// The row containing this distance from the top, by bisection over format intervals.
    pub fn row_at(&self, y: f64) -> usize {
        let y = y.max(0.0);
        let b = &self.bands[self.bands.partition_point(|b| b.top <= y).saturating_sub(1)];
        b.start.saturating_add(((y - b.top) / b.height).floor() as usize)
    }

    /// Project filtered rows into displayed coordinates, merging equal neighbours again.
    pub fn shown(&self, rows: &[usize]) -> Self {
        let mut result = Self { bands: Vec::new() };
        for (i, &r) in rows.iter().enumerate() {
            result.push(i, self.height(r));
        }
        if result.bands.is_empty() {
            result.push(0, self.height(0));
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cellformat::CellFormat;
    use crate::column::{ColumnData, Designation};
    use crate::{Column, ColumnId};

    #[test]
    fn million_row_columns_and_overrides_keep_compact_geometry() {
        let mut s = Worksheet::new("sheet");
        s.columns.push(Column::new(ColumnId(1), "A", Designation::X, ColumnData::numeric(vec![])));
        s.apply_cell_format(
            FormatTarget::whole_columns(vec![ColumnId(1)]),
            CellFormat { size_pt: Some(20.0), ..Default::default() },
        );
        s.apply_cell_format(
            FormatTarget::cells(vec![ColumnId(1)], 12, Some(13)),
            CellFormat { size_pt: Some(9.0), ..Default::default() },
        );
        let g = RowHeights::new(&s, &s.format_index(), 16.0, |f| f64::from(f.size_pt.unwrap_or(9.0)) * 2.0);
        assert_eq!(g.bands.len(), 3);
        assert_eq!(g.height(1_000_000), 40.0);
        assert_eq!(g.offset(14), 12.0 * 40.0 + 2.0 * 18.0);
        for row in [0, 11, 12, 13, 14, 1_000_000] {
            assert_eq!(g.row_at(g.offset(row) + g.height(row) / 2.0), row);
        }
        let shown = g.shown(&[11, 13, 20]);
        assert_eq!(shown.offset(3), 98.0);
    }
}
