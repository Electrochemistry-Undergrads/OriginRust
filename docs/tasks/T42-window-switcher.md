# T42: Window Switcher (Ctrl+Tab)

**Status:** claimed 2026-11-15 (session "mac-multipdf", macOS, no Origin), branch `window-switcher`.
**Size:** M. **Source:** `origin-2026-2027-gaps.md` (Worksheets and project workflows: Window
Switcher, Origin 2025b).

## Goal

Hold Ctrl and press Tab to show a switcher of the open child windows with previews; each Tab moves
the selection, Shift+Tab moves back, releasing Ctrl activates the selected window.

## Origin facts (docs, not measured)

- 2025b: "Ctrl+Tab (forward) and Ctrl+Shift+Tab (backward) to toggle child windows with
  preview." [W:originlab.com/2025b]
- "Press Ctrl key and then click Tab key to display the Window Switcher"; "Click the Tab key can
  switch window until Ctrl is released"; it "displays thumbnails of all windows in the same folder
  including the floating windows and hide windows"; the selected window has a blue frame; arrow
  keys and a mouse click also select; clicking Ctrl alone closes it. Script: `doc -nav` then Tab.
  [W:origin-help/operate-childwindow-basic]
- Not documented: order, minimized windows, Esc, sizes. Sheaf's choices (unmeasured): most
  recently activated first (so one Ctrl+Tab returns to the previous window, as Windows Alt+Tab),
  Esc cancels, Enter confirms, thumbnails rendered from the window's content at a small size and
  cached per frame while open.

## Where

- `crates/app/src/app.rs` (window activation order / z-order, folders, hidden windows; an MRU list
  updated on activation), a new `crates/app/src/switcher.rs` overlay (grid of cards: thumbnail,
  name, type icon, blue frame), key handling before the grid/editors consume Tab (the worksheet
  grid uses Tab for cell moves: only plain Tab, not Ctrl+Tab), Window menu entry if fitting.
- Thumbnails: graphs via the existing raster render at low resolution; workbooks a small drawn
  preview of the first cells or a type icon (Sheaf's choice).

## Done when

Ctrl+Tab / Ctrl+Shift+Tab cycle with an overlay of the same-folder windows (hidden included),
releasing Ctrl activates (hidden windows are shown), Esc cancels, click selects; UI tests cover
order, wrap-around, hidden windows, cancel and that grid Tab still moves cells.

## Don't

- Don't change Tab inside the worksheet grid, recent files (T38) or MCP/live-link code.
