# T475: Date tick formats of graph axes in .opj files

**Status:** done 2026-11-26 (Claude Windows "claude:3", opj-format.md 13.14, checked in Origin 2016).
**Size:** S. **Source:** feature-parity "Save project as .opj" gap "date tick formats other than month/day/year"; rest of T474.

## What it does

An axis with date tick labels keeps its date pattern in `.opj` files: Origin's tick format numbers (`layer.x.label.numFormat`, the low
nibble of byte `0x26`) are the Date display list, so the writer writes the ten Sheaf patterns that list has within 4 bits and the reader
maps every number back (a date-and-time format gives its date part). Other patterns are written as month/day/year with the warning as
before.

## Checked

`sheaf-date-ticks.opj` (ignored test `write_date_ticks_project_for_origin`): Origin reported label type 4 (date) with `numFormat` 5 and 12
for the day and `yymmdd` axes.

## Tests

- `crates/io/src/opj/write/graph_tests.rs`: `date_tick_formats_of_origins_list_are_written_and_read_back`.
