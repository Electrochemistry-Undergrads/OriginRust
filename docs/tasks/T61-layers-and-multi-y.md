# T61: Layers, insets and three/four Y axes

**Status:** in progress; recovered and refined 2026-11-15. Step 0 (Origin measurements, graphing 21) and slice 1 (add/delete layers, inset, icon Delete and Layer Properties) are done; slice 2 (axis offsets, Move to Layer) and slice 3 (3Ys/4Ys templates, Multiple Y Axes, CLI/MCP names, .sheaf/.opj handling) are done. Open: Open Dialog..., Show Active Layer Only and Set Active Layer By Layer Icon Only (unmeasured), Hide Layer hiding axes, Double click to open Plot Setup. Hide Layer should also hide axes; Open Dialog... and two icon-menu items stay grey (unmeasured). **Size:** L, three slices.
**Branch:** `layers-multi-y`. **Priority:** P0, wave 2.
**Spec:** [graphing](../spec/graphing.md) 1.3 and 12; ADR 0009.
**Inventory:** 20 locations tagged `T61`. **Ownership:** layer creation/deletion, icon actions,
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
