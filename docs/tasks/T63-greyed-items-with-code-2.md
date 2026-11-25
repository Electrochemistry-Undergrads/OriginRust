# T63: Existing-feature menu and toolbar wire-ups, round two

**Status:** open; F wired (2026-11-15), J wired (2026-11-15, Claude Windows, branch `t63-add-plot`), C reviewed and deferred to B8/T14; claim remaining groups separately. Group G wired 2026-11-15 (Object Edit Bring to Front/Send to Back reorder the selected drawn shape; branch `t63-g`; Origin's button state unmeasured). Groups A, B, D, E, H, I, K claimed 2026-11-15 (session "mac-multipdf", macOS, branch `greyed-2-abdehik`); that claim went stale, and groups H, A, B, I and K were taken over and wired 2026-11-19 (Claude Windows "t30-origin", branch `t63-fill`; workbook-model 13c, not measured). Recovered and refined 2026-11-15.
**Size:** M overall; each group S. **Branch:** `greyed-2-<group>`.
**Priority:** wave 3, parallel lane when files do not overlap wave 1/2 claims.
**Spec:** [command map](../spec/ui-command-map.md), [GUI](../spec/gui-look.md); T01's method.
**Inventory:** 31 initial candidate locations. Open Excel is the remaining location tagged
`T63` and is referred to B14. T182 wired group E on 2026-11-19 and measured its Origin 2016
behavior on 2026-11-24.
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
| A | Bar, Stacked Area, Box Chart, Histogram, Bubble toolbar (5) | Done 2026-11-19: the buttons run the Plot menu's `Cmd::Plot` templates (graphing "Step and spline line plots") |
| B | Horizontal Step, Vertical Step, Spline Connected (3) | Done 2026-11-19: `Template::StepHorizontal`, `StepVertical`, `SplineConnected` (the Line template with that connection); UI test in `ui_tests.rs` |
| C | Probability Plot, Q-Q Plot (2) | Reviewed: deferred to B8/T14; documented score/confidence defaults need feature work before these entries can be wired (see review below) |
| D | Open Excel (1) | T30 imports Excel into worksheets; Origin's item opens an Excel window. Reassign to B14 unless equivalent behaviour is established; do not relabel import as parity |
| E | Graph as Image, Multi-Page PDF without graph active (2) | Done 2026-11-19 (T182). Sheaf opens Export Graphs on every project graph. Origin 2016's workbook image and multipage PDF options open the active sheet. Measured 2026-11-24 (graphing 9.7a) |
| F | Standard/Explorer Duplicate (2) | Done: Standard toolbar uses the active window; Explorer window context item uses the clicked window; both reuse the existing duplicate command |
| G | Object Edit Bring to Front/Send to Back (2) | T24 shape ordering; only shape selection supported today, broader object order is B2 |
| H | Row Numbers/Uniform/Normal in menu and column context (6) | Done 2026-11-19: `Project::fill_quick` writes values (no formula) into every row of the sheet, one undo step; UI test `ui_tests/fill_pattern.rs` |
| I | X/Y Axis Tick Labels (2) | Done 2026-11-19: `PropsTab::XTickLabels`/`YTickLabels` open the axis dialog on that axis's Tick Labels tab; the axis right-click Tick Labels... does too |
| J | Add Plot to Layer: Line, Scatter, Line + Symbol, Column, Area (5) | Done: measured in Origin 2016 (each item opens Layer Contents for the active layer with that type on the plot type button); Sheaf opens its Layer Contents the same way, Area added to the button's list; UI tests `ui_tests/add_plot_to_layer.rs` |
| K | Column-header Frequency Count (1) | Done 2026-11-19: runs the same Frequency Counts window as Statistics > Descriptive Statistics > Frequency Counts... (Origin's `freqcounts` for both) |

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

**Follow-up 2026-11-19 (T151):** the options this review asked for (scores, a confidence band, data on
X) were added, and both Plot menu entries are live (statistics 16.2a, not measured).

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

Current audit (2026-11-16, codex/t63-group-f): both live dispatch hooks and all four
existing duplicate-entry regressions remain present. Those regressions pass in the
latest 4,369-test suite (eight ignored), with fmt/strict all-target Clippy and all
912 source/fixture/asset hashes unchanged. No duplicate implementation or new
Native claim is added. The inventory reports T63=20; the earlier count of 27
predated G/J wiring. All remaining T63 groups stay with their existing owners.

## Don't

Do not reorder measured menus, enable operations with different meaning, take another group's
claim, or introduce broad new features here. Coordinate edits to menus/toolbar hot spots.
