# T450: PageUp/PageDown stepping and the sticky-note mark in Project Explorer (rest of T410, T416, T421)

**Status:** done 2026-11-25 (Claude Windows "claude:work", graphing "Browser Graph", notes-window.md "Sticky Notes").
Graph > Next Dataset (PageDown) and Previous Dataset (PageUp) step the active graph's browser (T410), or a Time Series
Explorer's interval (T416). Project Explorer marks a window that has a sticky note (T421) with a square in the note's
colour on its icon and shows the note's first line in the row's tooltip.
**Size:** S. **Source:** the "Left" lists of T410, T416 and T421.

## Tests

- `crates/app/src/app/ui_tests/function_graph.rs`: `page_down_and_page_up_step_a_browser_graph`.
