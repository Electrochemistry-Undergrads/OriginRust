# T482: Stretchy delimiters in LaTeX graph text

**Status:** done 2026-11-26 (Claude Windows "claude:3", graphing "LaTeX math in graph text"). Rest of T436/T480/T481; Sheaf's own
layout (Origin 2016 has no LaTeX).
**Size:** S. **Source:** origin-2026-2027-gaps.md, "Built-in equation rendering".

## What it does

`\left( ... \right)` (also `[ ]`, `\{ \}`, `|`, `\langle \rangle`, `.` for none) draws its delimiters as lines as tall as the ink
of what they enclose, through the markup escape `\dl(lr,body)`; groups nest (brackets inside braces around a fraction).

## Tests

- `crates/plot/src/text_fraction_tests.rs`: `stretchy_delimiters_span_what_they_enclose`.
- `crates/plot/src/latex.rs`: the `\left`/`\right` translations (kinds, a comma inside, an unclosed group).
