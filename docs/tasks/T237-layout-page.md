# T237: Layout pages (B14, slice 1)

**Status:** slices 1 to 9 done 2026-11-20 (Claude Windows "claude:2"), **not measured**.
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

- Layout > Page Setup... (`dialogs/layout_page_setup.rs`; moved to Format in slice 6): width and height in inches, cm or
  points, paper presets, Portrait/Landscape, one undo step. Tests:
  `layout_page_setup::tests::presets_keep_the_orientation_and_units_convert` and
  `layout_page::page_setup_turns_the_layout_page_to_a4_portrait_in_one_undo_step`.

## Done in slice 5 (2026-11-20)

- Layout > Add Worksheet...: `LayoutContent::Sheet { book, sheet }`, drawn with the worksheet image
  (`sheaf_plot::sheet_image_raster`) on screen and in the page export
  (`render_layout_item`, `layout_item_aspect`). Tests: `layout_page::tests` (core),
  `layout_render::tests` (plot), `layout_page::add_worksheet_puts_a_sheet_picture_on_the_layout_page`.

## Done in slice 6 (2026-11-20)

- The page window moved to Format > Layout Page... [F2] (Origin's place) and has a page colour
  (`LayoutPage::background`, white by default). Test:
  `layout_page::layout_page_properties_turn_the_page_to_a4_portrait_in_one_undo_step`.

## Done in slice 7 (2026-11-20)

- Export Page as Image... also writes SVG and PDF (`sheaf_plot::render_layout_svg`: nested item
  SVGs with their ids prefixed). Tests: `layout_render::tests` and
  `layout_page::a_layout_page_is_exported_as_a_picture_file`.

## Done in slice 8 (2026-11-20)

- Text items: `LayoutContent::Text { text, size }` (`LayoutItem` is no longer `Copy`), Layout > Add
  Text... and double-click editing (`dialogs/layout_text.rs`), drawn from
  `sheaf_plot::layout_text_svg` on screen and in every export. Tests: `layout_page::tests` (core),
  `layout_render::tests` (plot), `layout_page::layout_text_is_added_and_a_double_click_edits_it`.

## Done in slice 9 (2026-11-20)

- EMF pictures: `EMR_STRETCHDIBITS` in `emf.rs` (scene `Item::Image` and `write_emf_picture`);
  `render_layout_emf`; File > Print... on a layout page. Tests: `emf::picture_tests`,
  `layout_render::tests`, `layout_page::file_print_takes_the_active_layout_page`.

## Left (next slices)

- Other page properties (printer page, theme).
- Rich text and drawn objects (lines, rectangles) on the page.
- Writing the page to `.opj`.
