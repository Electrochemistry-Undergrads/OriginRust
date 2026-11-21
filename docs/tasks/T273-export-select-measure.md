# T273: Export of the selected cells measured in Origin

**Status:** done 2026-11-21 (Claude Windows "claude:3"). HANDOFF next candidates B. Origin 2016's
`expASC select:=1` after `worksheet -s` (`re-workspace/scratch/t273`, local) writes the label rows of
the selected columns, the selected rows, and with `rowlabel:=1` the sheet's row numbers: exactly what
T247's `write_selection` writes (byte for byte for a block). Origin's extra empty field at the end of
the Long Name line for a whole selected column is not copied.
**Size:** S. **Spec:** `docs/spec/import-ascii.md` 13.1.

## Tests

- `crates/io/src/export.rs`: `a_selection_writes_what_origin_writes`.
