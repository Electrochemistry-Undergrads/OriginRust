# T72: Copy Format and Paste Format on graphs (B2, slice 1)

**Status:** open (step 1 done 2026-11-16: Origin 2016 measured, spec graphing 24; step 2, the Rust, not started).
**Size:** M. **Priority:** wave 4 (B2). **Spec:** graphing 24. **Inventory:** the Edit menu's Copy
Format (nine choices) and Paste Format... [Graph]; Graph axis, legend and page > Copy Format.
Not in this slice (stay `B2`): object cut/copy/paste, align, same size, group, order across kinds,
snap to grid, Object Properties. Not in this slice (stay `B11`): Save Format as Theme... and
Theme files. A Codex session edits drawn-shape selection and resize handles in `graph_view.rs`: put
new code in new modules and keep the hooks there minimal.

## Goal

A format clipboard for graphs. "Copy Format" copies a chosen group of properties of the page, an
axis or the legend; "Paste Format" applies them to another graph, axis or legend, in one undo step.

## Origin facts (measured 2026-11-15/16, Origin 2016; raw files in `re-workspace/scratch/t72`, local)

Full detail and the "unmeasured" list are in graphing 24. In short:

- **Where it lives.** Edit menu (graph window): Copy Format ▸ (All | All Style Formats, Colors,
  Fonts, Background, Symbol, Line and Fill | Dimensions, Scales | Text) and **Paste Format...**
  (with dots, opens the "Apply Formats" window). Graph page context menu (right-click on the page
  outside the layer): Copy Format ▸ (Page Background, Legend Translation Mode, Dimensions, Scales |
  Colors, Fonts, Background, All Style Formats | All) and **Paste Format** (no dots, acts at once),
  then Save Format as Theme.... Axis context menu: Copy Format ▸ (Styles, Scales, Dimensions | All),
  and, once an axis format is on the clipboard, **Paste Format To** ▸ (Tick Direction, Ticks, Line,
  Tick Labels, Title Styles, Colors, Thickness, Styles | Scales | Title Dimensions, Title Text |
  All). Legend context menu (right-click on the legend frame; on an entry it is Hide Data Plot /
  Hide Others): Copy Format ▸ (Styles, Dimensions | All). The inventory's shorter lists ("All, All
  Style Formats, Axis" and so on) are Origin's collapsed menus; the full lists above are the ones to build.
- **What the clipboard holds decides the paste menu:** a graph format gives "Paste Format"; an axis
  format gives "Paste Format To ▸" (on the page menu its first level is Bottom Axes, All X Axes,
  All Axes, All, each with a further submenu that was not opened).
- **Graph, Copy Format All then Paste Format** (page menu to page menu, no dialog): the target takes
  the axis colours, tick-label sizes and colours, axis thickness, layer background colour, x scale
  ends, layer width and position, page width, the text object's size and colour and **its text**,
  and the plots' colours and symbol sizes. Undo text: **"Undo Apply Theme"**.
- **Per choice** (pasted into an untouched graph, values read back): Dimensions = layer width, left
  and top, page width. Scales = x from/to. Colors = axis line, tick-label and text colours, layer
  background, plot line and symbol colours (no sizes, no fonts, no scales). Fonts = tick-label sizes
  and text size (no colours). All Style Formats = colours + fonts + thickness (no scales, no
  dimensions, no text content). Background = layer background (axis thickness also read 4, unexplained).
  Page Background and Legend Translation Mode showed no readable change (unmeasured).
- **Axis, Copy Format All then Paste Format To > All** on the selected X axis: that axis took the
  colour, tick-label size and colour and the scale; the Y axis and the layer were untouched. Undo
  text again "Undo Apply Theme".
- **Edit > Paste Format...** opens the modeless **Apply Formats** window, slowly (about 4 s): Apply
  From (Clipboard / Theme File with a list and a browse button, Description), then for an axis format
  "Formats to Apply" check boxes (All, Line, Colors, Scales, Tick Direction, Tick Labels, Thickness,
  Title Dimensions, Ticks, Title Styles, Styles, Title Text), Apply Scope ("Selection" with an axis
  selected, "Active Graph" otherwise) and Apply To ("Bottom Axes"); buttons Undo Apply, Apply, Close,
  Save, Save As.... For a graph format it shows a Property/Value grid, List View, Collections,
  "Apply Selected Properties Only" and Apply Scope "Active Graph" (contents unmeasured).
- **Mismatch:** no error or message. A one-plot target of a two-plot source took the first plot's
  colours and symbol size; a three-plot target of a two-plot source got a third style Sheaf cannot
  derive from the source; a two-layer target: the active layer read back changed (the other layer was not read). Rules are partly unmeasured.

## Where

- New `crates/core/src/format_clip.rs`: `FormatScope` (Page, Axis, Legend), `FormatPart` (the choices
  above), `FormatClip` (captured values) with `capture(graph, scope, part)` and
  `apply(&mut graph, clip, target, parts)`; pure and unit-tested. Properties from `Graph`, `Layer`,
  `Axis`, `Legend`, `TextLabel`, `Plot` in `crates/core/src/graph.rs`.
- New `crates/app/src/format_clip_ui.rs`: the clipboard (also copied to the OS clipboard as text so a
  second Sheaf window sees it: Origin's survives restarts), the Apply Formats window, one `GraphOp`.
- `crates/app/src/menus.rs`: Edit menu (`Copy Format` near line 742, `Paste Format...` near 755), axis,
  legend and page context menus (`grey_sub("Copy Format", ...)` near lines 1874, 1971, 1999).
  Match the measured lists; undo text "Apply Theme".

## Steps

1. Done: measurements and this brief.
2. Core capture/apply with tests (one test per choice: only the listed properties change).
3. Menus and context menus, Paste Format and Paste Format To, Apply Formats window, Undo text.
4. Measure the leftovers below if Origin is free; then enable the items and update the inventory
   (replace the old short lists by the measured ones, keep the count script green).

## Done when

Every Copy Format list above exists and does what Origin's item does (graph, axis, legend), Paste
Format and Paste Format To work from the context menus and the Edit menu opens Apply Formats, one
undo step named "Apply Theme", UI tests, inventory and counts updated, graphing 24 completed.

## Don't

- Don't copy text content with All Style Formats (only All does).
- Don't paste a graph format into an axis or the reverse silently: the clipboard type chooses the menu.
- Don't touch `graph_view.rs` beyond a hook; Codex edits shape selection there.
- Don't enable "Save Format as Theme..." (B11) or the object commands (B2).

## Unmeasured (measure before building, or choose and say so)

Symbol, Line and Fill and Text choices; Page Background and Legend Translation Mode; legend paste
(and its Copy Format effect); second-level submenus of Paste Format To; the layer-area context menu's
Copy Format; graph clipboard on an axis target and the reverse; which layer or plot gets what on
mismatched layers and plots; the Apply Formats grid and scope list; the pencil button's menu.
