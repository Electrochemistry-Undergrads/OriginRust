# T237: Layout pages (B14, slice 1)

**Status:** slice 1 done 2026-11-20 (Claude Windows "claude:2"), **not measured**.
**Size:** L (several slices). **Source:** greyed inventory package B14 ("New: Layout",
"Standard: New Layout").
**Spec:** `docs/spec/layout-page.md`.

## Done in slice 1

- `Window::Layout(LayoutPage)` with graph items in page fractions, and the `Window` match arms
  everywhere: rename, append, duplicate, extract, templates, window files, `.opj` (left out with a
  note), MCP context, Project Explorer and `sheaf info`.
- File > New > Layout and the Standard toolbar's New Layout. `WindowKind::Layout` gives the page a
  menu bar with Layout > Add Graph... (`dialogs/layout_add_graph.rs`). The page view is in
  `layout_view.rs`.
- Tests: `layout_page::tests` (core) and `ui_tests::layout_page`.

## Left (next slices)

- Moving and resizing items with the mouse.
- Add Worksheet....
- Page properties.
- Print and export of the page.
- Writing the page to `.opj`.
