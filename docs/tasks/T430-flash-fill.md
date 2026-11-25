# T430: Column > Flash Fill (Origin 2026 Flash Fill and Smart Formula)

**Status:** done 2026-11-24 (Claude Windows "claude:work", workbook-model.md 13r, expression-language.md `token`).
Type what a column should hold in one or more of its cells, then use Column > Flash Fill.... Sheaf finds one rule that
builds every example from the columns to the left: fixed text, a whole cell, a word (counted from the start or the
end) or its first letter, each as typed or in upper or lower case. The Flash Fill window shows the rule as a formula
and a preview of the first rows. Fill Values writes the texts; Keep Formula sets the column formula (recalculated
automatically), so the column follows its sources. Either is one undo step. Formulas use the new expression function
`token(s, n)`. Sheaf's own design (Origin 2016 has no Flash Fill).
**Size:** M. **Source:** origin-2026-2027-gaps.md, "Flash Fill and Smart Formula".

## Left

- Numbers and dates as typed values (only texts are matched), substrings that are not whole words, conditional
  rules, filling as you type (Origin's suggestion while typing), and choosing the source columns.

## Tests

- `crates/expr/src/tests.rs`: `token_takes_the_nth_word_from_either_end`.
- `crates/core/src/flash_fill.rs`: names turned round with an initial, two columns joined with case, dates
  rearranged, a second example settling an ambiguity, no rule, the column filled and the formula giving the same texts.
- `crates/app/src/app/ui_tests/function_graph.rs`: `flash_fill_fills_values_or_keeps_the_formula`.
