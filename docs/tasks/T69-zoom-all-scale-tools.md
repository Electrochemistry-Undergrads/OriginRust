# T69: View > Zoom All and the Tools-bar Scale In / Scale Out (the rest of B1)

**Status:** done 2026-11-15. Measurement in [graphing](../spec/graphing.md) section 23; built in `crates/app/src/graph_scale.rs` with small hooks in graph_view.rs, app.rs, menus.rs, toolbar.rs; tests in `ui_tests/graph_scale.rs`. Unbuilt: Ctrl+drag "Enlarged" graph, double-click restore, the combo's Zoom All percentage, drawing objects outside the page (Zoom All fits them but the page raster does not show them), axis breaks / polar / ternary layers for Scale In.
**Size:** S. **Inventory:** 3 locations: View > Zoom All [Graph]; Tools toolbar: the two magnifier buttons (Scale In, Scale Out).

## Goal

View > Zoom All zooms the active graph window so that all its objects, also those outside the page, are in view. The Tools-bar magnifiers rescale the axes of the active layer: Scale In to a dragged rectangle, Scale Out back through the previous scales.

## Origin facts (measured 2026-11-15, Origin 2016; details and the unmeasured list in graphing.md 23)

- **Correction of T65 and gui-look 4.x:** Zoom All is not a presentation of all graphs. The "Hit ESC to exit..." reminder is Full Screen's (T66, graphing 22.4). Zoom All is a view zoom of the active graph only; no dialog, no mode, no keys; with four graph windows only the active one changed.
- Zoom All fits the bounding box of the page and all objects, also those far outside the page, into the window (a label placed far right gave a 48 % view with the page small at the top-left corner); repeating it changes nothing; Whole Page (Ctrl+W) returns to the page fitted at 100 %. With nothing outside the page the view still showed the page smaller than Whole Page (about 0.87) with a small margin and a scroll bar. The percentage in the zoom combo does not follow one ratio: unmeasured.
- Scale In (first magnifier, tool tip "Axes magnification, double-click to restore, ctrl-drag to create zoom graph"): choose it, drag a rectangle, release: the active layer's x and y ends become exactly the rectangle's edges (x left to right, y bottom to top), through the axis's own scale (log axes in log10), ticks recalculated. One drag only: the Pointer tool is selected again. A plain click or a 3 px drag changes nothing. A rectangle outside the layer frame extends the range (not clipped). Several steps nest, each read against the current view.
- Scale Out (second magnifier): one click or one drag of any rectangle restores the previous scale of the active layer (a stack, one step per use) and ends the tool; with nothing to restore it does nothing.
- Only the active layer changes. A linked layer follows in the linked axis only (x of a right-Y layer follows, its own y does not); with layer 2 active the rectangle is mapped through layer 2's own y axis.
- Undo: Edit menu text "Undo Zoom In/Out" (Ctrl+Z), one scale step per undo.
- Ctrl+drag with Scale In makes a new graph window "Enlarged" (content unmeasured); double-click "to restore" did nothing in two runs (unmeasured what it needs).

## Where

- New module `crates/app/src/graph_scale.rs` (state of the two tools, the rectangle to axis ends mapping, the per-window Scale Out stack, Zoom All's bounding box): graph_view.rs is edited by other sessions, so keep its hooks to a few lines (a call that feeds pointer events to the active tool and one that paints the dotted rectangle).
- T65's view transform (`crates/app/src/graph_zoom.rs`, `GraphUi` zoom and scroll offset in `graph_view.rs`) is reused as is: Zoom All only sets zoom and scroll through it. T66's `view_modes.rs` Full Screen is not reused: Zoom All has no reminder, no mode and no keys.
- `crates/app/src/menus.rs` (View: `grey("Zoom All")`) and `crates/app/src/toolbar.rs` (Tools bar `g(ZoomIn, ...)`, `g(ZoomOut, ...)`); `crates/app/src/actions.rs` for the actions; the axis ends are set through the same model call the Axes dialog uses (look up with `graphify query "axis from to set range layer"`) so undo is one step "Zoom In/Out".
- Bounding box of the objects outside the page: `sheaf_plot` already knows the page and its objects for hit-testing (`sheaf_plot::hit`); reuse its rectangles.
- `docs/tasks/greyed-items.md`, `PLAN.md`, `docs/spec/graphing.md` section 23, `docs/spec/gui-look.md` 4.x.

## Steps

1. Done: measure Zoom All and Scale In / Scale Out in Origin (graphing 23).
2. Zoom All: bounding box of the page and the objects, set zoom and scroll through the T65 transform (Origin draws the page at the top-left; Sheaf keeps the page centred as T65 lays it out, a known departure, margin about 10 px, factor never above 1); idempotent; Whole Page returns. Enable the View item.
3. Scale In and Scale Out tools: one-shot tools with the dotted rubber band, mapping through the layer's scale type (linear, log10; check how Sheaf's other scale types such as reciprocal or offset reciprocal behave: unmeasured in Origin), the active layer only, linked axes following, the per-window stack, one undo step "Zoom In/Out" per use. Enable the two Tools-bar buttons (tool tips from our own words: "Scale In: drag a rectangle to magnify the axes" and "Scale Out: step back to the previous axes").
4. Tests (`crates/app/src/app/ui_tests.rs`): Zoom All with an object outside the page, twice (same result), Whole Page back; Scale In rectangle gives the rectangle's ends; nested steps and Scale Out back to the original; Scale Out on an empty stack does nothing; click and 3 px drag do nothing; log axis; linked right-Y layer; undo per step; the tool ends after one use. Delete the three items from greyed-items.md and run `python tools/greyed-count.py --write`.

## Done when

The three items are enabled and behave as in graphing.md 23, tests and the three standard checks pass, and what stays unbuilt (Ctrl+drag "Enlarged" graph, double-click restore, the combo's Zoom All percentage) is listed in graphing 23.4 and here.

## Don't

- Don't build a presentation or reminder for Zoom All (that was a mis-reading; it is Full Screen's). Don't change the T65 transform or the exported page: Zoom All is screen-only; Scale In/Out change the layer's axis ends (a real model change with undo).
- Don't make Scale Out a factor-based zoom out: Origin steps back through the stack.
- Don't copy Origin's icons or help text. Don't act on the layer under the pointer: Origin acts on the active layer.
