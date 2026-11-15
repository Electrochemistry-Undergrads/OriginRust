# T51: Sunburst plot

**Status:** claimed 2026-11-15 (session "mac-multipdf", macOS, no Origin), branch `sunburst`.
**Size:** M. **Source:** `origin-2026-2027-gaps.md` (Graphing: Sunburst and ridgeline plots, Origin
2022; the sunburst half; ridgeline is T44).

## Goal

Plot > Categorical > Sunburst Plot draws a hierarchy of categories as concentric rings (one ring per
level) with angular sizes proportional to a value column, coloured by value as Origin does.

## Origin facts (docs, not measured)

- Origin 2022 feature "Sunburst Plot"; release note: "Plot: Categorical: Sunburst Plot"; Mini Toolbar
  "Collapse Expand Mode" drills into a clicked wedge. [W:originlab.com/2022; ReleaseNotes ORG-23725]
- Template `Sunburst.OTP`; data: at least two text columns and one numeric column; the last numeric
  column holds the values, all other columns are categories and sub-categories; each level is a ring.
  Wedge colour follows its value; a category's colour follows its children's mean value; missing text
  appears unfilled. Plot Details: Pie Geometry tab (Rotation, Radius/Center, Doughnut) and Labels
  tab; the sunburst is a variant of the Doughnut plot. [W:origin-help/sunburst-plot]
- Not documented (Sheaf's choices, unmeasured): innermost ring = first (top) level, start angle 90°
  (12 o'clock) clockwise, doughnut hole 30 % of the radius, equal ring widths, 0.5 pt white borders,
  colour map = the shared ramp (`plots::map_color`) over the leaf value range, labels (name, hidden
  when the wedge is too small; orientation along the wedge's mid-angle, flipped to stay upright),
  order as worksheet first appearance; Collapse/Expand mode left.

## Where

- Reuse the T50 hierarchy builder (`crates/calc/src/treemap_tree.rs`: aggregation, duplicates,
  skipped values, order) and `Graph::remap_columns` for the new column-id fields.
- Reuse the pie/doughnut geometry settings from T12 if they exist (rotation, radius, hole) — share,
  don't duplicate. `crates/core`: `Template::Sunburst`, plot kind/settings; `crates/plot`: ring wedges
  (arcs as paths), colours, labels; `crates/app`: Plot > Categorical > Sunburst Plot, Plot Details
  geometry/labels controls; CLI `-t sunburst`, MCP; `.opj` warns.

## Done when

Sunbursts draw from the menu/CLI for 2-4 levels; wedge angles match values (tests), colours by value
with category means, labels; spec section graphing.md 14.27.

## Don't

- Don't change pie/doughnut, treemap or other plot kinds' output.
