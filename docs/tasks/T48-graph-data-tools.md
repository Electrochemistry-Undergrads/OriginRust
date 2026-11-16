# T48: Graph Data menu tools: Mask, Remove Bad and Move Data Points

**Status:** done, merged 2026-11-15 (Claude macOS "mac-datatools"; graphing.md 20): core `Document::mask_points`/`remove_bad_points`/`move_point`, the three Data menu modes with Data Reader hit-testing (offsets, multipliers, log axes, waterfall, Skip Points, clipping), analysis links follow masks. Reviewed by three agents with each finding checked; nine fixed. Follow-up 2026-11-15 (graphing.md 20.4 to 20.6): Mask and Remove Bad measured in Origin 2016 and Sheaf changed to match (click masks, Tab active plot, Space freehand, U unmask, Remove Bad = pick then Enter, undo texts "Set Mask Data" and "Remove Data Point", reach about 9 points, right-click ends). Left: Move Data Points could not be measured (unchanged Sheaf design), other plot kinds, Hide Masked Points, filters, clipped points, Skip Points, waterfall unmeasured in Origin, the menu's plot list and the toolbar flyout; display range and markers. Brief written from
parity row "Data menu" (P1, ✗: every item of the graph's Data menu is grey) and T10's leftover "masking
from the graph".
**Size:** M. **Branch:** `graph-data-tools`. **Model:** Sonnet. No Origin needed for the mechanism; cursor
shapes, colours and region rules are measured once on Windows.
**Spec:** a new section in `docs/spec/graphing.md` ("Data menu tools"); masks are in workbook-model 13.1a.

## Goal

On a graph, the owner can mask outliers by dragging a rectangle over them (Data > Mask Data Points), delete
a bad point by clicking it (Data > Remove Bad Data Points: its Y cell becomes missing) and drag a point to a
new place (Data > Move Data Points: its X and Y cells change). Every change goes to the worksheet the plot
comes from, is one undo step, and analysis and plots follow as they do for worksheet edits and masks.

## Origin facts

- Measured already (workbook-model 13.1a): worksheet masks, Mask Change Color, Disable Masking, Hide
  Masked Points; masked points are drawn in the mask colour and skipped by analysis.
- Not measured: Origin's graph Data menu tools (the cursor while active, whether Mask Data Points works on
  the active plot only or all plots in the layer, rectangle vs click selection, Esc/double-click to end,
  whether Remove Bad Data Points deletes the row or sets the cell missing, whether Move Data Points moves
  X and Y or Y only, and the Ctrl/Shift modifiers). Measure on Windows with `tools/origin-ref` and record
  in graphing.md. Until then Sheaf's choices, labelled: active plot only; rectangle drag to mask, click on
  a masked point unmasks; Remove Bad sets the plot's Y cell missing (rows kept); Move changes X and Y;
  Esc or the Pointer tool ends the mode.

## Where

- `crates/app/src/menus.rs` `data_menu`: enable Mask Data Points, Remove Bad Data Points, Move Data Points
  (keep Set Display Range, Reset to Full Range, Mark Data Range, Clear Data Markers, Analysis Markers and
  Lock Position grey: they need a column display range, which Sheaf does not have yet).
- `crates/app/src/graph_view.rs` and the graph tool state (find how the Data Reader and the drawing tools
  are modes of the graph: `GraphTool` or similar): three new modes; hit-testing of points through the
  plot's data mapping (reuse the Data Reader's nearest-point code; respect offsets, multipliers, log axes,
  filtered rows, hidden masked points, error bars not counted); the rectangle overlay while dragging.
- `crates/core`: an edit that masks/unmasks a set of (column, row) cells, one that sets cells missing and
  one that sets two cells' values, each as one `Document::edit` step with a clear label ("Mask 4 points of
  B(Y)", "Remove bad point B(Y) row 12", "Move point B(Y) row 12"); reuse the existing mask API.
- A status-bar hint while a mode is active; the Tools toolbar shows no tool pressed.

## Steps

1. Spec section with Sheaf's choices labelled and the Windows measurements listed.
2. Core edits with tests (mask/unmask by cells, missing, move; undo restores; recalculating columns and
   analysis links follow).
3. App modes, menu items, hit-testing and overlay; UI tests: a drag over two points masks exactly those two
   cells of the right column; a click on a masked point unmasks it; Remove Bad on a point sets that Y cell
   missing; Move drags a point to the expected data coordinates (linear and log axes); each is one undo
   step; Esc leaves the mode; nothing happens on a graph with no plots.
4. Parity row to ◐ (display range and markers left), T10 Status updated.

## Done when

The three Data menu tools work on line, scatter and line+symbol plots with the results above, fmt, clippy
and tests pass.

## Don't

- Don't delete worksheet rows in Remove Bad Data Points.
- Don't change cells of a column that is a formula/recalculating column without the same rule the worksheet
  uses for typing into such a column (find it; follow it).
- Don't enable the display-range items.
