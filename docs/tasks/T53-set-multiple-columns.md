# T53: Set Multiple Columns Values (Ctrl+Shift+Q)

**Status:** claimed 2026-11-15 (branch `set-multiple-columns`, Claude macOS "mac-multicol"). Brief written from
T09's open item, parity row "Set Column Values dialog extras" (Set Multiple Columns Values missing) and
inventory MATH-MULTI-01.
**Size:** M. **Branch:** `set-multiple-columns`. **Model:** Sonnet. No Origin needed for the mechanism; the
window layout and the column-index variable are measured once on Windows.
**Spec:** `docs/spec/expression-language.md` (the `j` variable) and `docs/spec/gui-look.md` (Set Values).

## Goal

With several columns selected, Column > Set Multiple Columns Values... (Ctrl+Shift+Q, both menus that list
it) opens the Set Values window for all of them at once: one formula, applied to each selected column in
turn, where `j` is the 1-based position of the column being set (so `wcol(j-1)*2` doubles the column to its
left, `col(A)*j` scales by position). Recalculate (None, Manual, Auto) and the row range apply to every
column; the whole operation is one undo step.

## Origin facts

- Not measured. From Origin's public documentation (paraphrased): Set Multiple Columns Values uses the Set
  Values window for the selected columns; `i` is the row index and `j` the column index; the formula is
  stored on each column. Measure on Windows with `tools/origin-ref`: the title, whether `j` is the sheet
  position or the position in the selection, what each column's stored formula looks like afterwards (the
  literal text with `j`, or `j` substituted), how Auto recalculation treats `wcol(j-1)` chains, and
  whether a selection that includes a locked or text column is refused. Until then Sheaf's choices,
  labelled: `j` is the sheet position; the formula text with `j` is stored on each column and evaluated
  with that column's `j`; a dependency on `wcol(j-1)` is resolved per column for Auto recalculation.

## Where

- `crates/expr`: a `j` variable (column position) provided by the host like `i`; check that `j` does not
  clash with an existing name rule (a column named J? Origin's precedence) and document the rule.
- `crates/core/src/formula.rs` (`SheetHost`): supply `j` for the column being evaluated; dependency
  extraction (`cell_reads`/column reads) must resolve `wcol(j±k)` per column so Auto recalculation and the
  circular-chain check work (a chain `wcol(j-1)` over columns B..E is fine; a column reading itself is a
  cycle).
- `crates/app/src/dialogs/set_values.rs`: a multi-column mode (title names the columns, the column
  navigation buttons step through the selection or are hidden); `menus.rs`: enable the two greyed
  "Set Multiple Columns Values..." items (Ctrl+Shift+Q) when two or more columns are selected.
- One `Document::edit` step for the whole set.

## Steps

1. Spec text (Sheaf's choices labelled; Origin measurements listed).
2. `j` in expr and the sheet host, per-column dependency resolution, with tests (values, Auto chains,
   cycle detection, single-column Set Values unaffected, a column literally named `j`).
3. Dialog mode and menus; UI tests: Ctrl+Shift+Q with B:D selected sets all three with `col(A)*j`; one
   undo step; Auto recalculation follows a change in A; the single-column window unchanged.
4. Parity row note, T09 Status.

## Done when

Selecting several columns and setting one formula with `j` fills each column correctly, stores the
formula on each, recalculates per the chosen mode, undoes in one step; fmt, clippy and tests pass.

## Don't

- Don't add a script runtime ("Before Formula Scripts" stays read-only).
- Don't change how single-column Set Column Values stores or evaluates formulas.
