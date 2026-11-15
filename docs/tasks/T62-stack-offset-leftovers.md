# T62: Remaining stack and offset commands

**Status:** claimed 2026-11-15 (session "mac-multipdf", macOS, no Origin), branch `stack-leftovers`, all three slices. **Size:** M, three independent slices.
**Branch:** `stack-leftovers`. **Priority:** P0, wave 2.
**Spec:** [graphing](../spec/graphing.md) 13 (especially measured Stack tab 13.14), 14.12;
ADR 0010. **Inventory:** 8 locations tagged `T62`.
**Ownership:** stack-mode menu/model/rendering/range, stacked histogram template, waterfall colour
mapping. Coordinate panel/template work with T60/T61; reuse their helpers after they land.

## Goal

Graph > Offset Grouped Data in Layer changes the active layer's stack mode with a current-mode
check. Add Cumulative to lines/scatter, vertically stacked histograms sharing X, and Y/Z-colour
mapped waterfall curves with a colour scale.

## Origin facts

Graphing 13.14 records None, Cumulative, Incremental, Constant, Auto (Gap 8% when chosen) and
further controls. Cumulative sums prior plots' Y values pointwise (graphing 13).
**Not measured:** menu prompts/defaults, cumulative treatment of differing X/missing rows, shared
versus per-panel histogram bins/order/gaps/labels, waterfall Y/Z mapping and palettes/levels/scale.
Measure on Windows and export fixtures; mark provisional rules as Sheaf choices.

## Where

- `crates/core/src/graph.rs`: `Stack::{None, Auto, Constant, Individual, Waterfall}` and
  `plot_offsets` exist; Cumulative does not. `core/stack.rs`, `plot/stack.rs`, `core/autorange.rs`
  must agree on cumulative values and extents, including fitted followers and filtered rows.
- `crates/app/src/menus.rs`: the entire offset submenu is currently `grey_sub`; route live modes
  through history, matching Plot Details. Its settings must stay synchronized.
- `crates/core/src/panels.rs`, `template.rs`: existing histograms/panels underpin stacked histograms.
- Shared colour maps/scales in `crates/plot`: reuse for waterfall; establish what supplies Z.
- Shared CLI/MCP options and `.sheaf`/`.opj` preservation or explicit warning/companion.

## Steps

1. Wire None/Constant/Auto/Individual using existing settings and add Cumulative. Confirm whether
   Constant/Auto prompt or use stored values. Test mode ticks, Plot Details synchronization,
   no-op/Undo/Redo, three-line cumulative values, missing/unequal X, filters and automatic range.
   Add a cumulative CLI mode through shared args where supported; remove five inventory locations.
2. Build Stacked Histograms with measured or explicitly provisional bin/order rules; GUI and
   `sheaf plot -t stacked-histograms`, golden/empty-data tests; remove its inventory location.
3. Add waterfall Y/Z colour mapping and scale, shared options, fixtures and round trips; remove
   the remaining two locations. Record unsupported native `.opj` details and save warnings.

## Done when

The menu changes stack mode, three columns plot as stacked histograms, colour mapping works,
and model/range/export/save/undo agree. Eight locations are implemented or reassigned with reasons;
task-workflow checks pass.

## Don't

Do not change measured Auto/Constant/Individual/Waterfall placement (`places` is shared with the
`.opj` writer), rebuild worksheet Stack Columns or stacked areas/bars, or add 3D Waterfall (B15).
