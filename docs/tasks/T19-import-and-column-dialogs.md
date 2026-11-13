# T19: Import dialogs and Column Properties as Origin

**Status:** open; the Import Wizard part needs Origin's pages 2-6 seen first (owner screenshots or a
scripted run). Check branch `text-numeric` (touches `crates/io/src/ascii.rs`, `column.rs`) before
editing those files. **Size:** M. **Branch:** `import-dialogs`. **Model:** Sonnet.
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
   time patterns (3 against 14); Column Width (greyed); label-row options; name length limits as
   Origin (13-character book, 18-character column: verify in Origin first). Unverified: Origin may
   show one "Err" designation where Sheaf has separate X error / Y error.

## Done when

Each item done or noted with the reason; screenshots via `--show import:FILE` and `--show column`;
format fixtures against Origin's display of the same numbers; checks pass.
