# T74: Fill Column With patterns (A set of Numbers, Date/Time values, Text & Numeric)

**Status:** done 2026-11-16 (Claude Windows "t30-origin", branch `b5-fill`; workbook-model.md 13c): the three items in the Column menu and the column-header menu, measured against Origin 2016's patternN/D/T and copied (month 31 days, year 365 days, random with replacement, rows below kept). Left: Origin's window layout and its text quoting.
**Size:** M. **Source:** greyed inventory package B5 (Column > Fill Column With and the column-header
context menu: A set of Numbers..., A set of Date/Time values..., Arbitrary set of Text&Numeric values...).
Row Numbers and the random fills are T63's, not this brief's.
**Spec:** a new section in `docs/spec/workbook-model.md` ("Fill Column With patterns").

## Goal

The three items open a window each and fill the selected column(s) with a generated pattern, one undo
step: numbers from/to/increment with per-value and per-sequence repeats or a random order of a set of a
given size; dates or times from/to by an increment in a unit; text items (space separated, quotes for
items with spaces) repeated or random. CLI/MCP are out of scope.

## Origin facts

Origin 2016's X-Functions `patternN`, `patternD`, `patternT` (variables read with `tools/origin-ref/xf-args.py`):
`patternN irng from(1) to(10) inc(1) mode(repeat|random) onerepeat(1) seqrepeat(1) size(10)`;
`patternD irng format(date|time) display custom from to tfrom tto inc(1) unit(sec..year, default day)
mode`; `patternT irng text hint mode onerepeat seqrepeat size`. Measure their output in Origin (row counts,
repeat order, random mode, date units, an existing column's rows) and copy it.

## Done when

The three items are enabled in both menus, fill as Origin does on the measured cases (fixtures), undo in
one step, fmt/clippy/tests pass, the greyed inventory and spec are updated.
