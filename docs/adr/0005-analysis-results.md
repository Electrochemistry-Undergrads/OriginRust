# ADR 0005: Where analysis results go

Status: accepted (2026-11-11)

## Context

The analysis tools (integrate, differentiate, smooth, normalize, subtract baseline, interpolate,
find peaks, statistics) are plain functions over number slices in `sheaf-calc`. The app and the
command-line program need one rule for which rows a tool uses and where its results appear in the
project. Origin spreads this over the Results Log, new columns at the end of the sheet, and new
sheets, depending on the tool and its dialog settings.

## Decision

`Project::analyze` in `sheaf-core` (module `analysis`) is the only entry point; the app's Analysis
menu and `sheaf analyze` both call it.

1. **Input.** A run works on one or more Y columns of one sheet. The X values of each come from
   the column designations (nearest X column to the left; row numbers if the sheet has none).
2. **Rows.** All rows, a row range (`from..=to`, counted from 1), or an X interval. An X interval
   selects the first unbroken run of rows whose X lies inside it. For steadily rising or falling X
   that is the usual "X from .. to". For a cyclic voltammetry sweep, which passes every potential
   twice, it is the first pass; the other pass is chosen with a row range.
3. **One value per row** (derivative, smoothed curve, normalized curve, baseline-subtracted curve,
   running integral): a new Y column is inserted directly after its source column, behind any
   error or label columns that belong to the source. It therefore shares the source's X column and
   plots correctly without further setup. Rows outside the chosen range are missing values. The
   new column gets a long name ("Smoothed Current"), a unit and a comment that says what was done.
4. **A few numbers per column** (integration: area, limits, peak position, height, width;
   statistics): a new sheet in the same workbook (`Integrate1`, `Statistics1`, ...) with one row
   per input column. The first column is text and names the input ("C: Current, Scan 2").
5. **Results with their own length** (interpolated curves, peak lists): a new sheet with an X and
   a Y column per input column (`Interpolate1`, `Peaks1`).
6. **Units.** The integral's unit is the product of the Y and X units; current times seconds is
   written as charge (A·s = C, mA·s = mC). The derivative's unit is the quotient.
7. Each run is one undo step, named after the tool. On error nothing changes.

Sorting (`Project::sort_sheet`) reorders all columns of a sheet by one key column; missing keys go
last. Filtering (`Project::filter_sheet`) keeps the rows for which a column-formula condition is
true, either in a new sheet or in place.

## Reasons

- Inserting next to the source instead of appending at the end keeps the X pairing right in
  sheets with several X columns (each scan of an imported CH Instruments file has its own).
- Result tables as sheets, not as a text log: the numbers can be plotted, exported and used in
  column formulas, and they are saved with the project without a new file-format element.
- One entry point keeps the app and the command-line program identical and testable without a
  window.

## Not decided here

- Choosing the range by dragging on a graph (Origin's integration gadget) is not built yet; it is
  listed in the roadmap proposals.
- Recalculating results when the source data change is not built (see roadmap proposals).
