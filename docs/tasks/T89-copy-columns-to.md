# T89: Copy Columns to...

**Status:** Copy Columns to landed 2026-11-16 (workbook-model 13g). Ignore Hidden Rows now
defaults to on and compacts the visible source rows. Clearing it copies all stored rows.
System parameter rows and the Origin window layout remain open. Workspace verification pending.
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
