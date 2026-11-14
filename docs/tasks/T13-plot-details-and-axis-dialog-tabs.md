# T13: Missing Plot Details tabs and controls

**Status:** item 1 mostly done (Drop Lines and Label tabs, line Transparency, Fill Area Under Curve in the Normal mode, measured, graphing spec 18) and the layer Background colour (item 3); left: Offset tab, the other fill modes and Pattern tab for fills, items 2, 3 (Size/Speed, Display, Link Axes Scales), 4-7. Origin's tab lists and defaults are measured (spec 18), on `main`. **Size:** M. **Model:** Sonnet.
**Detail:** audit B "Plot Details", "Layer Contents (F12) and Plot Setup".
**Spec:** `docs/spec/gui-look.md` (Plot Details), `docs/spec/ui-command-map.md` D1/D3.

## Today

`crates/app/src/dialogs/graph_details.rs` (plot, layer tabs) and `graph_props.rs` (page node titled
"Page Properties"). Window 900x560 (Origin about 1005x650). Layer Contents (F12) and Plot Setup
both open the Plot Details layer tab (`menus.rs`).

## Items

1. Plot tabs: Drop Lines, Label, Offset tabs; Line transparency; Fill Area Under Curve; Pattern
   colour/transparency/gradient (Pattern row and Spacing "Width (in %)" are greyed today).
2. Group tab: Edit Mode, increment rows, Group Members, Subgrouping (after `opj-groups-frames`
   merges, which models plot groups).
3. Layer: Size/Speed, Display tabs; Link Axes Scales; Background tab (greyed).
4. Page: Print/Dimensions, Legends/Titles, Miscellaneous, Display.
5. Plot tree text `[Book1]Sheet1! "Conc"(X), "Signal"(Y) [1*:10*]` instead of the short name.
6. Separate Layer Contents dialog (two lists, add/remove) and Plot Setup dialog, as Origin.
7. "Workbook" button and ">>" side panel.

Keep Sheaf's extra Left/Right Y choice on the Group tab (double-Y needs it).

## Done when

Each item done or noted; screenshots via `--show graph-settings:plots|page`; UI tests; checks pass.
