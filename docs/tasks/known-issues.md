# Known issues and worksheet verification

Preserved from HANDOFF.md on 2026-11-15. These are recorded results, not a new validation run.
Update the relevant brief/spec when resolving one; the historical snapshot is in
[the archived handoff](../archive/HANDOFF-2026-11-15.md).

## Integration after the archived snapshot

- T62 stack and offset leftovers (2026-11-15, macOS, no Origin, branch `stack-leftovers`): Graph > Offset Grouped Data in
  Layer is live (one undo step, tick, no-op on the current mode, synced with Plot Details); `Stack::Cumulative` sums
  line/scatter/line+symbol plots pointwise by row (missing, filtered or out-of-range rows add 0; fits ride on the
  leader's sum; `--stack cumulative`); Plot > Statistics > Stacked Histograms and `-t stacked-histograms` (one panel per
  column from the top, linked X, bins from the union of all values); Plot > Y-offset/Waterfall Y/Z colour mapping
  (`-t waterfall-y|waterfall-z`, segment colours by mean Y or curve colours by Z from the long name/comments/index, colour
  scale beside the layer). `.opj` writes plain curves and warns; `.sheaf` keeps all. All unmeasured (graphing 13.15-13.17).

- T59 worksheet cell formatting steps 1, 2 and 4 (2026-11-15, macOS, no Origin, branch `cell-format`):
  `cellformat.rs` ranges (column ids, open-ended row spans, label rows; whole columns are one record) with
  upkeep for row insert/delete, sorts (formats follow rows), column delete, duplicate/append and analysis
  retirement; grid, frozen overlay, label rows, print, PDF and worksheet images draw bold/italic/underline/font/
  size/colours/fill/alignment (conditional colours win); Format Cells window from the cell, column and row menus
  and Format > Cell... (one undo per OK/Apply); `.sheaf` and companion keep formats, `.opj` and Excel export warn.
  Format > Worksheet and Set Data Style stay grey; toolbar adapter waits for T58. All unmeasured (gui-look
  "Cell formatting (T59)", workbook-model 13b).

- T64 multiple comparisons plot (2026-11-15, macOS, no Origin): the report sheet of a one-way ANOVA with a means
  comparison or a two-sample t-test (button in the sheet tab strip, Statistics > Multiple Comparisons Plot...) makes a graph
  of the groups (bar, box, violin, scatter; SD, SEM or 95 % CI error bars) with significance brackets (asterisks or
  p-values read from the report, significant only with a threshold; Holm methods show the step-down adjusted p and every
  method follows the report's Significant column), stacked by span in axis space (log axes too) with the Y axis growing,
  or in a reserved band above the frame when the Y maximum is fixed, the level step never below the label height;
  pairs stored by group index; `analyze anova|ttest --comparison-plot OUT`, MCP, not written to .opj (warning). Documents
  only, unmeasured; two-way, paired and one-sample left (statistics.md 23).
- T52 slope annotation (2026-11-15, macOS, no Origin): Tools toolbar "Slope" picks two points of one plot
  (Data Reader snapping); a segment, dashed rise/run legs and "slope = ..." linked to those rows and the plot's
  X/Y columns; per decade on log axes (Tafel), units from the drawn axes' columns; follows moved plots;
  Delete/undo, properties dialog, `sheaf plot --slope A,B`, MCP `style.slope` (graphing.md 17.6, unmeasured).

- T02 Column Browser row ranges (2026-11-15, Windows, feature `d9b4af7`): the lower Rows
  control opens Range with Entire Column(s), From and To. Each added range keeps its own
  bounds, including after removal of a preceding range. Native integer-prefix conversion,
  reversed/single-row intervals, empty columns and block endpoint references are preserved;
  Go To still uses the original sheet's column end. Eighteen valid Origin cases and ten new
  UI regressions cover transfer, validation, reset, Cancel/Escape and keyboard focus. All 39
  Go To tests pass; formatting, strict workspace all-target Clippy and 3,316 workspace tests
  pass (eight existing ignored), with MCP teardown/ROC/filters/formula formatting integrated.
  Exclude and saved preferences remain (gui-look section 5).

- T25 proxy disconnect cleanup (2026-11-15, Windows, feature `ef98bf7`): disposing of the
  active link signals its reader and polls cancellation every 250 ms, releasing its socket
  clone even when shutdown does not wake the receive. Live partial frames survive polls;
  cancelled partial replies are discarded. A real-TCP regression omits shutdown and checks
  peer EOF for idle/partial replies. The original ten-second integration assertion is unchanged;
  1,750 stress checks pass. Formatting, strict workspace all-target Clippy and 3,306 tests pass
  (eight existing ignored), with formula formatting, digital filters and ROC integrated.

## Known issues

- T01 sorting: Sort Range and range Custom respect selected cell/row bounds. Sort Columns
  Ascending/Descending move selected rows together; Each Column modes sort independently.
  Sort Columns and column-header Custom now use the existing nested-sort dialog. Masks,
  outside rows/columns and Undo/Redo are covered; see workbook-model.md 13.1.

- Filtered table navigation: keyboard movement, edit commits and Enter/Tab within blocks skip
  hidden rows/columns. New/imported-table regressions ensure hidden values are not edited;
  filter changes repair the active cell (workbook-model.md 13.3).

- T10 filtered plots: graphs, exports, automatic axes/offsets and histogram/box/pie summaries
  skip worksheet-filtered rows. Masks, labels and error bars retain source-row references;
  the Data Reader skips hidden points. Filter Disable/Remove and Undo/Redo restore the view.
  Graph gadgets follow filters and masks since T209. Category/spoke compression and filter
  overrides remain open (workbook-model 13.2a).

- T07 clipboard: quoted table fields keep commas within one cell and preserve tabs, line breaks
  and quotes through Copy/Cut/Paste. Both new and imported-table UI paths cover Undo/Redo.
  External CSV/TSV paste detects separators outside quotes; Origin Copy defaults and pasting
  into other applications remain unmeasured (workbook-model.md 13.4).

- T05 PSD graph export: Export Graphs > Photoshop Document and CLI/API `.psd` output support
  RGB, grayscale and 256 colors with DPI metadata. The image is flattened onto white, without
  editable graph layers. PSD dimensions are limited before pixel allocation; graphing.md 9.4
  records the published references and limits. TIFF now also offers 256 Colors, with the
  selected None/PackBits/LZW compression and DPI retained; Monochrome, 16 Colors and 16-bit Color (BMP, TGA) added 2026-11-14 with Origin's per-format lists, ordered dither and 5-5-5 layout (graphing 9.4).

- T19 import separators: **Other** reads a set of delimiter characters, with optional
  **Treat Consecutive Delimiters as One**; **Multiple Characters** reads a whole sequence.
  CLI equivalents: `--delimiter-chars` / `--combine-delimiters` and `--delimiter` respectively.
  Explicit modes handle custom-delimiter quoted multiline fields; quoting, empty-field choices
  and the remaining Auto-detection bootstrap limitation are in import-ascii.md section 2.
  Partial Import now selects first/last rows and columns plus repeating read/skip patterns, with
  source labels and parameter values preserved (shared wizard controls and CLI; section 9).
  Batch scope **All files except first** (CLI `--partial-except-first`) keeps the first data file
  full while selecting later files; preview and final import share the same options. Custom
  range lists remain open.

- T08 text substitution: `substitute(s,new,find[,n])$` follows Origin's documented argument order;
  omitted/zero occurrence replaces all, a positive integer chooses one. Function > Text exposes it.
  Text-returning functions accept `$`; worksheet regressions cover Auto recalculation and undo.
  Built-in column headers (`G/L/U/C/O`, e.g. `col(A)[U]$`) now support text/numeric reads,
  Auto/Manual recalculation after header edits, and self-label references without false cycles.
  `char(n)$` and `code(s)` create characters and read code points (Function > Text); the
  published references, Unicode bounds and unmeasured edge choices are in expression-language.md section 3.
  Typed `format(x,fmt)$` shares Text's explicit formatter; formatting columns work with both names.
  `text(col(A))$` and indexed-cell Text calls use the numeric source column format and update after
  format edits. Origin global/`@SD` defaults remain open; explicit empty formats stay general.

- [x] **Single yellow-cell selection** (fixed 2026-11-13): labels have their own `Sel::Label`
      selection, with a border around just that cell; column letters still select columns.
      Typing/F2, navigation, clipboard/clear, Escape, and undo work without targeting data beneath
      the label. Formula cells keep Set Column Values. See workbook-model 13.3 and `owner_regression_*`.

- [x] **Cell shortcuts and interaction logic across all tables** (fixed 2026-11-13):
      shared label/data selection and focus handling is covered through both File > New and CSV
      import. Regressions exercise empty-cell mouse selection, typing, Enter/Tab/Shift+Tab,
      navigation, clipboard/paste growth, clear, and undo on both paths. Specific newly reported
      interactions can be added to this coverage; the unconfirmed drag observation stays below.

- [x] **Reversed typing (`a` then `b` became `ba`)** (reported/reproduced/fixed 2026-11-13):
      a replacement edit reused an old caret at zero. Each new data/label edit now has a fresh,
      cell-specific editor state with the caret after the initial text. Tests cover both table
      creation paths, stale carets after Home/Escape, Unicode input, and cursor movement during an edit.

### Table UI walkthrough (2026-11-13)

Scope: table interactions only, using synthetic data in a separate Sheaf instance. Checked the
running release build; its exact commit was not verified and it was not rebuilt for this pass.

- [x] **Fixed: Tab after editing loses worksheet keyboard focus.** Reproduction: click A1
      in a new table, type `8`, press Tab, then type `9`. A1 commits as 8 and the selection
      outline moves to B1, but the next digit is ignored. Shift+Tab and arrow navigation also
      stop controlling the table until a cell is clicked again. Reproduced twice in the new
      table and also in an imported CSV table (click A1, type `6`, Tab, then `9`). Expected:
      committing with Tab leaves keyboard focus in the next cell so entry and navigation continue.
      Reproduced in the UI harness and fixed: both editors retain Tab instead of letting egui's
      widget traversal steal focus. Regression checks successive typing after Tab and Shift+Tab
      on new and imported tables.
- [ ] **Needs confirmation: mouse drag selects only the final cell.** In an imported table,
      dragging from A1 to B3, then from A1 to B5, left only B3 or B5 selected rather than the
      rectangular range. Observed in two automated drag attempts; confirm with a manual drag
      before treating it as an application defect, since input timing could be involved.
      2026-11-13: the UI harness selects the block with every event timing tried (one jump, jump and
      release in one frame, press and jump in one frame; `a_drag_over_cells_selects_the_block_whatever_the_event_timing`),
      so the automated drags were most likely the cause; still worth one manual drag by the owner.
      Fast press-and-move frames select the correct block in new/imported-table UI regressions;
      the original release-build observation was not reproduced as an application defect.
- [x] With grid focus, Shift+arrow keys select a rectangular block in the imported table.
      Ctrl+C/Ctrl+V copied and pasted a 2-by-2 block; Ctrl+Z removed the paste in one step
      and Ctrl+Shift+Z restored it. Typing in a new-table data cell and committing with Tab
      worked up to the subsequent focus failure described above.
- [x] Continue table UI coverage (2026-11-20, T212, `ui_tests/table_walkthrough.rs`): on a new and an
      imported table, Ctrl++ / Ctrl+- on a cell shift only that column, Ctrl+Space then Ctrl+- / Ctrl++
      delete and insert whole columns, Move Columns > Right, a typed value in an added sheet and
      Ctrl+PageUp / PageDown between sheets, each with Undo. Found: an added sheet had no columns to
      type into; Insert and Add now give it A(X) and B(Y) (workbook-model 13h).
      T215 adds the sheet controls on both tables: Insert places the sheet before the active one,
      Duplicate copies into independent columns, Delete and its Undo, and a block copied on one
      sheet pasted into another; no defect found.

