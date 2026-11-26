# T478: Legend background colour in .opj files

**Status:** done 2026-11-26 (Claude Windows "claude:3", opj-format.md 10.8, measured in Origin 2016).
**Size:** S. **Source:** the writer's note "the legend background colour was not written"; opj-format 10.8.

## What it does

A legend's background colour is read from and written to `.opj` files: the colour code at `0x1A` of the legend's style block
(`FC FF FF FF` for none), with or without the frame. Origin's own white legends now open with a white background.

## Measured

`re-workspace/scratch/t477/lg.opj`: `legend.fillcolor=color(yellow)` against a plain legend changed only `0x1A`. A file Sheaf wrote
(ignored test `write_legend_fill_project_for_origin`) gave `legend.fillcolor` 7, 30769937 and -4 in Origin, and Origin drew the
three legends as written (`png478`).

## Tests

- `crates/io/src/opj/write/graph_tests.rs`: `a_legend_background_colour_is_written_and_read_back`, `what_the_file_cannot_hold_is_named`
  (the background now reads back).
