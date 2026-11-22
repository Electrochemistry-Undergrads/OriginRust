# T324: Measure Wind Rose - Raw Data against Origin 2016

**Status:** done 2026-11-21 (Claude Windows "claude:fe4b", graphing 36). Origin 2016's `plot_windrose` (Plot >
Specialized > Wind Rose - Raw Data) was measured on three sets of speeds. It uses 16 sectors centred on north and
counts, not percent. Its speed step is the 1-2-5 value nearest a fifth of the top speed, its bands run up to the
first multiple above it with an open ">= top" band, and its colours are black, yellow, red, blue, green, cyan.
Sheaf's Raw Data now does the same.
**Size:** S. **Follows:** T113. **Spec:** graphing 36.

## Left

- Origin's WindRose1 output sheet, the Calm count and its note, compass labels on the circle, the legend with the
  fastest band first, the options window. Binned Data: measured by T327.

## Tests

- `crates/core/src/windrose.rs`: the steps, the bands and the colours.
- `crates/plot/src/wind_rose_tests.rs`: `raw_data_piles_the_counts_of_the_bands_in_each_sector`.
