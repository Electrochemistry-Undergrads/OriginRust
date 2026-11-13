# T11: More axis scales and axis features

**Status:** items 1 (all eight scales, measured, spec 3.8) and 2 (Reverse, First Tick, Prefix/Suffix) done on branch `axis-scales`; 3-6 open (axis breaks, special ticks and tick label formula, Exchange X-Y, dialog Apply To/tab names). Not yet done for item 1: Origin picture comparison of our render, minor ticks of the list scales, the reciprocal step rule. **Size:** L, one
commit per feature. **Branch:** `axis-more`. **Model:** Sonnet.
**Detail:** audit C "Missing scales", "Missing axis features"; B "Axis dialog".
**Spec:** `docs/spec/graphing.md` sections 3-4 (axes, ticks), 12 (linked layers).

## Today

- `ScaleKind` has Linear and Log10 only (`crates/core/src/graph.rs`, `pub enum ScaleKind`).
  Tick resolution: `resolve_linear` and the log path in `crates/plot/src/ticks.rs`.
- Axis dialog `crates/app/src/dialogs/graph_axis.rs`: Special Ticks and Breaks are placeholders;
  greyed Prefix/Suffix, Reverse, First Tick, Title Rotate; no "Apply To..."; uses
  Horizontal/Vertical on every tab (Origin: Horizontal/Vertical on Scale and Breaks, Bottom/Top/
  Left/Right on the others).
- Layer links are None or Straight (`graph.rs`); no custom top-X formula.

## Features, in order

1. Scales: ln, log2, reciprocal, offset reciprocal (spec lists formulas); then probability,
   probit, logit, Weibull. Each needs tick rules from Origin (measure tick positions with
   `layer.x.type` set and `layer -a`, like `autoscale*.lines.txt`).
2. Reverse axis; First Tick; Prefix/Suffix on tick labels.
3. Axis breaks (one break first).
4. Special ticks; tick label formula; minor tick labels; tick colour and thickness.
5. Exchange X-Y Axes (Graph menu, greyed).
6. Dialog: "Apply To...", Bottom/Top/Left/Right tabs as Origin, Tick Labels sub-tabs Format/Table/
   Minor Tick Labels.

## Done when

Each scale's ticks match Origin on 3+ measured ranges (fixture test); `.opj` write/read of the
new scale (opj-format.md has the axis type field) or named in the save warnings; checks pass.
