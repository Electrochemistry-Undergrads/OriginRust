# T231: Add Sparklines on import (parity "Sparklines, auto width, write protection on import")

**Status:** done 2026-11-20 (Claude Windows "claude:1"), **not measured** beyond the list of the impASC
window. Import Options > Add Sparklines (`No`, `Yes`, `Yes(if less than 50 columns)`) marks the imported
columns for sparklines and turns the sheet's Sparklines row on; in the import window, dialog themes and
`sheaf import --sparklines no|yes|auto`. Sheaf's default is No (Origin's is the third) because the `.opj`
writer does not keep sparklines yet.
**Size:** S. **Source:** feature-parity "Sparklines, auto width, write protection on import".
**Spec:** `docs/spec/import-ascii.md` 10 ("Add Sparklines").

## Tests

- `crates/io/src/ascii.rs`: default No; Yes and the 50-column rule (49 yes, 50 no).
- `crates/io/src/batch.rs`: Start New Columns, Start New Rows and Replace turn the target sheet's row on.
- `crates/cli/tests/import_text_options.rs`: `--sparklines yes`.
- `crates/app/src/app/ui_tests/import_themes.rs`: the list in the import window.

## Left

- Origin's default once `.opj` keeps sparklines; auto column width (`nAutoSize`); write protection
  (`DataWriteProtection`); the Excel and CH Instruments importers.
