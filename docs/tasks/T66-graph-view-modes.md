# T66: Graph view modes, Show toggles, Show Data Info, Full Screen (B1, slice 2)

**Status:** partly done (2026-11-15, branch `graph-view-modes`, Claude Windows). Done: the four view modes (menu, Ctrl+2/1/0, ticks, saved in the `.sheaf` file), Show > Layer Icons, Full Screen with the reminder, Esc and the arrow keys (menu, Ctrl+Shift+J, page context item). Still grey, 10 items: Active Layer Indicator, Axis Layer Icons, Object Grid, Layer Grid, Frame, Labels, Data, Active Layer Only, Master Items on Screen and Show Data Info, because Origin's effect for them is unmeasured (graphing 22.3). Step 1 (Origin 2016 measurement) is in [graphing](../spec/graphing.md) section 22. Zoom In/Out, Whole Page and Pan are T65 (done); Zoom All stays `B1`.
**Size:** M. **Inventory:** 17 locations: View > Print View, Page View, Window View, Draft View; View > Show (ten items); View > Show Data Info; View > Full Screen; graph page context item "View Full Screen".

## Goal

The View menu of a graph window switches how the page is shown (Print, Page, Window, Draft), the Show items turn parts of the display on and off, and Full Screen shows only the page over the whole screen until Esc. As in Origin, a view mode is a property of the graph window and is saved with the project.

## Origin facts (measured 2026-11-15, Origin 2016; details and the unmeasured list in graphing.md 22)

- Modes: Print View Ctrl+2, Page View Ctrl+1 (the default, ticked), Window View (no key), Draft View Ctrl+0; exactly one is ticked. LabTalk `page.viewmode`: 1 Print, 2 Page, 3 Window, 4 Draft (other values are ignored).
- Page and Print View draw the page at its own size with the grey window background where the page does not fill the window. Window View and Draft View stretch the page to fill the whole window (no grey area). Draft View draws the same content without the smoother text of the other modes (the lines and symbols are the same).
- The mode is saved in the project: set to Window View, saved, reopened, `page.viewmode` is 3.
- Show submenu, in order: Layer Icons, Active Layer Indicator, Axis Layer Icons | Object Grid, Layer Grid | Frame, Labels, Data, Active Layer Only | Master Items on Screen. Ticks by default: Layer Icons, Labels, Data (the rest off). Show Data Info is a separate item below the submenu, ticked by default.
- Layer Icons off removes the numbered layer icons ("1", "2") at the top-left of the page. Its click needs no dialog.
- Full Screen (Ctrl+Shift+J, also the page right-click item): the graph page fills the whole screen with no menu, toolbars or title bar; a "Reminder Message" dialog (Hit ESC to exit; Arrow keys shift to different graphs; Home and End key to first and last graph; a "Do not show this message in the future" tick; OK) is shown first. Esc ends it and the window is back as before.

## Where

- New `crates/app/src/view_modes.rs`: the enums and state (`ViewMode`, `ShowFlags`), the Full Screen state and its overlay; keep hooks in `graph_view.rs` to a few lines (it is edited by the zoom/pan and shape sessions).
- `crates/app/src/menus.rs` (the View menu of a graph: `grey("Show Data Info")`, `grey_keyed("Full Screen", ...)`, the Show submenu list, the four mode items) and the graph page context menu ("View Full Screen", `grey_keyed` near `Copy Page`).
- Model: the mode per graph window is saved in the `.sheaf` file; the `.opj` reader/writer map `page.viewmode` only if the OPJ block is found (check `docs/spec/opj-format.md` before promising it).
- `docs/tasks/greyed-items.md`, `PLAN.md`, `docs/spec/graphing.md` section 22.

## Steps

1. Sheaf draws the page to fit the window (Page View's picture). Window View fills the window with the page (aspect not kept) and Draft View does the same with plain text. Do those with the menu ticks and Ctrl+1/2/0 first. Enable an item only when it does what Origin's does.
2. Layer Icons tick (default on) hides the layer icon row.
3. Measure the unmeasured Show items in Origin (see graphing.md 22.5; the View menu collapses and its short form changes after use, so click the chevron and take a shot before clicking an item, or turn off Tools > Options > Miscellaneous > Enable Collapsible Menus), then do each item that is measured; leave the rest grey and listed.
4. Full Screen: overlay over the whole window, Esc ends it, the reminder dialog with its tick; Ctrl+Shift+J and the page context item.
5. Show Data Info: measure what window it opens (the item is ticked, so it is a toggle for a Data Info window; unmeasured) before enabling.
6. Tests: a UI test per mode and per tick in `crates/app/src/app/ui_tests.rs`; `--show` screenshots; delete the enabled items from greyed-items.md and run `python tools/greyed-count.py --write`.

## Done when

The four modes (tick, keys, saved with the graph), the Show items that are measured, Full Screen with Esc and Show Data Info behave as in graphing.md 22, tests pass, and the rest is listed in greyed-items.md as before.

## Don't

- Don't touch the zoom/pan transform (T65). Don't enable an item that does not do what Origin's does. Don't copy Origin's icons or help text.
- Don't assume Print View and Page View differ in this window size: they looked the same; measure with a page larger than the window before drawing a difference.
