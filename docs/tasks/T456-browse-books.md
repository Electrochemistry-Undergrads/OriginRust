# T456: Browser Graph across workbooks (rest of T410)

**Status:** done 2026-11-25 (Claude Windows "claude:work", graphing "Browser Graph"). The Browser Graph window offers
a third choice: the same column of the same sheet in every workbook (by sheet name, else position; by column name,
else position). The caption names the book.
**Size:** S. **Source:** T410 "Left".

## Tests

- `crates/core/src/browser.rs`: `a_browser_by_sheet_takes_the_same_column_of_every_sheet` (the across-books part).
