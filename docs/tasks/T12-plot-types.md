# T12: More plot types

**Status:** items 1-2 done on branch `plot-types`, not merged (stacked, 100 % stacked, horizontal Bar, floating column; X and XY error bars, direction, cap width). Left: stacked bar and floating bar variants, items 3-6. **Size:** L, one commit per type. **Branch:** `plot-types`. **Model:** Sonnet.
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
