# T315: Two-digit years in text import, as Origin 2016

**Status:** done 2026-11-21 (Claude Windows "claude:2"), measured.
Feature-parity "Date and time detection" (P0) gap "no two-digit years". Origin 2016's `impasc` reads
M/d/yy as a date column shown as M/d/yyyy, with 00 to 44 in the 2000s and 45 to 99 in the 1900s; it
leaves d/M/yy, dd.MM.yy, yy-MM-dd and dd-MMM-yy as text. Sheaf reads a two-digit year in every
year-last form it already knew (with two-digit day and month after dots and dashes) using that window.
**Size:** S. **Spec:** `docs/spec/import-ascii.md` 8. Probe: `re-workspace/scratch/import-yy/`.

## Tests

- `datetime::tests::two_digit_years_use_origins_window` (core),
  `ascii::tests::two_digit_years_make_a_date_column` (io).

## Left

- Whether Origin's window slides with the current year; custom date patterns and a per-column choice.
