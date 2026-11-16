# T71: 2 and 3 Point Segment plots (B4, slice 2)

**Status:** done 2026-11-15 (branch `b4-segments`, Claude Windows, session fe4b37eb; graphing 5.2).
**Size:** S. **Inventory:** Plot > Line + Symbol > 2 Point Segment, 3 Point Segment (2).

## Origin facts (measured 2026-11-15, Origin 2016, scratch `re-workspace/scratch/t71`)

Plot > Line + Symbol has Line + Symbol, Line Series, 2 Point Segment and 3 Point Segment. The segment
items make a line + symbol graph (square symbols, size 9) whose plot has line connection 2 (`get %C -l`
gave 2) or 3: points are joined in pairs or threes from the first, the groups not. Plot > Symbol >
Scatter Central draws the scatter with both axes crossing in the middle of the boxed frame; it needs an
X axis placed inside the frame, which Sheaf's axis offset (T61, Y only) does not do yet, so it stays
`B4`, as does Line Series.

## Built

`Connect::Segment2`/`Segment3` (core), the drawing (`plot/src/plots.rs`), templates `segment2` and
`segment3` (menu items and `sheaf plot -t segment2`), the connect lists of the plot settings windows,
`.opj` read/write codes 2 and 3; tests for the paths, the templates, the `.opj` round trip and the menu.
