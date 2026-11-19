# T160: User workbook templates (T56/T103 follow-up)

**Status:** done 2026-11-19 (Claude Windows "acct1"), **not measured**. File > Save Template As... on a
workbook saves its sheets, columns, labels, formats and formulas without values as a user workbook
template; File > New > From Template lists the saved workbook templates and makes an empty workbook from
one (one undo step); Plot > User Templates > Manage... renames and deletes them.
**Size:** M. **Source:** greyed inventory (`T56` File > Save Template As... [Workbook]); T103's leftover.
**Spec:** `docs/spec/workbook-model.md` 13o; `docs/spec/graphing.md` 24.

## Goal

The workbook side of Origin's user templates: save a workbook's structure (no data) under a name and
start new workbooks from it.

## Origin facts

Origin 2016 saves workbook templates (`.otw`) from File > Save Template As... and lists them in File >
New > From Template (measured in T103). What an Origin workbook template keeps was not measured (Origin
COM unavailable on 2026-11-19); Sheaf's choices are in the spec section.

## Left

- Measure in Origin: what a workbook template keeps (rows, formulas, formats, data), its default name.
- Save Workbook As Analysis Template (B11); CLI and MCP access to workbook templates.
