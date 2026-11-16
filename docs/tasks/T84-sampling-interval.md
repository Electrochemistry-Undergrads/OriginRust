# T84: Set Sampling Interval and Show X Column

**Status:** done 2026-11-16 (Claude Windows "t30-origin", workbook-model.md 13d): both items in the Column menu and the column header menu, measured in Origin 2016 (plots use the interval even with an X column to the left; Show X Column inserts a Disregard column, or an X column and clears the interval; no interval fails), `.sheaf` keeps it, the `.opj` save warning names it. Left: reading Origin's interval from `.opj`, analysis tools using it, the sampling-interval label row.
**Size:** M. **Source:** greyed inventory package B5 (Column > Set Sampling Interval... and Show X Column...,
and the same two items of the column-header context menu).
**Spec:** a new section in `docs/spec/workbook-model.md` ("Sampling interval").

## Goal

A Y column can carry its own X as an initial value and an increment (with an X long name and units), as
Origin's column sampling interval: plots of that column use it, and Show X Column turns it into a real X
column. Set Sampling Interval opens a window with Initial X Value, X Increment, Long Name and Units; Show X
Column (optionally clearing the interval) inserts the X column.

## Origin facts

Origin 2016's X-Functions `colint` (`rng x0(1) inc(1) units lname`; "To clear the sampling interval, set
both Initial X Value and X Increment to 1") and `colshowx` (`rng clear(0)`). Measure in Origin: which X a
plot of the column uses (the interval or the X column to its left), where Show X Column puts the new
column and what it is named, labelled and holds, whether the interval survives, and what clearing does.

## Done when

Both items enabled in both menus and behave as measured (fixtures), the interval is saved in `.sheaf`
(and named in the `.opj` save warning if the writer cannot store it), plots use it, one undo step each,
fmt/clippy/tests pass, inventory and spec updated.
