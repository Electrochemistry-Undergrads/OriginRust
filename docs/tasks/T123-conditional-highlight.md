# T123: Conditional Formatting > Conditional Highlight...

**Status:** claimed 2026-11-17 (Claude Windows, scheduled session "cond-heatmap", branch `cond-highlight`).
**Size:** M. **Source:** greyed inventory package B7 (the last T35 leftover in that submenu).
**Spec:** `docs/spec/gui-look.md` "Conditional formatting" (a new subsection).

## Goal

Colour the cells of the selection that meet one condition, or two joined by And or Or.

## Origin facts (X-Function `wcolor`, docs.originlab.com/x-function/ref/wcolor and "Details of TreeNodes in Wcolor"; not measured)

- `trrule.cond1`/`cond2` codes: 0 equal to, 1 not equal to, 2 greater than, 3 greater than or equal
  to, 4 less than, 5 less than or equal to, 6 between, 7 text that contains, 8 custom (Date/Time
  columns say after, on or after, before, on or before). `val11` (default 0) and `val12` (default 1,
  Between's upper value); `text1`; `custom1` (a condition in `x`, e.g. `x == "Male"`).
- `join` 0 None (default), 1 And, 2 Or adds `cond2`. Default colours: Background 30067455 =
  RGB(255,202,202), Text Color 20132812 = RGB(204,51,51) (the Min Max maximum colours).
- `extend` colours the whole row; values may be statistics functions from a fly-out menu.

## Steps

1. Core: `Rule::Highlight` (new `condformat/highlight.rs`), evaluation, tests.
2. App: the window, menu item, UI tests; XLSX snapshot.
3. Spec subsection, inventory, index.

## Done when

The item opens the window and the cells meeting the condition(s) are coloured live, saved and
removable; extend-to-row, statistics values, text custom conditions and date pickers may be left
(stated in Status). fmt, strict Clippy and the full tests pass.
