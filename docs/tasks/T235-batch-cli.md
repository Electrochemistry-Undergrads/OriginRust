# T235: `sheaf batch` (Batch Processing without a window)

**Status:** done 2026-11-20 (Claude Windows "claude:3"). `sheaf batch FILES... -t TEMPLATE` runs a saved
workbook or analysis template (`.sheafbooktemplate`) on data files as File > Batch Processing... does
(T165), with the summary of T233 (`--summary SHEET`, `--identifier NAME`, `--delete-books`), and saves
the project (`-o`, `.sheaf` or `.opj`, `--append`) and/or writes the summary as delimited text
(`--summary-csv`). The run is `Project::batch_process`, which the window now uses too.
**Size:** S. **Source:** T165 and T233 "Left" (CLI access).
**Spec:** `docs/spec/workbook-model.md` 13p.

## Behaviour

- Files are read with the usual read options (`--format`, ...); a file that cannot be read is skipped
  with a warning; none read is an error.
- A `--summary` sheet the template does not have is an error before anything is read or written; it
  names the template's sheets. With neither `-o` nor `--summary-csv` there is nothing to write: error.
- Prints one line per book (its name, the file, the columns filled), notes for left-out columns and
  analysis problems, and the summary's name and rows.

## Tests

- `crates/cli/tests/batch.rs`: books and their analysis runs saved, an unreadable file skipped; the
  summary written as CSV with the books deleted; an unknown sheet and a missing output refused.
- The window's tests (`crates/app/src/app/ui_tests/user_templates.rs`) run through the same
  `Project::batch_process`.

## Left

- A template given by name from the template folder. (MCP access: T238.)
