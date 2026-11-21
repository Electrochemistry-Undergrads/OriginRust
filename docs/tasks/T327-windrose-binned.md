# T327: Measure Wind Rose - Binned Data against Origin 2016

**Status:** done 2026-11-21 (Claude Windows "claude:fe4b", graphing 36). Origin 2016's Plot > Specialized > Wind Rose
- Binned Data opens no window. Eight directions 45 degrees apart with three frequency columns gave 22.5 degree
wedges piled in column order and coloured black, yellow, red, as Raw Data's bands. Sheaf's Binned Data wedges are
now 22.5 degrees wide (360 / rows above 16 rows).
**Size:** S. **Follows:** T113, T324. **Spec:** graphing 36.

## Left

- Compass labels on the circle and the legend with the last column first (as for Raw Data, T324).

## Tests

- `crates/plot/src/wind_rose_tests.rs`: `binned_data_wedges_are_a_sixteenth_of_the_circle_as_origin`.
