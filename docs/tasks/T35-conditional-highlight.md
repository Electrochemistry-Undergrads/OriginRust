# T35: Conditional formatting: highlight min/max and top/bottom

**Status:** claimed 2026-11-15 (session "mac-multipdf", macOS, no Origin), branch `cond-format`.
**Size:** M. **Source:** `origin-2026-2027-gaps.md` (Worksheets: Conditional highlight of extrema,
Origin 2025b), with Origin's older Highlight Top Bottom in the same frame.

## Goal

Worksheet > Conditional Formatting colours cells by rule, live as values change: the minimum and
maximum of each row or each column of a range, and the top/bottom N items or N %. Rules are kept
per sheet, saved in `.sheaf`, and can be removed.

## Origin facts (docs, not measured)

- 2025b: "Conditional formatting to highlight min/max column-wise or row-wise". [W:originlab.com/2025b]
- Menu Worksheet: Conditional Formatting: (submenu), also on the right-click menu of selected
  cells. Items include Highlight Min Max, Highlight Top Bottom, Conditional Highlight, Conditional
  Heatmap, Conditional Pareto, Conditional Outliers, Remove Conditional Format and Conditional
  Format Manager. A rule's Name defaults to its range's first and last cell, e.g. `C1R5-C4R10`.
  Where rules overlap, the upper rule in the Manager wins. [W:origin-help/conditional-format]
- Highlight Min Max = X-Function `wcolorbycr rng name dir max.fill max.font min.fill min.font`;
  `dir` 0 row-wise (default), 1 column-wise; each row/column independent of the others. Default
  colours (Origin RGB ints `0x01BBGGRR`): max fill 30067455 = RGB(255,202,202), max font 20132812 =
  RGB(204,51,51), min fill 33085116 = RGB(188,214,248), min font 26692113 = RGB(17,74,151).
  [W:x-function/ref/wcolorbycr]
- Highlight Top Bottom (`wcolortb`): Top N Items, Top N%, Bottom N Items, Bottom N%, on the
  selected columns. Default N and colours not read. [W:origin-help/conditional-format]
- Not stated: whether colours update live, how ties and missing values count, saving. Sheaf's
  choices go in the worksheet spec, labelled unmeasured.

## Where

- `crates/core` worksheet model: `Vec<CondFormat>` per sheet (range, rule, fill/font colours, name),
  serde default empty; evaluation as a pure function (rule + column data -> cell styles), reused by
  the grid; `.opj` save warns.
- `crates/app/src/grid.rs` `body_cell` (fill and text colour from the evaluated rules, cached per
  frame or per data version, not recomputed per cell), `menus.rs` (Worksheet submenu + cell context
  menu), small dialogs for Min Max (Row-wise / Column-wise, colours) and Top Bottom (mode, N,
  colours), Remove Conditional Format on the selection, `app.rs` actions with undo.

## Done when

Min Max (row- and column-wise) and Top/Bottom N / N % colour cells as in Origin's defaults, update
when values change, survive save/reload, are removed by Remove Conditional Format, and are covered
by core evaluation tests and UI tests. Heatmap, Pareto, Outliers, custom Conditional Highlight and
the Manager are left (listed in this Status).

## Don't

- Don't recompute rules per cell per frame on large sheets.
- Don't change filters, masks, frozen panes or Go To.
