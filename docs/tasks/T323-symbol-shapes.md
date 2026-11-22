# T323: Origin 2016's symbol shapes, interiors and fill colour

**Status:** done 2026-11-21 (Claude Windows "claude:b284", graphing 5.4). The parity row "Symbols" said Sheaf had 8
shapes and filled or open while Origin has 20 shapes and 12 interiors and a separate fill colour. Origin 2016 was
measured (`tools/origin-ref/symbols.lines.txt`, exported PNGs, and a project Origin saved): shapes 1-20 except 14
(right arrow, which drew nothing), interiors 0-11, Open filled white and Hollow not, x centre reaching the corners,
halves in the edge colour, `set -cf` without visible effect, and the .opj curve bytes 0x17 (shape) and 0x18 (interior).
Sheaf now draws the 19 shapes (the text shapes write the row number or letter; the legend shows "1" or "a" as Origin
does), the 12 interiors and a fill colour for the open part; Plot Details > Symbol lists them; .opj files read and
write both bytes.
**Size:** M. **Spec:** graphing 5.4, opj-format 10.5.

## Left

- Fill colours other than white are not measured (`set -cf` changed nothing); the sphere's gradient is approximated by
  two lighter discs.
- The Symbol Type increment steps through the first 8 shapes (Origin's default list has 16); the interior increment
  still toggles solid and open.
- Symbol size from a column with a formula, user bitmap symbols.

## Tests

- `crates/plot/src/symbol_shape_tests.rs`: open/hollow/fill colour, halves, centre marks, new outlines, sphere, text shapes and legend.
- `crates/core/src/graph.rs`: `origin_numbers_name_every_shape_and_interior_once`,
  `a_symbol_saved_before_interiors_reads_as_it_was_and_new_fields_are_written_only_when_set`, increments keep a shape
  beyond the list (`group_increments_run_from_the_first_plot_and_dependent_edits_reach_every_plot`).
- `crates/io/src/opj/write/graph_tests.rs`: `every_origin_symbol_shape_and_interior_comes_back`.
- `crates/app/src/app/ui_tests.rs`: `plot_details_symbol_tab_offers_origins_shapes_interiors_and_fill_colour`.
