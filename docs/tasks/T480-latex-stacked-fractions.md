# T480: Stacked fractions in LaTeX graph text

**Status:** done 2026-11-26 (Claude Windows "claude:3", graphing "LaTeX math in graph text"). Rest of T436; Origin 2023b's built-in
equation rendering, which Origin 2016 lacks, so the layout is Sheaf's own.
**Size:** S. **Source:** origin-2026-2027-gaps.md, "Built-in equation rendering": stacked fractions remain.

## What it does

`\frac{a}{b}` (`\dfrac`, `\tfrac`) in a `$...$` span draws as a stacked fraction: the label markup gains `\fr(num,den)` (a
`Fraction` layout segment) with both parts at 70 % size centred over a bar; fractions nest and work in rotated axis titles. Plain-text
readers of the runs see `a/b`.

## Tests

- `crates/plot/src/text_fraction_tests.rs`: `a_fraction_stacks_its_parts_centred_over_a_bar`,
  `latex_frac_draws_stacked_and_reads_as_a_slash_in_plain_text`.
- `crates/plot/src/latex.rs`: the `\frac` translations (a comma inside a part).
