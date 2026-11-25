# T410: Graph > Browser Graph (Origin 2020b, browsing across sheets as in 2024)

**Status:** done 2026-11-24 (Claude Windows "claude:work", graphing "Browser Graph"). Graph > Browser Graph... makes
one plot of the active layer step through similarly structured datasets: the Y columns of its sheet, or the column in
the same place in every sheet of its workbook. The Previous and Next buttons of a bar above the graph show the other
datasets, with the layer rescaled to each. A run of steps makes one undo step, and the browser saves with the project.
Remove Browser leaves the plot on the dataset it shows. Sheaf's own design (Origin 2016 has no browser graph).
**Size:** S. **Source:** origin-2026-2027-gaps.md, "Browser Graph".

## Left

- Several browsed plots per graph, browsing across workbooks, a list to pick a dataset from, the browser in .opj
  files and in the CLI/MCP. (Keyboard stepping: T450.)

## Tests

- `crates/core/src/browser.rs`: the Y columns of the sheet (text columns left out, X and Y error from the sheet),
  the same column of every sheet, a lone column refused, rescaling, save round trip, removal.
- `crates/app/src/app/ui_tests/function_graph.rs`: `browser_graph_steps_through_the_columns_from_the_bar`.
