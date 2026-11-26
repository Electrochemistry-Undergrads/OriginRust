# Import PDF Tables (T463, 2026-11-25; Origin 2021's Import PDF Tables app, Sheaf's own reader, not measured)

Origin 2021 added an app that extracts table cells from a PDF into worksheets. Origin 2016 has none, and Sheaf uses no
PDF library: `crates/io/src/pdf_tables.rs` is a small reader of its own.

## Where

- File > Import > PDF Tables... picks one or more `.pdf` files; each becomes a new workbook named after its file, with
  no import options window and no graph. A PDF dropped on the window, opened with Import Wizard / Single ASCII, or
  re-imported (File > Import > Re-Import Directly) goes the same way; the automatic importer recognises a PDF by its
  `%PDF-` header.
- One worksheet per page that has text, named `Page<n>` after the page's number; pages without text are skipped. The
  import's summary says how many pages were read and how many had text.

## Reading the file

- Objects are found by their `N G obj` headers (the last definition of a number wins), so files with damaged or
  missing cross-reference tables still read; objects inside compressed object streams (`/Type /ObjStm`) are read
  too. Stream filters read: `FlateDecode` and `ASCIIHexDecode`; a page content stream with another filter is
  skipped with a warning.
- Pages come from the catalog's page tree, in order, with resources inherited from parent nodes (all `/Type /Page`
  objects in object order when there is no catalog).
- Text operators run: `BT`, `Tf`, `TL`, `Tc`, `Tw`, `Td`, `TD`, `Tm`, `T*`, `Tj`, `TJ` (one run per `TJ`, its
  numbers as kerning), `'` and `"`, with `q`, `Q` and `cm` for the graphics matrix.
- A string's text comes from the font's `ToUnicode` CMap (`bfchar` and `bfrange`, the code length from the
  codespace range) when it has one; otherwise a simple font's encoding: WinAnsi (Windows-1252), Standard or MacRoman
  as WinAnsi, with `Differences` read through glyph names (single letters, `uniXXXX`, and the common names of digits
  and signs). Composite fonts without a `ToUnicode` map give no text.
- Glyph advances come from the font's `FirstChar` and `Widths` (half an em when missing), plus `Tc`, and `Tw` on a
  space of a one-byte font.
- Encrypted files are refused with a message; a file with no text (a scanned PDF of pictures) is refused saying so.

## The table

- Text runs whose baselines lie within 0.35 of the font size of each other are one line; lines from the top of the
  page down are the rows.
- Runs of a line closer than 0.7 em join into one cell, with a space between them when the gap is wider than 0.15 em.
- A column is a band of the page across which cells of any rows overlap, so left-, right- and centre-aligned columns
  each make one column; cells are placed by their middle. Two cells of a row in one band join with a space.
- A column whose filled cells are all numbers (a Unicode minus read as `-`) is numeric; otherwise it is text (a header
  above numbers keeps its column as text). The first column is X, the others Y.

## Not done

- Pictures of text (OCR), rotated text, cells that span several columns (they merge the columns they span), several
  tables on one page, choosing pages or a region, LZW / ASCII85 / image filters, and Type 3 fonts.
