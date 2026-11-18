# T65: Graph view zoom and pan (B1, slice 1)

**Status:** the View and Graph toolbar commands and the Zoom-Panning tool landed 2026-11-15
(gui-look 4.x). T69 added the Tools-bar Scale In/Out and Zoom All. The Standard toolbar zoom
combo now shows the active graph's zoom, offers the five measured presets and accepts typed
whole percentages. Formatting, strict workspace Clippy, Nextest (4,500 passed and
8 skipped), doc tests, inventory and eight Python tooling tests passed.
**Size:** M. **Priority:** wave 4 (B1), the first backlog package after waves 1-3 were claimed.
**Spec:** [gui-look](../spec/gui-look.md) section 4 (graph window mouse actions); add a "Zoom and
pan" subsection there. **Inventory:** 9 locations tagged `T65` (View > Zoom In, Zoom Out, Whole
Page; Graph toolbar Zoom In, Zoom Out, Whole Page; Tools toolbar Zoom In, Zoom Out, Pan). The rest
of B1 (view modes, Show toggles, Zoom All, Show Data Info, Full Screen) stays `B1`.

## Goal

Look closely at part of a graph and move around it, as in Origin: Zoom In by a click at the point
to enlarge, Zoom Out, Whole Page back to the page fitting the window, and Pan by dragging. The
model is not changed: zoom and pan are view state of the open graph window, and every click, drag
and hit-test keeps working at any zoom.

## Origin facts (measured 2026-11-15, Origin 2016, scratch `re-workspace/scratch/t60/zoom`)

- View menu (expanded): Print View (Ctrl+2), Page View (Ctrl+1, ticked), Window View, Draft View
  (Ctrl+0), Zoom In (Ctrl+I), Zoom Out (Ctrl+M), Whole Page (Ctrl+W), Zoom All, Show >, Show Data
  Info. The collapsed menu shows only Zoom In and Whole Page.
- **Zoom In is a mode**: choosing it changes nothing until a click on the page; the click zooms to
  200 % around the clicked point, scroll bars appear, and the Standard toolbar's zoom combo shows
  200%.
- **Zoom Out acts at once**: from 200 % it went back to 100 % without a click.
- At 100 % the whole page shows in the window (the combo said 100% with the page fitted to the
  window when the graph opened).
- **Zoom All** (corrected by T69, graphing 23.1: it is a zoom of the active graph to all objects; the reminder below is Full Screen's) was first noted as a different thing: a presentation of all graphs with a reminder ("Hit ESC to exit,
  arrow keys to shift to different graphs, Home and End to the first and last graph"); not in this
  slice.
- The Standard toolbar combo lists 25, 50, 75, 100 and 200 percent, accepts 300 when typed,
  and displays the current zoom rounded to a whole percent. Gui-look 4.x records the measured
  pan and zoom commands and Sheaf's validation and centre anchoring policy.

## Where

- `crates/app/src/graph_view.rs`: `show()` computes `fit` (UI points per page point so the page
  fits the window) and `image_rect`, stored as `state.view = Some((image_rect, fit))`; hit-testing
  and drags convert through `state.view`. Add a zoom factor and a scroll offset to `GraphUi`, apply
  them to `fit` and `image_rect`, clip painting to the window rectangle, and keep every conversion
  going through the same transform. The raster is rendered at `fit * pixels_per_point`: cap its
  size (for example 4096 px a side) and let the texture scale beyond that.
- `crates/app/src/menus.rs` (`grey_keyed("Zoom In", ctrl(Key::I))` etc. in the View menu) and
  `crates/app/src/toolbar.rs` (`g(ZoomIn, ...)`, `g(Hand, "Pan")` on the Graph and Tools bars):
  live items, a tool state for Zoom In and Pan like the other graph tools (`GraphTool`).
- `crates/app/src/actions.rs`: actions for zoom in (mode), zoom out, whole page, pan.

## Steps

1. Measure the open questions above; record them in gui-look.
2. View transform with zoom and scroll, the scroll bars, clipping; Whole Page; Zoom Out; Zoom In
   mode with the click point kept under the pointer; Pan tool drag. Hit-testing, selection handles,
   drags of labels/legends/shapes, the Data Reader and the data tools at 200 % and 400 %.
3. Enable the 9 locations, delete them from greyed-items, UI tests (zoom in at a point and the point
   stays under the pointer, zoom out, whole page, pan, a click selects the right object when zoomed,
   Escape leaves the Zoom In mode), screenshot check.

## Don't

Do not change the graph model or the exported page: zoom is screen-only. Do not build the view
modes (Print/Page/Window/Draft), the Show toggles, Zoom All or Full Screen (rest of B1).
