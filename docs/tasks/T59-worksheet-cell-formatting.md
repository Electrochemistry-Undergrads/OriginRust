# T59: Worksheet cell formatting (Format Cells, Format toolbar on cells)

**Status:** open. Written 2026-11-15 from the owner's priority "basic formatting (bolding, italics,
etc.)" and the greyed Format Cells items.
**Size:** L. **Branch:** `cell-format`. **Model:** Sonnet. The model and grid drawing need no
Origin; the Format Cells window is measured on Windows.
**Spec:** `docs/spec/workbook-model.md` (new section "Cell formats"), `docs/spec/gui-look.md` 3.
**Depends on:** T58 slice 3 for the toolbar's `FormatTarget` (start with the model and the window;
add the toolbar target when T58's dispatch is on `main`).
**Greyed items closed (6):** Format > Worksheet..., Format > Cell... [Workbook]; context menus:
column header, data cell and row number > Format Cells...; data cell > Set Data Style > Default.

## Goal

A person selects cells, rows or columns in a worksheet and makes them bold, italic or underlined,
changes the font, size, text colour and fill colour, and sets the alignment, from the Format toolbar
and from a Format Cells window. The formats are kept with the cells through sort, insert, delete,
move, duplicate and append, saved in `.sheaf`, and shown in printed sheets and the worksheet image
drawn by `crates/plot/src/sheet.rs`. Label rows (Long Name, Units, Comments) can be formatted too.

## Today

- No per-cell style exists. A column has a display format (Column Properties, T19).
- Conditional formatting (T35) gives cells a fill and a font colour computed from rules:
  `CellStyle { fill, font }` and `CellStyles` in `crates/core/src/condformat.rs`.
- Cell notes (T36, `crates/core/src/notes.rs`) are per-sheet data kept through sort and row/column
  edits by `crates/core/src/sheetops.rs`: the pattern to copy.
- `crates/app/src/grid.rs` draws the cells (`body_cell`), frozen panes and the note tab.

## Origin facts

- Measured (gui-look.md 3): the cell, column-header and row-number menus hold "Format Cells..."; the
  cell menu has a "Set Data Style" submenu; pressing Ctrl+U with a column selected switches on the
  Underline button of the Format toolbar and underlines the cell text.
- Origin's X-Function for it is `wcellformat` (xfunction-index.md).
- **Not measured, measure on Windows:** the Format Cells window (controls, order, defaults, Apply
  to), what Format > Worksheet... opens, the Set Data Style list, which toolbar buttons act on cells
  (superscript and Greek probably need rich text), whether a manual format or a conditional format
  wins on one cell, how formats move on sort, and what `.opj` stores. Until measured, Sheaf's
  choices (label them): a manual format is under a conditional format's colours; formats follow
  their rows on sort.

## Where

- `crates/core/src/cellformat.rs` (new) and `Worksheet.cell_formats`: font style flags, optional
  family, size, text colour, fill colour, horizontal alignment, stored for ranges (column id plus
  row spans, and label rows) so a whole formatted column costs one entry. serde default, not
  written when empty.
- `crates/core/src/sheetops.rs`: keep formats through every row/column operation that keeps notes.
- `crates/app/src/grid.rs`: draw with the cell's format; one lookup structure per document
  revision as `condformat::evaluate` does, so drawing a million rows stays fast.
- `crates/app/src/dialogs/cell_format.rs` (new): the Format Cells window.
- `crates/app/src/toolbar.rs`: worksheet target of T58's `FormatTarget` (bold, italic, underline,
  font, size, font colour, fill colour); Ctrl+B/I/U with a cell selection.
- `crates/plot/src/sheet.rs` and `crates/core/src/sheet_print.rs`: printed and exported sheets.
- `crates/io`: `.sheaf` saves the formats; `.opj` save warns (ADR 0008); Excel export writes the
  font style and colours through `rust_xlsxwriter` when it is a small step, else a leftover.

## Steps

1. Spec section with the model and Sheaf's choices; model with tests (set, clear, merge of
   overlapping ranges, upkeep through sort, insert, delete, move, duplicate, append; undo).
2. Grid drawing and the Format Cells window (one undo step per OK); UI tests on a new and an
   imported table, with frozen panes and a filter on; a million-row sheet still scrolls.
3. Toolbar target and shortcuts; enable the six items and delete them from greyed-items.md.
4. Print and worksheet image; `.sheaf` round trip; `.opj` warning; Excel export if small.
5. Origin pass (Windows): measure the window and rules, record them, change what differs.

## Done when

Bold, italic, underline, font, size, text colour, fill and alignment can be set on any selection
from the toolbar and the window, survive sort/insert/delete/save/reopen, print as shown, and the
checks pass.

## Don't

- Don't change the column's numeric display format here (Column Properties owns it).
- Don't build Merge cells, rich text inside one cell or cell borders here (B6 / leftovers).
- Don't store one entry per cell for a whole-column format.
- Don't slow the grid: no per-cell allocation while drawing.
