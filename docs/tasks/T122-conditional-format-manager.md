# T122: Conditional Formatting > Conditional Format Manager...

**Status:** done 2026-11-17 (Claude Windows, scheduled session "cond-heatmap", branch `cond-manager`; gui-look.md "Conditional Format Manager (T122)"), not measured. Left: changing a format's range (reselect or type), dragging rows to reorder.
**Size:** M. **Source:** greyed inventory package B7 (T35 leftover, after T120 and T121).
**Spec:** `docs/spec/gui-look.md` "Conditional formatting" (a new subsection).

## Goal

A window that lists the conditional formats of the active worksheet, top rule first, where each
can be renamed, its rule edited in the window that made it, moved up or down, or removed.

## Origin facts (docs.originlab.com/origin-help/conditional-format, not measured: Origin 2016 has none)

- Worksheet: Conditional Formatting: Conditional Format Manager lists all conditional formats in the
  active worksheet. **Name**: double-click to rename. **Range**: reselect or type a new range.
  **Rule**: an Edit Rule button reopens the rule's dialog. **Move Up**/**Move Down** (or drag a
  row) reorder; **Remove** deletes the selected format. A new format goes to the top; the upper
  format wins where ranges overlap.

## Steps

1. App: new `dialogs/cond_manager.rs` (list, rename, Edit Rule, Move Up/Down, Remove), edit mode
   of `dialogs/cond_format.rs` that replaces the rule in place; menu item and action; UI tests.
2. Spec subsection, inventory, index.

## Done when

The item opens the window; rename, edit, reorder and remove each work with undo; editing a range
by typing or reselecting is left (stated in Status) unless it fits. fmt, strict Clippy and the full
tests pass.
