# ADR 0014: Cell links (Paste Link) as per-cell references kept in step by the document

Status: accepted (2026-11-20, Windows). Task T211 (B5).

## Context

Origin's Edit > Paste Link, Paste Link Transpose and Remove Links (and the same items on a data cell and
a column header) make worksheet cells that show other cells: a linked cell follows its source when the
source changes. PLAN.md (B5) asked for a decision on the link model before building, because Sheaf had
no cell that "shows another cell": every value lives in its column's data, and formulas exist only per
column (Set Column Values). Origin's own storage (cell formulas such as `=[Book1]Sheet1!B2`) and its
exact paste rules could not be measured (Origin COM was unavailable on 2026-11-20).

Facts from the code:

- Plots, analysis, export and formulas read `Column::data` only. A link that lives only in the display
  would leave all of them seeing the old value.
- `Document::edit` already brings derived data up to date inside the undo step of every edit
  (`recalculate_since` for formula columns, `update_analysis_links` for recalculating analyses).
- Column ids are unique in the project, and every operation that gives columns new ids or deletes them
  goes through `Worksheet::remap_note_columns` (also used by T194, T200, T202 and T205 for cell objects).

## Decision

1. A link is a record on the target sheet: `CellLink { column, row, source, source_row }`, at most one per
   cell, saved with the sheet (`cell_links`, serde-default, absent when empty). The source is any cell of
   the project, named by its column id and row.
2. The value is copied, not computed on read: `Project::refresh_cell_links` writes each source cell's
   value into its target cell (numbers exactly, text as text, a missing source as an empty cell), only
   where it differs. `Document::edit` runs it before the formula and analysis recalculation, and again
   after them when those changed a source, all inside the same undo step. Everything that reads column
   data therefore sees linked values, and Undo restores source and links together.
3. Paste Link links the block last copied in Sheaf (Copy, not Cut) cell by cell, from the clicked cell
   down and right; Paste Link Transpose swaps rows and columns. Cells that would fall right of the
   sheet's last column are left out (with a message). Remove Links drops the links of the selected cells
   (or whole selected columns from the column header) and keeps their values. Typing into a linked cell
   replaces its value and drops its link.
4. Links follow column re-ids and deletions through `remap_note_columns` (a deleted target or source
   column drops the link). Workbook templates drop links. `.opj` and other exports write the values only.

## Consequences

- No new value kinds or lazy evaluation: links cost one pass over the links per edit (nothing when a
  project has none).
- A chain of links settles in one or two passes per edit; a long chain may need more edits to settle,
  and a cycle simply copies values around without looping (each pass is bounded).
- Paste Link of the graph window (Origin's link of a picture) is not part of this and stays grey.
