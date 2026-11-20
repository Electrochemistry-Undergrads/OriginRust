//! Printing a worksheet as a table (File > Print and Print Preview with a workbook active). Pure
//! arithmetic and drawing instructions, no printer and no fonts: the sheet is cut into pages by
//! row heights and column widths in points ([`paginate`]), and each page is described as a list of
//! [`PrintOp`]s in points from the top left corner of the printed area. `sheaf_plot` draws the
//! operations as EMF, PDF or a preview picture, so Print Preview shows what the printer gets.
//! See `docs/spec/graphing.md`, section "Printing", for what Origin does and what is Sheaf's rule.

use crate::cellformat::{FormatIndex, HAlign, LabelKind, ResolvedStyle};
use crate::color::Color;
use crate::column::{Column, DEFAULT_WIDTH, Designation};
use crate::condformat::CellStyles;
use crate::page_setup::PageSetup;
use crate::worksheet::Worksheet;
use std::ops::Range;

mod row_heights;
pub use row_heights::RowHeights;

/// The measures of a printed table, in points.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct PrintStyle {
    pub font_pt: f64,
    pub row_h: f64,
    /// Space between a cell's edge and its text.
    pub pad: f64,
    /// Width of the column of row numbers at the left of every page.
    pub rownum_w: f64,
    /// Width of a column that is `DEFAULT_WIDTH` characters wide.
    pub default_col_w: f64,
    pub narrowest_col_w: f64,
    /// The line of "Page n of m" under the table; 0 for no page numbers.
    pub footer_h: f64,
    pub grid_pt: f64,
}

impl Default for PrintStyle {
    fn default() -> Self {
        PrintStyle {
            font_pt: 8.0,
            row_h: 12.0,
            pad: 3.0,
            rownum_w: 30.0,
            default_col_w: 50.0,
            narrowest_col_w: 6.0,
            footer_h: 16.0,
            grid_pt: 0.5,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
enum LabelRow {
    LongName,
    Units,
    Comments,
    Formula,
    Param(String),
}

impl LabelRow {
    fn kind(&self) -> LabelKind {
        match self {
            LabelRow::LongName => LabelKind::LongName,
            LabelRow::Units => LabelKind::Units,
            LabelRow::Comments => LabelKind::Comments,
            LabelRow::Formula => LabelKind::Formula,
            LabelRow::Param(n) => LabelKind::Param(n.clone()),
        }
    }

    fn title(&self) -> String {
        match self {
            LabelRow::LongName => "Long Name".into(),
            LabelRow::Units => "Units".into(),
            LabelRow::Comments => "Comments".into(),
            LabelRow::Formula => "F(x)=".into(),
            LabelRow::Param(n) => n.clone(),
        }
    }

    fn text(&self, c: &Column) -> String {
        match self {
            LabelRow::LongName => c.long_name.clone(),
            LabelRow::Units => c.units.clone(),
            LabelRow::Comments => c.comments.lines().next().unwrap_or_default().to_string(),
            LabelRow::Formula => c.formula.clone().unwrap_or_default(),
            LabelRow::Param(n) => c.params.get(n).cloned().unwrap_or_default(),
        }
    }
}

/// The name in the grey row of a column head: the letter and, in brackets, the kind of data and
/// the number of the data set it belongs to ("A(X1)", "B(Y1)", "D(X2)").
pub fn head_name(sheet: &Worksheet, position: usize) -> String {
    let column = &sheet.columns[position];
    let tag = column.designation.tag();
    if tag.is_empty() {
        return column.short_name.clone();
    }
    match column.designation {
        Designation::Label => format!("{}({tag})", column.short_name),
        _ => {
            let group = sheet.columns[..=position].iter().filter(|c| c.designation == Designation::X).count().max(1);
            format!("{}({tag}{group})", column.short_name)
        }
    }
}

/// What a worksheet prints: its visible columns and the rows its filter lets show, with the label
/// rows that have any text. Borrowed from the sheet; cell texts are made as pages are drawn.
#[derive(Clone, Debug)]
pub struct SheetTable<'a> {
    sheet: &'a Worksheet,
    /// Positions in `sheet.columns` of the columns that are not hidden.
    cols: Vec<usize>,
    /// 0-based sheet rows that show, in order.
    rows: Vec<usize>,
    labels: Vec<LabelRow>,
    style: PrintStyle,
    /// Manual cell formats and the conditional-format colours, so the print shows the sheet as
    /// the grid does.
    formats: FormatIndex,
    cond: Option<CellStyles>,
    row_heights: RowHeights,
    label_heights: Vec<f64>,
}

impl<'a> SheetTable<'a> {
    pub fn new(sheet: &'a Worksheet, style: PrintStyle) -> Self {
        Self::with_metrics(sheet, style, &|_, size| size * 1.2 + 2.0)
    }

    /// Supply installed font line metrics without coupling the model to a font database.
    pub fn with_metrics(
        sheet: &'a Worksheet,
        style: PrintStyle,
        line_height: &dyn Fn(ResolvedStyle<'_>, f64) -> f64,
    ) -> Self {
        let cols: Vec<usize> = (0..sheet.columns.len()).filter(|&i| !sheet.columns[i].hidden).collect();
        let n = sheet.row_count();
        let rows = sheet.shown_rows(n).unwrap_or_else(|| (0..n).collect());
        let mut labels = vec![LabelRow::LongName, LabelRow::Units, LabelRow::Comments, LabelRow::Formula];
        labels.extend(sheet.parameter_names().into_iter().map(LabelRow::Param));
        labels.retain(|l| cols.iter().any(|&c| !l.text(&sheet.columns[c]).is_empty()));
        let formats = sheet.format_index();
        let cond = (!sheet.cond_formats.is_empty()).then(|| crate::condformat::evaluate(sheet, &sheet.cond_formats));
        let height = |st: ResolvedStyle<'_>| {
            if st.size_pt.is_none() && st.font.is_none() {
                return style.row_h;
            }
            let size = st.size_pt.map_or(style.font_pt, |p| {
                f64::from(crate::cellformat::clamp_cell_pt(p)) * style.font_pt / GRID_BODY_PT
            });
            line_height(st, size).max(style.row_h)
        };
        let row_heights = RowHeights::new(sheet, &formats, style.row_h, height).shown(&rows);
        let label_heights = labels
            .iter()
            .map(|l| {
                cols.iter()
                    .map(|&c| height(formats.resolve_label(sheet, sheet.columns[c].id, &l.kind())))
                    .fold(style.row_h, f64::max)
            })
            .collect();
        SheetTable { sheet, cols, rows, labels, style, formats, cond, row_heights, label_heights }
    }

    /// The manual format of a data cell with the conditional colours over it.
    fn cell_style(&self, column: &Column, row: usize) -> ResolvedStyle<'a> {
        let manual = self.formats.resolve(self.sheet, column.id, row);
        manual.with_conditional(self.cond.as_ref().and_then(|c| c.get(column.id, row)))
    }

    /// The text of a cell in the column at `cx` of width `cw`, placed by its alignment (or the
    /// default of its kind) and drawn in its manual font, size, colour and style.
    fn cell_text(&self, cx: f64, cw: f64, y: f64, text: String, default: Align, st: &ResolvedStyle<'_>) -> PrintOp {
        let s = &self.style;
        let align = match st.h_align {
            Some(HAlign::Left) => Align::Left,
            Some(HAlign::Center) => Align::Center,
            Some(HAlign::Right) => Align::Right,
            Some(HAlign::Auto) | None => default,
        };
        let x = match align {
            Align::Left => cx + s.pad,
            Align::Center => cx + cw / 2.0,
            Align::Right => cx + cw - s.pad,
        };
        PrintOp::Text {
            x,
            y,
            max_w: cw - 2.0 * s.pad,
            text,
            align,
            bold: st.bold,
            size: st
                .size_pt
                .map_or(s.font_pt, |p| f64::from(crate::cellformat::clamp_cell_pt(p)) * s.font_pt / GRID_BODY_PT),
            fmt: TextFmt {
                italic: st.italic,
                underline: st.underline,
                family: st.font.map(str::to_string),
                color: st.text_color,
            },
        }
    }

    pub fn column_count(&self) -> usize {
        self.cols.len()
    }

    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    /// Height of the head: the grey name row and the label rows.
    pub fn head_height(&self) -> f64 {
        self.style.row_h + self.label_heights.iter().sum::<f64>()
    }

    /// Width in points of each printed column.
    pub fn column_widths(&self) -> Vec<f64> {
        let k = self.style.default_col_w / f64::from(DEFAULT_WIDTH);
        self.cols
            .iter()
            .map(|&c| (f64::from(self.sheet.columns[c].shown_width()) * k).max(self.style.narrowest_col_w))
            .collect()
    }

    /// The pages of a printed area `area_w` x `area_h` points, in printing order.
    pub fn pages(&self, area_w: f64, area_h: f64) -> Vec<PageSlot> {
        let footer = self.style.footer_h;
        let mut pages = Vec::new();
        for col in paginate(
            &self.column_widths(),
            0,
            &PageBox {
                area_w,
                area_h: area_h - footer,
                head_h: self.head_height(),
                row_h: self.style.row_h,
                rownum_w: self.style.rownum_w,
            },
        ) {
            let mut start = 0;
            loop {
                let top = self.row_heights.offset(start);
                let room = (area_h - footer - self.head_height()).max(0.0);
                let mut end = self.row_heights.row_at(top + room + 1e-9).min(self.row_count());
                // row_at returns the row at an exact boundary; a partial row belongs on the next page.
                if self.row_heights.offset(end) > top + room + 1e-8 {
                    end = end.saturating_sub(1);
                }
                end = end.max(start.saturating_add(1)).min(self.row_count());
                pages.push(PageSlot { number: pages.len() + 1, rows: start..end, cols: col.cols.clone() });
                if end >= self.row_count() {
                    break;
                }
                start = end;
            }
        }
        pages
    }

    /// Where the pages of a printed area `area_w` x `area_h` points break (Page Break Preview Lines,
    /// T185): the sheet rows that end a page and the columns that end a band of columns, the last
    /// row and column left out (nothing follows them).
    pub fn page_breaks(&self, area_w: f64, area_h: f64) -> PageBreaks {
        let pages = self.pages(area_w, area_h);
        let mut breaks = PageBreaks::default();
        for page in &pages {
            if page.rows.end < self.rows.len()
                && let Some(&row) = page.rows.end.checked_sub(1).and_then(|i| self.rows.get(i))
            {
                breaks.rows.insert(row);
            }
            if page.cols.end < self.cols.len()
                && let Some(&col) = page.cols.end.checked_sub(1).and_then(|i| self.cols.get(i))
            {
                breaks.columns.insert(self.sheet.columns[col].id);
            }
        }
        breaks
    }

    /// The pages of the sheet on the paper of `setup`: the area inside the margins.
    pub fn pages_for(&self, setup: &PageSetup) -> (Vec<PageSlot>, (f64, f64)) {
        let area = print_area(setup);
        (self.pages(area.0, area.1), area)
    }

    /// The drawing instructions of one page; `of` is the number of pages for the page number.
    pub fn draw_page(&self, slot: &PageSlot, of: usize) -> Vec<PrintOp> {
        let s = &self.style;
        let widths = self.column_widths();
        let head_h = self.head_height();
        let line = Color::rgb(128, 128, 128);
        let head_fill = Color::rgb(222, 222, 222);
        let label_fill = Color::rgb(255, 255, 214);
        let mut ops = Vec::new();
        let cols: Vec<usize> = slot.cols.clone().collect();
        let rows: Vec<usize> = slot.rows.clone().collect();
        let width: f64 = s.rownum_w + cols.iter().map(|&c| widths[c]).sum::<f64>();
        let body_top = self.row_heights.offset(slot.rows.start);
        let row_top = |r| head_h + self.row_heights.offset(r) - body_top;
        let label_top = |i: usize| s.row_h + self.label_heights[..i].iter().sum::<f64>();
        let height = row_top(slot.rows.end);
        // Fills first so the grid is drawn over them.
        ops.push(PrintOp::Fill { x: 0.0, y: 0.0, w: width, h: s.row_h, color: head_fill });
        ops.push(PrintOp::Fill { x: 0.0, y: s.row_h, w: width, h: head_h - s.row_h, color: label_fill });
        ops.push(PrintOp::Fill { x: 0.0, y: head_h, w: s.rownum_w, h: height - head_h, color: head_fill });
        // Manual and conditional fills of cells and label cells.
        if !self.formats.is_empty() || self.cond.is_some() {
            let mut x = s.rownum_w;
            for &c in &cols {
                let cw = widths[c];
                let column = &self.sheet.columns[self.cols[c]];
                for (i, l) in self.labels.iter().enumerate() {
                    if let Some(color) = self.formats.resolve_label(self.sheet, column.id, &l.kind()).fill {
                        ops.push(PrintOp::Fill { x, y: label_top(i), w: cw, h: self.label_heights[i], color });
                    }
                }
                for &r in &rows {
                    if let Some(color) = self.cell_style(column, self.rows[r]).fill {
                        ops.push(PrintOp::Fill { x, y: row_top(r), w: cw, h: self.row_heights.height(r), color });
                    }
                }
                x += cw;
            }
        }
        // The grid.
        for y in std::iter::once(0.0)
            .chain((0..=self.labels.len()).map(label_top))
            .chain(rows.iter().map(|r| row_top(r + 1)))
        {
            ops.push(PrintOp::Line { x1: 0.0, y1: y, x2: width, y2: y, width: s.grid_pt, color: line });
        }
        let mut x = 0.0;
        ops.push(PrintOp::Line { x1: x, y1: 0.0, x2: x, y2: height, width: s.grid_pt, color: line });
        x += s.rownum_w;
        ops.push(PrintOp::Line { x1: x, y1: 0.0, x2: x, y2: height, width: s.grid_pt, color: line });
        let mut xs = Vec::with_capacity(cols.len());
        for &c in &cols {
            xs.push(x);
            x += widths[c];
            ops.push(PrintOp::Line { x1: x, y1: 0.0, x2: x, y2: height, width: s.grid_pt, color: line });
        }
        let base = |top: f64, height: f64, size: f64| top + (height + size * 0.7) / 2.0 - 0.5;
        let cell_size = |st: &ResolvedStyle<'_>| {
            st.size_pt.map_or(s.font_pt, |p| f64::from(crate::cellformat::clamp_cell_pt(p)) * s.font_pt / GRID_BODY_PT)
        };
        // Head: names, then the label rows.
        for (k, &c) in cols.iter().enumerate() {
            let (cx, cw) = (xs[k], widths[c]);
            let pos = self.cols[c];
            ops.push(PrintOp::Text {
                x: cx + cw / 2.0,
                y: base(0.0, s.row_h, s.font_pt),
                max_w: cw - 2.0 * s.pad,
                text: head_name(self.sheet, pos),
                align: Align::Center,
                bold: true,
                size: s.font_pt,
                fmt: TextFmt::default(),
            });
            for (i, l) in self.labels.iter().enumerate() {
                let text = l.text(&self.sheet.columns[pos]);
                if text.is_empty() {
                    continue;
                }
                let st = self.formats.resolve_label(self.sheet, self.sheet.columns[pos].id, &l.kind());
                ops.push(self.cell_text(
                    cx,
                    cw,
                    base(label_top(i), self.label_heights[i], cell_size(&st)),
                    text,
                    Align::Left,
                    &st,
                ));
            }
        }
        for (i, l) in self.labels.iter().enumerate() {
            ops.push(PrintOp::Text {
                x: s.rownum_w - s.pad,
                y: base(label_top(i), self.label_heights[i], s.font_pt),
                max_w: s.rownum_w - 2.0 * s.pad,
                text: l.title(),
                align: Align::Right,
                bold: false,
                size: s.font_pt,
                fmt: TextFmt::default(),
            });
        }
        // Body: row numbers and the cells as the sheet shows them.
        for &r in &rows {
            let top = row_top(r);
            ops.push(PrintOp::Text {
                x: s.rownum_w - s.pad,
                y: base(top, self.row_heights.height(r), s.font_pt),
                max_w: s.rownum_w - 2.0 * s.pad,
                text: (self.rows[r] + 1).to_string(),
                align: Align::Right,
                bold: false,
                size: s.font_pt,
                fmt: TextFmt::default(),
            });
            for (k, &c) in cols.iter().enumerate() {
                let column = &self.sheet.columns[self.cols[c]];
                let text = column.shown_text(self.rows[r]);
                if text.is_empty() {
                    continue;
                }
                let numeric = !matches!(column.data, crate::column::ColumnData::Text(_))
                    && column.data.text_cell(self.rows[r]).is_none();
                let (cx, cw) = (xs[k], widths[c]);
                let default = if numeric { Align::Right } else { Align::Left };
                let st = self.cell_style(column, self.rows[r]);
                ops.push(self.cell_text(
                    cx,
                    cw,
                    base(top, self.row_heights.height(r), cell_size(&st)),
                    text,
                    default,
                    &st,
                ));
            }
        }
        if s.footer_h > 0.0 {
            ops.push(PrintOp::Text {
                x: 0.0,
                y: height + s.footer_h * 0.75,
                max_w: width,
                text: format!("Page {} of {of}", slot.number),
                align: Align::Left,
                bold: false,
                size: s.font_pt,
                fmt: TextFmt::default(),
            });
        }
        ops
    }
}

/// The printed area in points: the paper of `setup` (a sheet is taller than wide unless the setup
/// says landscape) minus the margins. Zero or negative sizes mean the margins leave no room.
pub fn print_area(setup: &PageSetup) -> (f64, f64) {
    let (pw, ph) = setup.paper_pt(0.0, 1.0);
    (pw - setup.margins.left - setup.margins.right, ph - setup.margins.top - setup.margins.bottom)
}

/// Horizontal text placement of a [`PrintOp::Text`] about its `x`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Align {
    Left,
    Center,
    Right,
}

/// One drawing instruction of a printed page, in points from the top left of the printed area.
#[derive(Clone, Debug, PartialEq)]
pub enum PrintOp {
    Fill {
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        color: Color,
    },
    Line {
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        width: f64,
        color: Color,
    },
    /// Text at its baseline `y`; cut (not wrapped) to `max_w` points when drawn.
    Text {
        x: f64,
        y: f64,
        max_w: f64,
        text: String,
        align: Align,
        bold: bool,
        size: f64,
        /// Manual cell format beyond bold and size (Format Cells); default for all other text.
        fmt: TextFmt,
    },
}

/// The parts of a text's look that only formatted cells use.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextFmt {
    pub italic: bool,
    pub underline: bool,
    /// A font family name; `None` is the printout's own font.
    pub family: Option<String>,
    /// `None` is black.
    pub color: Option<Color>,
}

/// The font size of the grid's body text in points; a formatted size is scaled by the ratio of
/// the printed font to this, so that a cell looks as big next to the others as it does on screen.
const GRID_BODY_PT: f64 = 9.0;

/// Where printed pages break in a sheet (see [`SheetTable::page_breaks`]).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PageBreaks {
    /// 0-based sheet rows after which a new page starts.
    pub rows: std::collections::BTreeSet<usize>,
    /// Columns after which a new band of pages starts.
    pub columns: std::collections::BTreeSet<crate::ids::ColumnId>,
}

/// One printed page: which of the table's shown rows and visible columns it holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PageSlot {
    /// 1-based position in the printing order.
    pub number: usize,
    /// Indices into the table's shown rows.
    pub rows: Range<usize>,
    /// Indices into the table's visible columns.
    pub cols: Range<usize>,
}

/// The measures [`paginate`] cuts by, in points.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct PageBox {
    pub area_w: f64,
    /// The height for head and rows (the footer is already taken off).
    pub area_h: f64,
    pub head_h: f64,
    pub row_h: f64,
    /// The row-number column, repeated on every page.
    pub rownum_w: f64,
}

/// Splits `n_rows` rows and columns of `widths` points into pages: the rows go down first (each
/// page takes as many rows as fit under the head), then the next band of columns starts again at
/// the first row. Every page holds at least one row and one column, so a column wider than the
/// page gets a page of its own (its text is cut). Always at least one page.
pub fn paginate(widths: &[f64], n_rows: usize, b: &PageBox) -> Vec<PageSlot> {
    let per_page = (((b.area_h - b.head_h) / b.row_h).floor().max(1.0)) as usize;
    let mut row_bands = Vec::new();
    let mut r = 0;
    while r < n_rows {
        let end = (r + per_page).min(n_rows);
        row_bands.push(r..end);
        r = end;
    }
    if row_bands.is_empty() {
        row_bands.push(0..0);
    }
    let mut col_bands = Vec::new();
    let mut c = 0;
    while c < widths.len() {
        let mut end = c;
        let mut used = b.rownum_w;
        while end < widths.len() && (end == c || used + widths[end] <= b.area_w + 1e-9) {
            used += widths[end];
            end += 1;
        }
        col_bands.push(c..end);
        c = end;
    }
    if col_bands.is_empty() {
        col_bands.push(0..0);
    }
    let mut pages = Vec::new();
    for cols in &col_bands {
        for rows in &row_bands {
            pages.push(PageSlot { number: pages.len() + 1, rows: rows.clone(), cols: cols.clone() });
        }
    }
    pages
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::column::{Column, ColumnData};
    use crate::ids::ColumnId;
    use crate::page_setup::{Margins, Orientation, Paper};

    fn bx() -> PageBox {
        PageBox { area_w: 200.0, area_h: 100.0, head_h: 20.0, row_h: 10.0, rownum_w: 20.0 }
    }

    #[test]
    fn rows_go_down_first_then_the_next_columns() {
        // 8 rows fit under the head; 25 rows are 4 bands (8, 8, 8, 1). 100 + 100 + 100 wide
        // columns with a 20 wide row-number column: two fit (20 + 180), the third is a new band.
        let pages = paginate(&[90.0, 90.0, 90.0], 25, &bx());
        assert_eq!(pages.len(), 8);
        assert_eq!((pages[0].rows.clone(), pages[0].cols.clone()), (0..8, 0..2));
        assert_eq!(pages[3].rows, 24..25);
        assert_eq!(pages[3].cols, 0..2);
        assert_eq!((pages[4].rows.clone(), pages[4].cols.clone()), (0..8, 2..3));
        assert_eq!(pages[7].number, 8);
    }

    #[test]
    fn an_exact_fit_does_not_start_a_new_page() {
        let pages = paginate(&[90.0, 90.0], 8, &bx());
        assert_eq!(pages.len(), 1);
        let pages = paginate(&[90.0, 90.0], 9, &bx());
        assert_eq!(pages.len(), 2);
    }

    #[test]
    fn a_column_wider_than_the_page_still_gets_a_page() {
        let pages = paginate(&[500.0, 30.0], 3, &bx());
        assert_eq!(pages.iter().map(|p| p.cols.clone()).collect::<Vec<_>>(), vec![0..1, 1..2]);
    }

    #[test]
    fn a_page_too_short_for_the_head_still_holds_one_row() {
        let b = PageBox { area_h: 5.0, ..bx() };
        assert_eq!(paginate(&[50.0], 3, &b).len(), 3);
    }

    #[test]
    fn an_empty_sheet_is_one_page() {
        let pages = paginate(&[], 0, &bx());
        assert_eq!(pages.len(), 1);
        assert_eq!((pages[0].rows.clone(), pages[0].cols.clone()), (0..0, 0..0));
    }

    fn sheet() -> Worksheet {
        let mut ws = Worksheet::new("Sheet1");
        let mut a =
            Column::new(ColumnId(1), "A", Designation::X, ColumnData::numeric((1..=40).map(f64::from).collect()))
                .with_labels("Index", "n", "");
        a.width = Some(8);
        let b = Column::new(
            ColumnId(2),
            "B",
            Designation::Y,
            ColumnData::numeric((1..=40).map(|i| f64::from(i) * 0.5).collect()),
        )
        .with_labels("Half", "", "first scan");
        let c = Column::new(
            ColumnId(3),
            "C",
            Designation::Y,
            ColumnData::text((1..=40).map(|i| format!("t{i}")).collect()),
        );
        ws.columns = vec![a, b, c];
        ws
    }

    fn setup() -> PageSetup {
        PageSetup { paper: Paper::Letter, orientation: Orientation::Portrait, margins: Margins::all(72.0) }
    }

    #[test]
    fn the_printed_area_is_the_paper_inside_the_margins() {
        assert_eq!(print_area(&setup()), (468.0, 648.0));
        let land = PageSetup { orientation: Orientation::Landscape, ..setup() };
        assert_eq!(print_area(&land), (648.0, 468.0));
    }

    #[test]
    fn the_head_has_a_row_per_label_row_that_has_text() {
        let ws = sheet();
        let t = SheetTable::new(&ws, PrintStyle::default());
        // Long Name, Units and Comments have text somewhere; F(x)= has none.
        assert_eq!(t.head_height(), PrintStyle::default().row_h * 4.0);
        assert_eq!(t.column_widths(), vec![50.0, 50.0, 50.0]);
    }

    #[test]
    fn hidden_columns_and_filtered_rows_are_left_out() {
        let mut ws = sheet();
        ws.columns[1].hidden = true;
        let t = SheetTable::new(&ws, PrintStyle::default());
        assert_eq!(t.column_count(), 2);
        let (pages, _) = t.pages_for(&setup());
        assert_eq!(pages.len(), 1);
        let ops = t.draw_page(&pages[0], 1);
        let texts: Vec<&str> = ops
            .iter()
            .filter_map(|o| if let PrintOp::Text { text, .. } = o { Some(text.as_str()) } else { None })
            .collect();
        assert!(texts.contains(&"A(X1)") && texts.contains(&"C(Y1)"));
        assert!(!texts.contains(&"B(Y1)") && !texts.contains(&"first scan"));
        assert!(texts.contains(&"Page 1 of 1"));
    }

    #[test]
    fn cells_print_as_the_sheet_shows_them() {
        let mut ws = sheet();
        ws.columns[1].data = ColumnData::numeric(vec![0.841470985, f64::NAN]);
        let t = SheetTable::new(&ws, PrintStyle::default());
        let (pages, _) = t.pages_for(&setup());
        let ops = t.draw_page(&pages[0], 1);
        let has = |s: &str| ops.iter().any(|o| matches!(o, PrintOp::Text { text, .. } if text == s));
        assert!(has("0.84147"), "five decimals as the grid shows");
        assert!(has("t40"));
    }

    fn styled() -> crate::cellformat::CellFormat {
        crate::cellformat::CellFormat {
            bold: Some(true),
            italic: Some(true),
            underline: Some(true),
            font: Some("Courier New".into()),
            size_pt: Some(24.0),
            text_color: Some(Color::rgb(200, 0, 0)),
            fill: Some(Color::rgb(0, 200, 0)),
            h_align: Some(HAlign::Center),
            rich_text: None,
            float: None,
        }
    }

    fn text_op<'a>(ops: &'a [PrintOp], t: &str) -> &'a PrintOp {
        ops.iter().find(|o| matches!(o, PrintOp::Text { text, .. } if text == t)).expect(t)
    }

    #[test]
    fn sizes_are_clamped_like_the_grid() {
        use crate::cellformat::{CellFormat, FormatTarget, MAX_CELL_PT, MIN_CELL_PT};
        let mut ws = sheet();
        let size = |pt: f32| CellFormat { size_pt: Some(pt), ..Default::default() };
        ws.apply_cell_format(FormatTarget::cells(vec![ColumnId(3)], 0, Some(0)), size(0.25));
        ws.apply_cell_format(FormatTarget::cells(vec![ColumnId(3)], 1, Some(1)), size(500.0));
        let t = SheetTable::new(&ws, PrintStyle::default());
        let (pages, _) = t.pages_for(&setup());
        let ops = t.draw_page(&pages[0], 1);
        let PrintOp::Text { size: small, .. } = text_op(&ops, "t1") else { unreachable!() };
        let PrintOp::Text { size: big, .. } = text_op(&ops, "t2") else { unreachable!() };
        assert_eq!(*small, f64::from(MIN_CELL_PT) * 8.0 / 9.0);
        assert_eq!(*big, f64::from(MAX_CELL_PT) * 8.0 / 9.0);
    }

    #[test]
    fn formatted_cells_print_styled_and_unformatted_ones_do_not() {
        use crate::cellformat::FormatTarget;
        let mut ws = sheet();
        ws.apply_cell_format(FormatTarget::cells(vec![ColumnId(3)], 1, Some(1)), styled());
        let t = SheetTable::new(&ws, PrintStyle::default());
        let (pages, _) = t.pages_for(&setup());
        let ops = t.draw_page(&pages[0], 1);
        let PrintOp::Text { bold, size, fmt, align, x, .. } = text_op(&ops, "t2") else { unreachable!() };
        assert!(*bold && fmt.italic && fmt.underline);
        assert_eq!(fmt.family.as_deref(), Some("Courier New"));
        assert_eq!(fmt.color, Some(Color::rgb(200, 0, 0)));
        assert_eq!(*align, Align::Center);
        assert_eq!(*size, 24.0 * 8.0 / 9.0, "scaled like the printed font");
        // The cell's centre: the row number column (30) plus two 50 pt columns plus half of the third.
        assert_eq!(*x, 30.0 + 100.0 + 25.0);
        let PrintOp::Text { bold, fmt, align, .. } = text_op(&ops, "t1") else { unreachable!() };
        assert!(!*bold && *fmt == TextFmt::default() && *align == Align::Left);
        let fills: Vec<_> = ops
            .iter()
            .filter(|o| matches!(o, PrintOp::Fill { color, .. } if *color == Color::rgb(0, 200, 0)))
            .collect();
        assert_eq!(fills.len(), 1, "one filled cell");
        let PrintOp::Fill { x, w, h, y, .. } = fills[0] else { unreachable!() };
        assert_eq!((*x, *w), (130.0, 50.0));
        assert!(*h > 24.0, "the large font grows its printed row");
        assert_eq!(*y, t.head_height() + 12.0, "the second data row");
    }

    #[test]
    fn printed_label_cells_take_their_format_and_conditional_colours_win() {
        use crate::cellformat::{CellFormat, FormatTarget, LabelKind};
        use crate::condformat::{CondFormat, CondRange, Rule};
        let mut ws = sheet();
        ws.apply_cell_format(
            FormatTarget::label_rows(vec![ColumnId(1)], vec![LabelKind::LongName]),
            CellFormat { bold: Some(true), fill: Some(Color::rgb(1, 2, 3)), ..Default::default() },
        );
        // B: manual fill and text colour on every row; a conditional rule colours the maximum.
        ws.apply_cell_format(
            FormatTarget::whole_columns(vec![ColumnId(2)]),
            CellFormat {
                fill: Some(Color::rgb(9, 9, 9)),
                text_color: Some(Color::rgb(9, 9, 8)),
                bold: Some(true),
                ..Default::default()
            },
        );
        let colours = |fill, font| crate::condformat::Colours { fill: Some(fill), font: Some(font) };
        let rule = CondFormat::new(
            &ws,
            CondRange::whole_columns(vec![ColumnId(2)]),
            Rule::MinMax {
                dir: crate::condformat::Direction::ColumnWise,
                max: colours(crate::condformat::MAX_FILL, crate::condformat::MAX_FONT),
                min: colours(crate::condformat::MIN_FILL, crate::condformat::MIN_FONT),
            },
        );
        ws.cond_formats.push(rule);
        let t = SheetTable::new(&ws, PrintStyle::default());
        let (pages, _) = t.pages_for(&setup());
        let ops = t.draw_page(&pages[0], 1);
        assert!(matches!(text_op(&ops, "Index"), PrintOp::Text { bold: true, .. }));
        assert!(ops.iter().any(|o| matches!(o, PrintOp::Fill { color, .. } if *color == Color::rgb(1, 2, 3))));
        // Of 40 rows of B, the smallest and the largest get the conditional fill instead.
        let manual = |o: &&PrintOp| matches!(o, PrintOp::Fill { color, .. } if *color == Color::rgb(9, 9, 9));
        assert_eq!(ops.iter().filter(manual).count(), 38);
        assert!(ops.iter().any(|o| matches!(o, PrintOp::Fill { color, .. } if *color == crate::condformat::MIN_FILL)));
        let PrintOp::Text { fmt, bold, .. } = text_op(&ops, "0.5") else { unreachable!() };
        assert_eq!(fmt.color, Some(crate::condformat::MIN_FONT));
        assert!(*bold, "manual bold stays under a conditional colour");
        let PrintOp::Text { fmt, .. } = text_op(&ops, "1.5") else { unreachable!() };
        assert_eq!(fmt.color, Some(Color::rgb(9, 9, 8)), "unmarked cells keep the manual colour");
    }

    #[test]
    fn a_long_sheet_runs_over_pages_and_numbers_them() {
        let mut ws = sheet();
        for c in &mut ws.columns {
            c.width = Some(40); // 250 pt each: only one fits beside the row numbers on a portrait page
        }
        let t = SheetTable::new(&ws, PrintStyle::default());
        let (pages, area) = t.pages_for(&setup());
        assert_eq!(area, (468.0, 648.0));
        // (648 - 16 footer - 48 head) / 12 = 48 rows per page: 40 rows fit; 468 - 30 = 438: one column.
        assert_eq!(pages.len(), 3);
        let last = t.draw_page(&pages[2], 3);
        assert!(last.iter().any(|o| matches!(o, PrintOp::Text { text, .. } if text == "Page 3 of 3")));
        // Nothing is drawn outside the printed area.
        for p in &pages {
            for o in t.draw_page(p, pages.len()) {
                if let PrintOp::Line { x2, y2, .. } = o {
                    assert!(x2 <= area.0 + 1e-6 && y2 <= area.1 + 1e-6);
                }
            }
        }
    }

    #[test]
    fn filtered_rows_are_left_out_and_keep_their_own_numbers() {
        let mut ws = sheet();
        ws.filter = Some(crate::rowfilter::RowFilter {
            condition: "A > 38".into(),
            enabled: true,
            keep: (1..=40).map(|i| i > 38).collect(),
        });
        let t = SheetTable::new(&ws, PrintStyle::default());
        assert_eq!(t.row_count(), 2);
        let (pages, _) = t.pages_for(&setup());
        let ops = t.draw_page(&pages[0], 1);
        let has = |s: &str| ops.iter().any(|o| matches!(o, PrintOp::Text { text, .. } if text == s));
        assert!(has("39") && has("40") && !has("t1") && !has("t38"));
    }

    #[test]
    fn page_breaks_mark_the_last_row_and_column_of_each_band_but_the_last() {
        let mut sheet = Worksheet::new("S");
        for k in 0..6 {
            let mut c = Column::new(
                ColumnId(k + 1),
                crate::column::default_short_name(k as usize),
                crate::column::Designation::Y,
                ColumnData::numeric((0..200).map(f64::from).collect()),
            );
            c.width = Some(30);
            sheet.columns.push(c);
        }
        let table = SheetTable::new(&sheet, PrintStyle::default());
        let (w, h) = (300.0, 400.0);
        let pages = table.pages(w, h);
        let breaks = table.page_breaks(w, h);
        let row_ends: std::collections::BTreeSet<usize> =
            pages.iter().filter(|p| p.rows.end < 200).map(|p| p.rows.end - 1).collect();
        assert!(!row_ends.is_empty(), "200 rows take more than one page");
        assert_eq!(breaks.rows, row_ends);
        assert!(!breaks.rows.contains(&199), "nothing follows the last row");
        let bands = pages.iter().map(|p| (p.cols.start, p.cols.end)).collect::<std::collections::BTreeSet<_>>().len();
        assert_eq!(breaks.columns.len(), bands - 1);
        // A wide page takes every row and column: no breaks.
        assert_eq!(table.page_breaks(10_000.0, 100_000.0), PageBreaks::default());
    }
}
