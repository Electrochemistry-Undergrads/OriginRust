# T179: Matrix books, heat maps, contour plots and `sheaf matrix` (B14, slice 1)

**Status:** slices 1 to 3 done 2026-11-19 (Claude Windows "obj-align"). It ports the matrix branch of the
offline session originrust-3a (`worktree-agent-a79f85b1afb5f0034`, commits 7994c3b3..8c21175b,
2026-11-12, never merged) onto current main, with the original authorship kept. Its behaviour was
measured on Origin 2016 (`docs/spec/matrices.md`, fixtures in `crates/core/tests/data/origin_matrix.json`).
**Size:** L. **Source:** greyed inventory package B14.
**Spec:** `docs/spec/matrices.md`; `docs/spec/project-file-format.md` (matrix members).

## Done in slice 1

- Core: `Window::Matrix(MatrixBook)` with sheets of Z values and their X/Y ranges, operations
  (`MatrixOp`: set values, transpose, flip, rotate, expand, shrink, statistics), worksheet <-> matrix
  conversions (`matrix_convert`) and XYZ gridding of regular data. Every `Window` match now has a
  matrix arm: rename, append, duplicate, extract, templates, window files (none), MCP context.
- Files: `.sheaf` stores matrix sheets as `data/m<id>.f64` members. `.opj` leaves matrices out with a note.
- Plot: `Layer::field` (`FieldPlot`) draws contour colour fill, contour lines with labels or heat-map
  cells with a colour scale, from a matrix sheet or XYZ columns. Scattered XYZ data is triangulated for
  contours. The goldens were regenerated for main's newer default page size: fills, texts and element
  counts are unchanged, and only the geometry scales.
- CLI: `sheaf matrix new|convert|set|transform|to-sheet|stats|plot`, with the arguments now in
  `sheaf_ops::args`. `sheaf info` lists matrix books. The old `--first-column` of `matrix convert` is
  `--coords-in-first-column`, because the import options now own `--first-column`.

## Done in slice 2 (2026-11-19)

- The app: `crates/app/src/matrix_view.rs` draws the matrix window. It has a grid of the active sheet,
  editing on double click (one undo step) and sheet tabs. File > New > Matrix... and the Standard
  toolbar's New Matrix make a book. See matrices spec 7a. Test: `app/ui_tests/matrix_window.rs`.

## Done in slice 3 (2026-11-19)

- The matrix menu bar (`WindowKind::Matrix`) with the Matrix menu and Plot > Contour/Heat Map, and
  the same three plot items in the workbook Plot menu for X, Y and Z columns. See matrices spec 7b.
  The windows are in `dialogs/matrix_ops.rs`. Test:
  `matrix_window::the_matrix_menu_transforms_sets_values_converts_and_plots_the_active_sheet`.

## Left (next slices)

- Matrix Statistics in the app, X/Y coordinate headers, selection and copy, Convert to Matrix in the
  Worksheet menu, Gray Scale Map, and polar and ternary contours.
- Convert to Matrix in the Worksheet menu, profiles, image windows and 2D FFT.
- Writing matrices to `.opj`.
