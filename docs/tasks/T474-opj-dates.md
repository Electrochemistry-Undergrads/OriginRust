# T474: Date and time columns in .opj files

**Status:** done 2026-11-26 (Claude Windows "claude:3", opj-format.md 4.3 `0x1E`, measured in Origin 2016).
**Size:** S. **Source:** feature-parity "Open .opj" gap "dates read as day numbers".

## What it does

A column with Origin's Date, Time, Month or Day of Week display opens with Sheaf's matching format (the 20 Date, 17 Time and 3 + 3
name subformats, in the order of workbook-model.md), so its day numbers show as dates; saving writes those formats back. The
column record keeps the kind at `0x1E` (3 Date, 2 Time, 4 Month, 5 Day of Week) and the subformat minus 1 at `0x1F`, over Origin's
Numeric cells. A Sheaf date pattern that is not in Origin's list keeps its numbers with the display-format warning. The "day
numbers, not formatted dates" warning now names only the columns whose display is not known.

## Measured

`re-workspace/scratch/t474/d1.opj`: 43 columns, `wks.colN.format` 4/3/5/6 with every subformat. `sheaf-dates.opj` (written by
the ignored test `write_dates_project_for_origin`) opened in Origin with the formats and subformats written and the texts of
workbook-model.md's lists.

## Tests

- `crates/io/src/opj/write/feature_tests.rs`: `date_and_time_displays_are_written_and_read_back`,
  `a_date_column_record_holds_type_and_subformat`.
- `crates/io/src/opj/tests.rs`: `local_origin_file_gives_date_and_time_displays` (Origin's own file, local only).
