# T37: Violin plot

**Status:** done 2026-11-15 (session "violin", Claude macOS, no Origin), branch `violin-plot`; spec in graphing.md 14.22. Left: the "Violin with Box" toolbar button (Origin has one on the 2D Graphs toolbar; Sheaf has none yet), Split Violin, grouped violins from indexed data, violin settings in Plot Details (bandwidth rule, extend, scale mode, scale to maximum, curve next to box, binned data), and measuring Origin's defaults (nothing here is measured).
**Size:** M. **Source:** `origin-2026-2027-gaps.md` (Graphing: Violin and parallel-coordinate
plots, Origin 2019; the violin half only; parallel coordinates are left).

## Goal

Plot > Statistics > Violin Plot (and Origin's violin variants that fit the existing box-chart
model) draws each Y column's distribution as a mirrored kernel-density silhouette at 1, 2, 3, ...
in plot order, like the box chart, with the box/quartiles/points inside as Origin's variants do.

## Origin facts (docs, not measured)

From Origin's help pages (violin plot, create violin plot, Plot Details Distribution/Box/Data/Spacing tabs, LabTalk
`ksdensity` and `kernelwidth`), summarised; details and URLs in graphing.md 14.22:

- Origin 2019; a "fancy variation of box chart" with a kernel density curve. Seven variants: Violin Plot, Violin with
  Box, Violin with Point, Violin with Quartile, Violin with Stick, Split Violin, Half Violin. The help never describes
  what each draws.
- Each Y column is a violin; column names or labels are the tick labels. Split Violin: last column has two categories.
- Gaussian kernel; bandwidth Scott `0.9 A n^-1/5` or Silverman `1.059 A n^-1/5`, `A = min(SD, IQR/1.349)`, or k x SD
  or a number. The curve extends past min and max by Extend/100 bandwidths. Scale by Count, Width or Area.
- Not documented: grid size, default scale, default extend, maximum width, default colours/fill/outline, inner marks.

## Where

- `crates/core/src/graph.rs` (`PlotKind`), `crates/core/src/template.rs` (`Template`).
- `crates/plot/src/data.rs` (box statistics), `plots.rs` (box drawing), `layout.rs` (categories).
- `crates/calc` for the kernel density estimate (bandwidth rule as Origin documents it).
- `crates/app/src/menus.rs` (Plot > Statistics), toolbar, `dialogs/graph_details.rs` kind name.
- `crates/ops` / CLI `-t violin`, MCP template list; `.opj` save warning.
- `docs/spec/graphing.md` 14.22 (new).

## Done when

The menu item, CLI and MCP make a violin graph from one or more Y columns; the silhouette, its
inner box/quartile/point marks and its scale follow Origin's documented defaults; it saves in
`.sheaf`; tests cover the density numbers (against an independent reference), drawing geometry,
the menu and loading old files; graphing.md 14.22 records Origin's facts and Sheaf's choices.

## Don't

- Don't touch fill-under/sidelines code (`codex/t13-fill-sidelines`) or axis break drawing
  (`axis-breaks`); don't change box charts of existing graphs.
