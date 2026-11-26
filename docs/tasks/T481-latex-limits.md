# T481: Large operators with limits in LaTeX graph text

**Status:** done 2026-11-26 (Claude Windows "claude:3", graphing "LaTeX math in graph text"). Rest of T436/T480; Sheaf's own layout
(Origin 2016 has no LaTeX).
**Size:** S. **Source:** origin-2026-2027-gaps.md, "Built-in equation rendering": limits remain.

## What it does

`\sum`, `\prod`, `\coprod`, `\bigcup`, `\bigcap`, `\bigoplus`, `\bigotimes` (at 140 %) and `\lim`, `\max`, `\min`, `\sup`, `\inf`
draw their scripts below and above them, centred over one column, through the markup escape `\lm(op,below,above)`. `\int` keeps
its scripts beside it.

## Tests

- `crates/plot/src/text_fraction_tests.rs`: `a_large_operator_takes_its_limits_below_and_above_centred`.
- `crates/plot/src/latex.rs`: the `\sum`, `\lim` and `\int` translations.
