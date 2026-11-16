# T68: Vertical Drop Line and Column + Label plots (B4 slice 1)

**Status:** done 2026-11-15 (Claude Windows, branch `plot-types-b4`; graphing 18.12). Left: the rest of B4
in PLAN order (Grouped Columns (indexed data), Grouped Box Charts, Interval Plot, ...); the X axis and frame of
the Column template itself still differ from Origin's ColumnLabel export (Sheaf draws the top/right frame lines
and ticks every 2 here; Origin ticks every category, range 0.5..6.5) — that is the Column template, not this slice.
**Size:** S. **Branch:** `plot-types-b4`.

**Goal:** enable Plot > Symbol > Vertical Drop Line and Plot > Column/Bar/Pie > Column + Label as Origin 2016's
`DROPLINE` and `ColumnLabel` templates.

**Origin facts** (measured 2026-11-15, `re-workspace/scratch/t68`, `plotxy ... ogl:=<new template:=...>`,
1600 px PNG exports at 2.07 px per point):
- DROPLINE: scatter symbols with a vertical drop line from each point to the bottom of the frame (to the
  axis, not to zero: the axis ran 1..7 for data 1.5..6), 1 px (0.5 pt), in each plot's colour; the legend shows
  the symbol only. Two plots: black square, red circle (the plain scatter, `plot:=201`, gave squares for both).
- ColumnLabel: the column plot (red, then green) with each value written above its own column in Arial
  22 pt (digit 33 px high against 27 px for the 18 pt tick labels), about 5.8 pt above the top; with two
  columns per category each label is centred on its own bar.

**Where:** `Template::VerticalDropLine`, `Template::ColumnLabel` (`crates/core/src/template.rs`, CLI names
`vertical-drop-line`, `column-label`); per-plot settings in `Project::plot_columns`; label centring in
`draw_data_labels` (`crates/plot/src/plots.rs`); Plot menu items in `crates/app/src/menus.rs`.

**Steps:** templates and settings; label centring on grouped bars; menu items; tests; spec, inventory.

**Done when:** both menu items make Origin's graph; fmt, strict Clippy and the workspace tests pass.

**Don't:** touch Line Series / 2- and 3-Point Segment or the step/spline items (T63 B).
