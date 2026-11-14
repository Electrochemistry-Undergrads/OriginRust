# T07: Worksheet export and clipboard as Origin

**Status:** done, merged (`826f390`) (export measured and matched, export window added); clipboard text separators/quotes/line breaks now round-trip (workbook-model 13.4). Origin clipboard defaults remain unmeasured. **Size:** S. **Branch:** `sheet-export`. **Model:** Sonnet.
**Detail:** audit E "Different" (export paragraph) and "Partial" (clipboard).
**Spec:** `docs/spec/import-ascii.md` (export, search "expASC"), `docs/spec/workbook-model.md`.

## Today

- `crates/io/src/export.rs` `write_delimited`: missing values as an empty field; numbers in General,
  Decimal, Scientific, Significant columns as shortest round-trip text, ignoring the column format
  (dates/times are written as shown); user parameter rows left out; active sheet only; delimiter
  from the file extension; no options window. Called from `app.rs` (search "export" near the File
  menu handler).
- Clipboard (`crates/core/src/clipboard.rs`) copies displayed text (5 decimals for General) and
  includes parameter rows in "copy with label rows". So export and copy disagree.
- General format: `NumberFormat::format` doc says 9 significant digits (`crates/core/src/format.rs`),
  but grid and copy use `shown_text` in `column.rs` (5 decimals between 1e-4 and 1e15, else 5
  significant digits).

## Origin facts

- Origin's default text for a missing value in export is `--` (wm s8 in the spec; **not run in
  Origin**: measure with `expASC` via `tools/origin-ref/run-labtalk.ps1` before changing).
- Export options in Origin (expASC): which label rows, delimiter, numeric display "as displayed" or
  full precision, whole book or one sheet. Measure the defaults.

## Steps

1. Measure expASC defaults on a sheet with a missing value, a 7-digit number in General, a Decimal:2
   column and a parameter row. Record in import-ascii.md.
2. Make `write_delimited` follow those defaults; add an options struct field per measured choice; a
   small options window in the app (Origin's layout from the spec).
3. Fix the `NumberFormat::format` doc or code so the 9-vs-5 digit statement is true.

## Done when

Export of the probe sheet equals Origin's file byte for byte (except line endings if Origin differs
by OS); checks pass.

## Result (branch `sheet-export`)

Origin's `expASC` defaults are in `docs/spec/import-ascii.md` section 13.1 (raw files in
`re-workspace/scratch/sheet-export/`). Export now writes `--` for missing values, 15-digit full precision
(`format_full_precision` in `crates/core/src/format.rs`), CR LF lines, Origin's label-row order, quoting
and separators; the File > Export Worksheet window (`dialogs/export_sheet.rs`) sets the options. Not done:
Copy (clipboard) writes the shortest round-trip text for "full precision" (kept so a paste reads back
the same bits) and its label-row selection/order is unchanged, because Origin's own Copy was not measured; the `.csv`
whole-book export and Origin's `select`/`addxcol` options are not implemented.

Clipboard follow-up (2026-11-14): Copy/Cut now quote text fields so commas in one column do not
paste into extra columns, and tabs, line breaks and quotes survive instead of being flattened.
Paste detects separators outside quoted fields and handles quoted one-column/multiline text.
Core regressions and new/imported-table UI coverage exercise the complete round trip and undo.
See workbook-model.md 13.4 for limits; this is a Sheaf correctness fix, not measured Origin parity.
