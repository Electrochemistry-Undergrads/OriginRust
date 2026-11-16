# T97: Export > Worksheet as Image...

**Status:** done 2026-11-16 (Claude Windows "t30-origin", workbook-model.md 13k): `expWks` options and picture measured; Sheaf exports the active sheet as PNG/PDF/SVG in Origin's look (label rows and cells, no heads, numbers or grid). Left: other image types, several sheets, size and overwrite options.
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
