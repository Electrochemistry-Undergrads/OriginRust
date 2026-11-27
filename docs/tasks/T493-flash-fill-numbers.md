# T493: Flash Fill keeps numbers and dates

**Status:** done 2026-11-27 (Claude Windows "claude:3", workbook-model 13r). Left item of T430/T468 (claude:work's,
paused); Origin 2026's Flash Fill, Sheaf's own rules (not measured).
**Size:** S. **Source:** origin-2026-2027-gaps.md, "Flash Fill and Smart Formula": numbers and dates.

## What it does

Flash Fill reads date and time cells as the sheet shows them (they were read as day numbers), and Fill Values keeps
a number or date column as numbers or dates when every result reads as one as typed into it; a Text & Numeric column
takes each result as typed; otherwise the column becomes text, as before.

## Tests

- `crates/core/src/flash_fill.rs`: `numbers_and_dates_stay_numbers_and_dates`.

## Left

- Arithmetic on numbers and dates (a year plus one, a date's month as a number from its value), a number result in
  Keep Formula, suggestions while typing.
