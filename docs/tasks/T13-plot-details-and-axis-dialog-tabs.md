# T13: Missing Plot Details tabs and controls

**Status:** done on `plot-details`, not merged (2026-11-14, macOS, no Origin: nothing was measured;
every Origin detail is listed as "not measured" in `docs/spec/graphing.md` section 18, to be checked
on the Windows machine with `tools/origin-ref`). All seven items are in: plot tabs (Drop Lines, Label,
Offset, line transparency, Fill Area Under Curve, pattern colour/transparency/gradient, bar Width in %),
Group tab (Edit Mode, increments for colour, line style and symbol shape, members, subgrouping by size),
layer tabs (Background, Size, Display, Link Axes Scales), page tabs (Print/Dimensions, Legends/Titles,
Miscellaneous, Display), the tree text, separate Layer Contents and Plot Setup dialogs, the Workbook
button and the ">>" side panel (rows and row range). Each control writes the model, is drawn by the
renderer and round-trips in the project file with serde defaults.
**Left out because it cannot be made real here (no dead controls):** the speed part of Size/Speed;
Drop Lines "Skip Points"; Label "White Out", frame, margin, attach-to, automatic overlap avoidance and
leader line; Offset multipliers; subgrouping by column label; the Pattern preview box; print margins
of Print/Dimensions; writing the new settings to `.opj` (the writer names them in its save warnings).
**Open:** compare layout, tab rows and window sizes with Origin on the Windows machine. **Size:** M.
**Branch:** `plot-details`. **Model:** Sonnet.
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
