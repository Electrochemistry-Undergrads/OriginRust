# T19: Import dialogs and Column Properties as Origin

**Status:** item 1 (ASCII import) done (merged 2026-11-13): multi-character and fixed-width delimiters, Data Import Options (rename columns, short-name line, rename sheet and book, apply the new range to plots), Origin's mode names, Apply button; added: Other as a set of characters with optional consecutive-run combining, separate Multiple Characters mode, custom-delimiter multiline fields and CLI equivalents; added: row/column first-last ranges and repeating read/skip patterns in the API, CLI and shared Partial Import controls; left: custom range lists and file-name trimming. Item 3 (Column Properties) done (merged 2026-11-13): Format list Numeric/Text/Text & Numeric/Time/Date/Month/Day of Week, Engineering, Engineering SI and Decimal:1000 displays, 19 date and 14 time patterns, 18-character short-name limit; Column Width already supports per-column widths; left: label-row options, 13-character book limit, Err designation. Item 2: Origin's wizard pages measured (2026-11-13, import-ascii.md 16) and Sheaf's wizard given its seven pages in Origin's order and titles; left: the controls of those pages Sheaf does not offer (subheader lines, variable extraction, saved filters, text qualifier, custom range lists).
Text & Numeric columns exist since `text-numeric` (merged 2026-11-13;
`ColumnData::Mixed`). **Size:** M. **Branch:** `import-dialogs`. **Model:** Sonnet.
**Detail:** audit B "Column Properties", "Import Wizard", "ASCII import"; E "Partial" (import).
**Spec:** `docs/spec/import-ascii.md`, `docs/spec/gui-look.md`, `docs/spec/workbook-model.md`.

## Items

1. **ASCII import** (`crates/app/src/dialogs/import.rs`, `crates/io/src/ascii.rs`): add
   multi-character delimiters and Fixed width; "Data Import Options" (rename columns from header,
   rename sheet, apply new range to plots); mode labels as Origin ("New books", "Replace existing
   data"; Sheaf's are in `crates/io/src/batch.rs`); an Apply button. Column ranges/patterns optional.
2. **Import Wizard** (`crates/app/src/dialogs/import_wizard.rs`): first page already matches. Filter
   drop-down adds CH Instruments and Delimited Text (Origin shows `<None>` with no file chosen);
   Target Window "Worksheet" is always enabled; file-name naming options reduced to two ticks. Pages
   2-6 were never seen: get them, then build.
3. **Column Properties** (`crates/app/src/dialogs/column_props.rs`, formats in
   `crates/core/src/format.rs`): display types Text & Numeric (Origin's default), Month, Day of
   Week; Engineering and decimal-with-commas formats; more date patterns (7 against about 20) and
   time patterns (3 against 14); Column Width (per column, already enabled); label-row options; name length limits as
   Origin (13-character book, 18-character column: verify in Origin first). Unverified: Origin may
   show one "Err" designation where Sheaf has separate X error / Y error.

## Done when

Each item done or noted with the reason; screenshots via `--show import:FILE` and `--show column`;
format fixtures against Origin's display of the same numbers; checks pass.
