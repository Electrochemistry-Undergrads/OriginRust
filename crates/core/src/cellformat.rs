//! Worksheet cell formats (T59): bold, italic, underline, font, size, text and fill colour and
//! horizontal alignment of data cells and of label rows. Origin's Format Cells window and the
//! Format toolbar set them; which controls Origin has, its defaults and how manual and
//! conditional colours combine are not measured. Sheaf's rules are its own choices and are
//! recorded in docs/spec/workbook-model.md "Cell formats".
//!
//! A sheet keeps an ordered list of [`FormatRange`]s. A range names columns by their stable id
//! and either a span of data rows or some label rows, and carries a [`CellFormat`] whose unset
//! properties leave the cell as it was. Where ranges overlap, the later range wins property by
//! property. Formatting whole columns is one record, never one per row. Row inserts, deletes
//! and sorts move the spans with the cells (like cell notes), column deletes drop ids.

use crate::color::Color;
use crate::condformat::CellStyle;
use crate::ids::ColumnId;
use crate::worksheet::Worksheet;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ops::Range;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum HAlign {
    /// The default of the cell's kind (left for text, right for numbers); lets a later range
    /// undo an earlier alignment.
    Auto,
    Left,
    Center,
    Right,
}

/// The look of a cell; `None` leaves that property to earlier ranges or to the default.
/// An empty `font` name means "the default font", a size of 0 "the default size", a fill or text
/// colour with zero alpha "no fill" / "the default colour" and [`HAlign::Auto`] "the default
/// alignment"; each lets a later range undo an earlier one.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CellFormat {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bold: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub italic: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub underline: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size_pt: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_color: Option<Color>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill: Option<Color>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub h_align: Option<HAlign>,
}

impl CellFormat {
    pub fn is_empty(&self) -> bool {
        *self == CellFormat::default()
    }

    /// Sets every property that `over` sets.
    pub fn merge(&mut self, over: &CellFormat) {
        macro_rules! take {
            ($($f:ident),*) => {$(if over.$f.is_some() { self.$f = over.$f.clone(); })*};
        }
        take!(bold, italic, underline, font, size_pt, text_color, fill, h_align);
    }

    /// [`CellFormat::merge`] without cloning the font name (the caller borrows it instead).
    fn merge_except_font(&mut self, over: &CellFormat) {
        macro_rules! take {
            ($($f:ident),*) => {$(if over.$f.is_some() { self.$f = over.$f.clone(); })*};
        }
        take!(bold, italic, underline, size_pt, text_color, fill, h_align);
    }

    /// Unsets every property that `by` sets (the earlier range no longer matters for them).
    fn forget(&mut self, by: &CellFormat) {
        macro_rules! drop {
            ($($f:ident),*) => {$(if by.$f.is_some() { self.$f = None; })*};
        }
        drop!(bold, italic, underline, font, size_pt, text_color, fill, h_align);
    }
}

/// The data rows of a range: first row and last row (0-based, inclusive); `None` for the last row
/// runs to the end of the data and beyond.
pub type RowSpan = (usize, Option<usize>);

/// The label rows of a worksheet's column heads.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LabelKind {
    LongName,
    Units,
    Comments,
    Formula,
    Param(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FormatTarget {
    /// Data cells of these columns; `rows` `None` is every row of the column (whole columns).
    Cells {
        columns: Vec<ColumnId>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        rows: Option<RowSpan>,
    },
    /// The label cells of these columns in these label rows.
    LabelRows { columns: Vec<ColumnId>, rows: Vec<LabelKind> },
}

impl FormatTarget {
    /// Data cells of `columns` in rows `first..=last` (`None` runs to the end of the data); a span
    /// from row 0 to the end is stored as whole columns. Columns are kept sorted and unique.
    pub fn cells(columns: Vec<ColumnId>, first: usize, last: Option<usize>) -> Self {
        FormatTarget::Cells { columns: canon(columns), rows: (first != 0 || last.is_some()).then_some((first, last)) }
    }

    pub fn whole_columns(columns: Vec<ColumnId>) -> Self {
        FormatTarget::Cells { columns: canon(columns), rows: None }
    }

    pub fn label_rows(columns: Vec<ColumnId>, rows: Vec<LabelKind>) -> Self {
        FormatTarget::LabelRows { columns: canon(columns), rows }
    }

    pub fn columns(&self) -> &[ColumnId] {
        match self {
            FormatTarget::Cells { columns, .. } | FormatTarget::LabelRows { columns, .. } => columns,
        }
    }
}

fn canon(mut columns: Vec<ColumnId>) -> Vec<ColumnId> {
    columns.sort();
    columns.dedup();
    columns
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FormatRange {
    pub target: FormatTarget,
    pub style: CellFormat,
}

// ----- row spans ---------------------------------------------------------------------------

/// A span as inclusive bounds, the open end as `usize::MAX`.
fn bounds(rows: Option<RowSpan>) -> (usize, usize) {
    match rows {
        None => (0, usize::MAX),
        Some((a, b)) => (a, b.unwrap_or(usize::MAX)),
    }
}

fn span(first: usize, last: usize) -> Option<RowSpan> {
    if last == usize::MAX { (first != 0).then_some((first, None)) } else { Some((first, Some(last))) }
}

fn span_covers(outer: Option<RowSpan>, inner: Option<RowSpan>) -> bool {
    let (a, b) = bounds(outer);
    let (c, d) = bounds(inner);
    a <= c && d <= b
}

/// `rows` without the rows of `cut`: zero, one or two spans.
fn span_minus(rows: Option<RowSpan>, cut: Option<RowSpan>) -> Vec<Option<RowSpan>> {
    let (a, b) = bounds(rows);
    let (c, d) = bounds(cut);
    if d < a || c > b {
        return vec![rows];
    }
    let mut out = Vec::new();
    if a < c {
        out.push(span(a, c - 1));
    }
    if b > d {
        out.push(span(d + 1, b));
    }
    out
}

/// Spans of the new rows of the rows in `rows` after `order` put row `order[i]` at row `i` (rows
/// beyond the sheet length `n` stay where they are, rows the order leaves out are dropped).
fn span_reorder(rows: Option<RowSpan>, order: &[usize], n: usize) -> Vec<Option<RowSpan>> {
    // Whole columns look the same after any order.
    if rows.is_none() {
        return vec![None];
    }
    let (a, b) = bounds(rows);
    let moved: Vec<usize> =
        order.iter().enumerate().filter(|(_, src)| **src >= a && **src <= b && **src < n).map(|(i, _)| i).collect();
    let mut runs: Vec<(usize, usize)> = Vec::new();
    for r in moved {
        match runs.last_mut() {
            Some(last) if r == last.1 + 1 => last.1 = r,
            _ => runs.push((r, r)),
        }
    }
    if b >= n {
        runs.push((a.max(n), b));
    }
    runs.sort();
    let mut merged: Vec<(usize, usize)> = Vec::new();
    for (s, e) in runs {
        match merged.last_mut() {
            Some(last) if last.1 == usize::MAX || s <= last.1 + 1 => last.1 = last.1.max(e),
            _ => merged.push((s, e)),
        }
    }
    merged.into_iter().map(|(s, e)| span(s, e)).collect()
}

fn span_contains(rows: Option<RowSpan>, row: usize) -> bool {
    match rows {
        None => true,
        Some((a, b)) => row >= a && b.is_none_or(|b| row <= b),
    }
}

// ----- resolved styles ---------------------------------------------------------------------

/// What a cell looks like after all ranges (and conditional colours) are applied. Borrows the
/// font name from the sheet so that resolving a cell allocates nothing.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ResolvedStyle<'a> {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub font: Option<&'a str>,
    pub size_pt: Option<f32>,
    pub text_color: Option<Color>,
    pub fill: Option<Color>,
    pub h_align: Option<HAlign>,
}

impl ResolvedStyle<'_> {
    /// True when nothing differs from an unformatted cell.
    pub fn is_default(&self) -> bool {
        *self == ResolvedStyle::default()
    }

    /// Conditional formatting wins over the manual fill and text colour of a cell (Sheaf's
    /// choice, unmeasured); bold, italic, underline, font, size and alignment are unaffected.
    pub fn with_conditional(mut self, cond: Option<CellStyle>) -> Self {
        if let Some(c) = cond {
            self.fill = c.fill.or(self.fill);
            self.text_color = c.font.or(self.text_color);
        }
        self
    }
}

fn visible(c: Option<Color>) -> Option<Color> {
    c.filter(|c| c.a != 0)
}

/// Which ranges of a sheet reach each column, for fast per-cell lookups. Built once per document
/// revision; resolving a cell walks the few ranges of its column.
#[derive(Clone, Debug, Default)]
pub struct FormatIndex {
    cells: HashMap<ColumnId, Vec<u32>>,
    labels: HashMap<ColumnId, Vec<u32>>,
}

impl FormatIndex {
    pub fn new(sheet: &Worksheet) -> Self {
        let mut ix = FormatIndex::default();
        for (i, r) in sheet.cell_formats.iter().enumerate() {
            let map = match &r.target {
                FormatTarget::Cells { .. } => &mut ix.cells,
                FormatTarget::LabelRows { .. } => &mut ix.labels,
            };
            for c in r.target.columns() {
                map.entry(*c).or_default().push(i as u32);
            }
        }
        ix
    }

    pub fn is_empty(&self) -> bool {
        self.cells.is_empty() && self.labels.is_empty()
    }

    /// The format of a data cell of `sheet` (the sheet the index was built from).
    pub fn resolve<'a>(&self, sheet: &'a Worksheet, column: ColumnId, row: usize) -> ResolvedStyle<'a> {
        let Some(list) = self.cells.get(&column) else { return ResolvedStyle::default() };
        let mut f = CellFormat::default();
        let mut font: Option<&'a str> = None;
        for &i in list {
            let r = &sheet.cell_formats[i as usize];
            let FormatTarget::Cells { rows, .. } = &r.target else { continue };
            if span_contains(*rows, row) {
                f.merge_except_font(&r.style);
                if let Some(name) = &r.style.font {
                    font = Some(name.as_str());
                }
            }
        }
        finish(f, font)
    }

    /// The format of a label cell.
    pub fn resolve_label<'a>(&self, sheet: &'a Worksheet, column: ColumnId, row: &LabelKind) -> ResolvedStyle<'a> {
        let Some(list) = self.labels.get(&column) else { return ResolvedStyle::default() };
        let mut f = CellFormat::default();
        let mut font: Option<&'a str> = None;
        for &i in list {
            let r = &sheet.cell_formats[i as usize];
            let FormatTarget::LabelRows { rows, .. } = &r.target else { continue };
            if rows.contains(row) {
                f.merge_except_font(&r.style);
                if let Some(name) = &r.style.font {
                    font = Some(name.as_str());
                }
            }
        }
        finish(f, font)
    }
}

fn finish(f: CellFormat, font: Option<&str>) -> ResolvedStyle<'_> {
    ResolvedStyle {
        bold: f.bold.unwrap_or(false),
        italic: f.italic.unwrap_or(false),
        underline: f.underline.unwrap_or(false),
        font: font.filter(|n| !n.is_empty()),
        size_pt: f.size_pt.filter(|p| *p > 0.0),
        text_color: visible(f.text_color),
        fill: visible(f.fill),
        h_align: f.h_align.filter(|a| *a != HAlign::Auto),
    }
}

// ----- the sheet's operations --------------------------------------------------------------

impl Worksheet {
    pub fn format_index(&self) -> FormatIndex {
        FormatIndex::new(self)
    }

    /// The format of one data cell. Walks every range: use a [`FormatIndex`] to draw many cells.
    pub fn cell_format_at(&self, column: ColumnId, row: usize) -> ResolvedStyle<'_> {
        FormatIndex::new(self).resolve(self, column, row)
    }

    /// Applies `style` to the target: the properties it sets override what the cells had. Adds
    /// one range (or extends the last one when it is the same target or the same style on
    /// neighbouring cells) and drops what the new range makes redundant. Returns true when the
    /// sheet changed.
    pub fn apply_cell_format(&mut self, target: FormatTarget, style: CellFormat) -> bool {
        let before = self.cell_formats.clone();
        let target = match target {
            FormatTarget::Cells { columns, rows } => {
                FormatTarget::Cells { columns: canon(columns), rows: rows.filter(|r| *r != (0, None)) }
            }
            FormatTarget::LabelRows { columns, mut rows } => {
                rows.dedup();
                FormatTarget::LabelRows { columns: canon(columns), rows }
            }
        };
        if target.columns().is_empty() || style.is_empty() {
            return false;
        }
        // Older ranges wholly inside the new one lose the properties the new one sets.
        for r in &mut self.cell_formats {
            if covers(&target, &r.target) {
                r.style.forget(&style);
            }
        }
        self.cell_formats.retain(|r| !r.style.is_empty());
        let joined = match self.cell_formats.last_mut() {
            Some(last) if last.target == target => {
                last.style.merge(&style);
                true
            }
            Some(last) => last.style == style && join(&mut last.target, &target),
            None => false,
        };
        if !joined {
            self.cell_formats.push(FormatRange { target, style });
        }
        self.cell_formats != before
    }

    /// Removes the formats of the target's cells (Clear Formats); ranges that reach beyond it are
    /// cut. Data cells and label cells are separate targets. Returns true when anything changed.
    pub fn clear_cell_formats(&mut self, target: &FormatTarget) -> bool {
        let old = std::mem::take(&mut self.cell_formats);
        let mut out = Vec::with_capacity(old.len());
        let mut changed = false;
        for r in old {
            match (&r.target, target) {
                (FormatTarget::Cells { columns, rows }, FormatTarget::Cells { columns: tc, rows: tr }) => {
                    let (hit, keep): (Vec<ColumnId>, Vec<ColumnId>) = columns.iter().partition(|c| tc.contains(c));
                    if hit.is_empty() {
                        out.push(r);
                        continue;
                    }
                    changed = true;
                    if !keep.is_empty() {
                        out.push(FormatRange {
                            target: FormatTarget::Cells { columns: keep, rows: *rows },
                            style: r.style.clone(),
                        });
                    }
                    for piece in span_minus(*rows, *tr) {
                        out.push(FormatRange {
                            target: FormatTarget::Cells { columns: hit.clone(), rows: piece },
                            style: r.style.clone(),
                        });
                    }
                }
                (FormatTarget::LabelRows { columns, rows }, FormatTarget::LabelRows { columns: tc, rows: tr }) => {
                    let (hit, keep): (Vec<ColumnId>, Vec<ColumnId>) = columns.iter().partition(|c| tc.contains(c));
                    let left: Vec<LabelKind> = rows.iter().filter(|l| !tr.contains(l)).cloned().collect();
                    if hit.is_empty() || left.len() == rows.len() {
                        out.push(r);
                        continue;
                    }
                    changed = true;
                    if !keep.is_empty() {
                        out.push(FormatRange {
                            target: FormatTarget::LabelRows { columns: keep, rows: rows.clone() },
                            style: r.style.clone(),
                        });
                    }
                    if !left.is_empty() {
                        out.push(FormatRange {
                            target: FormatTarget::LabelRows { columns: hit, rows: left },
                            style: r.style.clone(),
                        });
                    }
                }
                _ => out.push(r),
            }
        }
        self.cell_formats = out;
        changed
    }

    /// Renames the columns of the ranges (`map` gives the new id, or `None` when the column is
    /// gone) and drops ranges left with no column. Used wherever a sheet's columns get new ids
    /// (duplicate, append) or are deleted.
    pub fn remap_format_columns(&mut self, map: &dyn Fn(ColumnId) -> Option<ColumnId>) {
        if self.cell_formats.is_empty() {
            return;
        }
        for r in &mut self.cell_formats {
            let columns = match &mut r.target {
                FormatTarget::Cells { columns, .. } | FormatTarget::LabelRows { columns, .. } => columns,
            };
            *columns = canon(columns.iter().filter_map(|c| map(*c)).collect());
        }
        self.cell_formats.retain(|r| !r.target.columns().is_empty());
    }

    /// Rebuilds the data-cell ranges: a range with columns both in and out of `touched` is split in
    /// two, and the rows of the part in `touched` become the spans `f` returns (none drops it).
    fn map_format_rows(
        &mut self,
        touched: &dyn Fn(ColumnId) -> bool,
        f: &dyn Fn(Option<RowSpan>) -> Vec<Option<RowSpan>>,
    ) {
        let old = std::mem::take(&mut self.cell_formats);
        for r in old {
            let FormatTarget::Cells { columns, rows } = &r.target else {
                self.cell_formats.push(r);
                continue;
            };
            let (hit, rest): (Vec<ColumnId>, Vec<ColumnId>) = columns.iter().partition(|c| touched(**c));
            if hit.is_empty() {
                self.cell_formats.push(r);
                continue;
            }
            if !rest.is_empty() {
                self.cell_formats.push(FormatRange {
                    target: FormatTarget::Cells { columns: rest, rows: *rows },
                    style: r.style.clone(),
                });
            }
            for piece in f(*rows) {
                self.cell_formats.push(FormatRange {
                    target: FormatTarget::Cells { columns: hit.clone(), rows: piece },
                    style: r.style.clone(),
                });
            }
        }
    }

    /// Moves the formats of these columns for `count` rows inserted before row `at`: spans that
    /// start at or below it move down, a span that contains it grows. Whole columns stay whole.
    pub fn formats_insert_rows(&mut self, columns: &[ColumnId], at: usize, count: usize) {
        if self.cell_formats.is_empty() || count == 0 {
            return;
        }
        self.map_format_rows(&|c| columns.contains(&c), &|rows| {
            let Some((a, b)) = rows else { return vec![None] };
            let (a, b) = if at <= a {
                (a + count, b.map(|l| l + count))
            } else if b.is_none_or(|l| at <= l) {
                (a, b.map(|l| l + count))
            } else {
                (a, b)
            };
            vec![Some((a, b))]
        });
    }

    /// Cuts the deleted `rows` out of the spans of these columns and moves up what is below; a
    /// span that was entirely deleted is dropped.
    pub fn formats_delete_rows(&mut self, columns: &[ColumnId], rows: Range<usize>) {
        if self.cell_formats.is_empty() || rows.is_empty() {
            return;
        }
        let len = rows.len();
        self.map_format_rows(&|c| columns.contains(&c), &|span| {
            let Some((a, b)) = span else { return vec![None] };
            if let Some(l) = b
                && a >= rows.start
                && l < rows.end
            {
                return Vec::new();
            }
            let a = if a >= rows.end { a - len } else { a.min(rows.start) };
            let b = b.map(|l| {
                if l >= rows.end {
                    l - len
                } else if l >= rows.start {
                    rows.start - 1
                } else {
                    l
                }
            });
            vec![Some((a, b))]
        });
    }

    /// Puts the formats of the columns for which `columns` is true on the rows `order` moves
    /// them to (new row i takes old row `order[i]`, as `Column::reorder_rows` does); rows the
    /// order leaves out lose their formats, rows at or beyond `rows` stay. The formats follow the
    /// rows, as cell notes do (Sheaf's choice, unmeasured).
    pub fn formats_reorder_rows(&mut self, columns: &dyn Fn(ColumnId) -> bool, order: &[usize], rows: usize) {
        if self.cell_formats.is_empty() {
            return;
        }
        self.map_format_rows(columns, &|span| span_reorder(span, order, rows));
    }
}

/// True when every cell of `inner` is a cell of `outer`.
fn covers(outer: &FormatTarget, inner: &FormatTarget) -> bool {
    match (outer, inner) {
        (FormatTarget::Cells { columns: oc, rows: or }, FormatTarget::Cells { columns: ic, rows: ir }) => {
            ic.iter().all(|c| oc.contains(c)) && span_covers(*or, *ir)
        }
        (FormatTarget::LabelRows { columns: oc, rows: or }, FormatTarget::LabelRows { columns: ic, rows: ir }) => {
            ic.iter().all(|c| oc.contains(c)) && ir.iter().all(|l| or.contains(l))
        }
        _ => false,
    }
}

/// Extends `into` by `add` when the two reach the same rows of different columns, or the same
/// columns with touching or overlapping row spans (the styles are known to be equal). Returns
/// whether it did.
fn join(into: &mut FormatTarget, add: &FormatTarget) -> bool {
    match (into, add) {
        (FormatTarget::Cells { columns: ic, rows: ir }, FormatTarget::Cells { columns: ac, rows: ar }) => {
            if ir == ar {
                ic.extend(ac.iter().copied());
                *ic = canon(std::mem::take(ic));
                return true;
            }
            if *ic == *ac {
                let (a, b) = bounds(*ir);
                let (c, d) = bounds(*ar);
                if c <= b.saturating_add(1) && a <= d.saturating_add(1) {
                    *ir = span(a.min(c), b.max(d));
                    return true;
                }
            }
            false
        }
        (FormatTarget::LabelRows { columns: ic, rows: ir }, FormatTarget::LabelRows { columns: ac, rows: ar }) => {
            if ir == ar {
                ic.extend(ac.iter().copied());
                *ic = canon(std::mem::take(ic));
                true
            } else if ic == ac {
                for l in ar {
                    if !ir.contains(l) {
                        ir.push(l.clone());
                    }
                }
                true
            } else {
                false
            }
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::column::{Column, ColumnData, Designation};
    use crate::condformat::CellStyle;

    fn sheet() -> Worksheet {
        let mut s = Worksheet::new("Sheet1");
        for i in 0..3u64 {
            s.columns.push(Column::new(
                ColumnId(i + 1),
                crate::column::default_short_name(i as usize),
                Designation::Y,
                ColumnData::numeric(vec![3.0, 1.0, 2.0, 5.0, 4.0]),
            ));
        }
        s
    }

    const A: ColumnId = ColumnId(1);
    const B: ColumnId = ColumnId(2);
    const C: ColumnId = ColumnId(3);

    fn bold() -> CellFormat {
        CellFormat { bold: Some(true), ..Default::default() }
    }

    fn fill(r: u8, g: u8, b: u8) -> CellFormat {
        CellFormat { fill: Some(Color::rgb(r, g, b)), ..Default::default() }
    }

    fn is_bold(s: &Worksheet, c: ColumnId, row: usize) -> bool {
        s.cell_format_at(c, row).bold
    }

    fn rows_of(s: &Worksheet) -> Vec<(Vec<u64>, Option<RowSpan>)> {
        s.cell_formats
            .iter()
            .map(|r| match &r.target {
                FormatTarget::Cells { columns, rows } => (columns.iter().map(|c| c.0).collect(), *rows),
                _ => panic!("label range"),
            })
            .collect()
    }

    #[test]
    fn an_unformatted_cell_is_default_and_old_json_loads_without_formats() {
        let s = sheet();
        assert!(s.cell_format_at(A, 0).is_default());
        let json = serde_json::to_string(&s).unwrap();
        assert!(!json.contains("cell_formats"));
        let back: Worksheet = serde_json::from_str(&json).unwrap();
        assert!(back.cell_formats.is_empty());
    }

    #[test]
    fn applying_sets_properties_for_the_selected_cells_only() {
        let mut s = sheet();
        assert!(s.apply_cell_format(FormatTarget::cells(vec![A, B], 1, Some(2)), bold()));
        assert!(is_bold(&s, A, 1) && is_bold(&s, B, 2));
        assert!(!is_bold(&s, A, 0) && !is_bold(&s, A, 3) && !is_bold(&s, C, 1));
        assert!(!s.apply_cell_format(FormatTarget::cells(vec![A], 0, None), CellFormat::default()), "empty style");
        assert!(!s.apply_cell_format(FormatTarget::cells(vec![], 0, None), bold()), "no columns");
    }

    #[test]
    fn later_ranges_override_earlier_ones_property_by_property() {
        let mut s = sheet();
        s.apply_cell_format(
            FormatTarget::cells(vec![A], 0, Some(4)),
            CellFormat { bold: Some(true), fill: Some(Color::rgb(1, 2, 3)), ..Default::default() },
        );
        s.apply_cell_format(
            FormatTarget::cells(vec![A], 2, Some(2)),
            CellFormat { bold: Some(false), italic: Some(true), ..Default::default() },
        );
        let r = s.cell_format_at(A, 2);
        assert!(!r.bold && r.italic);
        assert_eq!(r.fill, Some(Color::rgb(1, 2, 3)));
        assert!(s.cell_format_at(A, 1).bold && !s.cell_format_at(A, 1).italic);
    }

    #[test]
    fn a_transparent_fill_and_an_empty_font_undo_earlier_ones() {
        let mut s = sheet();
        s.apply_cell_format(
            FormatTarget::cells(vec![A], 0, Some(1)),
            CellFormat { fill: Some(Color::rgb(9, 9, 9)), font: Some("Courier New".into()), ..Default::default() },
        );
        assert_eq!(s.cell_format_at(A, 0).font, Some("Courier New"));
        s.apply_cell_format(
            FormatTarget::cells(vec![A], 0, Some(0)),
            CellFormat { fill: Some(Color::TRANSPARENT), font: Some(String::new()), ..Default::default() },
        );
        let r = s.cell_format_at(A, 0);
        assert_eq!((r.fill, r.font), (None, None));
        assert_eq!(s.cell_format_at(A, 1).fill, Some(Color::rgb(9, 9, 9)));
    }

    #[test]
    fn whole_column_styling_is_one_record_and_open_ranges_follow_the_data() {
        let mut s = Worksheet::new("Big");
        s.columns.push(Column::new(A, "A", Designation::Y, ColumnData::numeric(vec![0.0; 1_000_000])));
        let t0 = std::time::Instant::now();
        s.apply_cell_format(FormatTarget::whole_columns(vec![A]), bold());
        s.apply_cell_format(
            FormatTarget::cells(vec![A], 0, None),
            CellFormat { italic: Some(true), ..Default::default() },
        );
        assert_eq!(s.cell_formats.len(), 1, "same target merges into one record");
        assert_eq!(s.cell_formats[0].target, FormatTarget::Cells { columns: vec![A], rows: None });
        assert!(is_bold(&s, A, 999_999) && is_bold(&s, A, 5_000_000));
        let ix = s.format_index();
        for r in (0..1_000_000).step_by(1000) {
            assert!(ix.resolve(&s, A, r).italic);
        }
        assert!(t0.elapsed().as_secs() < 5);
        // A block that runs to the end keeps following the data.
        let mut s2 = sheet();
        s2.apply_cell_format(FormatTarget::cells(vec![A], 3, None), bold());
        assert!(is_bold(&s2, A, 4) && is_bold(&s2, A, 40));
    }

    #[test]
    fn identical_neighbours_coalesce() {
        let mut s = sheet();
        s.apply_cell_format(FormatTarget::cells(vec![A], 0, Some(1)), bold());
        s.apply_cell_format(FormatTarget::cells(vec![B], 0, Some(1)), bold());
        assert_eq!(rows_of(&s), vec![(vec![1, 2], Some((0, Some(1))))], "same rows, more columns");
        s.apply_cell_format(FormatTarget::cells(vec![A, B], 2, Some(3)), bold());
        assert_eq!(rows_of(&s), vec![(vec![1, 2], Some((0, Some(3))))], "touching rows join");
        s.apply_cell_format(FormatTarget::cells(vec![A, B], 6, Some(7)), bold());
        assert_eq!(s.cell_formats.len(), 2, "a gap keeps two ranges");
        s.apply_cell_format(FormatTarget::cells(vec![C], 0, None), fill(1, 1, 1));
        assert_eq!(s.cell_formats.len(), 3);
    }

    #[test]
    fn a_covering_range_replaces_what_it_makes_redundant() {
        let mut s = sheet();
        s.apply_cell_format(FormatTarget::cells(vec![A], 1, Some(2)), bold());
        s.apply_cell_format(FormatTarget::cells(vec![A], 1, Some(2)), fill(1, 2, 3));
        s.apply_cell_format(FormatTarget::whole_columns(vec![A, B]), bold());
        assert_eq!(s.cell_formats.len(), 2, "{:?}", s.cell_formats);
        assert!(s.cell_formats.iter().all(|r| r.style.bold.is_none() || r.target.columns().len() == 2));
        assert_eq!(s.cell_format_at(A, 1).fill, Some(Color::rgb(1, 2, 3)));
        assert!(is_bold(&s, A, 1) && is_bold(&s, B, 0));
    }

    #[test]
    fn clear_cuts_ranges_over_the_selection() {
        let mut s = sheet();
        s.apply_cell_format(FormatTarget::whole_columns(vec![A, B]), bold());
        assert!(s.clear_cell_formats(&FormatTarget::cells(vec![B], 1, Some(2))));
        assert!(is_bold(&s, A, 1) && is_bold(&s, B, 0) && is_bold(&s, B, 3));
        assert!(!is_bold(&s, B, 1) && !is_bold(&s, B, 2));
        assert!(!s.clear_cell_formats(&FormatTarget::cells(vec![C], 0, None)), "nothing there");
        assert!(s.clear_cell_formats(&FormatTarget::whole_columns(vec![A, B])));
        assert!(s.cell_formats.is_empty());
    }

    #[test]
    fn label_rows_are_formatted_and_cleared_apart_from_data() {
        let mut s = sheet();
        s.apply_cell_format(FormatTarget::label_rows(vec![A], vec![LabelKind::Units]), bold());
        let ix = s.format_index();
        assert!(ix.resolve_label(&s, A, &LabelKind::Units).bold);
        assert!(!ix.resolve_label(&s, A, &LabelKind::LongName).bold);
        assert!(!ix.resolve_label(&s, B, &LabelKind::Units).bold);
        assert!(!ix.resolve(&s, A, 0).bold, "data cells are separate");
        s.apply_cell_format(FormatTarget::label_rows(vec![A], vec![LabelKind::LongName]), bold());
        assert_eq!(s.cell_formats.len(), 1, "label rows of one column join");
        // Clearing a data selection leaves labels alone; clearing one label row cuts the range.
        assert!(!s.clear_cell_formats(&FormatTarget::whole_columns(vec![A])));
        assert!(s.clear_cell_formats(&FormatTarget::label_rows(vec![A], vec![LabelKind::Units])));
        let ix = s.format_index();
        assert!(ix.resolve_label(&s, A, &LabelKind::LongName).bold);
        assert!(!ix.resolve_label(&s, A, &LabelKind::Units).bold);
    }

    #[test]
    fn conditional_colours_win_and_everything_else_stays() {
        let mut s = sheet();
        s.apply_cell_format(
            FormatTarget::whole_columns(vec![A]),
            CellFormat {
                bold: Some(true),
                fill: Some(Color::rgb(10, 10, 10)),
                text_color: Some(Color::rgb(20, 20, 20)),
                ..Default::default()
            },
        );
        let r = s.cell_format_at(A, 0);
        let both =
            r.with_conditional(Some(CellStyle { fill: Some(Color::rgb(1, 1, 1)), font: Some(Color::rgb(2, 2, 2)) }));
        assert_eq!((both.fill, both.text_color), (Some(Color::rgb(1, 1, 1)), Some(Color::rgb(2, 2, 2))));
        assert!(both.bold, "manual bold stays");
        let fill_only = r.with_conditional(Some(CellStyle { fill: Some(Color::rgb(1, 1, 1)), font: None }));
        assert_eq!(
            fill_only.text_color,
            Some(Color::rgb(20, 20, 20)),
            "manual text colour stays when only the fill is conditional"
        );
        assert_eq!(r.with_conditional(None), r);
    }

    #[test]
    fn insert_rows_moves_and_grows_spans_of_the_inserted_columns_only() {
        let mut s = sheet();
        s.apply_cell_format(FormatTarget::cells(vec![A, B], 2, Some(3)), bold());
        s.apply_cell_format(FormatTarget::whole_columns(vec![A]), fill(1, 1, 1));
        s.insert_rows(&[0], 1, 2); // before row 1 in A
        assert!(is_bold(&s, A, 4) && is_bold(&s, A, 5) && !is_bold(&s, A, 2));
        assert!(is_bold(&s, B, 2) && is_bold(&s, B, 3) && !is_bold(&s, B, 4), "B did not move: the range split");
        assert_eq!(s.cell_format_at(A, 1).fill, Some(Color::rgb(1, 1, 1)), "whole column stays whole");
        // Inserting inside a span grows it.
        s.insert_rows(&[1], 3, 1);
        assert!(is_bold(&s, B, 2) && is_bold(&s, B, 3) && is_bold(&s, B, 4) && !is_bold(&s, B, 5));
    }

    #[test]
    fn delete_rows_shifts_shrinks_and_drops() {
        let mut s = sheet();
        s.apply_cell_format(FormatTarget::cells(vec![A], 1, Some(3)), bold());
        s.apply_cell_format(FormatTarget::cells(vec![B], 4, None), fill(1, 1, 1));
        s.delete_rows(&[0, 1], 0..1);
        assert_eq!(rows_of(&s), vec![(vec![1], Some((0, Some(2)))), (vec![2], Some((3, None)))]);
        s.delete_rows(&[0], 1..2);
        assert_eq!(rows_of(&s)[0], (vec![1], Some((0, Some(1)))));
        s.delete_rows(&[0], 0..2);
        assert_eq!(s.cell_formats.len(), 1, "the wholly deleted span is gone");
        assert_eq!(rows_of(&s)[0].0, vec![2]);
        s.apply_cell_format(FormatTarget::cells(vec![A], 0, Some(4)), bold());
        s.delete_rows(&[0], 2..9);
        assert_eq!(rows_of(&s).last().unwrap().1, Some((0, Some(1))), "span cut at the deleted rows");
    }

    #[test]
    fn sorting_moves_formats_with_their_rows() {
        let mut s = sheet();
        s.apply_cell_format(FormatTarget::cells(vec![A], 0, Some(0)), bold());
        s.apply_cell_format(FormatTarget::cells(vec![B], 0, Some(0)), fill(7, 7, 7));
        s.apply_cell_format(
            FormatTarget::whole_columns(vec![A]),
            CellFormat { italic: Some(true), ..Default::default() },
        );
        // Sort A ascending: [3,1,2,5,4] -> order of old rows [1,2,0,4,3]; A and B move.
        s.formats_reorder_rows(&|c| c == A || c == B, &[1, 2, 0, 4, 3], 5);
        assert!(is_bold(&s, A, 2) && !is_bold(&s, A, 0));
        assert_eq!(s.cell_format_at(B, 2).fill, Some(Color::rgb(7, 7, 7)));
        assert!(s.cell_format_at(A, 3).italic, "whole column italics untouched");
        // Columns that do not move keep their formats where they are.
        let mut t = sheet();
        t.apply_cell_format(FormatTarget::cells(vec![A, C], 0, Some(0)), bold());
        t.formats_reorder_rows(&|c| c == A, &[1, 2, 0, 4, 3], 5);
        assert!(is_bold(&t, A, 2) && !is_bold(&t, A, 0));
        assert!(is_bold(&t, C, 0) && !is_bold(&t, C, 2));
    }

    #[test]
    fn a_reorder_joins_rows_that_end_up_together_and_keeps_open_tails() {
        let mut s = sheet();
        s.apply_cell_format(FormatTarget::cells(vec![A], 0, Some(1)), bold());
        // Rows 0 and 1 go to rows 3 and 4: one span.
        s.formats_reorder_rows(&|_| true, &[2, 3, 4, 0, 1], 5);
        assert_eq!(rows_of(&s), vec![(vec![1], Some((3, Some(4))))]);
        let mut t = sheet();
        t.apply_cell_format(FormatTarget::cells(vec![A], 1, None), bold());
        t.formats_reorder_rows(&|_| true, &[4, 3, 2, 1, 0], 5);
        assert_eq!(
            rows_of(&t),
            vec![(vec![1], Some((0, Some(3)))), (vec![1], Some((5, None)))],
            "old rows 1..=4 are now rows 3..=0; the old row 0 is row 4; the tail beyond the data stays open"
        );
        assert!(is_bold(&t, A, 0) && !is_bold(&t, A, 4) && is_bold(&t, A, 99));
        // Rows an order leaves out lose the format.
        let mut u = sheet();
        u.apply_cell_format(FormatTarget::cells(vec![A], 0, Some(4)), bold());
        u.formats_reorder_rows(&|_| true, &[0, 2, 4], 5);
        assert_eq!(rows_of(&u), vec![(vec![1], Some((0, Some(2))))]);
    }

    #[test]
    fn remap_renames_and_drops_columns() {
        let mut s = sheet();
        s.apply_cell_format(FormatTarget::whole_columns(vec![A, B]), bold());
        s.apply_cell_format(FormatTarget::label_rows(vec![A], vec![LabelKind::Units]), bold());
        s.remap_format_columns(&|c| (c == B).then_some(ColumnId(10)));
        assert_eq!(s.cell_formats.len(), 1, "the label range lost its only column");
        assert_eq!(s.cell_formats[0].target.columns(), &[ColumnId(10)]);
        s.remap_format_columns(&|_| None);
        assert!(s.cell_formats.is_empty());
    }

    #[test]
    fn formats_round_trip_through_json() {
        let mut s = sheet();
        s.apply_cell_format(
            FormatTarget::cells(vec![A, C], 1, None),
            CellFormat {
                bold: Some(true),
                font: Some("Arial".into()),
                size_pt: Some(14.0),
                text_color: Some(Color::rgb(1, 2, 3)),
                fill: Some(Color::rgb(4, 5, 6)),
                h_align: Some(HAlign::Center),
                ..Default::default()
            },
        );
        s.apply_cell_format(FormatTarget::label_rows(vec![B], vec![LabelKind::Param("T".into())]), bold());
        let back: Worksheet = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        assert_eq!(back.cell_formats, s.cell_formats);
    }

    #[test]
    fn row_span_arithmetic() {
        assert_eq!(span_minus(None, Some((2, Some(3)))), vec![Some((0, Some(1))), Some((4, None))]);
        assert_eq!(span_minus(Some((2, Some(3))), None), Vec::<Option<RowSpan>>::new());
        assert_eq!(span_minus(Some((2, Some(3))), Some((9, None))), vec![Some((2, Some(3)))]);
        assert!(span_covers(None, Some((5, None))) && !span_covers(Some((1, Some(3))), Some((1, None))));
    }
}
