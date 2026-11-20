# T251: Export preview Image and Graph tabs

**Status:** done 2026-11-20 (Claude Windows "claude:3"), the Graph tab's content not measured. The
Export Graphs preview pane gets Origin's Image and Graph tabs: Image is the exported picture, Graph
the whole page with the exported area outlined.
**Size:** S. **Source:** feature-parity "Export preview" (P1).
**Spec:** `docs/spec/graphing.md` 9.7c.

## Tests

- `crates/app/src/dialogs/export.rs`: `the_graph_tab_outlines_the_exported_area_on_the_page`.
- `crates/app/src/app/ui_tests.rs`: `file_export_graphs_opens_the_export_window_and_close_closes_it`
  (the tabs).

## Left

- Compare the Graph tab with Origin's.
