# T330: Group and Subject columns in .opj files

**Status:** done 2026-11-21 (Claude Windows "claude:3"). feature-parity "Categorical columns" (P1, gap
"the .opj file codes of Group and Subject are unknown"). Measured in Origin 2016 (`re-workspace/scratch/t330`,
local): LabTalk `wks.col.type` 8 is Group and 9 is Subject; the column record stores them as role 7 and 8
(the type minus one), with the Y type word 0x6121, belonging to the X column. The `.opj` reader now reads
7 and 8 as Group and Subject and the writer writes them, instead of storing "unused" with a warning;
Origin 2016 read both roles back from a file Sheaf wrote.
**Size:** S. **Spec:** `docs/spec/opj-format.md` 4.3.

## Tests

- `crates/io/src/opj/write/feature_tests.rs`: `group_and_subject_columns_keep_their_roles_in_the_opj`.
- `crates/io/src/opj/write/tests.rs`: `the_roles_of_columns_map_to_the_codes_origin_uses`.
