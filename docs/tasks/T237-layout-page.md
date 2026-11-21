# T237: Layout pages (B14, slice 1)

**Status:** slices 1 to 4 done 2026-11-20 (Claude Windows "claude:2"), **not measured**.
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

## Done in slice 2 (2026-11-20)

- Click to select, drag to move, corner handle to resize, and Delete; one undo step each
  (`layout_view.rs`, `Action::LayoutSetItem` and `Action::LayoutDeleteItem`). Test:
  `layout_page::layout_items_are_moved_resized_and_deleted_with_the_mouse`.

## Done in slice 3 (2026-11-20)

- Layout > Export Page as Image... at 300 dpi (PNG, JPEG, TIFF, BMP): `layout_render.rs` in
  sheaf-plot composes the graph pictures on a white page. Tests:
  `layout_render::tests::graph_pictures_land_at_their_places_on_a_white_page` and
  `layout_page::a_layout_page_is_exported_as_a_picture_file`.

## Done in slice 4 (2026-11-20)

- Layout > Page Setup... (`dialogs/layout_page_setup.rs`): width and height in inches, cm or
  points, paper presets, Portrait/Landscape, one undo step. Tests:
  `layout_page_setup::tests::presets_keep_the_orientation_and_units_convert` and
  `layout_page::page_setup_turns_the_layout_page_to_a4_portrait_in_one_undo_step`.

## Left (next slices)

- Add Worksheet....
- Other page properties (background, printer page).
- Print, and vector (SVG/PDF/EMF) export of the page.
- Writing the page to `.opj`.
