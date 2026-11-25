# Importing GraphPad Prism XML files (`.pzfx`)

T447. Origin 2026b's GraphPad Prism connector app imports Prism data into workbooks; Origin 2016 has no such reader,
so this is Sheaf's own and not measured in Origin. Sheaf reads Prism's XML format (`.pzfx`, any extension: the file is
recognised by a `<GraphPadPrismFile` root near its start) through the ordinary import (`sheaf import FILE`, File >
Import, drag and drop; `sheaf_io::pzfx`). The binary `.prism` and `.pzf` files are not read.

## 1. What is read

- The `GraphPadPrismFile` root's `Table` and `HugeTable` elements, in file order; each becomes a sheet named after its
  `Title` (the long name too; a repeated title gets ` (2)`, ` (3)`), and the import's summary lists the sheets with
  their `TableType` (XY, OneWay, TwoWay, Survival, ...).
- `RowTitlesColumn`: a Label column "Row title" when any title is not empty.
- `XColumn` (or `XAdvancedColumn`): an X column named by its `Title` ("X" when empty); a second subcolumn (X error) is an
  X error column; non-numeric X values (dates written as text) make a text column.
- Each `YColumn`: one column per `Subcolumn`, long name the data set's `Title` ("Data set k" when empty), comments the
  replicate or role, and user parameters `Group` (the title) and `Role`. Roles follow the table's `YFormat`:
  `replicates` gives "Replicate 1", "Replicate 2", ... ("Value" for a single subcolumn); `SDN` Mean, SD, N; `SEN` Mean,
  SEM, N; `CVN` Mean, %CV, N; `SD`, `SE`, `CV` without N; `low-high` Mean, Plus error, Minus error;
  `upper-lower-limits` Mean, Upper limit, Lower limit. SD, SEM, %CV and Plus error columns are Y error columns, the rest
  Y. A format with a different number of subcolumns falls back to replicates.
- Cells: each `d` element is a row; an empty `d` is a missing value, a value that is not a number is left empty with a
  warning, and `Excluded="1"` keeps the value and masks the cell (Prism leaves it out of its analyses, Sheaf's masked
  cells are left out of fits and statistics).
- XML: elements, attributes in either quotes, text, the five named entities and numeric character references,
  comments, CDATA, processing instructions and a DOCTYPE. A malformed file or another root is refused.

## 2. What is not read

Prism's info sheets, analyses, graphs, layouts and notes; column and table formatting (widths, decimals); the binary
formats; templates. The workbook is a plain workbook whose columns carry the group and role in their comments and
user parameters (no separate structured-table model).

## 3. Tests

- `crates/io/src/pzfx.rs`: two tables (XY with row titles, replicates, an excluded and an empty cell; a column table
  in SDN format), entities, broken and foreign XML.
- `crates/cli/tests/import_prism.rs`: `sheaf import` of a `.pzfx` saved and loaded back.
