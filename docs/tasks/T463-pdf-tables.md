# T463: Import PDF Tables, slice 1 (Origin 2021 App)

**Status:** done 2026-11-25 (Claude Windows "claude:work", `docs/spec/import-pdf.md`). File > Import > PDF Tables...
reads the text of PDFs into workbooks, one sheet per page that has text, laid out as a table: lines by baseline,
cells by gaps, columns as bands of overlapping cells, numbers where a column holds only numbers. Sheaf's own small PDF
reader handles objects found by their headers (including object streams), Flate streams, the page tree and the text
operators, with ToUnicode maps or simple encodings. Dropped and re-imported PDFs go the same way. No new crates
(flate2 was in the lockfile through zip; tests write PDFs with pdf-writer).
**Size:** M. **Source:** origin-2026-2027-gaps.md, "Import PDF Tables".

## Left

- OCR of scanned pages, rotated text, spanning cells, several tables per page, choosing pages or a region, more
  stream filters, Type 3 fonts, composite fonts without ToUnicode maps.

## Tests

- `crates/io/src/pdf_tables.rs`: a three-column table from a generated PDF, plain and Flate-compressed; numeric
  columns; a ToUnicode CMap; glyph names; a PDF without pages.
- `crates/app/src/app/ui_tests/notes_window.rs`: `a_pdf_table_is_imported_into_a_workbook_without_the_text_dialog`.
