# T39: Line plots sorted by X for drawing only

**Status:** claimed 2026-11-15 (session "mac-multipdf", macOS, no Origin), branch `plot-sort-x`.
**Size:** M. **Source:** `origin-2026-2027-gaps.md` (Graphing: Plot-only X sorting, Origin 2023).

## Goal

A line or line-symbol plot can connect its points in increasing X order without the worksheet
being sorted, as Origin 2023's automatic X value sorting.

## Origin facts (docs, not measured)

- Origin 2023 highlights, "Line-Symbol Plot with automatic X value sorting": "Perform automatic X
  value sorting of data in line or line-symbol plots without having to sort the worksheet
  columns." Set per data plot with LabTalk `layer.plot1.line.sortx=1;`; "Once sorted, the graph
  can be saved as a template for future use". [W:originlab.com/2023]
- No dialog control or default is documented (default taken as off); the Plot Details Line tab
  page lists no sort option. [W:origin-help/pd-dialog-line-tab]
- Not documented: effect on symbols, drop lines, fill under, error bars, labels, missing values,
  Gap to Symbol/Skip Points, ties, descending order, spline connections. Sheaf's choices are
  labelled unmeasured in graphing.md.

## Where

- `crates/core/src/graph.rs`: a per-plot `sort_x` flag on the line style (serde default false,
  skipped when false); templates/`.sheaf` keep it; `.opj` save warns if not written.
- `crates/plot/src/plots.rs` / `data.rs`: the connection order of the line (and whatever the
  line drawing feeds: fill under, Gap to Symbol, No Line Break/missing values) uses the X-sorted
  order (stable, NaN X dropped from the line); per-point items (symbols, labels, error bars) are
  unchanged; hit testing/data reader stay on worksheet rows.
- `crates/app/src/dialogs/graph_details.rs` Line tab: a check box "Sort X for line connection"
  (Sheaf's own place and label), undo; CLI `sheaf plot --sort-x`, MCP plot option.

## Done when

A plot with `sort_x` draws its line in increasing X order in the app, exports and CLI; worksheet
rows are untouched; the setting saves and reloads; geometry and UI tests cover it.

## Don't

- Don't sort the worksheet, or change plots with the flag off (byte-identical exports).
