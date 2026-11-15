# T11: More axis scales and axis features

**Status:** items 1-2 merged (`3e9f619`); items 3-6 done on branch `axis-scales` (one break per axis, not measured in Origin; special ticks, tick label formula, minor tick labels, tick colour and thickness; Exchange X-Y Axes; Apply To..., Bottom/Top/Left/Right list, Tick Labels sub-tabs). Reciprocal default step/range now follows 57 Origin 2016 measurements (graphing 3.8; finite positive nonconstant data, default 8% margin); explicit increment still overrides it. Several breaks (1 to 7, Auto Position, Origin's gap and mark measured, graphing 3.6) done 2026-11-15 (branch `axis-breaks`). Left: Origin's tick-step change when breaks are added and the Break Half Length unit (unmeasured), reference lines (Origin 2016 has none; its Grids tab Additional Lines, Opposite and a line at a value of the other axis, done 2026-11-14, graphing 3.7), Origin picture comparison, .opj storage of the new axis fields (named in the save warnings). **Size:** L, one
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
