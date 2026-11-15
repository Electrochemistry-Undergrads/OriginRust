# T63: Existing-feature menu and toolbar wire-ups, round two

**Status:** open; claim groups separately. Recovered and refined 2026-11-15.
**Size:** M overall; each group S. **Branch:** `greyed-2-<group>`.
**Priority:** wave 3, parallel lane when files do not overlap wave 1/2 claims.
**Spec:** [command map](../spec/ui-command-map.md), [GUI](../spec/gui-look.md); T01's method.
**Inventory:** 31 candidate locations tagged `T63`; leads below require code/runtime verification.
**Ownership:** only the claimed group's dispatch/state/tests; no broad menus/toolbar refactor.

## Goal

Expose working features through their remaining entry points. State-disabled is distinct from
"Not available in Sheaf yet". A lead that needs real feature work is reassigned with a written
reason; enabling a misleading substitute does not complete parity.

## Origin facts

Existing task/spec measurements are the reference, not proof that a new entry point has identical
semantics. Plot-menu probability plots, export with no active graph, Fill Column With, Add Plot to
Layer and Frequency Count need targeted confirmation where their entry points differ. Measure
before calling them Origin-compatible; otherwise preserve disabled state or document/reassign.

## Where and groups

| Group | Candidate locations | Lead to verify |
|---|---|---|
| A | Bar, Stacked Area, Box Chart, Histogram, Bubble toolbar (5) | `Template` variants already in Plot menu; mirror live `Cmd::Plot` buttons in `toolbar.rs` |
| B | Horizontal Step, Vertical Step, Spline Connected (3) | `Connect::StepHorizontal`, `StepVertical`, `Spline` render; expose templates/shared args |
| C | Probability Plot, Q-Q Plot (2) | `AnalysisKind::ProbPlot`/`QQPlot`; verify whether Plot menu opens analysis or plots defaults |
| D | Open Excel (1) | T30 imports Excel into worksheets; Origin's item opens an Excel window. Reassign to B14 unless equivalent behaviour is established; do not relabel import as parity |
| E | Graph as Image, Multi-Page PDF without graph active (2) | T05/T32; either state-disable as Origin does or choose project graphs through Export Graphs |
| F | Standard/Explorer Duplicate (2) | `Cmd::DuplicateWindow` |
| G | Object Edit Bring to Front/Send to Back (2) | T24 shape ordering; only shape selection supported today, broader object order is B2 |
| H | Row Numbers/Uniform/Normal in menu and column context (6) | `sheaf-expr` row index, `rnd`, `grnd`; verify values versus stored formulas and one undo step |
| I | X/Y Axis Tick Labels (2) | `dialogs/graph_axis.rs`, Tick Labels tab for correct axis |
| J | Add Plot to Layer: Line, Scatter, Line + Symbol, Column, Area (5) | Existing Layer Contents/Plot Setup; confirm selection-driven versus chooser semantics |
| K | Column-header Frequency Count (1) | Existing histogram versus discrete frequency analysis; confirm correct operation |

## Steps

1. Claim named letters on main. Read/run the lead and inspect the existing twin's state rules.
2. Wire commands and target selection narrowly. If more than roughly two hours of feature work
   is required, reassign the item to its real package rather than weakening its semantics.
3. Add focused UI dispatch/state and resulting-behaviour coverage (toolbar Print is a pattern).
   Test no selection, wrong window, applicable selection, one undo and cancellation where relevant.
4. Delete enabled items or retag deferred ones with reason; run `python3 tools/greyed-count.py
   --write`, update counts/status, merge and release only that group's claim.

## Done when

All eleven groups are wired or reassigned with reasons; checks pass. A reassigned item remains
open in the inventory and its owning package; T63 completion alone does not claim its feature done.

## Don't

Do not reorder measured menus, enable operations with different meaning, take another group's
claim, or introduce broad new features here. Coordinate edits to menus/toolbar hot spots.
