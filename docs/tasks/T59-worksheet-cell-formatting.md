# T59: Worksheet cell formatting

**Status:** steps 1, 2 and 4 done 2026-11-15 (`cell-format`). Step 3 first toolbar slice adds selected cells/rows/columns/label targets to T58's shared dispatch: installed font, size, Bold/Italic, basic text colours, mixed-state summaries, stale-request protection and one-step Undo. Grid point sizes and derived row geometry are corrected, including print/image geometry. Basic Origin 2016 toolbar/font/row-growth measurements are in GUI 3; `codex/t59-format-toolbar` passes fmt, strict workspace/all-target Clippy, all 3,934 workspace tests (8 ignored), 17 toolbar regressions and the inventory check; the desktop rendering was inspected. Merged to main with the T74/Line Series integration; remaining work is listed below. Slice 2 (`codex/t59-worksheet-controls`) adds worksheet Underline, basic fill/None and contextual Ctrl+B/I/U after Origin measurements; fmt, strict workspace/all-target Clippy, all 3,971 workspace tests (8 ignored), 26 related toolbar regressions, the inventory check and an inspected isolated desktop capture pass. Remaining: Format > Worksheet and Set Data Style, vertical alignment, persistent/sticky row heights, Excel export of formats, full/custom palettes and remaining dialog/precedence measurements. Printing still omits empty label rows, including styled ones, under its existing T27 extent policy.
**Size:** L. **Branch:** `cell-format` (model/dialog), `codex/t59-format-toolbar` (toolbar/font geometry), `codex/t59-worksheet-controls` (underline/fill/keys). **Priority:** P0, wave 1.
**Spec:** [workbook model](../spec/workbook-model.md), new Cell formats section;
[GUI](../spec/gui-look.md) 3. **Inventory:** 2 remaining locations tagged `T59` (worksheet-wide scope and Set Data Style).
**Depends on:** T58's toolbar dispatch (slice 3). Model/dialog work can start independently.
**Ownership:** worksheet format model, range maintenance, grid and print drawing, Format Cells
window; worksheet adapter behind T58's dispatch. Coordinate worksheet/sheetops/grid edits with
existing cell-note, conditional-format and frozen-pane work.

## Goal

Format selected cells, rows, columns and label rows: bold/italic/underline, font/size, text/fill
colours and alignment. Formats follow the data through sheet operations, survive saves, and
appear in printed sheets and worksheet images. Numeric display format remains Column Properties.

## Origin facts

GUI section 3 records Format Cells on cell/column/row menus, Set Data Style on the cell menu,
and Ctrl+U underlining a selected column. The X-Function index names `wcellformat`.
**Measured 2026-11-16:** worksheet U button, Ctrl+B/I/U, Fill Color red/None, independent label
underline and data-only whole-column underline; GUI 3 records the synthetic UI runs.
**Measured 2026-11-15:** selected-cell Arial 9 pt default, Bold/Italic, Times New Roman, red text,
mixed-size blank field, first-cell mixed Bold state and row growth. Size-step buttons were disabled.
Reducing font size leaves Origin's enlarged height; Sheaf's current derived height can shrink.
**Not measured:** Format Cells controls/defaults/Apply-to, Format > Worksheet, Set Data Style
choices, rich text inside a cell, precedence of manual/conditional colours, sort behaviour and
`.opj` storage. Measure on Windows; label interim choices. The cloud draft proposed conditional
colours overriding manual colours and styles following rows during sort; these are proposals.

## Where

- New `crates/core/src/cellformat.rs`, compatible default/empty `Worksheet.cell_formats`.
  Store column IDs plus row spans and label-row addresses, with optional style properties.
  Whole-column styling must not allocate one record per row.
- `crates/core/src/notes.rs`, `sheetops.rs`: follow the existing maintenance pattern for row and
  column insert/delete/sort/move/duplicate/append, including column-ID remapping.
- `crates/core/src/condformat.rs`: existing `CellStyle`/`CellStyles` supply conditional colours;
  define explicit resolution with manual styles and cache lookups by document revision.
- `crates/app/src/grid.rs` (`body_cell`, frozen overlays), new `dialogs/cell_format.rs`,
  `toolbar.rs` worksheet target and contextual Ctrl+B/I/U shortcuts.
- `crates/plot/src/sheet.rs`, `crates/core/src/sheet_print.rs`: identical resolved styles.
- `crates/io`: `.sheaf` round trip; unsupported `.opj` fields warn/persist in companion per
  ADR 0008. Excel font/colour export if small, otherwise record as a leftover.

## Steps

1. Specify range/overlap/clear semantics and label-row addressing; implement model tests for
   all mutations above, undo/redo and append remapping. Record unmeasured decisions in the spec.
2. Draw styles and build Format Cells. One commit on OK/Apply; Cancel is inert. Test imported
   and new sheets, filtered selections, frozen panes and a whole-column million-row selection.
3. Add T58's worksheet adapter once its dispatch lands. Enable Format > Worksheet/Cell, the
   column/cell/row Format Cells items and Set Data Style > Default only when each is implemented.
   The worksheet entry must implement its measured worksheet-wide scope, not merely alias a
   cell dialog with misleading semantics.
4. Verify print/image rendering, save/reopen, `.opj` warnings/companion and any Excel export.
5. Measure Origin's remaining dialog and precedence rules; reconcile and record differences.

## Done when

Styles work from toolbar and dialog, survive selection and structural operations, print as shown,
round-trip without loss, and all six inventory locations are enabled or reassigned with reasons.
Task-workflow checks pass. Conditional and manual formats interact as documented.

## Don't

Do not add merged cells, borders or rich text inside a cell here. Do not alter numeric display
formats, allocate per-cell styles for a whole column, or allocate while drawing every cell.
