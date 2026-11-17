# T115: Plot > Specialized > Smith Chart

**Status:** claimed 2026-11-16 (Claude Windows "t30-origin", branch `smith-chart`).
**Size:** M. **Source:** greyed inventory package B4 (Plot > Specialized > Smith Chart).
**Spec:** `docs/spec/graphing.md` (a new section).

## Goal

A Smith chart as Origin's: impedance data (resistance and reactance columns) drawn on the unit
circle of the reflection coefficient, over the grid of constant-resistance circles and
constant-reactance arcs.

## Steps

1. Measure it in Origin 2016 with tools/origin-ref if automation works; otherwise build from
   Origin's documented behaviour and mark the section unmeasured (as T113 and T114).
2. Build it: the impedance-to-reflection mapping, a layer with the Smith grid, a template, the menu
   item.
3. Tests, a graphing.md section, greyed inventory recount.

## Done when

The menu item plots, the inventory no longer lists it, and fmt, strict Clippy and the full tests
pass.
