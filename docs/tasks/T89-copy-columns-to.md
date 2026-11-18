# T89: Copy Columns to...

**Status:** done 2026-11-16 (Claude Windows "t30-origin", workbook-model.md 13g): `colcopy` measured (new book renamed from A with labels, format and designation but no formula; into a column overwrites rightward; `<new>` appends; `<new>!<new>` adds a sheet), window with destination and check boxes, one undo step. Left: Ignore Hidden Rows, system Parameters rows, Origin's window layout.
**Size:** S. **Source:** greyed inventory package B5 (the column header's Copy Columns to...).
**Spec:** `docs/spec/workbook-model.md` (a new section).

## Goal

Copy Columns to... copies the selected columns into another (or a new) worksheet, as Origin 2016's
`colcopy`, with the parts Origin copies (data, labels, format, formula).

## Origin facts

Measure in Origin 2016 from LabTalk (`colcopy` and its options): where the copies go (destination sheet,
position, a new book), what they carry (long name, units, comments, designation, format, formula, user
parameters), and what happens to columns of the same name at the destination.

## Done when

The item is enabled and behaves as measured (fixtures), one undo step, fmt/clippy/tests pass, inventory
and spec updated.
