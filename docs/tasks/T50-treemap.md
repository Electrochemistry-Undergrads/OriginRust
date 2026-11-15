# T50: Treemap

**Status:** claimed 2026-11-15 (session "mac-multipdf", macOS, no Origin), branch `treemap`.
**Size:** M. **Source:** `origin-2026-2027-gaps.md` (Graphing: Treemap, Origin 2025b).

## Goal

Plot > Categorical > Treemap draws a hierarchy of categories (one or more category columns, first =
root level) as nested rectangles with areas proportional to a value column, with Origin's layout,
order, depth and gap options.

## Origin facts (docs, not measured)

- 2025b new graph type "Treemap plot". Menu "Plot > Categorical: Treemap", template `treemap.otpu`;
  data: multiple categorical columns and one value column. [W:originlab.com/2025b; origin-help/treemap]
- Plot Details Layout tab: Layout Split; Squarified (descending by size, largest top-left, ignores
  Order); Squarified+ (per row, keep the direction with the better aspect ratio); Pivot by Size
  (pivot = largest); Pivot by Middle (pivot = middle item); Pivot by Split Size (pivot splits the list
  into roughly equal areas). Order: Categorical Order, Alphabetical, by Size, by a column (Squarified
  variants only by size). Depth 1..N or Infinite. Gap between rectangles. Optional separate Layout and
  Order per level (not the first). [W:origin-help/pd-layout-tab-treemap]
- Not documented (Sheaf's choices, unmeasured): default layout (Squarified), default order, depth
  (Infinite), gap size, colouring (one palette colour per top-level category, lighter shades for
  deeper levels), labels (leaf name and value inside when it fits, hidden otherwise), borders,
  non-positive/missing values (skipped).

## Where

- `crates/calc` (or plot): treemap layout algorithms (squarified per Bruls et al., split, ordered
  pivot treemaps per Shneiderman & Wattenberg) as pure functions on (weights, rect) -> rects.
- `crates/core`: `Template::Treemap`, plot kind/settings (layout, order, depth, gap, per-level), `.opj`
  warns; `crates/plot` drawing (rects, colours, labels, clipping); `crates/app`: Plot > Categorical >
  Treemap, Plot Details Layout tab; CLI `-t treemap` (category columns + value column), MCP.

## Done when

Treemaps draw from the menu/CLI for 1-3 levels with every layout; areas match values (tests check
area ratios and no overlaps), order/depth/gap work; spec section in graphing.md.

## Don't

- Don't change other plot kinds' output or the parallel/hexbin colour code.
