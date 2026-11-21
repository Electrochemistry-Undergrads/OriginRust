# T313: Measure the Scatter Matrix against Origin 2016

**Status:** done 2026-11-21 (Claude Windows "claude:fe4b", graphing 35). Origin's `plot_matrix` window and its
graphs of 2, 3 and 4 variables were measured. They use the usual landscape page with the grid 8 % from the left
and 5 % from the top, cells 87/n % with no gap. Tick labels sit on every outer side except in the diagonal cells
(Show Tick and Label: All), and the bottom row and left column carry the names as axis titles. Symbols are
blue squares of size 9, and the tick step is a quarter of the range to two significant digits. Sheaf's square
matrix now does the same.
**Size:** S. **Follows:** T114. **Spec:** graphing 35.

## Left

- Origin's axis limits (examples recorded in graphing 35; the rule was not found).
- Origin's triangular layouts, box charts on the diagonal, the other Show Tick and Label modes, the
  confidence ellipse, the linear fit and its statistics.

## Tests

- `crates/core/src/scatter_matrix/tests.rs`: the page, the frames, the tick labels and titles of every cell,
  the symbols, the tick step.
