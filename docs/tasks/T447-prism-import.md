# T447: Import GraphPad Prism XML files

**Status:** done 2026-11-25 (Claude Windows "claude:gmail", `docs/spec/import-prism.md`). Gaps doc row "GraphPad
Prism connector" (Origin 2026b app), base slice: the import recognises Prism XML (`.pzfx`) files and makes a workbook
with a sheet per data table; row titles, X and every Y data set's replicate or Mean/SD/N subcolumns become columns
that keep their group and role (long name, comments, user parameters); excluded values are masked. No new crates. Not
measured in Origin.
**Size:** S. **Spec:** import-prism.

## Left

- The binary `.prism` and `.pzf` formats, Prism templates, info sheets, analyses and graphs; a structured-table model
  in the workbook itself.

## Tests

- `crates/io/src/pzfx.rs`: `tables_become_sheets_with_groups_replicates_and_masks`, `broken_or_foreign_xml_is_refused`.
- `crates/cli/tests/import_prism.rs`: `a_prism_file_imports_as_a_book_with_a_sheet_per_table`.
