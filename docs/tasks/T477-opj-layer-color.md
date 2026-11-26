# T477: Layer background colour in .opj files

**Status:** done 2026-11-26 (Claude Windows "claude:3", opj-format.md 10.2, measured in Origin 2016).
**Size:** S. **Source:** the writer's note "a layer background colour was not written (the companion file keeps it)".

## What it does

A graph layer's background colour is read from and written to `.opj` files: Origin keeps 2 at `0xA7` of the layer header for a filled
layer, the nearest position in its colour list at `0xA8`, and the colour code (list position or RGB) at `0x105`. The gradient and border
of a layer background are still named in a warning.

## Measured

`re-workspace/scratch/t477`: `layer.color=color(yellow)`, `color(red)` and `color(255,128,0)` against a plain graph (`p1.opj`, `p2.opj`).
A file Sheaf wrote (ignored test `write_layer_color_project_for_origin`) gave `layer.color` 2 and 30769937 (the RGB code of 17, 131, 213)
in Origin.

## Tests

- `crates/io/src/opj/write/graph_feature_tests.rs`: `a_layer_background_colour_is_written_where_origin_keeps_it`,
  `plot_decorations_are_named_in_the_save_warnings_and_a_layer_background_is_written`.
- `crates/app/src/app/ui_tests/layer_management.rs`: `display_background_rgb_renders_and_survives_native_and_companion_saves` (no warning
  for the `.opj` now).
