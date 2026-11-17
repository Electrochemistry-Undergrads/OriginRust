# T118: Data menu display range and data markers

**Status:** claimed 2026-11-16 (Claude Windows "t30-origin", branch `data-markers`).
**Size:** M. **Source:** greyed inventory package B3 (graph Data menu: Set Display Range, Reset to Full
Range, Mark Data Range, Clear Data Markers).
**Spec:** `docs/spec/graphing.md` (a new section).

## Goal

The graph Data menu's data markers and display range as Origin's: Mark Data Range puts two markers on
the active plot that the user moves to other points, Set Display Range plots only the rows between
them, Reset to Full Range plots every row again, Clear Data Markers removes the markers.

## Done when

The four items work from the Data menu (Ctrl+Alt+M, Ctrl+Alt+N), the inventory no longer lists them,
and fmt, strict Clippy and the full tests pass.
