# T297: Measure the Gray Scale Map against Origin 2016

**Status:** done 2026-11-21 (Claude Windows "claude:fe4b", matrices 6 and 7g). Measured on the picture
Origin 2016 exports of its CONTGRAY template. The map is drawn as cells in 8 major steps of 6 bands. Its
32 grays follow `i = min(floor(32 k / (n - 1)), 31)` with level `(i << 3) | (i >> 2)`, exact for all 48
bands. Values outside the scale are blue above and red below. X tick labels are upright. Sheaf's
`grayscale` template and Gray palette now match this.
**Size:** S. **Follows:** T201. **Spec:** matrices 6 (Colours), 7g.

## Left

- Origin's colour scale title (Sheaf draws none for any template).
- The grays for 32 bands or fewer (the rule is assumed to hold).

## Tests

- `crates/plot/src/field.rs`: `band_colours_follow_the_palettes_measured_in_origin` (all 48 measured grays,
  out-of-range colours).
- `crates/plot/src/field_tests.rs`: `golden_gray_scale_map_of_a_matrix` (golden `matrix-grayscale.svg`).
