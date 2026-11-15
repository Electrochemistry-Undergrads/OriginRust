# T37: Violin plot

**Status:** claimed 2026-11-15 (session "violin", Claude macOS, no Origin), branch `violin-plot`.
**Size:** M. **Source:** `origin-2026-2027-gaps.md` (Graphing: Violin and parallel-coordinate
plots, Origin 2019; the violin half only; parallel coordinates are left).

## Goal

Plot > Statistics > Violin Plot (and Origin's violin variants that fit the existing box-chart
model) draws each Y column's distribution as a mirrored kernel-density silhouette at 1, 2, 3, ...
in plot order, like the box chart, with the box/quartiles/points inside as Origin's variants do.

## Origin facts (docs, not measured)

To be filled from Origin's help pages (violin plot, Plot Details Box/Violin tab) before building;
everything Sheaf has to choose without Origin goes in graphing.md 14.22, labelled as such.

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
