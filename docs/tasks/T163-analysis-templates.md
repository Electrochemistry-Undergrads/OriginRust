# T163: Save Workbook As Analysis Template (B11 slice)

**Status:** done 2026-11-19 (Claude Windows "acct1"), **not measured**. File > Save Workbook As Analysis
Template... saves a T160 workbook template that also keeps the book's recalculating analysis runs; a book
made from it with File > New > From Template runs them again when data arrives.
**Size:** M. **Source:** greyed inventory package B11 (File > Save Workbook As Analysis Template... [Workbook]).
**Spec:** `docs/spec/workbook-model.md` 13o.

## Goal

Origin's analysis templates: a workbook with its analysis operations, reused on new data.

## Origin facts

Not measured: OriginLab's Analysis Template help pages returned 404 on 2026-11-19 and Origin COM was
unavailable. T103 measured that File > New > From Template lists workbook templates such as
`Assays.ogw` (an analysis template).

## Left

- Measure in Origin: the save dialog's options, whether data is kept, how operations recalculate on
  import, the `.ogw` folder.
- Batch Processing with an analysis template (B11), CLI and MCP access.
