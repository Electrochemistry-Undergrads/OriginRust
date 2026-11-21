# T312: Text in a numeric column (NonNum) as Origin 2016

**Status:** done 2026-11-21 (Claude Windows "claude:2"), measured.
Feature-parity "Non-numeric text, missing markers" (P0) gap "no choice of treatment (NonNum)". Origin
2016's `impasc options.Miscellaneous.NonNum` measured: 1 keeps the text, 0 and 5 stop at the bad row,
2 puts the rows after it in a new sheet (`data1_`), 3 in new columns from row 1; the bad row is left out.
Sheaf has `AsciiOptions::non_numeric` with the four behaviours, Miscellaneous > "Text in a numeric
column" in the import window and `--non-numeric` on the CLI (also kept in import themes).
**Size:** S. **Spec:** `docs/spec/import-ascii.md` 7. Probe: `re-workspace/scratch/import-nonnum/`.

## Tests

- `ascii::tests::non_numeric_cells_are_treated_as_origin_does` (io),
  `import_text_options::non_numeric_choices_cut_at_the_text_row` (cli).

## Left

- NonNum 4 (new hidden workbook), LeadingZeros; the choice in the Excel window, the wizard and MCP.
