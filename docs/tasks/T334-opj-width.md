# T334: Column widths in .opj files

**Status:** done 2026-11-21 (Claude Windows "claude:3"). The `.opj` writer warned "column(s) have their own
width". Measured in Origin 2016 (`re-workspace/scratch/t331`, local): the column record's `0x4A` holds the
width in tenths of a character (`wks.col2.width=14` stored 140; the default 67 reads as 7). The reader now
gives a column with another width its `Column::width` and the writer writes it, without the warning;
Origin read 14 and 20 back from a file Sheaf wrote. Hidden columns stay a warning: `wks.colN.hidden` did
not take in Origin 2016's script, so their bytes were not found.
**Size:** S. **Spec:** `docs/spec/opj-format.md` 4.3.

## Tests

- `crates/io/src/opj/write/feature_tests.rs`: `a_column_width_is_written_where_origin_keeps_it_and_read_back`,
  `hidden_columns_and_column_widths_are_named_in_the_warnings`.
