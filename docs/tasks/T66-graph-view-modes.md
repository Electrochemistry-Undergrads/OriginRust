# T66: Graph view modes, Show toggles, Show Data Info, Full Screen (B1, slice 2)

**Frame (2026-11-22).** Origin 2016's View > Show > Frame toggles only the
active layer's thin black outline. Sheaf supports the same toggle for
Cartesian layers in the View menu. It shares the Plot Details > Display
setting and saves it in `.sheaf` and `.opj` files (graphing 22.10).

**Active Layer Indicator (2026-11-21):** View > Show toggles a per-page highlight
of the active layer's primary Cartesian axes. The setting starts off and is saved
in Sheaf projects (graphing 22.9). Six disabled entries remain.

**T177 (2026-11-19):** View > Show > Labels and Data are live as page-wide switches of every layer
(LabTalk `layer.showLabels` / `layer.showData`, a documentation contract, graphing 22.8); seven items
remain grey.

**Status:** partly done (2026-11-15, branch `graph-view-modes`, Claude Windows). Done: the four view modes (menu, Ctrl+2/1/0, ticks, saved in the `.sheaf` file), Show > Layer Icons, Full Screen with the reminder, Esc and the arrow keys (menu, Ctrl+Shift+J, page context item). Remaining after the 2026-11-17 documented Active Layer Only wire-up: nine grey items, Active Layer Indicator, Axis Layer Icons, Object Grid, Layer Grid, Frame, Labels, Data, Master Items on Screen and Show Data Info; exact Native2016 effects remain unmeasured (graphing22.3/22.7). Step 1 (Origin 2016 measurement) is in [graphing](../spec/graphing.md) section 22. Zoom In/Out, Whole Page and Pan are T65 (done); Zoom All and the Scale In/Out tools are T69.
Active Layer Only follow-up (codex/t66-show-controls-audit, 2026-11-17): View >
Show > Active Layer Only and documented Ctrl+Alt+D call the published T61
Hide Other Layers engine for the current graph/active layer. The official guide
explicitly equates these commands; this is a documentation contract, not a fresh
Native2016 measurement (graphing22.7). Derived check, all-hidden/hidden-target,
show-all, single/stale target, history and companion policies remain the existing
Sheaf choices. Dynamic following after changing the active layer is unmeasured
and is not added here. The other nine items remain grey. Data Info/2019 tooltip,
2020 grid naming and Master Items screen/output distinctions are recorded for
future work; they are not mapped to unrelated existing flags.
Fmt, strict workspace/all-target Clippy, all4,448 tests (eight ignored),
inventory274 and eight Python tests pass with all926 source/fixture/asset hashes
frozen. Three new UI regressions cover all active targets, complete-project
history/settings, shortcut/icon equivalence, hidden/invalid/single/workbook cases,
rendering and native/companion saves. Fresh desktop View isolation, derived check,
show-all and one-step Undo match saved complete graphs; restored PNG pixels match
exactly. The one owned app and guard close0. Graph-view/grid source differs only
by one inert context default each; their engines and all other owners remain fixed.

Axis Layer Icons follow-up (2026-11-24): Origin 2016 badges identify their
axis's layer and select it on click. Sheaf's undoable page switch and its
screen-only badges are covered in [graphing 22.11](../spec/graphing.md#2211-view--show--axis-layer-icons-t66-origin-2016-measured-2026-11-24).
The setting is saved per graph in Sheaf and bare OPJ projects.

Object Grid follow-up (2026-11-25): Origin 2016 drew a page-wide alignment grid
beneath the graph and kept it after reopening. Sheaf saves the page switch in
`.sheaf` and bare OPJ and draws the default grid in the editing window.
[Graphing 22.12](../spec/graphing.md#2212-view--show--object-grid-t66-origin-2016-measured-2026-11-25)

**Size:** M. **Inventory:** three remaining locations, Layer Grid, Master Items
on Screen and Show Data Info.
The original17-location baseline also included the now-published view modes,
Layer Icons, Active Layer Only and Full Screen entries.

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
