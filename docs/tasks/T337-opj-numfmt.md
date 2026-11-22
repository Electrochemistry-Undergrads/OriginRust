# T337: Numeric display formats in .opj files

**Status:** done 2026-11-21 (Claude Windows "claude:3"). The `.opj` writer warned "column(s) have a
display format ... that the project file written here does not hold". Measured in Origin 2016
(`re-workspace/scratch/t337`, local) by setting `wks.colN.format/subformat/digitMode/digits`: the column
record's `0x1E` holds the display (high nibble: Decimal, Scientific, Engineering, Decimal:1,000) and `0x1F`
the digits (places + 3, `0x80 | n` significant, 0 default); the column becomes Numeric (type word 0x6001,
8-byte cells). The reader now sets `Column::format` from these bytes and the writer writes X and Y columns
of numbers with a numeric display that way (others keep the warning); Origin read every tested display back
from a file Sheaf wrote.
**Size:** S. **Spec:** `docs/spec/opj-format.md` 4.3.

## Tests

- `crates/io/src/opj/write/feature_tests.rs`: `numeric_displays_are_written_where_origin_keeps_them_and_read_back`.
- `crates/io/src/opj/tests.rs`: `local_origin_file_gives_the_numeric_displays` (Origin's file, local).
