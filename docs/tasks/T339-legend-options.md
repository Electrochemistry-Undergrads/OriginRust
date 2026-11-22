# T339: Legend `%(n, @option)` substitutions

**Status:** done 2026-11-21 (Claude Windows "claude:b284", graphing 6.1). The parity row "Legend" said the `%(n)`
options were ignored and the name was always the plot's label. Origin 2016 was asked on a graph window
(`string s$ = "%(1,@LL)"`, 22 options on a column with long name, units and comment and on one without): its answers
are the axis-title answers of T281 plus `@LM` (comment, else long name, else short name) and `@R` (the plotted range,
`[Book1]Sheet1!Col("Volt")[1:5]`); unknown options give `###`. Legends and free labels with plot references now
draw `%(n, option)` that way (`layout::legend_name`); `%(n)` alone is unchanged (it already matched Origin).
**Size:** S. **Spec:** graphing 6.1.

## Left

- The combined form `%(1,@(@LL(@LU)))`, `@LD n` user parameters, `@GF`, point-by-point `@V`/`@WT`.

## Tests

- `crates/plot/src/legend_option_tests.rs`: `legend_options_give_what_origin_2016_gives` (21 options on both columns, a drawn legend line).
- `crates/plot/src/legend.rs`: the parser keeps the option (`%(2.1,@LG)`).
