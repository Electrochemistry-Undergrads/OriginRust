# T436: LaTeX math in graph text, slice 1 (Origin 2023b built-in equation rendering)

**Status:** done 2026-11-24 (Claude Windows "claude:work", graphing "LaTeX math in graph text"). A `$...$` span in
any graph text (labels, axis titles, legends) is LaTeX math. It is turned into Sheaf's label markup (section 7) before
layout, so it draws with the label's font and exports as vectors: italic letters, `^`/`_` scripts (paired in one
column), Greek letters and symbols, `\frac` as a/b, `\sqrt`, accents, `\mathrm`/`\text`/`\mathbf` and upright
function names. A span is math only when it holds `\`, `^`, `_` or `{`, or starts with a letter and has no space at
either end, so `$5 and $10` and LabTalk `$(...)` are untouched. `\$` is a dollar sign. Sheaf's own design (Origin
2016 has no LaTeX).
**Size:** S. **Source:** origin-2026-2027-gaps.md, "Built-in equation rendering".

## Left

- Stacked fractions, operators with limits above and below, matrices, stretchy delimiters, display (block) equations,
  and LaTeX in worksheet metadata carried into legends and axis titles.

## Tests

- `crates/plot/src/latex.rs`: the translations, which spans are math, and a label through the markup parser
  (scripts scaled and raised, letters italic, prices untouched).
