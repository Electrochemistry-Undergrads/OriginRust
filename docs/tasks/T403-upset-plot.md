# T403: UpSet plots

**Status:** done 2026-11-23 (Claude Windows "claude:1", graphing 14.35), not measured (Origin 2027 feature).
Plot > Categorical > UpSet Plot and `sheaf plot -t upset [-y SET ...]`: membership columns (0/1, or text flags) as
sets; exclusive intersection sizes as bars over a dot matrix, set sizes as bars beside the set names.
`PlotKind::UpSet`, `sheaf_calc::upset`, `sheaf_core::project::flag_columns`.
**Size:** S. **Source:** HANDOFF C, origin-2026-2027-gaps "Hexbin, binned scatter and UpSet plots".

## Tests

- `crates/calc/src/upset.rs`: exclusive intersections and their order, short rows, the 40-intersection limit.
- `crates/plot/src/upset.rs`: names, sizes, counts and dots in the SVG; numeric and text flags.
- `crates/cli/tests/sankey.rs`: `an_upset_plot_takes_the_flag_columns_or_the_named_ones`.
- `crates/app/src/app/ui_tests.rs`: `plot_categorical_upset_plot_uses_the_selected_columns_or_the_flag_columns`.

## Left

- List-style input, sort and limit choices, colours by set or degree, axes for the bar charts, Plot Details.
