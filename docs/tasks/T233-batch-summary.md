# T233: Batch Processing summary sheet (parity "Summary table across sheets")

**Status:** done 2026-11-20 (Claude Windows "claude:3"), **not measured** (Origin's Batch Processing
dialog was not opened). File > Batch Processing... (T165) gets Summary Sheet (a sheet of the chosen
template, or none), Dataset Identifier and Delete Intermediate Books: after the books are made, a new
"Batch Summary" book stacks the rows of that sheet of every book, each led by its file name, in the
same undo step; the books made can be deleted once the summary holds their rows.
**Size:** S. **Source:** feature-parity "Summary table across sheets" (batch summary); T165's "Left".
**Spec:** `docs/spec/workbook-model.md` 13p.

## Tests

- `crates/core/src/book_template.rs`: `the_batch_summary_stacks_one_sheet_of_every_book_led_by_its_identifier`
  (rows of two books, units, designation, text cells, an Auto formula column up to date, no sheet).
- `crates/app/src/app/ui_tests/user_templates.rs`:
  `batch_processing_collects_a_summary_sheet_and_can_delete_the_books_it_made` (the window's fields,
  the analysis results in the summary, deleted books and their runs, one undo step, the default
  identifier).

## Left

- Measure Origin's dialog: its summary layout (one row per file or every row), the identifier choices
  (file name, path, a column of the file), Origin's Minimize and Hide choices for intermediate books.
- Summaries of several sheets, a summary of report sheets, MCP access (the CLI: `sheaf batch`, T235).
