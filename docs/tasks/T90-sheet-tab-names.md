# T90: Duplicate Without Data and Sheet Name to Show

**Status:** claimed 2026-11-16 (Claude Windows "t30-origin", branch `sheet-tab`).
**Size:** M. **Source:** greyed inventory package B6 (the sheet tab's Duplicate Without Data and Sheet
Name to Show > Short Name, Long Name, Both).
**Spec:** `docs/spec/workbook-model.md` (a new section).

## Goal

A worksheet gets a long name (as Origin's sheets have, next to the short name on the tab); the sheet tab
shows the short name, the long name or both as Origin's Sheet Name to Show; Duplicate Without Data adds a
copy of the sheet's structure without its values.

## Origin facts

Measure in Origin 2016: what Duplicate Without Data keeps (columns, names, labels, designations, formats,
formulas, row count) and where the copy goes and is named; what the three Sheet Name to Show choices
draw on the tab (and whether the choice is per sheet or per book) and how a sheet gets its long name.

## Done when

The four items are enabled and behave as measured, the long name and the choice are saved in `.sheaf`
(named in the `.opj` save warning if the writer cannot store them), one undo step each, fmt/clippy/tests
pass, inventory and spec updated.
