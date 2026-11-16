# T82: Line Series (B4 slice)

**Status:** in progress 2026-11-16 (branch `line-series`, Claude Windows, session fe4b37eb).
**Size:** S-M. **Inventory:** Plot > Line + Symbol > Line Series (1).

## Origin facts (measured 2026-11-16, Origin 2016, scratch `re-workspace/scratch/t82`)

Plot > Line + Symbol > Line Series makes the graph at once (no dialog). Each worksheet row of the
selected Y columns becomes one line: its values stand at X = 1, 2, ..., k (k = number of selected
columns, in column order) and are joined; the rows are not joined to each other. It is **one plot**
whose line connection is the k-point segment (`get %C -l` gave 2 with two columns, 3 with three),
filled black squares of size 8 (all rows the same colour), the X axis labelled with the column names
at 1..k (B, C, D) and titled "Series", the Y axis titled "Values"; X range 0.75..2.25 for two columns
and 0.5..3.5 for three. The window shows Origin's lock icon (the plotted data is made by the command);
the workbook keeps its one sheet and its columns (Origin holds the rearranged data out of sight).

## Sheaf

A new sheet `LineSeries` in the source workbook (Sheaf choice: Origin keeps the data hidden) with the
positions (row-major), the values (row-major) and the column names, one Line + Symbol plot with a
k-point segment connection, the column names as X tick labels, titles Series and Values.
