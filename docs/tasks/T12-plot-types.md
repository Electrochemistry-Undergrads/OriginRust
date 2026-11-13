# T12: More plot types

**Status:** items 1-5 and the pie of item 6 done on branch `plot-types-more` (items 1-2 merged `d0dbb37`): stacked area, 100 % stacked area, fill area between two columns; histogram (Origin's automatic bins) and box chart; bubble and colour mapped symbols; flat pie. Colour lists of column and area templates measured and copied (spec 14.14: Column and Box start at red, stacked column and area templates use the theme list). Left: polar (seen, spec 14.17), ternary, radar, 3D pie (Origin's Pie menu has 3D Color Pie and 2D B&W Pie only; Sheaf's extra "2D Color Pie" item should give way to those), stacked bar and floating bar variants, grouped box charts and box data points (need T10's group columns), histogram hatch fill and distribution curve, Origin's colour map and bubble size legend, drop lines; floating column fill (Origin blue, Sheaf black), 100 % labels and join lines; the `xy-error` golden was not looked at; none of the new types is written to `.opj` (the save warns). **Size:** L, one commit per type. **Branch:** `plot-types`. **Model:** Sonnet.
**Detail:** audit C "Missing plot types". **Spec:** `docs/spec/graphing.md` (plot types, section 2
styles), `docs/spec/feature-parity.md` (P0/P1 tags decide order).

## Today

`PlotKind` in `crates/core/src/graph.rs`: Line, Scatter, LineSymbol, Column, Area. Templates in
`crates/core/src/template.rs` (`enum Template`); drawing in `crates/plot/src/plots.rs`; menus in
`crates/app/src/menus.rs` (Plot menu `grey_sub` entries). Error bars: symmetric Y only.
Matrices, contour and heatmaps belong to session `originrust-3a`: do not start them.

## Order (owner's figures first)

1. Stacked column, 100 % stacked column, horizontal bars (Bar), floating column.
2. XY error bars: X error, direction (+/-/both), cap width (today fixed `half_cap = 0.6*size`).
3. Stacked area, fill area between two plots.
4. Histogram plot (bins exist in `crates/calc/src/freq.rs`) and box chart.
5. Bubble and colour-mapped symbols (size/colour from a column).
6. Pie; polar; ternary; radar (later).

Per type: add to PlotKind and Template, draw it, menu item live, CLI `sheaf plot -t <name>`, golden
image (`crates/plot/tests/golden`; see LESSONS on `SHEAF_UPDATE_GOLDEN`), `.opj` write or a save
warning. Compare the look with Origin via `tools/origin-ref/` export (`expGraph` with an explicit
folder: LESSONS "Origin writes to the owner's user folder...").

## Done when

Each type: drawn like Origin by eye on one probe, golden test, menu + CLI, `.opj` handling; checks pass.
