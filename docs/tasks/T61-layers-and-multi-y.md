# T61: Layers, insets and three/four Y axes

**Status:** in progress; recovered and refined 2026-11-15. Step 0 (Origin measurements, graphing 21) and slice 1 (add/delete layers, inset, icon Delete and Layer Properties) are done; slice 2 (axis offsets, Move to Layer) and slice 3 (3Ys/4Ys templates, Multiple Y Axes, CLI/MCP names, .sheaf/.opj handling) are done. Open: User Defined/Auto Offset/custom scales in Open Dialog..., Show Active Layer Only and Set Active Layer By Layer Icon Only (unmeasured), Double click to open Plot Setup. Hide Layer is a reversible whole-layer toggle (graphing 21.13); Hide Other Layers isolates/shows all whole layers from the published contract (21.14), with live 2016 behavior still unmeasured; Open Dialog... and the unsupported icon-menu items stay grey. **Size:** L, three slices.
Legend-anchor follow-up (`codex/multi-y-template-legends`): the four stock templates use
measured top-left positions above the frame, Arial 22 pt (YY-YY: 20 pt), and no frame or background.
A compatible top-left position variant keeps the left edge fixed when labels grow; existing
centre positions, Multiple Y Axes dialog plans and layer assignment remain unchanged.
The Legend editor and native/companion saves handle the anchor. Graphing 21.12 records
24 synthetic native controls/readbacks and the remaining spacing/membership limits.
Fmt, strict workspace/all-target Clippy and all 4,214 workspace tests pass (eight ignored),
including seven anchor/layout/editor/persistence regressions. Four updated SVG goldens and
fresh CLI PNGs were reviewed; inventory 310 and eight Python tooling tests pass. Source and
goldens stayed frozen through validation, with four test threads.

Hide Layer follow-up (`codex/t61-hide-layer-axes`, 2026-11-16): a compatible
visibility flag suppresses curves, axes and owned objects without changing plot
visibility or axis settings; the checked icon-menu entry toggles it back. Linked
geometry remains fixed and hidden points cannot be read/picked. Five private
Origin2016 controls confirm unchanged element flags/ranges/frames and identical
restore PNGs (graphing21.13). Native project saves warn that the unmeasured OPJ
visibility bit is omitted; the companion preserves it. Seven new regressions cover
rendering/geometry/picking, legacy records, menu history, readers and save/reopen.
Formatting, strict workspace/all-target Clippy and all 4,336 tests pass (eight existing ignored), including all seven new regressions, on integrated T112/T113/T114 source. The 290-location inventory, eight Python tooling tests and frozen source/fixture/asset hashes pass. Current desktop baseline/hidden/restored captures and PNG/SVG exports were inspected; owned Native/app/helper actors are closed. Other T61 leftovers remain open.

Hide Other Layers follow-up (`codex/t61-hide-other-layers`, 2026-11-16): the
existing entry toggles whole-layer isolation/show-all and derives its check mark
from visibility. Individual plot/axis/display choices and geometry remain intact;
hidden targets become visible, invalid/single-layer requests are no-ops, and each
change is one Undo. Origin's published guide establishes the basic reversible
contract; active-hidden/derived-check rules are explicit Sheaf choices, not fresh
2016 measurements. No new model/renderer/IO contracts or following-active mode.
Three new UI regressions cover all three targets, rendering/geometry, settings,
Undo/Redo, hidden targets and invalid/no-op history. Formatting, strict workspace/all-target Clippy and all 4,363 tests pass (eight existing ignored), including all six layer-visibility UI regressions, on integrated T118/T58 RGB/T60 viewport source. Inventory283, eight Python tooling tests and frozen 911 source/fixture/asset files pass. Fresh baseline/isolated/restored desktop captures and PNG/SVG exports were inspected; restored export pixels match exactly, and all own builders/apps/helpers are terminal. Remaining T61 claims and controls stay open.

Preset Open Dialog follow-up (`codex/t61-add-layer-dialog`, merged 2026-11-19 by takeover as
`add-layer-dialog-2`): Graph and graph-page routes
open New Layer (Axes) with the existing nine kinds and Activate New Layer. The official layadd
contract supports these controls; no new live Native measurement is claimed. Existing layer
helpers, links, geometry, history and saves are reused. Input graph/parent capture and
revision/generation rejection are explicit Sheaf policies. User Defined, automatic overlap
offsets and custom scale expressions remain disabled; no model/backend/IO contract changes.

**Branch:** `layers-multi-y`. **Priority:** P0, wave 2.
**Spec:** [graphing](../spec/graphing.md) 1.3 and 12; ADR 0009.
**Inventory:** 5 locations tagged `T61`. **Ownership:** layer creation/deletion, icon actions,
axis offsets, multi-Y templates/dialog. T60 owns arrangement/merge; coordinate `graph.rs` edits.

## Goal

Add the four named axis-layer kinds and an inset to a finished graph; add data through existing
Layer Contents/Plot Setup; delete layers safely. Make layer-icon settings and Layer Properties
work. Provide 3Ys Y-YY/Y-Y-Y, 4Ys Y-YYY/YY-YY and Multiple Y Axes, with independent coloured scales
and per-plot axis/layer choice in the app.

## Origin facts

Documentation describes the named layers, outward-offset extra Y axes and `plotmyaxes` window;
these are **not new measurements**. Measure all four Add Layer defaults (frame, axes, titles,
links), inset geometry, icon settings, each multi-Y template (axis offsets, colours, legends,
extra-column assignment) and Multiple Y Axes controls. Verify what Horizontal/Vertical Axis Layer
open; reassign them if not supported by the reference version. Capture a PNG per template.

## Where

- `crates/core/src/graph.rs`: `Graph.layers`, `Layer.link`, compatible axis-offset field if needed;
  pure add/delete helpers with link/index repair. A parent must precede its child.
- `crates/core/src/template.rs`: build multi-Y templates on `Template::DoubleY`; expose shared
  names in `sheaf_ops::args` for CLI/MCP (`3ys-y-yy`, `3ys-y-y-y`, `4ys-y-yyy`, `4ys-yy-yy`).
- `crates/plot`: axis placement, margins and hit-testing for displaced scales.
- `crates/app/src/actions.rs`, `toolbar.rs` (`Bar::Graph`), `menus.rs`, `graph_view.rs`: history,
  active layer, four Add Layer buttons, inset, layer icon settings, Layer Properties and selection.
  Use existing Plot Details/Layer Contents for per-plot assignment where appropriate.
- `crates/io/src/opj/write`: ordinary layer writing; preserve unsupported offsets in companion
  and warn per ADR 0008. Preserve compatible `.sheaf` defaults.

## Steps

1. Add four layer kinds/inset, delete and icon actions, Layer Properties at the selected layer.
   Measure deletion semantics or document provisional reparent/refusal policy. Test deleting a
   parent, the first/last layer, chains, dangling refs, selection repair, Undo/Redo and an inset golden.
2. Per-plot layer/axis choice and offset-axis rendering. Test margin growth, clipping, hit targets,
   linked ranges and template goldens; preserve Double-Y's measured colours/defaults.
3. Four multi-Y templates plus Multiple Y Axes dialog and shared CLI/MCP paths. Golden exports,
   insufficient-column validation and save/reopen regressions. Measure/refine and document defaults.
   Enable and remove the 20 inventory locations as their functionality lands.

## Done when

Second/third/fourth Y axes, insets and deletion work in the app; scales remain readable and linked
correctly; save/reopen/exports and undo preserve them. All locations are implemented or reassigned
with reasons, and task-workflow checks pass.

## Don't

Do not merge finished graphs or arrange grids (T60), permit forward/cyclic layer links, or regress
existing Double-Y templates. A new axis must participate in automatic margins and hit-testing.
