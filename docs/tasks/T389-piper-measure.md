# T389: Measure the Piper diagram against Origin 2016

**Status:** done 2026-11-23 (Claude Windows "claude:work", graphing 39). Origin 2016's `plotpiper` takes the six
columns in Sheaf's order (Ca, Mg, Na+K, Cl, SO4, CO3+HCO3), places the corners as Sheaf does, and draws gray grids
every 20 % with dotted ones between. Each side is labelled 0 to 100 toward its end corner and titled with the
column names ("D+E", "A+B" on the diamond), and each sample gets the next symbol and colour. Sheaf now draws the
same grids, labels, titles and sample symbols, and drops its corner labels.
**Size:** S. **Follows:** T117. **Spec:** graphing 39.

## Left

- Sample ID and Total Dissolved Solids columns; a legend entry per sample; Origin's three separate layers.

## Tests

- `crates/plot/src/piper_plot.rs`: `each_sample_is_drawn_three_times_on_equal_x_and_y_units` (colours, labels, titles).
- `crates/core/src/piper.rs`: `the_plot_takes_six_columns_in_order`.
