# T360: Repeat Last Analysis

**Status:** done 2026-11-22 (Claude Windows "claude:gmail", gui-look "Repeat Last Analysis"). HANDOFF C (Origin 2026
"Repeat Last Action"): Analysis > Repeat Last Analysis runs the last tool run from its window (OK or Apply), with the
same options, on the columns now chosen (or the active plot of a graph, or the sheet's Y columns), without opening the
window, as one undo step. Not in Origin 2016, so the rules are Sheaf's own.
**Size:** S. **Spec:** gui-look, Repeat Last Analysis.

## Left

- Repeating plots and fits, Duplicate Operation onto other sheets or books, a keyboard shortcut.

## Tests

- `crates/app/src/app/ui_tests.rs`: `repeat_last_analysis_runs_the_last_tool_on_the_new_selection` (nothing before a
  run; Integrate repeated on another column, undo label).
