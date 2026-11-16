# T79: Column Scatter (B4 slice)

**Status:** done 2026-11-16 (branch `column-scatter`, Claude Windows, session fe4b37eb; graphing 18.14). Left: the plot-type pickers of Plot Setup / Layer Contents / Change Plot to do not offer it (as for Box).
**Size:** S-M. **Inventory:** Plot > Symbol > Column Scatter (1).

## Origin facts (measured 2026-11-16, Origin 2016, scratch `re-workspace/scratch/t79`)

With Y columns B and C selected (9 rows; B = x², C = 3x + 20), Plot > Symbol > Column Scatter makes one
layer where every Y column is a scatter of its values standing at its position 1, 2, ... in plot order:
no line (`get %C -l` 0), filled diamonds (`get %C -k` 5) of size 8, coloured by the plot group (black,
red), the points of a column spread sideways by a small jitter (within about +-0.15 of the position)
so equal values do not hide each other. The X axis runs from 0.5 to n + 0.5 with one tick per column,
labelled with the column names (B, C); Y as an automatic range (-30..110 here); the Y title "Range";
the legend lists B and C.

## Where

A new `PlotKind::ColumnScatter` placed like `PlotKind::Box`/`Violin` (crates/core/src/graph.rs; drawing
and X axis labels in crates/plot where box/violin positions and labels are made), a deterministic jitter
(e.g. `compplot::jittered`'s golden-ratio sequence, a Sheaf choice: Origin's jitter rule was not measured),
template `column-scatter`, menu item, CLI name, `.opj` writer warning or mapping, tests, graphing spec.
