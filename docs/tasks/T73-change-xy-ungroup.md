# T73: Change X, Change Y and Ungroup on a plot (B3, slice 3)

**Status:** in progress 2026-11-15 (branch `change-xy`, Claude Windows, session fe4b37eb).
**Size:** S. **Inventory:** Graph plot (right-click): Change X · Change Y · Ungroup (3).

## Origin facts (measured 2026-11-15, Origin 2016, scratch `re-workspace/scratch/t73`)

- The plot right-click menu: Change Plot to >, Change X >, Change Y >, Hide Data Plot, Go to Book1,
  Ungroup, Mask >, Copy Format, Plot Details..., (more under the chevron).
- **Change X / Change Y** list the columns of the plot's sheet as `[Book1]Sheet1!A(X)`,
  `[Book1]Sheet1!B(Y)`, ... in column order. The plot's current column is ticked; the column it uses on
  the other axis is greyed. Choosing a column replaces the plot's X (Y) column: the legend entry and the
  axis title follow the new column (B became C), and the axes are **not** rescaled (Y stayed -25..125).
- On a plot of a group (two Y columns plotted together), **Change X and Change Y are greyed** and
  **Ungroup** is live; after Ungroup every plot keeps the colour it had (black, red), the legend stays,
  Change X/Y become live and Ungroup greys. On a plot that is in no group, Ungroup is greyed.

## Where

`crates/app/src/menus.rs` plot context menu (`grey`/`greys` for "Change X", "Change Y", "Ungroup");
`Plot::x`, `Plot::y` and the plot group field in `crates/core/src/graph.rs`; one undo step each; UI
tests; graphing spec (plot menu).
