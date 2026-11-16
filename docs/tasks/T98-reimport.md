# T98: Re-Import Directly and Re-Import...

**Status:** done 2026-11-16 (Claude Windows "t30-origin", workbook-model.md 13l): `reimport` measured (values replaced, columns kept, user labels kept, graphs follow); both items read the source again keeping column ids. Left: stored import options, Selected Range Only.
**Size:** S. **Source:** greyed inventory package B12 (File > Import > Re-Import Directly (Ctrl+4) and
Re-Import... with a workbook active).
**Spec:** `docs/spec/workbook-model.md` (a new section).

## Goal

Read a workbook's source file again (`Workbook::source`: path and importer) and replace its data, as
Origin 2016's `reimport` (`dialog:=direct` or `options`), keeping the columns (and so the graphs that
plot them) when the file still has them.

## Origin facts

Measure in Origin 2016 with `reimport`: what happens to the book's values, column names and labels, row
count, added or removed columns, the graphs of the book, and a missing source file.

## Done when

Both items are enabled and behave as measured (fixtures), one undo step, fmt/clippy/tests pass, inventory
and spec updated.
