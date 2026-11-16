# T97: Export > Worksheet as Image...

**Status:** claimed 2026-11-16 (Claude Windows "t30-origin", branch `wks-image`).
**Size:** S. **Source:** greyed inventory package B12 (File > Export > Worksheet as Image... with a workbook
active).
**Spec:** `docs/spec/workbook-model.md` (a new section).

## Goal

Export the active worksheet as a picture file, as Origin 2016's Worksheet as Image (X-Function `expWks`),
reusing the worksheet drawing of printing (`sheaf_plot::render_sheet_raster` and the PDF/EMF paths).

## Origin facts

Measure in Origin 2016: `expWks`'s options and defaults (image types, which part of the sheet: whole sheet
or selection, label rows, file name and folder), and what the picture holds.

## Done when

The item is enabled and exports as measured (a test reads the written file), fmt/clippy/tests pass,
inventory and spec updated.
