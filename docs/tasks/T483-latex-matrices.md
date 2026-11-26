# T483: Matrices in LaTeX graph text

**Status:** done 2026-11-26 (Claude Windows "claude:3", graphing "LaTeX math in graph text"). Rest of T436/T480-T482; Sheaf's own
layout (Origin 2016 has no LaTeX).
**Size:** S. **Source:** origin-2026-2027-gaps.md, "Built-in equation rendering": matrices remain.

## What it does

`\begin{pmatrix} a & b \\ c & d \end{pmatrix}` (and `matrix`, `bmatrix`, `Bmatrix`, `vmatrix`) draws a grid of centred cells inside the
environment's stretchy delimiters, through the markup escape `\mx(cols,cells...)`. Rows are spaced by their ink (a fraction in a cell
pushes the next row away) and the block is centred about the fraction bar's height.

## Left

- Axis titles placed by their font size could reach towards the tick labels: done in T484.

## Tests

- `crates/plot/src/text_fraction_tests.rs`: `a_matrix_lays_its_cells_out_in_centred_columns_between_its_delimiters`.
- `crates/plot/src/latex.rs`: the environment translations (delimiters, a trailing row break, a short row).
