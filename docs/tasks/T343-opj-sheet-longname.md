# T343: Sheet long names in .opj files

**Status:** done 2026-11-21 (Claude Windows "claude:3"). The `.opj` writer warned "sheet(s) have a long
name". Measured in Origin 2016 (`re-workspace/scratch/t339`, local): `wks.longname$` is stored as a
worksheet-layer annotation named `__LONGNAME`, laid out exactly as the column-formula annotations the
writer already wrote (header, text in the first block, two empty blocks). The reader now sets
`Worksheet::long_name` from it and the writer writes it (through `tail::formula_annotation`); Origin read
the long name back from a file Sheaf wrote. A tab that shows the long name (Sheet Name to Show) is still
named in the warnings.
**Size:** S. **Spec:** `docs/spec/opj-format.md` 4.5.

## Tests

- `crates/io/src/opj/write/feature_tests.rs`: `a_sheet_long_name_is_written_as_origins_longname_annotation`.
- `crates/io/src/opj/tests.rs`: `local_origin_file_gives_the_sheet_long_name` (Origin's file, local).
