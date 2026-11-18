# T87: Set As Begin, Set As End and Reset to Full Range

**Status:** done 2026-11-16 (Claude Windows "t30-origin", workbook-model.md 13f): measured with `set -b/-e` (cells outside shown blank, plots and statistics use only the rows inside), the five items enabled, the range saved in `.sheaf` and named in the `.opj` save warning, plots leave the rows out. T126/T127 add saved-range support to descriptive Statistics on Columns and Rows; other analysis readers and which columns Origin's menu commands change remain open.
**Size:** M. **Source:** greyed inventory package B5 (Edit > Set As Begin, Set As End, Reset to Full
Range on a workbook, and the data cell's Set as Begin and Set as End).
**Spec:** `docs/spec/workbook-model.md` (a new section, "Column data range").

## Goal

A column can carry a data range (first and last row) as Origin's: Set As Begin and Set As End on a
selected cell set it, Reset to Full Range clears it, and plots (and what Origin's measurements show
uses it) leave the rows outside it out.

## Origin facts

Measure in Origin 2016 from LabTalk: what the commands set (per column or per selection, Origin's
`wks.col.ebegin`/`eend` or the dataset's range), how the sheet shows the rows outside, whether plots and
statistics leave them out, and what Reset to Full Range clears.

## Done when

The five items are enabled and behave as measured (fixtures), the range is saved in `.sheaf` (and named
in the `.opj` save warning if the writer cannot store it), plots use it, one undo step each,
fmt/clippy/tests pass, inventory and spec updated.
