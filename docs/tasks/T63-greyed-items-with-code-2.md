# T63: Existing-feature menu and toolbar wire-ups, round two

**Status:** open; F wired (2026-11-15), C reviewed and deferred to B8/T14; claim remaining groups separately. Recovered and refined 2026-11-15.
**Size:** M overall; each group S. **Branch:** `greyed-2-<group>`.
**Priority:** wave 3, parallel lane when files do not overlap wave 1/2 claims.
**Spec:** [command map](../spec/ui-command-map.md), [GUI](../spec/gui-look.md); T01's method.
**Inventory:** 31 initial candidate locations; 27 remain tagged `T63` after C/F review. Remaining leads require code/runtime verification.
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
| C | Probability Plot, Q-Q Plot (2) | Reviewed: deferred to B8/T14; documented score/confidence defaults need feature work before these entries can be wired (see review below) |
| D | Open Excel (1) | T30 imports Excel into worksheets; Origin's item opens an Excel window. Reassign to B14 unless equivalent behaviour is established; do not relabel import as parity |
| E | Graph as Image, Multi-Page PDF without graph active (2) | T05/T32; either state-disable as Origin does or choose project graphs through Export Graphs |
| F | Standard/Explorer Duplicate (2) | Done: Standard toolbar uses the active window; Explorer window context item uses the clicked window; both reuse the existing duplicate command |
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

## Group C review (2026-11-15)

Both entries remain disabled and move to inventory package B8, with the existing T14
probability/Q-Q work as their dependency. Origin's [plot_prob reference](https://docs.originlab.com/x-function/ref/plot_prob/)
identifies the Plot menu entries and documents Blom scores and a checked 95% confidence band by
default. Its [plot guide](https://docs.originlab.com/origin-help/probplot-qqplot/)
also describes observed values on X and distribution quantiles on Y for Q-Q plots.

The existing Sheaf tools use Filliben positions, expose distribution/graph controls without a
score or confidence-band choice, and put theoretical Q-Q quantiles on X. A menu alias would
inherit those differences. Resolving them belongs to the T14 calculation/options work, beyond
this dispatch group. These are published-document checks, not new Origin 2016 measurements;
Statistics > Descriptive Statistics keeps its existing scipy-checked tools (statistics section 16).

## Group F result (2026-11-15)

The Standard Duplicate button now uses the Window > Duplicate command and its active-window
state. Explorer's window context item sends the clicked window's ID to the same existing app
operation, including when that window is hidden or another window is active. The copy is shown
and activated; one Undo removes it and Redo restores it. Workbook copies keep separate column
identities; graph copies retain their source-data references.

Four focused UI regressions cover empty projects/books, active versus Explorer-selected targets,
workbooks and graphs, hidden context targets, cancellation and Undo/Redo. The existing Explorer
context and Window > Duplicate UI tests also pass. Only the two dispatch hooks changed; no new
duplication engine or Origin numeric claim was introduced.

## Don't

Do not reorder measured menus, enable operations with different meaning, take another group's
claim, or introduce broad new features here. Coordinate edits to menus/toolbar hot spots.
