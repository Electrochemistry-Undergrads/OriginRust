# T60: Merge Graph Windows and Layer Management

**Status:** done 2026-11-15 (branch `merge-graphs`, Claude Windows). Origin 2016 measured (graphing
15.7c, 15.7d; probes `tools/origin-ref/merge-graph-*.lines.txt`, `fit-page-to-layers.lines.txt`):
Merge Graph Windows (core `merge.rs`, dialog, `sheaf merge-graphs`), Layer Management (Arrange,
Size/Position, Link; core `layer_manage.rs`), Fit Page to Layers (dialog and Format item). Fit Layer to
Graph did nothing measurable in Origin and is tagged `N`. Left: Merge's folder choices, extra layers
for empty cells and Common Scale options, panel labels, the merge preview; Layer Management's Add
(T61), Axes and remaining Display controls; MCP tool for merging. **Size:** L, merge then management.
Background-colour follow-up (`codex/t60-layer-display`, 2026-11-16): Display > Color >
Background Color supports selected layers, None, basic presets and opaque RGB. Native two-layer
Apply/Cancel/OK/mixed/None measurements are recorded in graphing 15.7d. Preview/history/no-op
and stale-source guards preserve other layer settings; existing render/native/companion save
paths are reused without model changes. Drawing Order, border colours/dimensions and element
scaling remain disabled. Fmt, strict workspace/all-target Clippy and all 4,229 workspace tests
pass (eight ignored), including six new semantic regressions and all 17 Layer Management tests.
Inventory 308 and eight Python tooling tests pass; source and goldens stayed frozen, using four
test threads. The desktop Display/RGB preview and independent PNG export were inspected.

**Branch:** `merge-graphs`. **Priority:** P0, wave 2, owner's stacked-figure priority.
**Spec:** [graphing](../spec/graphing.md) 15 (add Merge Graph Windows and Layer Management
subsections); ADR 0009. **Inventory:** 0 locations tagged `T60`.
**Ownership:** copying/arranging existing layers, merge and management dialogs, fit-layer/page.
T61 owns adding/deleting individual layers and insets. Agree on shared helpers before merging.

## Goal

Combine finished graph windows into a grid or vertical stack while keeping plots, axes, labels,
legends and styles. Preserve source graphs unless the user explicitly selects removal. Provide
Layer Management with layer list/preview, Arrange, Size/Position and axis links. Fit Layer to
Graph fills the page inside margins; Fit Page to Layers fits the page to its layers.

## Origin facts

The X-Function index names `merge_graph`; GUI section 4 records Layer Management menu locations.
**Not measured:** both windows, defaults/units, source graph selection, source multi-layer
placement, text/symbol scaling, legends, retained links, page sizing, Apply and fit margins.
Measure 2×1 and 3×1 merges of stock Line graphs and capture PNG fixtures. Until measured, row-first
placement, fixed point sizes, optional text scaling and shared-X are labelled Sheaf choices.

## Where

- `crates/core/src/panels.rs`: `PanelLayout`, `PanelOrder`, `PanelPage`, `Graph::arrange` already
  implement column-built panels. New `core/merge.rs` may deep-copy source layers and use layouts.
- `crates/core/src/graph.rs`: layer frames and `LayerLink { parent, x, y }`. Remap links to the
  copied parent's index; retain the invariant that parents precede children.
- New `crates/app/src/dialogs/merge_graphs.rs`, `layer_management.rs`; small `sheaf_plot` preview.
- `crates/ops`, `cli`, `mcp`: share the core merge operation; proposed CLI:
  `sheaf merge-graphs PROJECT --graph A --graph B --rows 2 --cols 1 -o out.png`.
- `.sheaf` can already store layers; `.opj` writes layers and companion-held links (ADR 0008/0009).

## Steps

1. Pure merge operation and validated source/layout options. Deep-copy graphs, preserve column
   references, source-internal geometry and links, remap parents, handle empty cells and duplicate
   source selections explicitly. A multi-layer source must stay together in one destination cell.
   Test 2×1, 1×2, three sources in 2×2, a Double-Y source and unchanged source/data graphs.
2. Merge dialog/menu plus CLI/MCP, one undo step for creation and any selected source removal.
   Test cancellation, missing references, source folders and a three-panel shared-X golden.
3. Layer Management list/preview, Arrange, Size/Position and Link; each Apply is one undo step.
   Validate frames and link cycles/ordering. Add Fit Layer to Graph and Fit Page to Layers without
   clipping titles/legends. Enable the remaining menu/context locations and remove their tags.
4. Measure Origin's windows, source scaling and layout; record differences and reconcile.

## Done when

Three finished graphs become a vertical shared-X figure through GUI and CLI; layers can be
arranged/resized/linked. Exports, undo and `.sheaf`/`.opj` companions retain content and geometry.
All seven locations are implemented or reassigned with reasons; task-workflow checks pass.

## Don't

Do not rebuild the measured Stack dialog or 2/4/9 Panel templates: they create graphs from columns.
Do not take T61's single-layer lifecycle work, change source data, or learn layout from Origin's
binary files; measure the UI and rendered exports.
