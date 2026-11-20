# T165: File > Batch Processing... (B11 slice)

**Status:** done 2026-11-19 (Claude Windows "acct1"), **not measured**. File > Batch Processing... runs a
saved workbook or analysis template (T160, T163) on a list of data files: one new book per file, the
file's columns in the template's data columns, the template's analysis run again, one undo step.
**Size:** M. **Source:** greyed inventory package B11 (File > Batch Processing... [Workbook, Graph]).
**Spec:** `docs/spec/workbook-model.md` 13p.

## Origin facts

Not measured (Origin COM unavailable on 2026-11-19). Origin's `batchProcess` X-Function runs an
analysis template on files and can collect a summary sheet.

## Left

- Measure Origin's Batch Processing dialog; the summary report sheet, dataset identifiers, saved import
  filters, deleting intermediate books; CLI and MCP access.
