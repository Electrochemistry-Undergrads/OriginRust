# T414: Repeat Last Analysis on All Sheets

**Status:** done 2026-11-24 (Claude Windows "claude:1", gui-look "Repeat Last Analysis"), not measured (Origin 2026
Duplicate Operation). Analysis > Repeat Last Analysis on All Sheets runs the last tool on the Y columns of every sheet
the workbook has when the command starts, as one undo step; sheets without Y columns or where the tool fails are
skipped and named.
**Size:** S. **Source:** HANDOFF C, the rest of T360.

## Tests

- `crates/app/src/app/ui_tests.rs`: `repeat_last_analysis_runs_the_last_tool_on_the_new_selection` (all sheets, one
  undo step).

## Left

- Other workbooks, choosing the sheets, repeating plots and fits.
